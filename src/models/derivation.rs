use super::JobsetID;

#[derive(Debug, Clone)]
pub struct Derivation {
    pub id: Option<i32>,
    pub jobset_id: i32,
    pub attribute_name: String,
    pub derivation_path: String,
}

impl Derivation {
    pub fn new(jobset_id: JobsetID, attribute_name: String, derivation_path: String) -> Self {
        Self {
            id: None,
            jobset_id,
            attribute_name,
            derivation_path,
        }
    }
}
