use core::{error, fmt};
use std::{os::unix::process::ExitStatusExt, process::Stdio, str::FromStr};

use serde_json::Value;
use tokio::process::Command;
use tracing::info;

use crate::models::Derivation;

#[derive(Debug)]
pub struct DerivationError {
    error: String,
}

impl DerivationError {
    pub fn new(error: String) -> Self {
        DerivationError { error }
    }

    pub fn from_str(error: &str) -> Self {
        DerivationError {
            error: String::from_str(error).unwrap(),
        }
    }
}

impl fmt::Display for DerivationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl error::Error for DerivationError {}

pub struct Drv {
    pub drv_path: String,
    pub name: String,
}

impl Drv {
    pub async fn get_derivation(output_path: &str) -> Result<Drv, DerivationError> {
        info!("Getting derivation path for: {}", output_path);

        let process = Command::new("nix")
            .arg("derivation")
            .arg("show")
            .arg(output_path)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| DerivationError::new(e.to_string()))?;

        let result = process
            .wait_with_output()
            .await
            .map_err(|e| DerivationError::new(e.to_string()))?;

        if !result.status.success() {
            if result.status.core_dumped() {
                return Err(DerivationError::new(format!(
                    "Core dumped: {}",
                    result.status.code().unwrap()
                )));
            }

            return Err(DerivationError::new(format!(
                "Exited abnormally: {}",
                result.status.code().unwrap(),
            )));
        }

        let stdout = String::from_utf8(result.stdout).unwrap();

        let parsed: Value = serde_json::from_str(&stdout).map_err(|e| {
            DerivationError::new(format!("Failed to parse '{}' | {}", stdout, e.to_string()))
        })?;

        // TODO: Fix this abomination
        let mut test: Vec<(String, Value)> = Vec::with_capacity(1);

        match parsed {
            Value::Object(obj) => {
                for (key, value) in obj {
                    test.push((key, value));
                    break;
                }
            }

            _ => {
                panic!("Failed to parse json!")
            }
        };

        let (drv_path, value) = test.remove(0);

        let value = value.get("name").unwrap().as_str().unwrap();

        Ok(Drv {
            drv_path,
            name: value.to_string(),
        })
    }
}
