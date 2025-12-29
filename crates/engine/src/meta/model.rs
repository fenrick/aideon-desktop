//! Internal meta-model descriptors and rule structures for Praxis.

use crate::error::{PraxisError, PraxisResult};
use crate::meta::{
    MetaAttribute, MetaEnumRule, MetaRelationship, MetaStringRule, MetaType, MetaValidationRules,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug)]
pub(super) struct TypeDescriptor {
    pub(super) attributes: BTreeMap<String, MetaAttribute>,
}

#[derive(Clone, Debug)]
pub(super) struct RelationshipDescriptor {
    pub(super) from: Vec<String>,
    pub(super) to: Vec<String>,
    pub(super) attributes: BTreeMap<String, MetaAttribute>,
}

#[derive(Clone, Debug)]
pub(super) struct AttributeRuleSet {
    pub(super) string_max: Option<usize>,
    pub(super) text_max: Option<usize>,
    pub(super) enum_case_sensitive: bool,
}

impl AttributeRuleSet {
    pub(super) fn from_validation(validation: Option<&MetaValidationRules>) -> Self {
        let attr_rules = validation.and_then(|rules| rules.attributes.as_ref());
        let string_max = attr_rules
            .and_then(|attrs| attrs.string.as_ref())
            .and_then(|rule: &MetaStringRule| rule.max_length);
        let text_max = attr_rules
            .and_then(|attrs| attrs.text.as_ref())
            .and_then(|rule: &MetaStringRule| rule.max_length);
        let enum_case_sensitive = attr_rules
            .and_then(|attrs| attrs.enum_rule.as_ref())
            .and_then(|rule: &MetaEnumRule| rule.case_sensitive)
            .unwrap_or(false);
        Self {
            string_max,
            text_max,
            enum_case_sensitive,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct RelationshipRule {
    pub(super) allow_self: Option<bool>,
    pub(super) allow_duplicate: Option<bool>,
}

pub(super) fn build_type_descriptors(
    types: &[MetaType],
) -> PraxisResult<BTreeMap<String, TypeDescriptor>> {
    let mut by_id: BTreeMap<String, &MetaType> = BTreeMap::new();
    for ty in types {
        by_id.insert(ty.id.clone(), ty);
    }
    let mut cache: HashMap<String, TypeDescriptor> = HashMap::new();
    for ty in types {
        resolve_type_descriptor(ty.id.clone(), &by_id, &mut cache, &mut Vec::new())?;
    }
    Ok(cache.into_iter().collect())
}

fn resolve_type_descriptor(
    id: String,
    types: &BTreeMap<String, &MetaType>,
    cache: &mut HashMap<String, TypeDescriptor>,
    stack: &mut Vec<String>,
) -> PraxisResult<TypeDescriptor> {
    if let Some(desc) = cache.get(&id) {
        return Ok(desc.clone());
    }
    if stack.contains(&id) {
        return Err(PraxisError::IntegrityViolation {
            message: format!("meta-model type inheritance cycle detected at '{id}'"),
        });
    }
    let ty = types
        .get(&id)
        .copied()
        .ok_or_else(|| PraxisError::IntegrityViolation {
            message: format!("meta-model missing type '{id}'"),
        })?;
    stack.push(id.clone());
    let mut attributes: BTreeMap<String, MetaAttribute> = BTreeMap::new();
    if let Some(parent_id) = &ty.extends
        && types.contains_key(parent_id)
    {
        let parent_desc = resolve_type_descriptor(parent_id.clone(), types, cache, stack)?;
        attributes.extend(parent_desc.attributes);
    }
    stack.pop();
    for attribute in &ty.attributes {
        attributes.insert(attribute.name.clone(), attribute.clone());
    }
    let descriptor = TypeDescriptor { attributes };
    cache.insert(id.clone(), descriptor.clone());
    Ok(descriptor)
}

pub(super) fn build_relationship_descriptors(
    relationships: &[MetaRelationship],
) -> BTreeMap<String, RelationshipDescriptor> {
    relationships
        .iter()
        .map(|rel| {
            let mut attrs = BTreeMap::new();
            for attr in &rel.attributes {
                attrs.insert(attr.name.clone(), attr.clone());
            }
            (
                rel.id.clone(),
                RelationshipDescriptor {
                    from: rel.from.clone(),
                    to: rel.to.clone(),
                    attributes: attrs,
                },
            )
        })
        .collect()
}

pub(super) fn relationship_rules(
    validation: Option<&MetaValidationRules>,
) -> HashMap<String, RelationshipRule> {
    validation
        .and_then(|rules| rules.relationships.as_ref())
        .map(|map| {
            map.iter()
                .map(|(id, rule)| {
                    (
                        id.clone(),
                        RelationshipRule {
                            allow_self: rule.allow_self,
                            allow_duplicate: rule.allow_duplicate,
                        },
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::{MetaAttribute, MetaAttributeKind};

    #[test]
    fn build_type_descriptors_inherits_parent_attributes() {
        let parent = MetaType {
            id: "Parent".into(),
            label: None,
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
        let child = MetaType {
            id: "Child".into(),
            label: None,
            category: None,
            extends: Some("Parent".into()),
            attributes: vec![MetaAttribute {
                name: "owner".into(),
                value_type: MetaAttributeKind::String,
                required: false,
                enum_values: vec![],
            }],
            effect_types: None,
        };
        let descriptors = build_type_descriptors(&[parent, child]).unwrap();
        let child_desc = descriptors.get("Child").expect("child desc");
        assert!(child_desc.attributes.contains_key("name"));
        assert!(child_desc.attributes.contains_key("owner"));
    }

    #[test]
    fn build_type_descriptors_rejects_inheritance_cycles() {
        let a = MetaType {
            id: "A".into(),
            label: None,
            category: None,
            extends: Some("B".into()),
            attributes: vec![],
            effect_types: None,
        };
        let b = MetaType {
            id: "B".into(),
            label: None,
            category: None,
            extends: Some("A".into()),
            attributes: vec![],
            effect_types: None,
        };
        let err = build_type_descriptors(&[a, b]).unwrap_err();
        assert!(matches!(err, PraxisError::IntegrityViolation { .. }));
        assert!(err.to_string().contains("cycle"));
    }
}
