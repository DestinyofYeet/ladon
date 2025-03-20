use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use {
    crate::hydracore::{DBError, DB},
    sqlx::{query, QueryBuilder, Sqlite},
};

pub struct JobDiff {
    pub evaluation_id: Option<i32>,
    pub attribute_name: Option<String>,
    pub derivation_path: Option<String>,
    pub state: Option<JobState>,
    pub finished: Option<DateTime<Utc>>,
}

impl JobDiff {
    pub fn new() -> Self {
        JobDiff {
            evaluation_id: None,
            attribute_name: None,
            derivation_path: None,
            state: None,
            finished: None,
        }
    }
}

#[cfg_attr(feature = "ssr", derive(sqlx::Type))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum JobState {
    ToBeBuilt,
    Building,
    Failed,
    Done,
}

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Job {
    pub id: Option<i32>,
    pub evaluation_id: i32,
    pub attribute_name: String,
    pub derivation_path: String,
    pub state: JobState,
    pub finished: Option<DateTime<Utc>>,
}

#[cfg(feature = "ssr")]
impl Job {
    pub fn new(evaluation_id: i32, attribute_name: String, derivation_path: String) -> Self {
        Self {
            id: None,
            evaluation_id,
            attribute_name,
            derivation_path,
            state: JobState::ToBeBuilt,
            finished: None,
        }
    }

    pub async fn add_to_db(&mut self, db: &DB) -> Result<(), DBError> {
        let mut conn = db.get_conn().await?;

        let result = query!(
            "
                insert into Jobs
                    (evaluation_id, attribute_name, derivation_path, state, finished)
                values
                    (?, ?, ?, ?, ?)
                returning id
            ",
            self.evaluation_id,
            self.attribute_name,
            self.derivation_path,
            self.state,
            self.finished,
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| DBError::new(e.to_string()))?;

        self.id = Some(result.id as i32);

        Ok(())
    }

    pub async fn update_job(&mut self, db: &DB, diff: JobDiff) -> Result<(), DBError> {
        let mut conn = db.get_conn().await?;
        let self_id = self.id.unwrap();

        let mut query: QueryBuilder<'_, Sqlite> = QueryBuilder::new("update Jobs set ");

        let mut separated = query.separated(", ");

        let mut has_updates = false;

        macro_rules! handle_field_base {
            ($field:ident, $column:literal, $val:ident => $assign_expr:expr) => {
                if let Some($val) = diff.$field {
                    self.$field = $assign_expr;
                    separated
                        .push(concat!($column, " = "))
                        .push_bind_unseparated(&self.$field);
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

        handle_field!(evaluation_id, "evaluation_id");
        handle_field!(attribute_name, "attribute_name");
        handle_field!(derivation_path, "derivation_path");
        handle_field!(state, "state");
        handle_field_some!(finished, "finished");

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

    pub async fn get_single(db: &DB, id: i32) -> Result<Option<Job>, DBError> {
        let mut conn = db.get_conn().await?;

        let result = sqlx::query_as::<_, Job>(
            "
                select *
                from Jobs
                where id = ?
            ",
        )
        .bind(id)
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| DBError::new(e.to_string()))?;

        Ok(result)
    }

    pub async fn get_all(db: &DB, jobset_id: i32) -> Result<Vec<Job>, DBError> {
        let mut conn = db.get_conn().await?;

        let result = sqlx::query_as::<_, Job>(
            "
                select *
                from Jobs
                where evaluation_id = ?
            ",
        )
        .bind(jobset_id)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| DBError::new(e.to_string()))?;

        Ok(result)
    }
}
