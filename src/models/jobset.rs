use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum JobsetState {
    IDLE = 0,
    RUNNING = 1,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Jobset {
    pub id: Option<i32>,
    project_id: Option<i32>,
    pub name: String,
    pub flake: String,
    pub description: String,
    pub check_interval: i32,
    pub last_checked: Option<DateTime<Utc>>,
    pub last_evaluated: Option<DateTime<Utc>>,
    pub evaluation_took: Option<i32>,
    pub state: Option<JobsetState>,
}
