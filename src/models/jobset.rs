use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type JobsetID = i32;

#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum JobsetState {
    UNKNOWN,
    IDLE,
    EVALUATING,
    BUILDING,
}

impl JobsetState {
    pub fn to_string(&self) -> String {
        String::from_str(match self {
            JobsetState::UNKNOWN => "unknown",
            JobsetState::IDLE => "idle",
            JobsetState::BUILDING => "building",
            JobsetState::EVALUATING => "evaluating",
        })
        .unwrap()
    }
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Jobset {
    pub id: Option<JobsetID>,
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

#[cfg(feature = "ssr")]
impl Jobset {
    pub async fn update_state(
        &mut self,
        state: JobsetState,
        db: &crate::hydracore::DB,
    ) -> Result<(), crate::hydracore::DBError> {
        use crate::hydracore::DBError;
        use tracing::trace;

        if self.id.is_none() {
            return Err(DBError::new(
                "Cannot update state: ID is not set!".to_string(),
            ));
        }

        trace!("Upating state: {:?} -> {:?}", self.state, state);
        self.state = Some(state.clone());
        Ok(db.update_jobset_state(self.id.unwrap(), state).await?)
    }
}
