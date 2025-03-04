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

    pub async fn add_to_db(&mut self, db: &DB) -> Result<(), DBError> {
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
                returning id
            ",
            proj_id,
            flake,
            name,
            desc,
            state,
            interval,
        )
        .fetch_one(&mut *conn)
        .await;

        if result.is_err() {
            return Err(DBError::new(result.err().unwrap().to_string()));
        }

        self.id = Some(result.unwrap().id as i32);

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

        let mut separated = query.separated(", ");

        let mut has_updates = false;

        macro_rules! handle_field_base {
            ($field:ident, $column:literal, $val:ident => $assign_expr:expr) => {
                if let Some($val) = diff.$field {
                    self.$field = $assign_expr;
                    separated
                        .push_unseparated(concat!($column, " = "))
                        .push_bind(&self.$field);
                    has_updates = true;
                }
            };
        }

        macro_rules! handle_field {
            ($field:ident, $column:literal) => {
                handle_field_base!($field, $column, value => value);
            };
        }

        macro_rules! handle_field_some {
            ($field:ident, $column:literal) => {
                handle_field_base!($field, $column, value => Some(value));
            };
        }

        handle_field!(name, "name");
        handle_field!(description, "description");
        handle_field!(flake, "flake");
        handle_field!(check_interval, "check_interval");

        handle_field_some!(last_checked, "last_checked");

        handle_field_some!(evaluation_took, "evaluation_took");

        handle_field_some!(state, "state");

        handle_field_some!(last_evaluated, "last_evaluated");

        handle_field_some!(error_message, "error_message");

        if !has_updates {
            return Ok(());
        }

        query.push(" where id = ").push_bind(self_id);
        let result = query.build().execute(&mut *conn).await;

        if result.is_err() {
            return Err(DBError::new(result.err().unwrap().to_string()));
        }

        Ok(())
    }
}
