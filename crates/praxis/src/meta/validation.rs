//! Praxis meta-model validation logic.

use crate::error::{PraxisError, PraxisResult};
use crate::meta::model::{AttributeRuleSet, RelationshipRule, TypeDescriptor};
use crate::meta::{MetaAttribute, MetaAttributeKind};
use crate::temporal::{EdgeVersion, NodeVersion};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::model::RelationshipDescriptor;

pub(super) fn validate_node(
    node: &NodeVersion,
    types: &BTreeMap<String, TypeDescriptor>,
    attr_rules: &AttributeRuleSet,
) -> PraxisResult<()> {
    let type_id = node
        .r#type
        .as_ref()
        .ok_or_else(|| PraxisError::ValidationFailed {
            message: format!("node '{}' missing type", node.id),
        })?;
    let descriptor = types
        .get(type_id)
        .ok_or_else(|| PraxisError::ValidationFailed {
            message: format!("node '{}' references unknown type '{type_id}'", node.id),
        })?;
    if let Some(props) = node.props.as_ref() {
        let map = props
            .as_object()
            .ok_or_else(|| PraxisError::ValidationFailed {
                message: format!("node '{}' props must be an object", node.id),
            })?;
        validate_attributes(&descriptor.attributes, map, attr_rules, |attr, reason| {
            format!("node '{}' attribute '{}': {}", node.id, attr.name, reason)
        })?;
    } else if descriptor.attributes.values().any(|attr| attr.required) {
        return Err(PraxisError::ValidationFailed {
            message: format!(
                "node '{}' missing required attributes for type '{}'",
                node.id, type_id
            ),
        });
    }

    Ok(())
}

pub(super) fn validate_edge(
    edge: &EdgeVersion,
    from_type: &str,
    to_type: &str,
    relationships: &BTreeMap<String, RelationshipDescriptor>,
    relationship_rules: &HashMap<String, RelationshipRule>,
    attr_rules: &AttributeRuleSet,
) -> PraxisResult<()> {
    let rel_type = edge
        .r#type
        .as_ref()
        .ok_or_else(|| PraxisError::ValidationFailed {
            message: format!(
                "edge '{}->{}' missing relationship type",
                edge.from, edge.to
            ),
        })?;
    let descriptor = relationships
        .get(rel_type)
        .ok_or_else(|| PraxisError::ValidationFailed {
            message: format!("edge uses unknown relationship '{rel_type}'"),
        })?;

    if !descriptor.from.iter().any(|ty| ty == from_type) {
        return Err(PraxisError::ValidationFailed {
            message: format!("edge type '{rel_type}' cannot originate from '{from_type}'"),
        });
    }
    if !descriptor.to.iter().any(|ty| ty == to_type) {
        return Err(PraxisError::ValidationFailed {
            message: format!("edge type '{rel_type}' cannot target '{to_type}'"),
        });
    }

    if let Some(rule) = relationship_rules.get(rel_type)
        && rule.allow_self == Some(false)
        && edge.from == edge.to
    {
        return Err(PraxisError::ValidationFailed {
            message: format!("relationship '{rel_type}' forbids self-links"),
        });
    }

    if let Some(props) = edge.props.as_ref() {
        let map = props
            .as_object()
            .ok_or_else(|| PraxisError::ValidationFailed {
                message: format!("edge '{}->{}' props must be an object", edge.from, edge.to),
            })?;
        validate_attributes(&descriptor.attributes, map, attr_rules, |attr, reason| {
            format!(
                "edge '{}->{}' attribute '{}': {}",
                edge.from, edge.to, attr.name, reason
            )
        })?;
    } else if descriptor.attributes.values().any(|attr| attr.required) {
        return Err(PraxisError::ValidationFailed {
            message: format!("edge type '{rel_type}' missing required attributes"),
        });
    }

    Ok(())
}

fn validate_attributes(
    expected: &BTreeMap<String, MetaAttribute>,
    provided: &serde_json::Map<String, Value>,
    rules: &AttributeRuleSet,
    context: impl Fn(&MetaAttribute, &str) -> String,
) -> PraxisResult<()> {
    for attr in expected.values() {
        if let Some(value) = provided.get(&attr.name) {
            if value.is_null() {
                return Err(PraxisError::ValidationFailed {
                    message: context(attr, "cannot be null"),
                });
            }
            attribute_value_ok(attr, value, rules, |reason| context(attr, &reason))?;
        } else if attr.required {
            return Err(PraxisError::ValidationFailed {
                message: context(attr, "is required"),
            });
        }
    }
    Ok(())
}

