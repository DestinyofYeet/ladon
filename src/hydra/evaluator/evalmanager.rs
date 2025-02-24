use std::future::IntoFuture;
use std::ops::Index;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, error, info};

use super::super::db::{DB, DBBuilds, DBDerivations};

use super::evaluator::{EvalError, EvalResult, Evaluator};

async fn wait_for_notification (mut notif_channel: Receiver<Notification>, db: DB){
        while let Some(notification) = notif_channel.recv().await {
            let done = Utc::now();
            info!("Received notification for task {}", notification.id);

            let unwrapped = notification.eval_result.lock().await;

            if unwrapped.is_err() {
                let err = unwrapped.as_ref().unwrap_err();
                error!("Build for task {} failed: {}", notification.id, err);
                continue;
            }

            let unwrapped = unwrapped.as_ref().unwrap();
            
            let time_took = unwrapped.finished_at.duration_since(unwrapped.started_at);

            let activities = &unwrapped.activities;

            let db_build = DBBuilds::new(
                unwrapped.flake.clone(),
                unwrapped.attribute.clone(),
                Some(done),
                false,
                Some(true),
                Some(time_took.as_secs()),
                unwrapped.logs.clone(),
            );

            let build_id = db.insert_build(db_build).await;
            if build_id.is_err(){
                error!("Failed to insert info for task {}: {}", notification.id, build_id.err().unwrap());
                continue;
            }
            info!("Inserted info for task {}", notification.id);
            let build_id = build_id.unwrap();

            info!("Found {} activites", activities.len());

            for activity in activities {
                let deriv = DBDerivations::new(
                    build_id,
                    activity.get_name(),
                    activity.get_log(),
                );

                let result = db.insert_derivation(deriv).await;

                if result.is_err() {
                    error!("Failed to insert info for derivation {}: {}", activity.get_name(), result.err().unwrap());
                    continue;
                }

                debug!("Successfully inserted info for derivation {}", activity.get_name());
            }
        }
}

type EvalReturnType = Arc<Mutex<Result<EvalResult, EvalError>>>;

type EvalJoinHandle = JoinHandle<EvalReturnType>;

type NotificationType = usize;

#[derive(Debug)]
pub struct Notification {
    id: usize,
    eval_result: EvalReturnType
}

pub struct EvalHandle {
    id: usize,
    handle: EvalJoinHandle,
}

pub struct EvalManager {
    evals: Vec<EvalHandle>,
    eval_counter: usize,
    notification_channel: Arc<Sender<Notification>>,
    notification_handle: JoinHandle<()>,
}
impl EvalManager {
    pub async fn new(db: DB) -> EvalManager {
        let (tx, rx) = mpsc::channel::<Notification>(1);
        let manager = EvalManager {
            evals: Vec::new(),
            eval_counter: 0,
            notification_channel: Arc::new(tx),
            notification_handle: tokio::spawn(async move {
                wait_for_notification(rx, db).await
            }),
        };


        manager
    }

    async fn get_join_handle(&mut self, handle_id: usize) -> EvalJoinHandle {
        let index = self.evals.iter().position(|entry| entry.id == handle_id).unwrap();
        return self.evals.remove(index).handle;
    }

    pub async fn schedule(&mut self, flake: &str, attribute: &str) -> Option<usize> {
        let mut eval = Evaluator::new(flake, attribute);

        let tx = self.notification_channel.clone();

        let id = self.eval_counter;
        self.eval_counter += 1;

        let handle = tokio::spawn(async move {
            let result = Arc::new(Mutex::new(eval.start().await));

            tx.send(Notification {
                id,
                eval_result: result.clone(),
            }).await.unwrap();
            info!("Notified EvalManager of task completion");

            return result;
        });


        let handle = EvalHandle {
            id,
            handle,
        };

        self.evals.push(handle);

        Some(id)
    }

    pub async fn wait_handle(&mut self, handle_id: usize) -> EvalReturnType {
        let handle = self.get_join_handle(handle_id);
        let result = handle.await.await.unwrap();
        return result;
    }

    pub async fn shutdown(self){
        let result = self.notification_handle.await;
        if result.is_err() {
            error!("Failed to wait on notification handle!");
        }
    }
}
