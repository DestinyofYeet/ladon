use std::{process::ExitStatus, sync::Arc};

use crate::hydracore::DBError;

use super::nix::{
    derivation::{DerivationInformation, DerivationState},
    store::Store,
};

use super::super::db::{DBJob, DB};

use super::nix::derivation::Derivation;
use super::nix::eval::Eval;

use crate::models::Project;

use serde_json::Value;
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
    time::Instant,
};
use tracing::{debug, error, info, trace};

#[derive(Debug, Clone)]
pub enum JobState {
    None = 0,
    Failed = 1,
    Evaluating = 2,
    Decoding = 3,
    Building = 4,
    Done = 5,
}

pub struct JobHandle {
    handle: usize,
}

impl JobHandle {
    fn new(id: usize) -> Self {
        JobHandle { handle: id }
    }
}

#[derive(Debug)]
pub struct Job {
    id: usize,
    flake_uri: String,
    state: JobState,
    handle: JoinHandle<()>,
    derivation: Option<Vec<DerivationInformation>>,
}

impl Job {
    fn new(id: usize, flake_uri: String, handle: JoinHandle<()>) -> Self {
        Job {
            id,
            flake_uri,
            state: JobState::None,
            handle,
            derivation: None,
        }
    }

    async fn set_state(&mut self, state: JobState, db: Option<&DB>) {
        if db.is_some() {
            let db = db.unwrap();
            let result = db.update_job_state(self.id, state.clone()).await;
            if result.is_err() {
                error!("Failed to update state in db: {}", result.err().unwrap())
            }
        }
        trace!(
            "State of {}: {:#?} -> {:#?}",
            self.flake_uri,
            self.state,
            state
        );
        self.state = state;
    }
}

pub struct EvalNotification {
    handle: usize,
    stdout: String,
    stderr: String,
    status: ExitStatus,
}

impl EvalNotification {
    pub fn new(handle: usize, stdout: String, stderr: String, status: ExitStatus) -> Self {
        EvalNotification {
            handle,
            stdout,
            stderr,
            status,
        }
    }
}

pub type EvalNotificationSender = Arc<Sender<EvalNotification>>;

pub struct RealiseNotification {
    handle: usize,
    stdout: String,
    stderr: String,
    status: ExitStatus,
    derivation_information: DerivationInformation,
    started_at: Instant,
}

impl RealiseNotification {
    pub fn new(
        handle: usize,
        stdout: String,
        stderr: String,
        status: ExitStatus,
        derivation_information: DerivationInformation,
        started_at: Instant,
    ) -> Self {
        Self {
            handle,
            stdout,
            stderr,
            status,
            derivation_information,
            started_at,
        }
    }
}

pub type RealiseNotificationSender = Arc<Sender<RealiseNotification>>;

struct CoordinatorData {
    jobs: Mutex<Vec<Job>>,
    realise_tx: RealiseNotificationSender,
    db: Mutex<DB>,
}

impl CoordinatorData {
    pub fn new(realise_tx: RealiseNotificationSender, db: DB) -> Self {
        CoordinatorData {
            jobs: Mutex::new(Vec::new()),
            realise_tx,
            db: Mutex::new(db),
        }
    }
}

pub struct Coordinator {
    job_counter: usize,
    eval_tx: EvalNotificationSender,
    eval_handle: JoinHandle<()>,

    realise_handle: JoinHandle<()>,
    data: Arc<Mutex<CoordinatorData>>,
}

impl Coordinator {
    pub fn new(db: DB) -> Self {
        let (eval_tx, eval_rx) = mpsc::channel::<EvalNotification>(1);
        let (realise_tx, realise_rx) = mpsc::channel::<RealiseNotification>(1);

        let data = Arc::new(Mutex::new(CoordinatorData::new(Arc::new(realise_tx), db)));

        let eval_data = data.clone();
        let realise_data = data.clone();

        Coordinator {
            job_counter: 0,
            eval_tx: Arc::new(eval_tx),
            eval_handle: tokio::spawn(async {
                Coordinator::on_eval_result(eval_rx, eval_data).await
            }),

            realise_handle: tokio::spawn(async {
                Coordinator::on_realise_result(realise_rx, realise_data).await
            }),
            data,
        }
    }

    pub async fn get_project(&self, id: i32) -> Result<Option<Project>, DBError> {
        self.data.lock().await.db.lock().await.get_project(id).await
    }

    pub async fn get_projects(&self) -> Result<Vec<Project>, DBError> {
        self.data.lock().await.db.lock().await.get_projects().await
    }

    pub async fn add_project(&self, name: &str, desc: &str) -> Result<(), DBError> {
        self.data
            .lock()
            .await
            .db
            .lock()
            .await
            .add_project(name, desc)
            .await
    }

    fn new_job_id(&mut self) -> usize {
        let counter = self.job_counter;
        self.job_counter += 1;

        counter
    }

