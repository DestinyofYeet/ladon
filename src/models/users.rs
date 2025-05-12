use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use {
    crate::hydracore::{DBError, DB},
    sqlx::query,
    uuid::Uuid,
};

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Option<i32>,
    pub name: String,
}

#[cfg(feature = "ssr")]
impl User {
    /// Returns the token for the user
    pub async fn add_to_db(&self, db: &DB) -> Result<String, DBError> {
        let mut conn = db.get_conn().await?;

        let result = query!(
            "
                insert into Users
                    (name)
                values
                    (?)
                returning id
            ",
            self.name,
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| DBError::new(e.to_string()))?;

        let token = Uuid::new_v4().to_string();

        let result = query!(
            "
                insert into Users_Tokens
                    (user_id, token)
                values
                    (?, ?)
            ",
            result.id,
            token,
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| DBError::new(e.to_string()))?;

        Ok(token)
    }
    pub async fn by_token(db: &DB, token: String) -> Result<Option<User>, DBError> {
        let mut conn = db.get_conn().await?;

        let result = query!(
            "
                select user_id
                from Users_Tokens
                where token = ?    
            ",
            token
        )
        .fetch_optional(&mut *conn)
        .await
        .map_err(|e| DBError::new(e.to_string()))?;

        if result.is_none() {
            return Ok(None);
        }

        let result = result.unwrap();

        let result = query!(
            "
                select *
                from Users
                where id = ?
            ",
            result.user_id
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| DBError::new(e.to_string()))?;

        return Ok(Some(User {
            id: Some(result.id as i32),
            name: result.name,
        }));
    }
    pub fn is_valid(db: &DB, token: String) {}
}
