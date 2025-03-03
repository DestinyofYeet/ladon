use core::fmt;
use std::str::FromStr;

use sqlx::{
    pool::PoolConnection,
    query, query_as,
    sqlite::{SqliteConnectOptions, SqlitePool},
    Sqlite,
};
use tracing::info;

use crate::models::{Jobset, JobsetState, Project};

fn convert_to_string<T: ToString>(some_option: Option<T>) -> String {
    if some_option.is_some() {
        return some_option.unwrap().to_string();
    } else {
        return "null".to_string();
    }
}
#[derive(Debug)]
pub struct DBError(String);

impl DBError {
    pub fn new(error: String) -> Self {
        DBError { 0: error }
    }
}

impl fmt::Display for DBError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct DB {
    pool: SqlitePool,
}

impl DB {
    pub async fn new(path: &str) -> Result<Self, DBError> {
        let path = String::new() + "sqlite://" + path;
        let opts = SqliteConnectOptions::from_str(&path)
            .map_err(|e| DBError::new(e.to_string()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        info!("Connecting to database: {}", path);
        let pool = SqlitePool::connect_with(opts).await;

        let db = pool.map_err(|e| DBError::new(e.to_string()))?;

        let db = DB { pool: db };

        let setup = db.setup().await;
        if setup.is_some() {
            return Err(setup.unwrap());
        };

        Ok(db)
    }

    async fn get_conn(&self) -> Result<PoolConnection<Sqlite>, DBError> {
        self.pool
            .acquire()
            .await
            .map_err(|e| DBError::new(e.to_string()))
    }

    async fn setup(&self) -> Option<DBError> {
        let mut conn = self.get_conn().await.unwrap();

        let result = sqlx::migrate!("./migrations")
            .run(&mut *conn)
            .await
            .map_err(|e| DBError::new(e.to_string()));

        if result.is_err() {
            return Some(result.err().unwrap());
        }

        info!("Applied database migrations (if necessary)");

        None
    }

    pub async fn get_project(&self, id: i32) -> Result<Option<Project>, DBError> {
        let mut conn = self.get_conn().await?;

        let project = query_as::<_, Project>(
            "
                select * 
                from Projects
                where id = ?
            ",
        )
        .bind(id)
        .fetch_optional(&mut *conn)
        .await;

        let project = project.map_err(|e| DBError::new(e.to_string()))?;

        Ok(project)
    }

    pub async fn get_projects(&self) -> Result<Vec<Project>, DBError> {
        let mut conn = self.get_conn().await?;

        let projects = query_as::<_, Project>("select * from Projects")
            .fetch_all(&mut *conn)
            .await;

        let projects = projects.map_err(|e| DBError::new(e.to_string()))?;

        Ok(projects)
    }

    pub async fn add_project(&self, name: &str, desc: &str) -> Result<(), DBError> {
        let name = name.to_string();
        let desc = desc.to_string();

        let mut conn = self.get_conn().await?;

        let result = query!(
            "
                insert into Projects 
                    (name, description)
                values
                    (?, ?)
            ",
            name,
            desc
        )
        .execute(&mut *conn)
        .await;

        if result.is_err() {
            return Err(DBError::new(result.err().unwrap().to_string()));
        }

        Ok(())
    }

    pub async fn add_jobset(&self, project_id: i32, jobset: Jobset) -> Result<(), DBError> {
        let mut conn = self.get_conn().await?;

        let name = jobset.name;
        let desc = jobset.description;
        let flake = jobset.flake;
        let interval = jobset.check_interval;
        let state = match jobset.state {
            None => JobsetState::IDLE,
            Some(value) => value,
        };

        let result = query!(
            "
                insert into Jobsets
                    (project_id, flake, name, description, state, check_interval)
                values
                    (?, ?, ?, ?, ?, ?)
            ",
            project_id,
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

    pub async fn get_jobsets(&self, project_id: i32) -> Result<Vec<Jobset>, DBError> {
        let mut conn = self.get_conn().await?;

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

    pub async fn get_jobset(&self, jobset_id: i32) -> Result<Option<Jobset>, DBError> {
        let mut conn = self.get_conn().await?;

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
}
