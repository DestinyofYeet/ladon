use std::{collections::VecDeque, sync::Arc};

use tokio::{sync::Mutex, task::JoinHandle};

use super::drv::{DependencyTree, DrvBasic, DrvDepTree};

type DepQueue = Arc<Mutex<VecDeque<DrvDepTree>>>;

pub enum BuildResult {
    Success,
    Failed,
}

pub struct BuildManager {
    queue: DepQueue,
    max_builders: i32,
    used_builders: i32,
    handle: Option<JoinHandle<()>>,
}

impl BuildManager {
    pub fn new(max_builders: i32) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            max_builders,
            used_builders: 0,
            handle: None,
        }
    }

    pub async fn queue(&self, tree: DrvDepTree) {
        let locked = &mut *self.queue.lock().await;
        locked.push_back(tree);
    }

    pub async fn start(&mut self) {
        let queue_clone = self.queue.clone();

        self.handle = Some(tokio::spawn(
            async move { BuildManager::run(queue_clone).await },
        ));
    }

    async fn run(queue: DepQueue) {}
}
