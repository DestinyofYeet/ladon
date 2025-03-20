use core::{error, fmt};
use std::{process::Stdio, sync::Arc, time::Instant};

use tokio::{
    process::Command,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex, Semaphore,
    },
    task::JoinHandle,
};
use tracing::{error, info};

use crate::hydracore::Coordinator;

#[derive(Debug)]
pub struct BuildError {
    error: String,
}

impl BuildError {
    pub fn new(error: String) -> Self {
        BuildError { error }
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl error::Error for BuildError {}

pub struct BuildResult {
    pub successful: bool,
    pub id: i32,
    pub path: String,
    pub took_secs: i32,
}

struct QueueItem {
    path: String,
    drv_id: i32,
}

pub type BuildTx = UnboundedSender<BuildResult>;

struct BuildSettings {
    max_builders: usize,
    build_tx: BuildTx,
}

pub struct BuildManager {
    queue: Arc<UnboundedSender<QueueItem>>,
}

impl BuildManager {
    pub fn new(build_tx: BuildTx, max_builders: usize) -> Self {
        let (sender, receiver) = unbounded_channel::<QueueItem>();

        let settings = BuildSettings {
            max_builders,
            build_tx,
        };

        tokio::spawn(async move {
            BuildManager::queue_consumer(receiver, settings).await;
        });

        BuildManager {
            queue: Arc::new(sender),
        }
    }

    pub async fn queue(&self, path: String, id: i32) {
        self.queue.clone().send(QueueItem { path, drv_id: id });
    }

    async fn queue_consumer(mut receiver: UnboundedReceiver<QueueItem>, settings: BuildSettings) {
        let semaphore = Arc::new(Semaphore::new(settings.max_builders));
        while let Some(item) = receiver.recv().await {
            let semaphore_clone = semaphore.clone();
            let build_tx_clone = settings.build_tx.clone();
            tokio::spawn(async move {
                let ticket = semaphore_clone.acquire().await.unwrap();
                info!("Queuing: {}", item.path);
                let start = Instant::now();
                let result = BuildManager::realise(&item.path).await;
                let took = start.elapsed().as_secs() as i32;
                drop(ticket);

                let mut message = BuildResult {
                    id: item.drv_id,
                    successful: true,
                    path: item.path,
                    took_secs: took,
                };
                if result.is_err() {
                    error!("Failed to realise store path: {}", result.err().unwrap());
                    message.successful = false;
                }

                let result = build_tx_clone.send(message);

                if result.is_err() {
                    error!(
                        "Failed to send build_done notification: {}",
                        result.err().unwrap()
                    )
                }
            });
        }
    }

    async fn realise(path: &str) -> Result<(), BuildError> {
        let mut command = Command::new("nix-store")
            .arg("--realise")
            .arg(path)
            .arg("-j")
            .arg("1")
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| BuildError::new(e.to_string()))?;

        let result = command
            .wait()
            .await
            .map_err(|e| BuildError::new(e.to_string()))?;

        if !result.success() {
            return Err(BuildError::new(format!(
                "Failed to realise store path: {}",
                path
            )));
        }

        Ok(())
    }
}
