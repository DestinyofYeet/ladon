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
    pub fn new(flake: &str, attribute: &str) -> Evaluator {
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
            .arg(self.flake_path.clone() + "#" + &self.flake_attribute)
            .arg("--log-format")
            .arg("internal-json")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let process = match process.spawn() {
            Ok(value) => value,
            Err(e) => { eprintln!("Failed to spawn nix build: {}", e); return;},
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
           
           println!("nix build process was: {}", status);
        });


        while let Some(line) = reader.next_line().await.unwrap() {
            let line = line.strip_prefix("@nix ").unwrap();

            println!("line: {}", line);
            let parsed: serde_json::Value = serde_json::from_str(&line).unwrap();

            // https://github.com/maralorn/nix-output-monitor/blob/main/lib/NOM/Parser/JSON.hs#L105
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
                    let mut process = self.data.as_ref().unwrap().eval_process.lock().await;

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
