//! Configuration for the Praxis engine.

use crate::meta::MetaModelConfig;

#[derive(Clone, Debug)]
pub struct PraxisEngineConfig {
    /// Allow commits with empty change sets. Defaults to `false`.
    pub allow_empty_commits: bool,
    /// Prefix applied to generated commit identifiers. Defaults to `"c"`.
    pub commit_id_prefix: String,
    /// Meta-model configuration controlling schema enforcement.
    pub meta_model: MetaModelConfig,
}

impl Default for PraxisEngineConfig {
    fn default() -> Self {
        Self {
            allow_empty_commits: false,
            commit_id_prefix: "c".into(),
            meta_model: MetaModelConfig::default(),
        }
    }
}
