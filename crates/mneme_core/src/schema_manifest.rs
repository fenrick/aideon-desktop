use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SchemaManifest {
    pub manifest_version: String,
    pub migrations: Vec<String>,
    pub tables: Vec<TableManifest>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableManifest {
    pub name: String,
    pub columns: Vec<ColumnManifest>,
    pub indexes: Vec<IndexManifest>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColumnManifest {
    pub name: String,
    pub logical_type: String,
    pub nullable: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IndexManifest {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}
