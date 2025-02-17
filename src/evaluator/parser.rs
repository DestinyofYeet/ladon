use std::{process::Stdio, str::FromStr, sync::Arc};

use tracing::{info, debug, warn, error};
use tokio::{io::{AsyncBufReadExt, BufReader}, process::{Child, Command}, sync::Mutex};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalMessageResult {
    action: String,
    id: u64,
    #[serde(rename="type")]
    result_type: i32,
    fields: serde_json::Value,
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
    #[serde(rename="type")]
    activity_type: i32,
    text: String,
    fields: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EvalMessageMsg {
    action: String,
    column: Option<u64>,
    file: Option<String>,
    level: u64,
    line: Option<u64>,
    msg: String,
}

#[derive(Debug, Eq, PartialEq)]
enum ActivityType {
    Unknown,
    CopyPath,
    FileTransfer,
    Realise,
    CopyPaths,
    Builds,
    Build,
    OptimiseStore,
    VerifyPaths,
    Substitute,
    QueryPathInfo,
    PostBuildHook,
    BuildWaiting,
    FetchTree,
}

impl ActivityType {
    pub fn parse(type_id: u64) -> Self {
        match type_id {
            0 => ActivityType::Unknown,
            100 => ActivityType::CopyPath,
            101 => ActivityType::FileTransfer,
            102 => ActivityType::Realise,
            103 => ActivityType::CopyPaths,
            104 => ActivityType::Builds,
            105 => ActivityType::Build,
            106 => ActivityType::OptimiseStore,
            107 => ActivityType::VerifyPaths,
            108 => ActivityType::Substitute,
            109 => ActivityType::QueryPathInfo,
            110 => ActivityType::PostBuildHook,
            111 => ActivityType::BuildWaiting,
            112 => ActivityType::FetchTree,
            
            _ => panic!("Failed to parse derivationType: {}", type_id),
        }
    }
}


struct Activity {
    name: Option<String>,
    build_id: u64,
    running: bool,
    nth_activity: i32,
    activity_type: ActivityType,
}

pub struct ActivityParser {
    nth_message: i32,
    messages: Vec<Activity>,
}

impl ActivityParser {
    pub fn new() -> Self {
        Self {
          nth_message: 0,
          messages: Vec::new(),
        }
    }
    pub fn parse_next(&mut self, line: String){
        
        let line = line.strip_prefix("@nix ").unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&line).unwrap();

        // https://github.com/maralorn/nix-output-monitor/blob/main/lib/NOM/Parser/JSON.hs#L105
        match parsed.get("action").unwrap().as_str().unwrap() {
            "start" => {
                self.nth_message += 1;
            
                let start_msg: EvalMessageStart = serde_json::from_value(parsed).expect(&format!("Failed to parse line into EvalMessageStart: {line}"));

                let name: Option<String> = start_msg.fields.map(|field| field.get(0).unwrap().to_string());
            
                let dev = Activity {
                    name,
                    build_id: start_msg.id,
                    running: true,
                    nth_activity: self.nth_message,
                    activity_type: ActivityType::parse(start_msg.activity_type as u64)
                };
            
                info!("[{}]{}> Started {:#?} activity: {}", dev.nth_activity, dev.name.as_ref().unwrap_or(&String::new()), dev.activity_type, start_msg.text);

                self.messages.push(dev);
            },

            "stop" => {
                let stop_msg: EvalMessageStop = serde_json::from_value(parsed).unwrap();

                let activity = self.messages.iter_mut().filter(|entry| entry.build_id == stop_msg.id).last().expect("Failed to find running activity although stop was received");

                activity.running = false;
                info!("[{}]{}> Finished {:#?} activity", activity.nth_activity, activity.name.as_ref().unwrap_or(&String::new()), activity.activity_type);

                self.messages.retain(|entry| !(entry.build_id == stop_msg.id));
            },

            "result" => {
                let result_msg: EvalMessageResult = serde_json::from_value(parsed).expect(&format!("Failed to parse to EvalMessageResult: {:#?}", line));

                let activity = self.messages.iter().filter(|entry| entry.build_id == result_msg.id).last().expect("Failed to find running activity although result was received");

                match result_msg.result_type {
                    100 => {debug!("{line}");},
                    101 => {
                        debug!("[{}] ", activity.nth_activity);
                    },
                    102 => {debug!("{line}");},
                    103 => {debug!("{line}");},
                    104 => {debug!("{line}");},
                    105 => {
                        // let number_array: Result<Vec<u64>, _> = serde_json::from_value(result_msg.fields);

                        // if number_array.is_ok() {
                        //     let fields = number_array.unwrap();
                        //     let (done, expected, running, failed) = (fields.get(0).unwrap(), fields.get(1).unwrap(), fields.get(2).unwrap(), fields.get(3).unwrap());
                        //     info!("[{}] Result: (done: {}, expected: {}, running: {}, failed: {})", activity.nth_activity, done, expected, running, failed)
                        // } else {
                        //     einfo!("Unexpected non-numbers array instead of numbers");
                        // }
                    },
                    106 => {
                        // let number_array: Result<Vec<u64>, _> = serde_json::from_value(result_msg.fields.clone());

                        // if number_array.is_ok() {
                        //     let fields = number_array.unwrap();
                        //     let (activity_type, number) = (fields.get(0).unwrap(), fields.get(1).unwrap());

                        //     let activity_type = ActivityType::parse(*activity_type);
                        
                        //     info!("[{}] Result: (activity: {:#?}, dunno: {})", activity.nth_activity, activity_type, number);
                        // } else {
                        //     let string_array: Result<Vec<String>, _> = serde_json::from_value(result_msg.fields.clone());
                        //     if string_array.is_ok() {
                        //         let fields = string_array.unwrap();
                        //         info!("[{}] Result: {}", activity.nth_activity, fields.join("|"));
                        //     } else {
                        //         einfo!("Unexpected Value: {:#?}", result_msg.fields)
                        //     }
                        // }
                    
                    },
                    107 => {debug!("{line}");},
                    _ => panic!("Failed to parse result type"),
                };

            },

            "msg" => {
                // info!("{line}");
                let msg_msg: EvalMessageMsg = serde_json::from_value(parsed).unwrap();
                info!("[msg] {}", msg_msg.msg);
            }
            _ => {
                error!("Failed to parse action: {}", parsed.get("action").unwrap());
                dbg!(&parsed);                
                return;
            },
        }
    }
}
