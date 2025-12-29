use aideon_mneme_core::{MnemeError, MnemeResult, SchemaManifest};

pub fn schema_manifest_json() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/schema_manifest.json"))
}

pub fn load_schema_manifest() -> MnemeResult<SchemaManifest> {
    serde_json::from_str(schema_manifest_json())
        .map_err(|err| MnemeError::storage(format!("schema manifest parse: {err}")))
}
