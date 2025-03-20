use std::{process::ExitStatus, sync::Arc, time::Duration};

use crate::{
    hydracore::{evaluator::nix::drv::DependencyTree, DBError},
    models::{Job, JobDiff, JobState, Jobset, JobsetDiff, JobsetState},
    routes::jobset::trigger_jobset,
    state::State,
};

use super::{
    super::db::DB,
    nix::{
        build::{BuildManager, BuildResult},
        drv::DrvBasic,
        eval::{Evaluation, EvaluationError},
    },
    notifications::EvalDoneNotification,
};

use crate::models::Project;

use chrono::{DateTime, Utc};
use tokio::{
    sync::{
        mpsc::{self, unbounded_channel, Receiver, Sender, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    time::Sleep,
};
use tracing::{debug, error, info, trace};

struct CoordinatorData {
    db: Arc<Mutex<DB>>,
    build_manager: Arc<Mutex<BuildManager>>,
}

impl CoordinatorData {
    pub fn new(db: DB, build_manager: BuildManager) -> Self {
        CoordinatorData {
            db: Arc::new(Mutex::new(db)),
            build_manager: Arc::new(Mutex::new(build_manager)),
        }
    }
}

pub struct Coordinator {
    data: Arc<Mutex<CoordinatorData>>,
    eval_tx: Arc<UnboundedSender<EvalDoneNotification>>,
}

impl Coordinator {
    pub fn new(db: DB) -> Self {
        let (build_tx, build_rx) = unbounded_channel::<BuildResult>();
        let data = Arc::new(Mutex::new(CoordinatorData::new(
            db,
            BuildManager::new(build_tx, 2),
        )));

        let (eval_tx, eval_rx) = unbounded_channel::<EvalDoneNotification>();

        let eval_data = data.clone();

        let _handle = tokio::spawn(async {
            Coordinator::on_eval_done(eval_rx, eval_data).await;
        });

        let build_data = data.clone();

        tokio::spawn(async move {
            Coordinator::on_build_done(build_rx, build_data).await;
        });

        Coordinator {
            data,
            eval_tx: Arc::new(eval_tx),
        }
    }

    pub async fn start_jobsets_timer(&self, state: Arc<State>) -> Result<(), DBError> {
        let db = self.get_db().await;
        let locked_db = db.lock().await;
        let projects = Project::get_all(&*locked_db).await?;

        for project in projects.iter() {
            let mut jobsets = Jobset::get_all(&*locked_db, project.id.unwrap()).await?;

            while let Some(jobset) = jobsets.pop() {
                if jobset.check_interval == 0 {
                    debug!(
                        "Disabling jobset timer for jobset {} because its 0",
                        jobset.name
                    );
                    continue;
                }
                Coordinator::start_jobset_timer(state.clone(), jobset);
            }
        }
        Ok(())
    }

    pub fn start_jobset_timer(state: Arc<State>, mut jobset: Jobset) {
        info!("Started jobset timer for {}", jobset.name);
        tokio::spawn(async move {
            loop {
                trace!(
                    "[Jobset timer: {}] Sleeping {} seconds",
                    jobset.name,
                    jobset.check_interval
                );
                _ = tokio::time::sleep(Duration::from_secs(jobset.check_interval as u64)).await;
                trace!("[Jobset timer: {}] Triggering jobset", jobset.name);

                let result = state
                    .clone()
                    .coordinator
                    .lock()
                    .await
                    .schedule_jobset(&mut jobset)
                    .await;

                if result.is_err() {
                    error!("Failed to trigger jobset: {}", result.err().unwrap());
                }
            }
        });
    }

    pub async fn get_db(&self) -> Arc<Mutex<DB>> {
        let locked = self.data.lock().await;
        locked.db.clone()
    }

    pub async fn schedule_jobset(&mut self, jobset: &mut Jobset) -> Result<(), EvaluationError> {
        if jobset.state == Some(JobsetState::Evaluating) {
            return Err(EvaluationError::new(
                "Evaluation already running".to_string(),
            ));
        }
        jobset
            .update_state(
                &*self.data.lock().await.db.lock().await,
                JobsetState::Evaluating,
            )
            .await
            .map_err(|e| EvaluationError::new(format!("DBError: {}", e.to_string())))?;

        _ = Evaluation::new(self.eval_tx.clone(), &jobset).await?;

        Ok(())
    }

    async fn on_eval_done(
        mut receiver: UnboundedReceiver<EvalDoneNotification>,
        data: Arc<Mutex<CoordinatorData>>,
    ) {
        while let Some(mut notification) = receiver.recv().await {
            info!("Received new evaluation notification");
            trace!("[lock] Attempting to get lock on data!");
            let locked = data.lock().await;
            trace!("[lock] Got lock on data!");

            let mut diff = JobsetDiff::new();
            diff.set_state(JobsetState::EvalFailed);

            let start = notification.get_started();
            let end = notification.get_finished();

            let duration = end - start;

            diff.set_last_checked(Utc::now());
            diff.set_last_evaluated(Utc::now());
            diff.set_evaluation_took(duration.num_seconds() as i32);

            trace!("[lock] Attempting to get lock on db");
            let db = locked.db.lock().await;
            trace!("[lock] Got lock on db!");

            let jobset = Jobset::get_single(&db, notification.jobset_id()).await;

            if jobset.is_err() {
                error!(
                    "Failed to get jobset from db: {}",
                    jobset.err().unwrap().to_string()
                );
                continue;
            }

            let jobset = jobset.unwrap();

            if jobset.is_none() {
                error!("Failed to find jobset!");
                continue;
            }

            let mut jobset = jobset.unwrap();

            if !notification.is_successful() {
                diff.set_error_message(notification.get_err().unwrap().to_string());

                let result = jobset.update_jobset(&db, diff).await;

                if result.is_err() {
                    error!(
                        "Failed to update jobset: {}",
                        result.err().unwrap().to_string()
                    );
                    continue;
                }

                continue;
            }

            diff.set_state(JobsetState::Idle);

            let result = jobset.update_jobset(&db, diff).await;

            if result.is_err() {
                error!(
                    "Failed to update jobset: {}",
                    result.err().unwrap().to_string()
                );
                continue;
            }

            let mut evaluation = crate::models::Evaluation::new(jobset.id.unwrap());

            let result = evaluation.add_to_db(&db).await;

            if result.is_err() {
                error!(
                    "Failed to add evaluation: {}",
                    result.err().unwrap().to_string()
                );
                continue;
            }

            let mut jobs = notification.get_jobs_copy().unwrap();

            for job in jobs.iter_mut() {
                let result = DrvBasic::get_derivation(&job.derivation_path).await;
                if result.is_err() {
                    error!("Failed to get derivation path: {}", result.err().unwrap());
                    continue;
                }

                let result = result.unwrap();

                job.derivation_path = result.drv_path;

                if job.attribute_name == "" {
                    job.attribute_name = result.name;
                }
            }

            for job in jobs.iter_mut() {
                let result = job.add_to_db(&db).await;
                if result.is_err() {
                    error!("Failed to add derivation to db!");
                    continue;
                }

                let mut diff = JobDiff::new();
                diff.state = Some(JobState::Building);
                let result = job.update_job(&*db, diff).await;

                if result.is_err() {
                    error!("Failed to update job: {}", result.err().unwrap());
                    continue;
                }

                locked
                    .build_manager
                    .lock()
                    .await
                    .queue(job.derivation_path.clone(), job.id.unwrap())
                    .await;
            }
        }
    }

    async fn on_build_done(
        mut reciever: UnboundedReceiver<BuildResult>,
        data: Arc<Mutex<CoordinatorData>>,
    ) {
        info!("Waiting  for build_done messages");
        while let Some(message) = reciever.recv().await {
            info!("Build done: {}", message.path);

            trace!("[lock] Attempts to get data lock");
            let locked = data.lock().await;
            trace!("[lock] Got data lock");

            trace!("[lock] Attempts to get db lock");
            {
                let db = locked.db.lock().await;
                let job = Job::get_single(&*db, message.id).await;

                if job.is_err() {
                    error!("Failed to get job: {}", job.err().unwrap());
                    continue;
                }

                let job = job.unwrap();
                if job.is_none() {
                    error!("Failed to find job!");
                    continue;
                }
                let mut job = job.unwrap();

                let mut diff = JobDiff::new();
                diff.state = Some(JobState::Successful);
                diff.finished = Some(Utc::now());
                diff.took = Some(message.took_secs);

                let result = job.update_job(&*db, diff).await;

                if result.is_err() {
                    error!("Failed to update job: {}", result.err().unwrap());
                    continue;
                }
            }
            trace!("[lock] Released db lock");
        }
    }
}
