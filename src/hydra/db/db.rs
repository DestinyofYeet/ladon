use core::fmt;

use async_sqlite::{Pool, PoolBuilder};

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
    pool: Pool,
}

impl DB {
    pub async fn new(path: &str) -> Result<Self, DBError> {
        let pool = PoolBuilder::new()
            .path(path)
            .journal_mode(async_sqlite::JournalMode::Memory)
            .open()
            .await;

        let db = pool.map_err(|e| DBError::new(e.to_string()))?;
        
        Ok(DB{
            pool: db
        })
    }
}
