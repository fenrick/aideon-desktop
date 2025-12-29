//! Meta-model configuration and source definitions for Praxis.

use crate::meta::MetaModelDocument;
use std::path::PathBuf;

/// Source definition for loading meta-model documents.
#[derive(Clone, Debug)]
pub enum MetaModelSource {
    /// Built-in `docs/data/meta/core-v1.json` embedded at compile time.
    EmbeddedCore,
    /// Load schema from a file on disk.
    File(PathBuf),
    /// Inline JSON string supplied programmatically.
    Inline(String),
    /// Already parsed document (used by tests).
    Document(MetaModelDocument),
}

/// Configuration describing how the registry should be initialised.
#[derive(Clone, Debug)]
pub struct MetaModelConfig {
    pub base: MetaModelSource,
    pub overrides: Vec<MetaModelSource>,
}

impl Default for MetaModelConfig {
    fn default() -> Self {
        Self {
            base: MetaModelSource::EmbeddedCore,
            overrides: Vec::new(),
        }
    }
}

impl MetaModelConfig {
    pub fn with_base(mut self, base: MetaModelSource) -> Self {
        self.base = base;
        self
    }

    pub fn with_overrides(mut self, overrides: Vec<MetaModelSource>) -> Self {
        self.overrides = overrides;
        self
    }

    pub fn add_override(mut self, source: MetaModelSource) -> Self {
        self.overrides.push(source);
        self
    }
}