    pub async fn schedule(&mut self, flake_uri: &str) -> bool {
        info!("New flake scheduled: {}", flake_uri);
        let mut eval = Eval::new(flake_uri);
        let job_id = self.new_job_id();
        let result = eval.start(self.eval_tx.clone(), job_id).await;

        if result.is_err() {
            error!(
                "Failed to schedule flake {}: {}",
                flake_uri,
                result.err().unwrap()
            );
            return false;
        }

        let result = result.unwrap();
        let mut job = Job::new(job_id, flake_uri.to_string(), result);

        let db_job = DBJob::new(
            flake_uri.to_string(),
            None,
            None,
            job.state.clone(),
            None,
            String::new(),
        );

        trace!("[lock] Attempting to get db");
        let result = self
            .data
            .lock()
            .await
            .db
            .lock()
            .await
            .insert_build(db_job)
            .await;
        if result.is_err() {
            eprintln!(
                "Failed to schedule flake {}: {}",
                flake_uri,
                result.err().unwrap()
            );

            return false;
        }
        trace!("[lock] Did db");

        trace!("[lock] Attemping to set job state");
        job.set_state(
            JobState::Evaluating,
            Some(&*self.data.lock().await.db.lock().await),
        )
        .await;
        trace!("[lock] Set job state");

        trace!("[lock] Attemping to add job to jobs array");
        self.data.lock().await.jobs.lock().await.push(job);
        trace!("[lock] Added job");

        true
    }

    pub async fn shutdown(self) {
        _ = self.eval_handle.await;
    }

    async fn on_eval_result(
        mut receiver: Receiver<EvalNotification>,
        data: Arc<Mutex<CoordinatorData>>,
    ) {
        while let Some(notification) = receiver.recv().await {
            debug!("Received eval results for {}", notification.handle);

            trace!("Attempting to get lock for realise_tx channel");
            let realise_tx = &data.lock().await.realise_tx.clone();
            trace!("Got tx channel");

            trace!("Attempting to get lock for job");
            let locked = data.lock().await;
            let mut locked_jobs = locked.jobs.lock().await;
            let job = locked_jobs
                .iter_mut()
                .find(|elem| elem.id == notification.handle)
                .expect(&format!("Failed to find element {}", notification.handle));

            trace!("Got job");

            if !notification.status.success() {
                error!("Nix evaluation failed!\nStderr: {}", notification.stderr);
                job.set_state(JobState::Failed, Some(&*locked.db.lock().await))
                    .await;
                continue;
            }

            let result: Value = serde_json::from_str(&notification.stdout).unwrap();

            let eval_information = Eval::get_paths_in_json(&result);

            job.set_state(JobState::Decoding, Some(&*locked.db.lock().await))
                .await;

            let derivation = Derivation::new(eval_information);

            let result = derivation.start().await;

            if result.is_err() {
                error!(
                    "Failed to get derivaiton information: {}",
                    result.err().unwrap()
                );
                continue;
            }

            let result = result.unwrap();

            trace!("Derivation results: {:#?}", result);

            job.derivation = Some(result);

            for derivation in job.derivation.as_mut().unwrap() {
                let result = Store::realise(derivation.clone(), realise_tx.clone(), job.id).await;

                if result.is_err() {
                    error!("Failed to start realisation: {}", result.err().unwrap());
                    continue;
                }

                derivation.state = DerivationState::Building;
            }

            trace!("[lock] Attempting to get db for state change!");
            job.set_state(JobState::Building, Some(&*locked.db.lock().await))
                .await;
            trace!("[lock] Did state change");
        }
    }

    async fn on_realise_result(
        mut receiver: Receiver<RealiseNotification>,
        data: Arc<Mutex<CoordinatorData>>,
    ) {
        while let Some(notification) = receiver.recv().await {
            debug!("Received realise results for {}", notification.handle);

            if !notification.status.success() {
                error!("Realisation process did not finish successfully!");
                continue;
            }

            trace!("[lock] Attempting to get lock for job");
            let locked = data.lock().await;
            let mut locked_jobs = locked.jobs.lock().await;

            let action = locked_jobs
                .iter_mut()
                .find(|elem| elem.id == notification.handle)
                .unwrap();
            trace!("[lock] Got job");

            for derivation in action.derivation.as_mut().unwrap().iter_mut() {
                if derivation.obj_name == notification.derivation_information.obj_name {
                    derivation.state = DerivationState::Done;
                    trace!("{} marked as done", derivation.obj_name);
                }
            }

            let mut all_done = true;

            for derivation in action.derivation.as_mut().unwrap().iter_mut() {
                if all_done {
                    all_done = derivation.state == DerivationState::Done;
                }
            }

            if all_done {
                trace!("[lock] Attempting to get db for state change");
                action
                    .set_state(JobState::Done, Some(&*locked.db.lock().await))
                    .await;
                trace!("[lock] Got db")
            }

            info!("Built {}", notification.derivation_information.obj_name);
        }
    }
}
