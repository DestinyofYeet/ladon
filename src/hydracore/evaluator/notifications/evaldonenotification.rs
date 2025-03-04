use chrono::{DateTime, Utc};

use crate::models::{Derivation, JobsetID};

pub struct EvalDoneNotification {
    started: DateTime<Utc>,
    finished: DateTime<Utc>,
    successfull: bool,
    error_msg: Option<String>,
    derivations: Option<Vec<Derivation>>,
    jobset_id: JobsetID,
}

impl EvalDoneNotification {
    pub fn new(
        started: DateTime<Utc>,
        finished: DateTime<Utc>,
        successfull: bool,
        error_msg: Option<String>,
        derivations: Option<Vec<Derivation>>,
        jobset_id: JobsetID,
    ) -> Self {
        Self {
            started,
            finished,
            successfull,
            error_msg,
            derivations,
            jobset_id,
        }
    }

    pub fn set_success(&mut self, value: bool) {
        self.successfull = value;
    }

    pub fn set_error(&mut self, error: String) {
        self.error_msg = Some(error);
    }

    pub fn set_derivations(&mut self, derivations: Vec<Derivation>) {
        self.derivations = Some(derivations);
    }

    pub fn get_started(&self) -> DateTime<Utc> {
        self.started
    }

    pub fn get_finished(&self) -> DateTime<Utc> {
        self.finished
    }

    pub fn is_successful(&self) -> bool {
        self.successfull
    }

    pub fn get_err(&self) -> Option<&str> {
        self.error_msg.as_deref()
    }

    pub fn get_derivations(&self) -> Option<&Vec<Derivation>> {
        self.derivations.as_ref()
    }

    pub fn get_derivations_copy(&mut self) -> Option<Vec<Derivation>> {
        let vec = self.derivations.clone();
        if vec.is_none() {
            return None;
        }

        let vec = vec.unwrap();
        Some(vec)
    }

    pub fn jobset_id(&self) -> JobsetID {
        self.jobset_id
    }
}
