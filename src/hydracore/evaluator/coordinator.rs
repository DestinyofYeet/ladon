use std::{process::ExitStatus, sync::Arc};

use crate::{hydracore::DBError, models::Jobset};

use super::nix::{
    derivation::{DerivationInformation, DerivationState},
    store::Store,
};

use super::super::db::DB;

use super::nix::derivation::Derivation;
use super::nix::eval::Eval;

use crate::models::Project;

use chrono::{DateTime, Utc};
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
pub enum EvaluationState {
    None = 0,
    Failed = 1,
    Evaluating = 2,
    Decoding = 3,
    Building = 4,
    Done = 5,
}

#[derive(Debug)]
pub struct Evaluation {
    id: usize,
    flake_uri: String,
    state: EvaluationState,
    derivation: Option<Vec<DerivationInformation>>,
}

impl Evaluation {
    fn new(id: usize, flake_uri: String) -> Self {
        Evaluation {
            id,
            flake_uri,
            state: EvaluationState::None,
            derivation: None,
        }
    }

    async fn set_state(&mut self, state: EvaluationState, db: Option<&DB>) {
        if db.is_some() {
            //let db = db.unwrap();
            //let result = db.update_job_state(self.id, state.clone()).await;
            //if result.is_err() {
            //    error!("Failed to update state in db: {}", result.err().unwrap())
            //}
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
    started_at: DateTime<Utc>,
}

impl RealiseNotification {
    pub fn new(
        handle: usize,
        stdout: String,
        stderr: String,
        status: ExitStatus,
        derivation_information: DerivationInformation,
        started_at: DateTime<Utc>,
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
    evaluations: Mutex<Vec<Evaluation>>,
    realise_tx: RealiseNotificationSender,
    db: Mutex<DB>,
}

impl CoordinatorData {
    pub fn new(realise_tx: RealiseNotificationSender, db: DB) -> Self {
        CoordinatorData {
            evaluations: Mutex::new(Vec::new()),
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

    pub async fn add_jobset(&mut self, project_id: i32, jobset: Jobset) -> Result<(), DBError> {
        self.data
            .lock()
            .await
            .db
            .lock()
            .await
            .add_jobset(project_id, jobset)
            .await
    }

    pub async fn get_jobsets(&mut self, project_id: i32) -> Result<Vec<Jobset>, DBError> {
        self.data
            .lock()
            .await
            .db
            .lock()
            .await
            .get_jobsets(project_id)
            .await
    }

    pub async fn get_jobset(&mut self, jobset_id: i32) -> Result<Option<Jobset>, DBError> {
        self.data
            .lock()
            .await
            .db
            .lock()
            .await
            .get_jobset(jobset_id)
            .await
    }

    fn new_eval_id(&mut self) -> usize {
        let counter = self.job_counter;
        self.job_counter += 1;

        counter
    }

    pub async fn schedule(&mut self, flake_uri: &str) -> bool {
        info!("New flake scheduled: {}", flake_uri);
        let mut eval = Eval::new(flake_uri);
        let eval_id = self.new_eval_id();
        let result = eval.start(self.eval_tx.clone(), eval_id).await;

        if result.is_err() {
            error!(
                "Failed to schedule flake {}: {}",
                flake_uri,
                result.err().unwrap()
            );
            return false;
        }

        let mut evaluation = Evaluation::new(eval_id, flake_uri.to_string());

        trace!("[lock] Attemping to set eval state");
        evaluation
            .set_state(
                EvaluationState::Evaluating,
                Some(&*self.data.lock().await.db.lock().await),
            )
            .await;
        trace!("[lock] Set eval state");

        trace!("[lock] Attemping to add eval to eval array");
        self.data
            .lock()
            .await
            .evaluations
            .lock()
            .await
            .push(evaluation);
        trace!("[lock] Added eval");

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

            trace!("Attempting to get lock for evaluations");
            let locked = data.lock().await;
            let mut locked_jobs = locked.evaluations.lock().await;
            let job = locked_jobs
                .iter_mut()
                .find(|elem| elem.id == notification.handle)
                .expect(&format!("Failed to find element {}", notification.handle));

            trace!("Got evaluation");

            if !notification.status.success() {
                error!("Nix evaluation failed!\nStderr: {}", notification.stderr);
                job.set_state(EvaluationState::Failed, Some(&*locked.db.lock().await))
                    .await;
                continue;
            }

            let result: Value = serde_json::from_str(&notification.stdout).unwrap();

            let eval_information = Eval::get_paths_in_json(&result);

            job.set_state(EvaluationState::Decoding, Some(&*locked.db.lock().await))
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
            job.set_state(EvaluationState::Building, Some(&*locked.db.lock().await))
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

            trace!("[lock] Attempting to get lock for evaluation");
            let locked = data.lock().await;
            let mut locked_jobs = locked.evaluations.lock().await;

            let action = locked_jobs
                .iter_mut()
                .find(|elem| elem.id == notification.handle)
                .unwrap();
            trace!("[lock] Got evaluation");

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
                    .set_state(EvaluationState::Done, Some(&*locked.db.lock().await))
                    .await;
                trace!("[lock] Got db")
            }

            info!("Built {}", notification.derivation_information.obj_name);
        }
    }
}
