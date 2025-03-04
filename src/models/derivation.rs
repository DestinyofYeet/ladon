use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use {
    crate::hydracore::{DBError, DB},
    sqlx::query,
};

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Job {
    pub id: Option<i32>,
    pub evaluation_id: i32,
    pub attribute_name: String,
    pub derivation_path: String,
}

#[cfg(feature = "ssr")]
impl Job {
    pub fn new(evaluation_id: i32, attribute_name: String, derivation_path: String) -> Self {
        Self {
            id: None,
            evaluation_id,
            attribute_name,
            derivation_path,
        }
    }

    pub async fn add_to_db(&mut self, db: &DB) -> Result<(), DBError> {
        let mut conn = db.get_conn().await?;

        let result = query!(
            "
                insert into Jobs
                    (evaluation_id, attribute_name, derivation_path)
                values
                    (?, ?, ?)
                returning id
            ",
            self.evaluation_id,
            self.attribute_name,
            self.derivation_path
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| DBError::new(e.to_string()))?;

        self.id = Some(result.id as i32);

        Ok(())
    }
}
