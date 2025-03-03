use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum JobsetState {
    UNKNOWN,
    IDLE,
    RUNNING,
}

impl JobsetState {
    pub fn to_string(&self) -> String {
        String::from_str(match self {
            JobsetState::IDLE => "idle",
            JobsetState::RUNNING => "running",
            JobsetState::UNKNOWN => "unknown",
        })
        .unwrap()
    }
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Jobset {
    pub id: Option<i32>,
    pub project_id: Option<i32>,
    pub name: String,
    pub flake: String,
    pub description: String,
    pub check_interval: i32,
    pub last_checked: Option<DateTime<Utc>>,
    pub last_evaluated: Option<DateTime<Utc>>,
    pub evaluation_took: Option<i32>,
    pub state: Option<JobsetState>,
}
