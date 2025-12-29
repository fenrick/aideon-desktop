//! Praxis meta-model document loading and merging.

use crate::error::{PraxisError, PraxisResult};
use crate::meta::config::MetaModelSource;
use crate::meta::{MetaModelDocument, MetaRelationship, MetaType};
use std::collections::HashMap;
use std::fs;

const CORE_META_JSON: &str = include_str!("../../../../docs/data/meta/core-v1.json");

pub(super) fn load_document(source: &MetaModelSource) -> PraxisResult<MetaModelDocument> {
    match source {
        MetaModelSource::EmbeddedCore => parse_document(CORE_META_JSON),
        MetaModelSource::File(path) => {
            let contents =
                fs::read_to_string(path).map_err(|err| PraxisError::IntegrityViolation {
                    message: format!("failed to read meta-model '{}': {err}", path.display()),
                })?;
            parse_document(&contents)
        }
        MetaModelSource::Inline(raw) => parse_document(raw),
        MetaModelSource::Document(doc) => Ok(doc.clone()),
    }
}

fn parse_document(raw: &str) -> PraxisResult<MetaModelDocument> {
    serde_json::from_str(raw).map_err(|err| PraxisError::IntegrityViolation {
        message: format!("failed to parse meta-model: {err}"),
    })
}

pub(super) fn merge_documents(
    mut base: MetaModelDocument,
    overlays: &[MetaModelDocument],
) -> PraxisResult<MetaModelDocument> {
    for layer in overlays {
        if layer.version != base.version {
            return Err(PraxisError::IntegrityViolation {
                message: format!(
                    "meta-model version mismatch: base={} overlay={}",
                    base.version, layer.version
                ),
            });
        }
        merge_types(&mut base.types, &layer.types);
        merge_relationships(&mut base.relationships, &layer.relationships);
        if layer.validation.is_some() {
            base.validation = layer.validation.clone();
        }
        if layer.description.is_some() {
            base.description = layer.description.clone();
        }
    }
    Ok(base)
}

fn merge_types(base: &mut Vec<MetaType>, overrides: &[MetaType]) {
    let mut index: HashMap<String, usize> = base
        .iter()
        .enumerate()
        .map(|(idx, ty)| (ty.id.clone(), idx))
        .collect();
    for ty in overrides {
        if let Some(pos) = index.get(&ty.id).copied() {
            base[pos] = ty.clone();
        } else {
            index.insert(ty.id.clone(), base.len());
            base.push(ty.clone());
        }
    }
}

fn merge_relationships(base: &mut Vec<MetaRelationship>, overrides: &[MetaRelationship]) {
    let mut index: HashMap<String, usize> = base
        .iter()
        .enumerate()
        .map(|(idx, rel)| (rel.id.clone(), idx))
        .collect();
    for rel in overrides {
        if let Some(pos) = index.get(&rel.id).copied() {
            base[pos] = rel.clone();
        } else {
            index.insert(rel.id.clone(), base.len());
            base.push(rel.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::config::MetaModelSource;
    use crate::meta::{MetaAttribute, MetaAttributeKind, MetaModelDocument};

    fn doc(
        version: &str,
        types: Vec<MetaType>,
        relationships: Vec<MetaRelationship>,
    ) -> MetaModelDocument {
        MetaModelDocument {
            version: version.into(),
            description: None,
            types,
            relationships,
            validation: None,
        }
    }

    #[test]
    fn load_inline_document_rejects_invalid_json() {
        let source = MetaModelSource::Inline("{not json".into());
        let err = load_document(&source).unwrap_err();
        assert!(matches!(err, PraxisError::IntegrityViolation { .. }));
        assert!(err.to_string().contains("failed to parse"));
    }

    #[test]
    fn merge_documents_rejects_version_mismatch() {
        let base = doc("v1", vec![], vec![]);
        let overlay = doc("v2", vec![], vec![]);
        let err = merge_documents(base, &[overlay]).unwrap_err();
        assert!(matches!(err, PraxisError::IntegrityViolation { .. }));
        assert!(err.to_string().contains("version mismatch"));
    }

    #[test]
    fn merge_documents_overrides_types_and_relationships_by_id() {
        let base_type = MetaType {
            id: "Capability".into(),
            label: Some("Capability".into()),
            category: None,
            extends: None,
            attributes: vec![MetaAttribute {
                name: "name".into(),
                value_type: MetaAttributeKind::String,
                required: true,
                enum_values: vec![],
            }],
            effect_types: None,
        };
        let overlay_type = MetaType {
            id: "Capability".into(),
            label: Some("Capability (override)".into()),
            category: None,
            extends: None,
            attributes: vec![MetaAttribute {
                name: "owner".into(),
                value_type: MetaAttributeKind::String,
                required: false,
                enum_values: vec![],
            }],
            effect_types: None,
        };
        let base_rel = MetaRelationship {
            id: "supports".into(),
            label: None,
            from: vec!["Capability".into()],
            to: vec!["Capability".into()],
            directed: Some(true),
            multiplicity: None,
            attributes: vec![],
        };
        let overlay_rel = MetaRelationship {
            id: "supports".into(),
            label: Some("Supports".into()),
            from: vec!["Capability".into()],
            to: vec!["Capability".into()],
            directed: Some(false),
            multiplicity: None,
            attributes: vec![],
        };

        let base = doc("v1", vec![base_type], vec![base_rel]);
        let overlay = doc("v1", vec![overlay_type], vec![overlay_rel]);
        let merged = merge_documents(base, &[overlay]).unwrap();
        assert_eq!(merged.types.len(), 1);
        assert_eq!(
            merged.types[0].label.as_deref(),
            Some("Capability (override)")
        );
        assert_eq!(merged.relationships.len(), 1);
        assert_eq!(merged.relationships[0].directed, Some(false));
    }
}
