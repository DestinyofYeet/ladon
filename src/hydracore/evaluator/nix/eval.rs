use std::collections::HashMap;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::{error, fmt, str::FromStr};

use serde_json::{self, Value};
use tokio::process::Child;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::{process::Command, time::Instant};

use tracing::{info, trace};

use super::super::coordinator::{EvalNotification, EvalNotificationSender};

#[derive(Debug)]
pub struct EvalInformation {
    pub name: String,
    pub nix_path: String,
}

impl EvalInformation {
    fn new(name: String, nix_path: String) -> Self {
        Self { name, nix_path }
    }
}

#[derive(Debug)]
pub struct EvalError {
    error: String,
}

impl EvalError {
    pub fn new(error: String) -> Self {
        EvalError { error }
    }

    pub fn from_str(error: &str) -> Self {
        EvalError {
            error: String::from_str(error).unwrap(),
        }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl error::Error for EvalError {}

struct ProcessData {
    done: bool,
}

impl ProcessData {
    fn new(done: bool, handle: Child) -> Self {
        ProcessData { done }
    }
}

pub struct EvalResult {
    data: Arc<Mutex<ProcessData>>,
    started: Instant,
}

impl EvalResult {
    pub async fn is_done(&self) -> bool {
        return self.data.lock().await.done;
    }
}

/// This will evaluate a nix expression and return a name and a nix output path.
pub struct Eval<'a> {
    flake_uri: &'a str,
}

impl<'a> Eval<'a> {
    pub fn new(flake_uri: &'a str) -> Self {
        Eval { flake_uri }
    }

    pub async fn start(
        &mut self,
        sender: EvalNotificationSender,
        handle: usize,
    ) -> Result<JoinHandle<()>, EvalError> {
        info!("Evaluating {}", self.flake_uri);

        let process = Command::new("nix")
            .arg("eval")
            .arg("--json")
            .arg(self.flake_uri)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let process = match process {
            Ok(value) => value,
            Err(e) => {
                return Err(EvalError::new(format!(
                    "Failed to spawn nix eval: {}",
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
                .send(EvalNotification::new(handle, stdout, stderr, status))
                .await;
        });

        Ok(handle)
    }

    pub fn get_paths_in_json(value: &Value) -> Vec<EvalInformation> {
        let mut map = HashMap::new();
        Eval::get_paths_recursive(&mut map, String::new(), value);

        let mut result = Vec::new();

        for (key, value) in map.iter() {
            result.push(EvalInformation::new(key.clone(), value.clone()));
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
                    Eval::get_paths_recursive(map, new_path, val);
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
}
