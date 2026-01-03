//! Meta-model definition, loading, and validation.

mod config;
mod loader;
mod model;
mod registry;
mod types;
mod validation;

pub use config::{MetaModelConfig, MetaModelSource};
pub use registry::MetaModelRegistry;
pub use types::{
    MetaAttribute, MetaAttributeKind, MetaAttributeRules, MetaEnumRule, MetaModelDocument,
    MetaMultiplicity, MetaRelationship, MetaRelationshipValidation, MetaStringRule, MetaType,
    MetaValidationRules,
};
