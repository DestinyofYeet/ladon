use core::{error, fmt};
use std::{
    collections::HashMap, os::unix::process::ExitStatusExt, process::Stdio, str::FromStr, sync::Arc,
};

use axum::Error;
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::{debug, error, info};

use tokio::{
    process::Command,
    sync::mpsc::{Sender, UnboundedSender},
    task::JoinHandle,
};

use crate::models::{Job, Jobset, JobsetID};

use super::super::notifications::EvalDoneNotification;

#[derive(Debug)]
pub struct EvaluationError {
    error: String,
}

impl EvaluationError {
    pub fn new(error: String) -> Self {
        EvaluationError { error }
    }
    pub fn from_str(error: &str) -> Self {
        EvaluationError {
            error: String::from_str(error).unwrap(),
        }
    }
}

impl fmt::Display for EvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl error::Error for EvaluationError {}
pub struct Evaluation {}

impl Evaluation {
    pub async fn new(
        sender: Arc<UnboundedSender<EvalDoneNotification>>,
        jobset: &Jobset,
    ) -> Result<JoinHandle<()>, EvaluationError> {
        if jobset.id.is_none() {
            return Err(EvaluationError::new("Jobset struct has no id!".to_string()));
        }
        let jobset_id = jobset.id.unwrap();

        info!("Evaluating: {}", jobset.flake);

        let process = Command::new("nix")
            .arg("eval")
            .arg("--json")
            .arg("--no-write-lock-file")
            .arg(&jobset.flake)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| EvaluationError::new(e.to_string()))?;

        let started = Utc::now();

        let handle = tokio::spawn(async move {
            let result = process.wait_with_output().await.unwrap();
            let status = result.status;

            let done = Utc::now();

            let mut notification =
                EvalDoneNotification::new(started, done, false, None, None, jobset_id);

            if !status.success() {
                if status.core_dumped() {
                    notification.set_error(format!(
                        "Nix eval failed to run successfully: Core dumped with code {}",
                        status.code().unwrap()
                    ));
                } else {
                    let stderr = String::from_utf8(result.stderr).unwrap();
                    notification.set_error(format!(
                        "Nix eval failed to run successfully: Code {} | Stderr: \n{}",
                        status.code().unwrap(),
                        stderr
                    ));
                }

                let result = sender.send(notification);
                if result.is_err() {
                    error!("Failed to send notification");
                }
                return;
            }

            let stdout = String::from_utf8(result.stdout).unwrap();

            debug!("stdout: {}", stdout);

            let value: Result<Value, _> = serde_json::from_str(&stdout);

            if value.is_err() {
                notification.set_error(format!("Failed to parse nix eval output: {}", stdout));
                let result = sender.send(notification);
                if result.is_err() {
                    error!("Failed to send notification");
                }
                return;
            }

            let value = value.unwrap();

            let derivations = get_derivation_information(&value, jobset_id);

            let notification =
                EvalDoneNotification::new(started, done, true, None, Some(derivations), jobset_id);

            let result = sender.send(notification);
            if result.is_err() {
                error!("Failed to send notification");
            }
        });

        Ok(handle)
    }
}

fn get_derivation_information(value: &Value, jobset_id: JobsetID) -> Vec<Job> {
    let mut map = HashMap::new();
    get_paths_recursive(&mut map, String::new(), value);

    let mut result = Vec::new();

    for (key, value) in map.iter() {
        result.push(Job::new(jobset_id, key.clone(), value.clone()));
    }

    result
}

fn get_paths_recursive(map: &mut HashMap<String, String>, current_path: String, value: &Value) {
    match value {
        Value::Object(obj) => {
            for (key, val) in obj {
                let new_path = if current_path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", current_path, key)
                };
                get_paths_recursive(map, new_path, val);
            }
        }

        _ => {
            let val_str = match value {
                Value::String(s) => s.clone(),
                _ => {
                    unreachable!()
                }
            };

            map.insert(current_path, val_str);
        }
    }
}
