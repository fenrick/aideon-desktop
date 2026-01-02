//! Praxis meta-model schema registry.

use crate::error::PraxisResult;
use crate::meta::MetaModelDocument;
use crate::meta::config::MetaModelConfig;
use crate::meta::loader::{load_document, merge_documents};
use crate::meta::model::{
    AttributeRuleSet, RelationshipDescriptor, RelationshipRule, TypeDescriptor,
    build_relationship_descriptors, build_type_descriptors, relationship_rules,
};
use crate::meta::validation::{validate_edge, validate_node};
use crate::temporal::{EdgeVersion, NodeVersion};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Materialised schema registry used by the Praxis engine for validation.
pub struct MetaModelRegistry {
    document: Arc<MetaModelDocument>,
    types: BTreeMap<String, TypeDescriptor>,
    relationships: BTreeMap<String, RelationshipDescriptor>,
    attr_rules: AttributeRuleSet,
    relationship_rules: HashMap<String, RelationshipRule>,
}

impl MetaModelRegistry {
    pub fn load(config: &MetaModelConfig) -> PraxisResult<Self> {
        let mut docs = Vec::new();
        for source in std::iter::once(&config.base).chain(config.overrides.iter()) {
            docs.push(load_document(source)?);
        }
        let (base, overlays) =
            docs.split_first()
                .ok_or_else(|| crate::error::PraxisError::IntegrityViolation {
                    message: "meta-model config missing base document".into(),
                })?;
        let merged = merge_documents(base.clone(), overlays)?;
        Self::from_document(merged)
    }

    pub fn embedded() -> PraxisResult<Self> {
        Self::load(&MetaModelConfig::default())
    }

    pub fn from_document(doc: MetaModelDocument) -> PraxisResult<Self> {
        let attr_rules = AttributeRuleSet::from_validation(doc.validation.as_ref());
        let relationship_rules = relationship_rules(doc.validation.as_ref());
        let type_descriptors = build_type_descriptors(&doc.types)?;
        let relationship_descriptors = build_relationship_descriptors(&doc.relationships);
        Ok(Self {
            document: Arc::new(doc),
            types: type_descriptors,
            relationships: relationship_descriptors,
            attr_rules,
            relationship_rules,
        })
    }

    pub fn document(&self) -> MetaModelDocument {
        (*self.document).clone()
    }

    pub fn validate_node(&self, node: &NodeVersion) -> PraxisResult<()> {
        validate_node(node, &self.types, &self.attr_rules)
    }

    pub fn validate_edge(
        &self,
        edge: &EdgeVersion,
        from_type: &str,
        to_type: &str,
    ) -> PraxisResult<()> {
        validate_edge(
            edge,
            from_type,
            to_type,
            &self.relationships,
            &self.relationship_rules,
            &self.attr_rules,
        )
    }

    pub fn allows_duplicate(&self, rel_type: &str) -> bool {
        self.relationship_rules
            .get(rel_type)
            .and_then(|rule| rule.allow_duplicate)
            .unwrap_or(true)
    }
}
