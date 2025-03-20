use core::{error, fmt};
use std::{collections::VecDeque, sync::Arc};

use async_recursion::async_recursion;
use tokio::{
    process::Command,
    sync::{
        mpsc::{channel, unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex, Semaphore,
    },
    task::JoinHandle,
};
use tracing::{debug, error};

use super::drv::{DependencyTree, DrvBasic, DrvDepTree};

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

pub struct BuildManager {
    queue: Arc<Mutex<UnboundedSender<DrvDepTree>>>,
    handle: Option<JoinHandle<()>>,
}

impl BuildManager {
    pub fn new(max_builders: usize) -> Self {
        let (sender, receiver) = unbounded_channel::<DrvDepTree>();

        tokio::spawn(async move {
            BuildManager::run(receiver, max_builders).await;
        });
        Self {
            queue: Arc::new(Mutex::new(sender)),
            handle: None,
        }
    }

    pub async fn queue(&self, tree: DrvDepTree) {
        let locked = &mut *self.queue.lock().await;
        locked.send(tree);
    }

    async fn run(mut queue: UnboundedReceiver<DrvDepTree>, max_builders: usize) {
        let semaphore = Arc::new(Semaphore::new(max_builders));
        while let Some(build) = queue.recv().await {
            debug!("Received new realise request!");
            let thread_semaphore = semaphore.clone();

            tokio::spawn(async move {
                #[async_recursion]
                async fn build_last(tree: &DrvDepTree, semaphore: &Arc<Semaphore>) {
                    for dependency in tree.children.iter() {
                        build_last(&dependency, &semaphore).await
                    }

                    let ticket = semaphore.acquire().await.unwrap();

                    let result = BuildManager::realise(&tree.data.drv_path).await;

                    drop(ticket);

                    if result.is_err() {
                        error!("Failed to realise derivation: {}", &tree.data.drv_path);
                    } else {
                        debug!("Realised derivation: {}", &tree.data.drv_path);
                    }
                }

                build_last(&build, &thread_semaphore).await;
            });
        }
    }

    /// blocking function
    async fn realise(path: &str) -> Result<(), BuildError> {
        let mut command = Command::new("nix-store")
            .arg("--realise")
            .arg(path)
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

    async fn on_build_result(mut queue: UnboundedReceiver<()>) {}
}
