use core::fmt;

use async_sqlite::{Pool, PoolBuilder};
use chrono::{DateTime, Utc};

fn convert_to_string<T: ToString>(some_option: Option<T>) -> String {
    if some_option.is_some() {
        return some_option.unwrap().to_string();
    } else {
        return "null".to_string()
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

pub struct DBBuilds {
    primary_key: Option<u64>,
    flake: String,
    attribute: String,
    finished: Option<DateTime<Utc>>,
    time_took: Option<u64>,
    running: bool,
    success: Option<bool>
}

impl DBBuilds {
    pub fn new(flake: String, attribute: String, finished: Option<DateTime<Utc>>, running: bool, success: Option<bool>, time_took: Option<u64>) -> Self {
        DBBuilds {
            primary_key: None,
            flake,
            attribute,
            finished,
            running,
            success,
            time_took
        }
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

        let db = DB {
            pool: db,
        };

        let setup = db.setup().await;
        if setup.is_some() {
            return Err(setup.unwrap())
        };
        
        Ok(db)
    }

    async fn setup(&self) -> Option<DBError> {
        let result = self.pool.conn(|conn| {
            conn.execute_batch("
                    BEGIN;
                    create table if not exists Builds (
                        flake text not null,
                        attribute text not null,
                        finished date,
                        timeTookSecs int,
                        running boolean not null,
                        success boolean
                    );

                    create table if not exists DerivationBuilds (
                        id rowid  not null,
                        path text not null,
                        successfull boolean not null,
                        output text
                    );
                    COMMIT;
                ")
        }).await;

        if result.is_err() {
            return Some(DBError::new(result.err().unwrap().to_string()));
        };

        None
    }


    pub async fn insert_build(&self, build: DBBuilds) -> Option<DBError> {
        let flake = build.flake;
        let attribute = build.attribute;
        let finished: String = match build.finished {
            Some(value) => {value.to_rfc3339()},
            None => {"null".to_string()}
        };
        let time_took = convert_to_string(build.time_took);
        let running = build.running.to_string();
        let success = convert_to_string(build.success);

        let result = self.pool.conn(|conn| {
            conn.execute("
                    insert into Builds
                        (flake, attribute, finished, timeTookSecs, running, success)
                        values
                        (?, ?, ?, ?, ?, ?);
                    commit;
                ", [flake, attribute, finished, time_took, running, success])
        }).await;

        if result.is_err() {
            return Some(DBError::new(result.err().unwrap().to_string()));
        };

        None
    }
}
