use std::{process::Stdio, str::FromStr, sync::Arc};

use tokio::{io::{AsyncBufReadExt, BufReader}, process::{Child, Command}, sync::Mutex};

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalMessageResult {
    action: String,
    id: u64,
    #[serde(rename="type")]
    result_type: i32,
    fields: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalMessageStop {
    action: String,
    id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalMessageStart {
    action: String,
    id: u64,
    level: Option<u64>,
    parent: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalMessageMsg {
    action: String,
    column: Option<String>,
    file: Option<String>,
    level: u64,
    line: Option<String>,
    msg: String,
}

pub struct Evaluator {
    flake_path: String,
    flake_attribute: String,
    running: Arc<Mutex<bool>>,
    process: Option<Arc<Mutex<Child>>>,
}

impl Evaluator {
    pub fn new(flake: &str, attribute: &str) -> Evaluator {
        return Evaluator {
            flake_path: String::from_str(flake).unwrap(),
            flake_attribute: String::from_str(attribute).unwrap(),
            running: Arc::new(Mutex::new(true)),
            process: None,
        }
    }

    pub async fn start(&mut self) {
        let mut process = Command::new("nix");

        let process = process
            .arg("eval")
            .arg(self.flake_path.clone() + "#" + &self.flake_attribute)
            .arg("--log-format")
            .arg("internal-json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        self.process = match process.spawn() {
            Ok(value) => Some(Arc::new(Mutex::new(value))),
            Err(e) => { eprintln!("Failed to spawn nix build: {}", e); return;},
        };

        let thread_running = self.running.clone();
        let thread_process = self.process.clone();

        let mut reader = BufReader::new(self.process.as_ref().unwrap().lock().await.stderr.take().unwrap()).lines();

        tokio::spawn(async move {
           let status = thread_process.unwrap().lock().await.wait().await.expect("nix build child failed to wait");
           {
               let mut running = thread_running.lock().await;
               *running = false;
           }
           
           println!("nix build process was: {}", status);
        });


        while let Some(line) = reader.next_line().await.unwrap() {
            let line = line.strip_prefix("@nix ").unwrap();

            println!("line: {}", line);
            let parsed: serde_json::Value = serde_json::from_str(&line).unwrap();

            match parsed.get("action").unwrap().as_str().unwrap() {
                "start" => {
                    let start_msg: EvalMessageStart = serde_json::from_value(parsed).unwrap();
                },

                "stop" => {
                    let stop_msg: EvalMessageStop = serde_json::from_value(parsed).unwrap();
                },

                "result" => {
                    let result_msg: EvalMessageResult = serde_json::from_value(parsed).unwrap();
                },

                "msg" => {
                    let msg_msg: EvalMessageMsg = serde_json::from_value(parsed).unwrap();
                }
                _ => {
                    eprintln!("Failed to parse action: {}", parsed.get("action").unwrap());
                    dbg!(&parsed);
                    let mut process = self.process.as_ref().unwrap().lock().await;

                    match process.kill().await {
                        Ok(_) => println!("Killed nix build"),
                        Err(e) => println!("Failed to kill nix build: {}", e),
                    }
                        
                    return;
                },
            }
        }

        return;
    }
}
