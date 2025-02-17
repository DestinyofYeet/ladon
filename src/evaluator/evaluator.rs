use std::{process::Stdio, str::FromStr, sync::Arc};

use tracing::{info, debug, warn, error};
use tokio::{io::{AsyncBufReadExt, BufReader}, process::{Child, Command}, sync::Mutex};

use serde::{Deserialize, Serialize};

use crate::parser;


struct EvaluatorData {
    is_running: Mutex<bool>,
    eval_process: Mutex<Child>,
}

pub struct Evaluator {
    flake_path: String,
    flake_attribute: String,
    data: Option<Arc<EvaluatorData>>,
}

impl Evaluator {
    pub fn new(flake: &str, attribute: &str) -> Self {
        return Evaluator {
            flake_path: String::from_str(flake).unwrap(),
            flake_attribute: String::from_str(attribute).unwrap(),
            data: None,
        }
    }

    pub async fn start(&mut self) {
        let mut process = Command::new("nix");

        let process = process
            .arg("build")
            .arg("--log-format")
            .arg("internal-json")
            .arg("--no-link")
            .arg(self.flake_path.clone() + "#" + &self.flake_attribute)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let process = match process.spawn() {
            Ok(value) => value,
            Err(e) => { error!("Failed to spawn nix build: {}", e); return;},
        };

        self.data = Some(Arc::new(EvaluatorData { is_running: (Mutex::new(true)), eval_process: (Mutex::new(process)) }));

        let mut reader = BufReader::new(self.data.as_ref().unwrap().eval_process.lock().await.stderr.take().unwrap()).lines();

        let thread_data = self.data.clone();

        tokio::spawn(async move {
           let status = thread_data.as_ref().unwrap().eval_process.lock().await.wait().await.expect("nix build child failed to wait");
           {
               let mut running = thread_data.as_ref().unwrap().is_running.lock().await;
               *running = false;
           }
           
           debug!("nix build process was: {}", status);
        });

        let mut parser = parser::ActivityParser::new();

        while let Some(line) = reader.next_line().await.unwrap() {
            parser.parse_next(line);
        }

        return;
    }
}
