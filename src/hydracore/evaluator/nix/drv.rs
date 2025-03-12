use core::{error, fmt};
use std::{
    future::Future, os::unix::process::ExitStatusExt, process::Stdio, str::FromStr, sync::Arc,
};

use futures::future::{join_all, try_join_all};

use serde_json::Value;
use tokio::{process::Command, task::JoinHandle};
use tracing::{debug, error, info};

use crate::models::Job;

pub type DrvDepTree = DependencyTree<DrvBasic>;

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

/// Returns (stdout, stderr) if successfull
async fn run_nix_derivation_show(path: &str) -> Result<(String, String), DerivationError> {
    let process = Command::new("nix")
        .arg("derivation")
        .arg("show")
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| DerivationError::new(e.to_string()))?;

    let result = process
        .wait_with_output()
        .await
        .map_err(|e| DerivationError::new(e.to_string()))?;

    let stderr = String::from_utf8(result.stderr).unwrap();
    let stdout = String::from_utf8(result.stdout).unwrap();

    if !result.status.success() {
        if result.status.core_dumped() {
            return Err(DerivationError::new(format!(
                "Core dumped: {}",
                result.status.code().unwrap()
            )));
        }

        return Err(DerivationError::new(format!(
            "Exited abnormally: {} | Stderr: {} | Stdout: {}",
            result.status.code().unwrap(),
            stderr,
            stdout,
        )));
    }

    return Ok((stdout, stderr));
}

#[derive(Debug, Clone)]
pub struct DrvBasic {
    pub drv_path: String,
    pub name: String,
}

impl DrvBasic {
    pub async fn get_derivation(output_path: &str) -> Result<DrvBasic, DerivationError> {
        info!("Getting derivation path for: {}", output_path);

        let (stdout, stderr) = run_nix_derivation_show(output_path).await?;

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

        Ok(DrvBasic {
            drv_path,
            name: value.to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct DependencyTree<T> {
    pub(crate) children: Vec<DependencyTree<T>>,
    pub(crate) data: T,
    pub(crate) built: bool,
}

impl<T> DependencyTree<T> {
    pub fn new(data: T) -> Self {
        Self {
            children: Vec::new(),
            data,
            built: false,
        }
    }
}

impl DrvDepTree {
    pub async fn generate(derivation_path: &str) -> Result<DrvDepTree, DerivationError> {
        debug!("Generating build plan for '{}'", derivation_path);
        let (stdout, _) = run_nix_derivation_show(derivation_path).await?;

        let parsed: Value = serde_json::from_str(&stdout).map_err(|e| {
            error!(
                "Failed to parse json: {} | Json:\n{}",
                e.to_string(),
                stdout
            );
            DerivationError::new(e.to_string())
        })?;

        let value = parsed.get(derivation_path).unwrap();

        let input_drvs = value.get("inputDrvs").unwrap();

        let inputs: Vec<DependencyTree<DrvBasic>> = try_join_all(
            input_drvs
                .as_object()
                .unwrap()
                .keys()
                .map(|key| DependencyTree::generate(&key)),
        )
        .await?;

        let current = DrvBasic {
            drv_path: derivation_path.to_string(),
            name: value.get("name").unwrap().as_str().unwrap().to_string(),
        };

        debug!("Done generating build plan for '{}'", derivation_path);

        let mut tree = DependencyTree::new(current);
        tree.children = inputs;

        Ok(tree)
    }
}