fn attribute_value_ok(
    attr: &MetaAttribute,
    value: &Value,
    rules: &AttributeRuleSet,
    format_error: impl Fn(String) -> String,
) -> PraxisResult<()> {
    match attr.value_type {
        MetaAttributeKind::String | MetaAttributeKind::Text => {
            validate_text(attr, value, rules, format_error)?
        }
        MetaAttributeKind::Number => validate_number(value, &format_error)?,
        MetaAttributeKind::Boolean => validate_boolean(value, &format_error)?,
        MetaAttributeKind::Enum => validate_enum(attr, value, rules, format_error)?,
        MetaAttributeKind::Datetime => validate_datetime(value, format_error)?,
        MetaAttributeKind::Blob => validate_blob(value, format_error)?,
    }
    Ok(())
}

fn validate_text(
    attr: &MetaAttribute,
    value: &Value,
    rules: &AttributeRuleSet,
    format_error: impl Fn(String) -> String,
) -> PraxisResult<()> {
    let text = value
        .as_str()
        .ok_or_else(|| PraxisError::ValidationFailed {
            message: format_error("expected string".into()),
        })?;
    let max = if attr.value_type == MetaAttributeKind::Text {
        rules.text_max
    } else {
        rules.string_max
    };
    if let Some(limit) = max
        && text.chars().count() > limit
    {
        return Err(PraxisError::ValidationFailed {
            message: format_error(format!(
                "exceeds max length {} ({} chars)",
                limit,
                text.chars().count()
            )),
        });
    }
    Ok(())
}

fn validate_number(value: &Value, format_error: &impl Fn(String) -> String) -> PraxisResult<()> {
    if value.is_number() {
        Ok(())
    } else {
        Err(PraxisError::ValidationFailed {
            message: format_error("expected number".into()),
        })
    }
}

fn validate_boolean(value: &Value, format_error: &impl Fn(String) -> String) -> PraxisResult<()> {
    if value.is_boolean() {
        Ok(())
    } else {
        Err(PraxisError::ValidationFailed {
            message: format_error("expected boolean".into()),
        })
    }
}

fn validate_enum(
    attr: &MetaAttribute,
    value: &Value,
    rules: &AttributeRuleSet,
    format_error: impl Fn(String) -> String,
) -> PraxisResult<()> {
    let text = value
        .as_str()
        .ok_or_else(|| PraxisError::ValidationFailed {
            message: format_error("expected string value for enum".into()),
        })?;
    let case_sensitive = rules.enum_case_sensitive;
    let matches = attr.enum_values.iter().any(|variant| {
        if case_sensitive {
            variant == text
        } else {
            variant.eq_ignore_ascii_case(text)
        }
    });
    if matches {
        Ok(())
    } else {
        Err(PraxisError::ValidationFailed {
            message: format_error(format!(
                "value '{}' not in [{}]",
                text,
                attr.enum_values.join(", ")
            )),
        })
    }
}

fn validate_datetime(value: &Value, format_error: impl Fn(String) -> String) -> PraxisResult<()> {
    let text = value
        .as_str()
        .ok_or_else(|| PraxisError::ValidationFailed {
            message: format_error("expected ISO-8601 string".into()),
        })?;
    if OffsetDateTime::parse(text, &Rfc3339).is_err() {
        return Err(PraxisError::ValidationFailed {
            message: format_error("invalid RFC3339 timestamp".into()),
        });
    }
    Ok(())
}

