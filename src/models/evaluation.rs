use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use {
    crate::hydracore::{DBError, DB},
    sqlx::query,
};

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Evaluation {
    pub id: Option<i32>,
    pub jobset_id: i32,
}

#[cfg(feature = "ssr")]
impl Evaluation {
    pub fn new(jobset_id: i32) -> Self {
        Self {
            id: None,
            jobset_id,
        }
    }

    pub async fn add_to_db(&mut self, db: &DB) -> Result<(), DBError> {
        let mut conn = db.get_conn().await?;

        let result = query!(
            "
                insert into Evaluations
                    (jobset_id)
                values
                    (?)
                returning id
                
            ",
            self.jobset_id
        )
        .fetch_one(&mut *conn)
        .await;

        if result.is_err() {
            return Err(DBError::new(result.err().unwrap().to_string()));
        }

        let result = result.unwrap();

        self.id = Some(result.id as i32);
        Ok(())
    }
}
