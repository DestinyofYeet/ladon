use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub type JobsetID = i32;

pub struct JobsetDiff {
    pub name: Option<String>,
    pub flake: Option<String>,
    pub description: Option<String>,
    pub check_interval: Option<i32>,
    pub last_checked: Option<DateTime<Utc>>,
    pub last_evaluated: Option<DateTime<Utc>>,
    pub evaluation_took: Option<i32>,
    pub state: Option<JobsetState>,
    pub error_message: Option<String>,
}

impl JobsetDiff {
    pub fn new() -> Self {
        Self {
            name: None,
            flake: None,
            description: None,
            check_interval: None,
            last_evaluated: None,
            last_checked: None,
            evaluation_took: None,
            state: None,
            error_message: None,
        }
    }

    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.name = Some(name);
        self
    }

    pub fn set_flake(&mut self, flake: String) -> &mut Self {
        self.flake = Some(flake);
        self
    }
    pub fn set_description(&mut self, description: String) -> &mut Self {
        self.description = Some(description);
        self
    }
    pub fn set_check_interval(&mut self, check_interval: i32) -> &mut Self {
        self.check_interval = Some(check_interval);
        self
    }
    pub fn set_last_checked(&mut self, last_checked: DateTime<Utc>) -> &mut Self {
        self.last_checked = Some(last_checked);
        self
    }

    pub fn set_last_evaluated(&mut self, last_evaluated: DateTime<Utc>) -> &mut Self {
        self.last_evaluated = Some(last_evaluated);
        self
    }

    pub fn set_evaluation_took(&mut self, evaluation_took: i32) -> &mut Self {
        self.evaluation_took = Some(evaluation_took);
        self
    }

    pub fn set_state(&mut self, state: JobsetState) -> &mut Self {
        self.state = Some(state);
        self
    }

    pub fn set_error_message(&mut self, error_message: String) -> &mut Self {
        self.error_message = Some(error_message);
        self
    }
}

#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum JobsetState {
    Unknown,
    Idle,
    Evaluating,
    Building,
    EvalFailed,
}

impl JobsetState {
    pub fn to_string(&self) -> String {
        String::from_str(match self {
            JobsetState::Unknown => "unknown",
            JobsetState::Idle => "idle",
            JobsetState::Building => "building",
            JobsetState::Evaluating => "evaluating",
            JobsetState::EvalFailed => "evaluation failed",
        })
        .unwrap()
    }
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Jobset {
    pub id: Option<JobsetID>,
    pub project_id: i32,
    pub name: String,
    pub flake: String,
    pub description: String,
    pub check_interval: i32,
    pub last_checked: Option<DateTime<Utc>>,
    pub last_evaluated: Option<DateTime<Utc>>,
    pub evaluation_took: Option<i32>,
    pub state: Option<JobsetState>,
    pub error_message: Option<String>,
}

#[cfg(feature = "ssr")]
use {
    crate::hydracore::{DBError, DB},
    sqlx::{query, QueryBuilder, Sqlite},
    tracing::trace,
};

#[cfg(feature = "ssr")]
impl Jobset {
    pub async fn get_all(db: &DB, project_id: i32) -> Result<Vec<Jobset>, DBError> {
        let mut conn = db.get_conn().await?;

        let result = sqlx::query_as::<_, Jobset>(
            "
                select * from Jobsets
                where project_id = ?
            ",
        )
        .bind(project_id)
        .fetch_all(&mut *conn)
        .await;

        if result.is_err() {
            return Err(DBError::new(result.err().unwrap().to_string()));
        }

        Ok(result.unwrap())
    }

    pub async fn get_single(db: &DB, jobset_id: i32) -> Result<Option<Jobset>, DBError> {
        let mut conn = db.get_conn().await?;

        let result = sqlx::query_as::<_, Jobset>(
            "
                select * from Jobsets
                where id = ?
            ",
        )
        .bind(jobset_id)
        .fetch_optional(&mut *conn)
        .await;

        if result.is_err() {
            return Err(DBError::new(result.err().unwrap().to_string()));
        }

        Ok(result.unwrap())
    }

