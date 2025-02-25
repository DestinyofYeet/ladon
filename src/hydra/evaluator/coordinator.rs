use std::{process::ExitStatus, sync::Arc};

use crate::db::DB;

use super::nix::{derivation::DerivationInformation, store::Store};

use super::nix::derivation::Derivation;
use super::nix::eval::Eval;

use serde_json::Value;
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
};
use tracing::{debug, error, info, trace};

#[derive(Debug, Clone)]
pub enum ActionState {
    None,
    Failed,
    Evaluating,
    Decoding,
    Building,
    Done,
}

pub struct ActionHandle {
    handle: usize,
}

impl ActionHandle {
    fn new(id: usize) -> Self {
        ActionHandle { handle: id }
    }
}

#[derive(Debug)]
pub struct Action {
    id: usize,
    flake_uri: String,
    state: ActionState,
    handle: JoinHandle<()>,
}

impl Action {
    fn new(id: usize, flake_uri: String, handle: JoinHandle<()>) -> Self {
        Action {
            id,
            flake_uri,
            state: ActionState::None,
            handle,
        }
    }

    fn set_state(&mut self, state: ActionState, db: Option<&DB>) {
        if db.is_some() {}
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
}

impl RealiseNotification {
    pub fn new(
        handle: usize,
        stdout: String,
        stderr: String,
        status: ExitStatus,
        derivation_information: DerivationInformation,
    ) -> Self {
        Self {
            handle,
            stdout,
            stderr,
            status,
            derivation_information,
        }
    }
}

pub type RealiseNotificationSender = Arc<Sender<RealiseNotification>>;

struct CoordinatorData {
    actions: Vec<Action>,
    realise_tx: RealiseNotificationSender,
    db: DB,
}

impl CoordinatorData {
    pub fn new(realise_tx: RealiseNotificationSender, db: DB) -> Self {
        CoordinatorData {
            actions: Vec::new(),
            realise_tx,
            db,
        }
    }
}

pub struct Coordinator {
    action_counter: usize,
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
            action_counter: 0,
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

    fn new_action_id(&mut self) -> usize {
        let counter = self.action_counter;
        self.action_counter += 1;

        counter
    }

    pub async fn schedule(&mut self, flake_uri: &str) -> bool {
        info!("New flake scheduled: {}", flake_uri);
        let mut eval = Eval::new(flake_uri);
        let action_id = self.new_action_id();
        let result = eval.start(self.eval_tx.clone(), action_id).await;

        if result.is_err() {
            error!(
                "Failed to schedule flake {}: {}",
                flake_uri,
                result.err().unwrap()
            );
            return false;
        }

        let result = result.unwrap();
        let mut action = Action::new(action_id, flake_uri.to_string(), result);

        action.set_state(ActionState::Evaluating);

        self.data.lock().await.actions.push(action);

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

            trace!("Attempting to get lock for action");
            let mut locked = data.lock().await;
            let action = locked
                .actions
                .iter_mut()
                .find(|elem| elem.id == notification.handle)
                .expect(&format!("Failed to find element {}", notification.handle));

            trace!("Got action");

            if !notification.status.success() {
                error!("Nix evaluation failed!\nStderr: {}", notification.stderr);
                action.set_state(ActionState::Failed);
                continue;
            }

            let result: Value = serde_json::from_str(&notification.stdout).unwrap();

            let eval_information = Eval::get_paths_in_json(&result);

            action.set_state(ActionState::Decoding);

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

            for derivation in result {
                let result = Store::realise(derivation, realise_tx.clone(), action.id).await;

                if result.is_err() {
                    error!("Failed to start realisation: {}", result.err().unwrap());
                    continue;
                }

                action.handle = result.unwrap();
                action.set_state(ActionState::Building);
            }
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

            trace!("Attempting to get lock for actions");
            let mut locked = data.lock().await;

            let action = locked
                .actions
                .iter_mut()
                .find(|elem| elem.id == notification.handle)
                .unwrap();
            trace!("Got action");

            action.set_state(ActionState::Done);

            info!("Built {}", notification.derivation_information.name);
        }
    }
}
