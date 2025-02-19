use std::{error, fmt, os::unix::process::ExitStatusExt, process::Stdio, str::FromStr, sync::Arc};

use tracing::{info, debug, warn, error};
use tokio::{io::{AsyncBufReadExt, BufReader}, process::{Child, Command}, sync::Mutex, time::Instant};

use super::parser;

struct EvaluatorData {
    is_running: Mutex<bool>,
    eval_process: Mutex<Child>,
}

pub struct Evaluator {
    flake_path: String,
    flake_attribute: String,
    data: Option<Arc<EvaluatorData>>,
}

#[derive(Debug)]
pub struct EvalResult {
    pub started_at: Instant,
    pub finished_at: Instant,
    pub flake: String,
    pub attribute: String,
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
        EvalError { error: String::from_str(error).unwrap() }
    }
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl error::Error for EvalError {
    
}

impl Evaluator {
    pub fn new(flake: &str, attribute: &str) -> Self {
        return Evaluator {
            flake_path: String::from_str(flake).unwrap(),
            flake_attribute: String::from_str(attribute).unwrap(),
            data: None,
        }
    }

    pub async fn start(&mut self) -> Result<EvalResult, EvalError> {
        let flake_path = self.flake_path.clone() + "#" + &self.flake_attribute;
        info!("Starting evaluation for: {}", flake_path);
        let mut process = Command::new("nix");

        let process = process
            .arg("build")
            .arg("--log-format")
            .arg("internal-json")
            .arg("--no-link")
            .arg(flake_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let process = match process.spawn() {
            Ok(value) => value,
            Err(e) => { return Err(EvalError::new(format!("Failed to spawn nix build: {}", e)));},
        };

        let started_at = Instant::now();

        self.data = Some(Arc::new(EvaluatorData { is_running: (Mutex::new(true)), eval_process: (Mutex::new(process)) }));

        let mut reader = BufReader::new(self.data.as_ref().unwrap().eval_process.lock().await.stderr.take().unwrap()).lines();

        let thread_data = self.data.clone();

        tokio::spawn(async move {
           let status = thread_data.as_ref().unwrap().eval_process.lock().await.wait().await.expect("nix build child failed to wait");
           {
               let mut running = thread_data.as_ref().unwrap().is_running.lock().await;
               *running = false;
           }           
        });

        let mut parser = parser::ActivityParser::new();

        while let Some(line) = reader.next_line().await.unwrap() {
            parser.parse_next(line);
        }

        let finished_at = Instant::now();

        let status = self.data.as_ref().unwrap().eval_process.lock().await.wait().await.unwrap();

        if status.success() {
            info!("Nix build exited successfully");
        } else if status.core_dumped() {
            return Err(EvalError::from_str("Nix build was core dumped!"));
        } else if status.signal().is_some(){
            return Err(EvalError::new(format!("Nix build was killed by a signal: {}", status.signal().unwrap())));
        } else {
            return Err(EvalError::new(format!("Nix build did not exit successfully: {}", status.code().unwrap())));
        };

        let flake_path = self.flake_path.clone();
        let attribute = self.flake_attribute.clone();

        return Ok({
            EvalResult { started_at, finished_at, flake: flake_path, attribute}
        });
    }
}
