use core::{error, fmt};
use std::{process::Stdio, sync::Arc};

use tokio::{
    process::Command,
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex, Semaphore,
    },
    task::JoinHandle,
};
use tracing::info;

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

pub enum BuildResult {
    Success,
    Failed,
}

struct QueueItem {
    path: String,
}

struct BuildSettings {
    max_builders: usize,
}

pub struct BuildManager {
    queue: Arc<UnboundedSender<QueueItem>>,
}

impl BuildManager {
    pub fn new(max_builders: usize) -> Self {
        let (sender, receiver) = unbounded_channel::<QueueItem>();

        let settings = BuildSettings { max_builders };

        tokio::spawn(async move {
            BuildManager::queue_consumer(receiver, settings).await;
        });

        BuildManager {
            queue: Arc::new(sender),
        }
    }

    pub async fn queue(&self, path: String) {
        self.queue.clone().send(QueueItem { path });
    }

    async fn queue_consumer(mut receiver: UnboundedReceiver<QueueItem>, settings: BuildSettings) {
        let semaphore = Arc::new(Semaphore::new(settings.max_builders));
        while let Some(item) = receiver.recv().await {
            let semaphore_clone = semaphore.clone();
            tokio::spawn(async move {
                let ticket = semaphore_clone.acquire().await.unwrap();
                info!("Queuing: {}", item.path);
                let result = BuildManager::realise(&item.path).await;

                drop(ticket);
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
