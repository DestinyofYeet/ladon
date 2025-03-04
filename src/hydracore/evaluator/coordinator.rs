use std::{process::ExitStatus, sync::Arc};

use crate::{
    hydracore::DBError,
    models::{Jobset, JobsetState},
};

use super::{
    super::db::DB,
    nix::eval::{Evaluation, EvaluationError},
};

use crate::models::Project;

use chrono::{DateTime, Utc};
use tokio::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Mutex,
    },
    task::JoinHandle,
};

struct CoordinatorData {
    db: Mutex<DB>,
}

impl CoordinatorData {
    pub fn new(db: DB) -> Self {
        CoordinatorData { db: Mutex::new(db) }
    }
}

pub struct Coordinator {
    data: Arc<Mutex<CoordinatorData>>,
}

impl Coordinator {
    pub fn new(db: DB) -> Self {
        let data = Arc::new(Mutex::new(CoordinatorData::new(db)));

        Coordinator { data }
    }

    pub async fn get_project(&self, id: i32) -> Result<Option<Project>, DBError> {
        self.data.lock().await.db.lock().await.get_project(id).await
    }

    pub async fn get_projects(&self) -> Result<Vec<Project>, DBError> {
        self.data.lock().await.db.lock().await.get_projects().await
    }

    pub async fn add_project(&self, name: &str, desc: &str) -> Result<(), DBError> {
        self.data
            .lock()
            .await
            .db
            .lock()
            .await
            .add_project(name, desc)
            .await
    }

    pub async fn add_jobset(&mut self, project_id: i32, jobset: Jobset) -> Result<(), DBError> {
        self.data
            .lock()
            .await
            .db
            .lock()
            .await
            .add_jobset(project_id, jobset)
            .await
    }

    pub async fn get_jobsets(&mut self, project_id: i32) -> Result<Vec<Jobset>, DBError> {
        self.data
            .lock()
            .await
            .db
            .lock()
            .await
            .get_jobsets(project_id)
            .await
    }

    pub async fn get_jobset(&mut self, jobset_id: i32) -> Result<Option<Jobset>, DBError> {
        self.data
            .lock()
            .await
            .db
            .lock()
            .await
            .get_jobset(jobset_id)
            .await
    }

    pub async fn schedule_jobset(&mut self, jobset: &mut Jobset) -> Result<(), EvaluationError> {
        jobset
            .update_state(
                JobsetState::EVALUATING,
                &*self.data.lock().await.db.lock().await,
            )
            .await
            .map_err(|e| EvaluationError::new(format!("DBError: {}", e.to_string())))?;

        _ = Evaluation::new(&jobset).await?;

        Ok(())
    }
}