fn validate_blob(value: &Value, format_error: impl Fn(String) -> String) -> PraxisResult<()> {
    if value.is_string() || value.is_object() || value.is_array() {
        Ok(())
    } else {
        Err(PraxisError::ValidationFailed {
            message: format_error("expected string/structured blob".into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::{MetaAttribute, MetaAttributeKind};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn rules() -> AttributeRuleSet {
        AttributeRuleSet {
            string_max: Some(5),
            text_max: Some(10),
            enum_case_sensitive: false,
        }
    }

    fn string_attr(name: &str, required: bool) -> MetaAttribute {
        MetaAttribute {
            name: name.into(),
            value_type: MetaAttributeKind::String,
            required,
            enum_values: vec![],
        }
    }

    fn enum_attr(name: &str, variants: &[&str]) -> MetaAttribute {
        MetaAttribute {
            name: name.into(),
            value_type: MetaAttributeKind::Enum,
            required: true,
            enum_values: variants.iter().map(|v| (*v).to_string()).collect(),
        }
    }

    #[test]
    fn validate_node_rejects_missing_type() {
        let node = NodeVersion {
            id: "n1".into(),
            r#type: None,
            props: None,
        };
        let types = BTreeMap::<String, TypeDescriptor>::new();
        let err = validate_node(&node, &types, &rules()).unwrap_err();
        assert!(matches!(err, PraxisError::ValidationFailed { .. }));
        assert!(err.to_string().contains("missing type"));
    }

    #[test]
    fn validate_node_checks_required_attributes_and_types() {
        let mut types = BTreeMap::<String, TypeDescriptor>::new();
        let mut attrs = BTreeMap::new();
        attrs.insert("name".into(), string_attr("name", true));
        types.insert("Capability".into(), TypeDescriptor { attributes: attrs });

        let node = NodeVersion {
            id: "n1".into(),
            r#type: Some("Capability".into()),
            props: None,
        };
        let err = validate_node(&node, &types, &rules()).unwrap_err();
        assert!(err.to_string().contains("missing required attributes"));

        let node = NodeVersion {
            id: "n1".into(),
            r#type: Some("Capability".into()),
            props: Some(json!({"name": "ok"})),
        };
        validate_node(&node, &types, &rules()).unwrap();
    }

    #[test]
    fn validate_node_rejects_null_or_overlong_string_values() {
        let mut types = BTreeMap::<String, TypeDescriptor>::new();
        let mut attrs = BTreeMap::new();
        attrs.insert("name".into(), string_attr("name", true));
        types.insert("Capability".into(), TypeDescriptor { attributes: attrs });

        let node = NodeVersion {
            id: "n1".into(),
            r#type: Some("Capability".into()),
            props: Some(json!({"name": null})),
        };
        let err = validate_node(&node, &types, &rules()).unwrap_err();
        assert!(err.to_string().contains("cannot be null"));

        let node = NodeVersion {
            id: "n1".into(),
            r#type: Some("Capability".into()),
            props: Some(json!({"name": "too-long"})),
        };
        let err = validate_node(&node, &types, &rules()).unwrap_err();
        assert!(err.to_string().contains("exceeds max length"));
    }

    #[test]
    fn validate_edge_enforces_endpoints_and_enum_values() {
        let mut relationships = BTreeMap::<String, RelationshipDescriptor>::new();
        relationships.insert(
            "rel_aa".into(),
            RelationshipDescriptor {
                from: vec!["A".into()],
                to: vec!["A".into()],
                attributes: BTreeMap::new(),
            },
        );
        let mut attrs = BTreeMap::new();
        attrs.insert("state".into(), enum_attr("state", &["open", "closed"]));
        relationships.insert(
            "rel_ab".into(),
            RelationshipDescriptor {
                from: vec!["A".into()],
                to: vec!["B".into()],
                attributes: attrs,
            },
        );
        let ruleset = rules();
        let mut rel_rules = HashMap::<String, RelationshipRule>::new();
        rel_rules.insert(
            "rel_aa".into(),
            RelationshipRule {
                allow_self: Some(false),
                allow_duplicate: None,
            },
        );
        rel_rules.insert(
            "rel_ab".into(),
            RelationshipRule {
                allow_self: Some(false),
                allow_duplicate: None,
            },
        );

        let edge_self = EdgeVersion {
            id: Some("e1".into()),
            from: "a".into(),
            to: "a".into(),
            r#type: Some("rel_aa".into()),
            directed: None,
            props: None,
        };
        let err =
            validate_edge(&edge_self, "A", "A", &relationships, &rel_rules, &ruleset).unwrap_err();
        assert!(err.to_string().contains("forbids self-links"));

        let edge_enum_ok = EdgeVersion {
            id: Some("e2".into()),
            from: "a".into(),
            to: "b".into(),
            r#type: Some("rel_ab".into()),
            directed: None,
            props: Some(json!({"state": "OPEN"})),
        };
        validate_edge(
            &edge_enum_ok,
            "A",
            "B",
            &relationships,
            &rel_rules,
            &ruleset,
        )
        .unwrap();

        let edge_enum_bad = EdgeVersion {
            props: Some(json!({"state": "invalid"})),
            ..edge_enum_ok
        };
        let err = validate_edge(
            &edge_enum_bad,
            "A",
            "B",
            &relationships,
            &rel_rules,
            &ruleset,
        )
        .unwrap_err();
        assert!(err.to_string().contains("not in"));
    }
}
