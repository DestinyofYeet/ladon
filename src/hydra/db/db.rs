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
    success: Option<bool>,
    logs: String,
}

impl DBBuilds {
    pub fn new(flake: String, attribute: String, finished: Option<DateTime<Utc>>, running: bool, success: Option<bool>, time_took: Option<u64>, logs: String) -> Self {
        DBBuilds {
            primary_key: None,
            flake,
            attribute,
            finished,
            running,
            success,
            time_took,
            logs
        }
    }
}

pub struct DBDerivations {
    id: Option<u64>,
    build_id: u64,
    name: String,
    log: String,
}

impl DBDerivations {
    pub fn new(build_id: u64, name: String, log: String) -> Self {
        DBDerivations {
            id: None,
            build_id,
            name,
            log, 
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
                        id integer not null,
                        flake text not null,
                        attribute text not null,
                        finished date,
                        timeTookSecs int,
                        running boolean not null,
                        success boolean,
                        logs text,

                        primary key (id)
                    );
                    
                    create table if not exists Derivations (
                        id integer not null,
                        buildID int not null,
                        path text not null,
                        output text,

                        primary key (id),
                        foreign key (buildID) references Builds(id)
                    );
                    COMMIT;
                ")
        }).await;

        if result.is_err() {
            return Some(DBError::new(result.err().unwrap().to_string()));
        };

        None
    }


    /// Inserts a DBBuilds object and returns the rowid if successful
    pub async fn insert_build(&self, build: DBBuilds) -> Result<u64, DBError> {
        let flake = build.flake;
        let attribute = build.attribute;
        let finished: String = match build.finished {
            Some(value) => {value.to_rfc3339()},
            None => {"null".to_string()}
        };
        let time_took = convert_to_string(build.time_took);
        let running = build.running.to_string();
        let success = convert_to_string(build.success);
        let logs = build.logs;

        let rowid = self.pool.conn(|conn| {
            let result: Result<u64, _>  = conn.query_row("
                    insert into Builds
                        (flake, attribute, finished, timeTookSecs, running, success, logs)
                        values
                        (?, ?, ?, ?, ?, ?, ?)
                        returning rowid;
                    commit;
                ", [flake, attribute, finished, time_took, running, success, logs], |row| row.get(0));
            if result.is_err(){
                return Err(result.err().unwrap())
            }
            Ok(result.unwrap())
        }).await;

        if rowid.is_err() {
            return Err(DBError::new(rowid.err().unwrap().to_string()));
        };

        Ok(rowid.unwrap())
    }

    pub async fn insert_derivation(&self, derivation: DBDerivations) -> Result<(), DBError> {
        let id = format!("{}", derivation.build_id);
        let name = derivation.name;
        let log = derivation.log;
        let result = self.pool.conn(|conn| {
         conn.execute("insert into Derivations
                 (buildID, path, output)
                 values
                 (?, ?, ?)
             ", [id, name, log])
        }).await;

        if result.is_err() {
            return Err(DBError::new(result.err().unwrap().to_string()));
        }
        Ok(())
    }
}
