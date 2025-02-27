use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
}

impl Project {
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description
        }
    }
}
