use serde::{Deserialize, Serialize};

pub type ProjectID = i32;

#[cfg_attr(feature = "ssr", derive(sqlx::FromRow))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectID,
    pub name: String,
    pub description: String,
}
