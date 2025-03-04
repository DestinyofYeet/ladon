use std::{process::ExitStatus, sync::Arc};

use crate::models::{Derivation, Jobset, JobsetDiff, JobsetState};

use super::{
    super::db::DB,
    nix::drv::Drv,
    nix::eval::{Evaluation, EvaluationError},
    notifications::EvalDoneNotification,
};

use crate::models::Project;

use chrono::{DateTime, Utc};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};
use tracing::{error, info, trace};

struct CoordinatorData {
    db: Arc<Mutex<DB>>,
}

impl CoordinatorData {
    pub fn new(db: DB) -> Self {
        CoordinatorData {
            db: Arc::new(Mutex::new(db)),
        }
    }
}

pub struct Coordinator {
    data: Arc<Mutex<CoordinatorData>>,
    eval_tx: Arc<Sender<EvalDoneNotification>>,
}

impl Coordinator {
    pub fn new(db: DB) -> Self {
        let data = Arc::new(Mutex::new(CoordinatorData::new(db)));

        let (eval_tx, eval_rx) = mpsc::channel::<EvalDoneNotification>(1);

        let eval_data = data.clone();

        let _handle = tokio::spawn(async {
            Coordinator::on_eval_done(eval_rx, eval_data).await;
        });

        Coordinator {
            data,
            eval_tx: Arc::new(eval_tx),
        }
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
        mut receiver: Receiver<EvalDoneNotification>,
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
                return;
            }

            let jobset = jobset.unwrap();

            if jobset.is_none() {
                error!("Failed to find jobset!");
                return;
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
                    return;
                }

                return;
            }

            diff.set_state(JobsetState::Idle);

            let result = jobset.update_jobset(&db, diff).await;

            if result.is_err() {
                error!(
                    "Failed to update jobset: {}",
                    result.err().unwrap().to_string()
                );
                return;
            }

            let mut evaluation = crate::models::Evaluation::new(jobset.id.unwrap());

            let result = evaluation.add_to_db(&db).await;

            if result.is_err() {
                error!(
                    "Failed to add evaluation: {}",
                    result.err().unwrap().to_string()
                );
                return;
            }

            let mut derivations = notification.get_derivations_copy().unwrap();

            for eval in derivations.iter_mut() {
                let result = Drv::get_derivation(&eval.derivation_path).await;
                if result.is_err() {
                    error!("Failed to get derivation path!");
                    return;
                }

                let result = result.unwrap();

                eval.derivation_path = result;
            }

            for derivation in derivations.iter_mut() {
                let result = derivation.add_to_db(&db).await;
                if result.is_err() {
                    error!("Failed to add derivatin to db!");
                    return;
                }
            }
        }
    }
}