    pub async fn add_to_db(&self, db: &DB) -> Result<(), DBError> {
        let mut conn = db.get_conn().await?;

        let name = &self.name;
        let desc = &self.description;
        let flake = &self.flake;
        let interval = self.check_interval;
        let state = self.state.clone().unwrap_or(JobsetState::Unknown);
        let proj_id = self.project_id;

        let result = query!(
            "
                insert into Jobsets
                    (project_id, flake, name, description, state, check_interval)
                values
                    (?, ?, ?, ?, ?, ?)
            ",
            proj_id,
            flake,
            name,
            desc,
            state,
            interval,
        )
        .execute(&mut *conn)
        .await;

        if result.is_err() {
            return Err(DBError::new(result.err().unwrap().to_string()));
        }

        Ok(())
    }

    pub async fn update_error(&mut self, db: &DB, error: &str) -> Result<(), DBError> {
        let mut diff = JobsetDiff::new();

        diff.set_error_message(error.to_string());

        self.update_jobset(db, diff).await?;

        Ok(())
    }

    pub async fn update_state(&mut self, db: &DB, state: JobsetState) -> Result<(), DBError> {
        if self.id.is_none() {
            return Err(DBError::new(
                "Cannot update state: ID is not set!".to_string(),
            ));
        }

        let mut diff = JobsetDiff::new();
        diff.set_state(state);

        self.update_jobset(db, diff).await?;

        Ok(())
    }

    pub async fn update_jobset(&mut self, db: &DB, diff: JobsetDiff) -> Result<(), DBError> {
        let mut conn = db.get_conn().await?;
        let self_id = self.id.unwrap();

        let mut query: QueryBuilder<'_, Sqlite> = QueryBuilder::new("update Jobsets set ");

        let mut need_comma = false;

        if let Some(name) = diff.name {
            self.name = name.clone();
            query.push(" name = ").push_bind(name);
            need_comma = true;
        }

        if let Some(desc) = diff.description {
            self.description = desc.clone();
            if need_comma {
                query.push(",");
            }
            need_comma = true;
            query.push(" description = ").push_bind(desc);
        }

        if let Some(flake) = diff.flake {
            self.flake = flake.clone();
            if need_comma {
                query.push(",");
            }
            need_comma = true;
            query.push(" flake = ").push_bind(flake);
        }
        if let Some(check_interval) = diff.check_interval {
            self.check_interval = check_interval;
            if need_comma {
                query.push(",");
            }
            need_comma = true;
            query.push(" check_interval = ").push_bind(check_interval);
        }
        if let Some(last_checked) = diff.last_checked {
            self.last_checked = Some(last_checked);
            if need_comma {
                query.push(",");
            }
            need_comma = true;
            query.push(" last_checked = ").push_bind(last_checked);
        }
        if let Some(evaluation_took) = diff.evaluation_took {
            self.evaluation_took = Some(evaluation_took);
            if need_comma {
                query.push(",");
            }
            need_comma = true;
            query.push(" evaluation_took = ").push_bind(evaluation_took);
        }
        if let Some(state) = diff.state {
            self.state = Some(state.clone());
            if need_comma {
                query.push(",");
            }
            need_comma = true;
            query.push(" state = ").push_bind(state);
        }
        if let Some(value) = diff.last_evaluated {
            self.last_evaluated = Some(value);
            if need_comma {
                query.push(",");
            }
            need_comma = true;
            query.push(" last_evaluated = ").push_bind(value);
        }
        if let Some(error_message) = diff.error_message {
            self.error_message = Some(error_message.clone());
            if need_comma {
                query.push(",");
            }
            //need_comma = true;
            query.push(" error_message = ").push_bind(error_message);
        }

        query.push(" where id = ").push_bind(self_id);
        let result = query.build().execute(&mut *conn).await;

        if result.is_err() {
            return Err(DBError::new(result.err().unwrap().to_string()));
        }

        Ok(())
    }
}
