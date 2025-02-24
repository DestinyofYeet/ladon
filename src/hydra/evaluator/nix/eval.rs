use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::{error, fmt, str::FromStr};

use serde_json;
use tokio::process::Child;
use tokio::sync::Mutex;
use tokio::{process::Command, time::Instant};

use tracing::info;

#[derive(Debug)]
pub struct EvalError {
    error: String,
}

impl EvalError {
    pub fn new(error: String) -> Self {
        EvalError { error }
    }

    pub fn from_str(error: &str) -> Self {
        EvalError { error: String::from_str(error).unwrap() }
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
    handle: Child,
    status: Option<Result<ExitStatus, std::io::Error>>,
}

impl ProcessData {
    fn new(done: bool, handle: Child) -> Self {
        ProcessData {
            done,
            handle,
            status: None,
        }
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
    flake: &'a str,
    attribute: &'a str,
}

impl <'a> Eval<'a>{
    pub fn new(flake: &'a str, attribute: &'a str) -> Self {
        Eval {
            flake,
            attribute
        }
    }

    pub async fn start(&mut self) -> Result<EvalResult, EvalError>{
        let uri = String::new() + self.flake + "#" + self.attribute;
        info!("Evaluating {}", uri);

        let process = Command::new("nix")
            .arg("eval")
            .arg(uri)
            .arg("--json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let process = match process {
            Ok(value) => value,
            Err(e) => {
                return Err(EvalError::new(format!("Failed to spawn nix eval: {}", e.to_string())));
            }
        };

        let started = Instant::now();

        let process_data = Arc::new(Mutex::new(ProcessData {
            done: false,
            handle: process,
            status: None,
        }));

        let thread_p_data = process_data.clone();

        tokio::spawn(async move {
            // this might fuck me later
            let status = thread_p_data.lock().await.handle.wait().await;
            let mut data = thread_p_data.lock().await;
            data.done = true;
            data.status = Some(status);
        });

        Ok(EvalResult {
            data: process_data,
            started,
        })
    }
}
