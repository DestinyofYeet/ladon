use std::{collections::HashMap, process::{ExitCode, ExitStatus}, sync::Arc};

use super::nix::eval::Eval;

use serde::{Serialize, Deserialize};

use serde_json::Value;
use tokio::{sync::{mpsc::{self, Receiver, Sender}, Mutex}, task::JoinHandle};
use tracing::{debug, error, info};

pub enum ActionState {
    None,
    Failed,
    Evaluating,
    Decoding,
    Building,
}

pub struct ActionHandle {
    handle: usize,
}

impl ActionHandle {
    fn new(id: usize) -> Self {
        ActionHandle {
            handle: id,
        }
    }
}

pub struct Action{
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

    fn set_state(&mut self, state: ActionState) {
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

#[derive(Serialize, Deserialize)]
struct EvalParseResult {
    result: HashMap<String, Value>
}

pub type EvalNotificationSender = Arc<Sender<EvalNotification>>;

struct CoordinatorData {
    actions: Vec<Action>
}

impl CoordinatorData {
    pub fn new() -> Self {
        CoordinatorData {
            actions: Vec::new(),
        }
    }
}

pub struct Coordinator {
    action_counter: usize,
    eval_tx: EvalNotificationSender,
    eval_handle: JoinHandle<()>,
    data: Arc<Mutex<CoordinatorData>>,
}

impl Coordinator {
    pub fn new() -> Self {
        let (eval_tx, eval_rx) = mpsc::channel::<EvalNotification>(1);

        let data = Arc::new(Mutex::new(CoordinatorData::new()));

        let eval_data = data.clone();
        
        Coordinator {
            action_counter: 0,
            eval_tx: Arc::new(eval_tx),
            eval_handle: tokio::spawn(async {
               Coordinator::on_eval_result(eval_rx, eval_data).await
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
            error!("Failed to schedule flake {}: {}", flake_uri, result.err().unwrap());
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


    async fn on_eval_result(mut receiver: Receiver<EvalNotification>, data: Arc<Mutex<CoordinatorData>>){
        while let Some(notification) = receiver.recv().await {
            info!("Received eval results for {}", notification.handle);

            let mut locked_data = data.lock().await;

            let action = locked_data.actions.iter_mut().find(|elem| elem.id == notification.handle).expect(&format!("Failed to find element {}", notification.handle));

            if !notification.status.success() {
                error!("Nix evaluation failed!\nStderr: {}", notification.stderr);
                action.set_state(ActionState::Failed);
                continue;
            }

            let result: Value = serde_json::from_str(&notification.stdout).unwrap();

            let map = Eval::get_paths_in_json(&result);

            dbg!(map);

        }
    }
}
