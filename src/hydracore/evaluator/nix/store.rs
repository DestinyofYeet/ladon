use core::{error, fmt};
use std::{process::Stdio, str::FromStr};

use tokio::{process::Command, task::JoinHandle, time::Instant};
use tracing::info;

use super::{
    super::coordinator::{RealiseNotification, RealiseNotificationSender},
    derivation::DerivationInformation,
};

#[derive(Debug)]
pub struct StoreError {
    error: String,
}

impl StoreError {
    pub fn new(error: String) -> Self {
        StoreError { error }
    }

    pub fn from_str(error: &str) -> Self {
        StoreError {
            error: String::from_str(error).unwrap(),
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl error::Error for StoreError {}

pub struct Store {}

impl Store {
    pub async fn realise(
        deriv_info: DerivationInformation,
        sender: RealiseNotificationSender,
        handle: usize,
    ) -> Result<JoinHandle<()>, StoreError> {
        let drv_path = &deriv_info.derivation_path;
        info!("Realising store path: {}", drv_path);

        let process = Command::new("nix-store")
            .arg("--realise")
            .arg(&drv_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let process = match process {
            Ok(value) => value,
            Err(e) => {
                return Err(StoreError::new(format!(
                    "Failed to spawn nix-store --realise: {}",
                    e.to_string()
                )));
            }
        };

        let started = Instant::now();

        let handle = tokio::spawn(async move {
            let result = process.wait_with_output().await.unwrap();
            let status = result.status;
            let stdout = String::from_utf8(result.stdout).unwrap();
            let stderr = String::from_utf8(result.stderr).unwrap();
            _ = sender
                .send(RealiseNotification::new(
                    handle, stdout, stderr, status, deriv_info, started,
                ))
                .await;
        });

        Ok(handle)
    }
}
