#[cfg(feature = "ssr")]
use tokio::sync::Mutex;

#[cfg(feature = "ssr")]
use crate::hydracore::Coordinator;

#[cfg(feature = "ssr")]
pub struct State {
    pub coordinator: Mutex<Coordinator>,
}
