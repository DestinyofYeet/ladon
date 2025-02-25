use core::{error, fmt};
use std::{process::Stdio, str::FromStr};

use serde_json::Value;
use tokio::process::Command;
use tracing::{debug, info};

use super::eval::EvalInformation;

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

#[derive(Debug)]
pub struct DerivationInformation {
    pub(crate) derivation_path: String,
    pub(crate) name: String,
    pub(crate) system: String,
    pub(crate) obj_name: String,
}

pub struct Derivation {
    eval_information: Vec<EvalInformation>,
}

impl Derivation {
    pub fn new(eval_information: Vec<EvalInformation>) -> Self {
        Derivation { eval_information }
    }

    async fn get_information(
        &self,
        information: &EvalInformation,
    ) -> Result<DerivationInformation, DerivationError> {
        debug!(
            "Getting derivation information for {}",
            information.nix_path
        );

        let process = Command::new("nix")
            .arg("derivation")
            .arg("show")
            .arg(&information.nix_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let process = match process {
            Ok(value) => value,
            Err(e) => {
                return Err(DerivationError::new(format!(
                    "Failed to spawn nix derivation show: {}",
                    e.to_string()
                )));
            }
        };

        let result = process.wait_with_output().await;

        if result.is_err() {
            return Err(DerivationError::new(format!(
                ".await failed: {}",
                result.err().unwrap()
            )));
        }

        let result = result.unwrap();

        if !result.status.success() {
            return Err(DerivationError::new(format!(
                "status code unsuccessfull: {}",
                result.status.code().unwrap()
            )));
        }

        let stdout = String::from_utf8(result.stdout).unwrap();

        let parsed: Value = serde_json::from_str(&stdout).unwrap();

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

        let name: String = value.get("name").unwrap().to_string();
        let system: String = value.get("system").unwrap().to_string();

        debug!("{}, {}, {}", name, system, drv_path);

        Ok(DerivationInformation {
            derivation_path: drv_path,
            name,
            system,
            obj_name: information.name.clone(),
        })
    }

    pub async fn start(&self) -> Result<Vec<DerivationInformation>, DerivationError> {
        let mut derivation_information = Vec::new();
        for entry in self.eval_information.iter() {
            let result = self.get_information(&entry).await?;
            derivation_information.push(result);
        }

        Ok(derivation_information)
    }
}
