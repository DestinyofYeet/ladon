use serde::{Deserialize, Serialize};
pub type ProjectID = i32;

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Option<ProjectID>,
    pub name: String,
    pub description: String,
}

#[cfg(feature = "ssr")]
use {
    crate::hydracore::{DBError, DB},
    sqlx::{query, query_as},
    tracing::trace,
};

#[cfg(feature = "ssr")]
impl Project {
    pub async fn get_single(db: &DB, id: i32) -> Result<Option<Project>, DBError> {
        let mut conn = db.get_conn().await?;

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

    pub async fn get_all(db: &DB) -> Result<Vec<Project>, DBError> {
        let mut conn = db.get_conn().await?;

        let projects = query_as::<_, Project>("select * from Projects")
            .fetch_all(&mut *conn)
            .await;

        let projects = projects.map_err(|e| DBError::new(e.to_string()))?;

        Ok(projects)
    }

    pub async fn add_to_db(&mut self, db: &DB) -> Result<(), DBError> {
        let name = &self.name;
        let desc = &self.description;

        let mut conn = db.get_conn().await?;

        let result = query!(
            "
                insert into Projects 
                    (name, description)
                values
                    (?, ?)
                returning id
            ",
            name,
            desc
        )
        .fetch_one(&mut *conn)
        .await;

        if result.is_err() {
            return Err(DBError::new(result.err().unwrap().to_string()));
        }

        self.id = Some(result.unwrap().id as i32);

        Ok(())
    }

    pub async fn delete(&self, db: &DB) -> Result<(), DBError> {
        let mut conn = db.get_conn().await?;

        let id = self.id.unwrap();

        _ = query!(
            "
                delete from Projects
                where id = ?
            ",
            id
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| DBError::new(e.to_string()))?;

        Ok(())
    }

    pub async fn update(&self, db: &DB) -> Result<(), DBError> {
        let mut conn = db.get_conn().await?;

        let id = self.id.unwrap();

        _ = query!(
            "
                update Projects
                set name = ?, description = ?
                where id = ?
            ",
            self.name,
            self.description,
            id
        )
        .execute(&mut *conn)
        .await
        .map_err(|e| DBError::new(e.to_string()))?;

        Ok(())
    }
}
