use serde_json::{Map, Value, json};

use crate::temporal::{ChangeSet, NodeVersion};

pub fn meta_model_seed_change_set() -> ChangeSet {
    let mut change = ChangeSet::default();
    change
        .node_creates
        .extend(META_TYPES.iter().map(node_for_type));
    change
        .node_creates
        .extend(META_RELATIONSHIPS.iter().map(node_for_relationship));
    change
}

fn node_for_type(spec: &MetaTypeSpec) -> NodeVersion {
    let mut props = Map::new();
    props.insert("name".into(), Value::String(spec.label.to_string()));
    props.insert(
        "description".into(),
        Value::String(format!("Definition of {}", spec.label)),
    );
    let payload = json!({
        "id": spec.id,
        "attributes": spec.attributes.iter().map(|attr| attr.name).collect::<Vec<_>>(),
    });
    props.insert("payload".into(), Value::String(payload.to_string()));
    props.insert("label".into(), Value::String(spec.label.to_string()));
    props.insert("category".into(), Value::String(spec.category.to_string()));
    if let Some(parent) = spec.extends {
        props.insert("extends".into(), Value::String(parent.to_string()));
    }
    if !spec.attributes.is_empty() {
        props.insert(
            "attributes".into(),
            Value::Array(spec.attributes.iter().map(attribute_to_json).collect()),
        );
    }
    NodeVersion {
        id: format!("meta:type:{}", spec.id),
        r#type: Some("MetaModelEntry".into()),
        props: Some(Value::Object(props)),
    }
}

fn node_for_relationship(spec: &MetaRelationshipSpec) -> NodeVersion {
    let mut props = Map::new();
    props.insert("kind".into(), Value::String("relationship".into()));
    props.insert("label".into(), Value::String(spec.label.to_string()));
    props.insert("type".into(), Value::String(spec.id.to_string()));
    props.insert(
        "from".into(),
        Value::Array(
            spec.from
                .iter()
                .map(|typ| Value::String(typ.to_string()))
                .collect(),
        ),
    );
    props.insert(
        "to".into(),
        Value::Array(
            spec.to
                .iter()
                .map(|typ| Value::String(typ.to_string()))
                .collect(),
        ),
    );
    let mut details = Map::new();
    details.insert("directed".into(), Value::Bool(spec.directed));
    if let Some((mul_from, mul_to)) = spec.multiplicity {
        details.insert(
            "multiplicity".into(),
            json!({ "from": mul_from, "to": mul_to }),
        );
    }
    if !spec.attributes.is_empty() {
        details.insert(
            "attributes".into(),
            Value::Array(spec.attributes.iter().map(attribute_to_json).collect()),
        );
    }
    props.insert("details".into(), Value::Object(details));
    props.insert(
        "name".into(),
        Value::String(format!("{} relationship", spec.label)),
    );
    props.insert(
        "description".into(),
        Value::String(format!(
            "Allows {} -> {}",
            spec.from.join(", "),
            spec.to.join(", ")
        )),
    );
    let payload = json!({
        "id": spec.id,
        "from": spec.from,
        "to": spec.to,
    });
    props.insert("payload".into(), Value::String(payload.to_string()));
    NodeVersion {
        id: format!("meta:relationship:{}", spec.id),
        r#type: Some("MetaModelEntry".into()),
        props: Some(Value::Object(props)),
    }
}

fn attribute_to_json(spec: &MetaAttributeSpec) -> Value {
    let mut map = Map::new();
    map.insert("name".into(), Value::String(spec.name.to_string()));
    map.insert("type".into(), Value::String(spec.value_type.to_string()));
    map.insert("required".into(), Value::Bool(spec.required));
    if !spec.enum_values.is_empty() {
        map.insert(
            "enum".into(),
            Value::Array(
                spec.enum_values
                    .iter()
                    .map(|&v| Value::String(v.into()))
                    .collect(),
            ),
        );
    }
    Value::Object(map)
}

struct MetaAttributeSpec {
    name: &'static str,
    value_type: &'static str,
    required: bool,
    enum_values: &'static [&'static str],
}

struct MetaTypeSpec {
    id: &'static str,
    label: &'static str,
    category: &'static str,
    extends: Option<&'static str>,
    attributes: &'static [MetaAttributeSpec],
}

struct MetaRelationshipSpec {
    id: &'static str,
    label: &'static str,
    from: &'static [&'static str],
    to: &'static [&'static str],
    directed: bool,
    multiplicity: Option<(&'static str, &'static str)>,
    attributes: &'static [MetaAttributeSpec],
}

const META_TYPES: &[MetaTypeSpec] = &[
    MetaTypeSpec {
        id: "ValueStreamStage",
        label: "Value Stream Stage",
        category: "Business",
        extends: Some("Stage"),
        attributes: &[
            MetaAttributeSpec {
                name: "name",
                value_type: "string",
                required: true,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "purpose",
                value_type: "string",
                required: false,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "owner",
                value_type: "string",
                required: false,
                enum_values: &[],
            },
        ],
    },
    MetaTypeSpec {
        id: "Capability",
        label: "Capability",
        category: "Business",
        extends: None,
        attributes: &[
            MetaAttributeSpec {
                name: "name",
                value_type: "string",
                required: true,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "tier",
                value_type: "enum",
                required: false,
                enum_values: &["Strategic", "Core", "Supporting"],
            },
            MetaAttributeSpec {
                name: "lifecycle",
                value_type: "enum",
                required: false,
                enum_values: &["Target", "Current", "Retire"],
            },
        ],
    },
    MetaTypeSpec {
        id: "BusinessProcess",
        label: "Process",
        category: "Business",
        extends: None,
        attributes: &[
            MetaAttributeSpec {
                name: "name",
                value_type: "string",
                required: true,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "description",
                value_type: "text",
                required: false,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "criticality",
                value_type: "enum",
                required: false,
                enum_values: &["High", "Medium", "Low"],
            },
        ],
    },
    MetaTypeSpec {
        id: "Application",
        label: "Application",
        category: "Application",
        extends: None,
        attributes: &[
            MetaAttributeSpec {
                name: "name",
                value_type: "string",
                required: true,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "vendor",
                value_type: "string",
                required: false,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "disposition",
                value_type: "enum",
                required: false,
                enum_values: &["Invest", "Tolerate", "Migrate", "Eliminate"],
            },
            MetaAttributeSpec {
                name: "lifecycle",
                value_type: "enum",
                required: false,
                enum_values: &["Plan", "Build", "Run", "Retire"],
            },
        ],
    },
    MetaTypeSpec {
        id: "DataEntity",
        label: "Data Entity",
        category: "Information",
        extends: None,
        attributes: &[
            MetaAttributeSpec {
                name: "name",
                value_type: "string",
                required: true,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "sensitivity",
                value_type: "enum",
                required: false,
                enum_values: &["Public", "Internal", "Confidential"],
            },
        ],
    },
    MetaTypeSpec {
        id: "TechnologyComponent",
        label: "Technology Component",
        category: "Technology",
        extends: None,
        attributes: &[
            MetaAttributeSpec {
                name: "name",
                value_type: "string",
                required: true,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "provider",
                value_type: "string",
                required: false,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "deployment",
                value_type: "enum",
                required: false,
                enum_values: &["On-Prem", "IaaS", "PaaS", "SaaS"],
            },
        ],
    },
    MetaTypeSpec {
        id: "PlanEvent",
        label: "Plan Event",
        category: "Planning",
        extends: None,
        attributes: &[
            MetaAttributeSpec {
                name: "name",
                value_type: "string",
                required: true,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "effective_at",
                value_type: "datetime",
                required: true,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "confidence",
                value_type: "number",
                required: false,
                enum_values: &[],
            },
            MetaAttributeSpec {
                name: "source.priority",
                value_type: "enum",
                required: false,
                enum_values: &["P0", "P1", "P2"],
            },
        ],
    },
];

const META_RELATIONSHIPS: &[MetaRelationshipSpec] = &[
    MetaRelationshipSpec {
        id: "serves",
        label: "Serves",
        from: &["Capability"],
        to: &["ValueStreamStage"],
        directed: true,
        multiplicity: Some(("many", "many")),
        attributes: &[],
    },
    MetaRelationshipSpec {
        id: "realises",
        label: "Realises",
        from: &["Application", "TechnologyComponent"],
        to: &["Capability", "BusinessProcess"],
        directed: true,
        multiplicity: Some(("many", "many")),
        attributes: &[],
    },
    MetaRelationshipSpec {
        id: "accesses",
        label: "Accesses",
        from: &["BusinessProcess", "Application"],
        to: &["DataEntity"],
        directed: true,
        multiplicity: None,
        attributes: &[MetaAttributeSpec {
            name: "mode",
            value_type: "enum",
            required: true,
            enum_values: &["read", "write", "readwrite"],
        }],
    },
    MetaRelationshipSpec {
        id: "hosts",
        label: "Hosts",
        from: &["TechnologyComponent"],
        to: &["Application"],
        directed: true,
        multiplicity: Some(("many", "many")),
        attributes: &[],
    },
    MetaRelationshipSpec {
        id: "plan_effect",
        label: "Plan Event Effect",
        from: &["PlanEvent"],
        to: &[
            "Capability",
            "BusinessProcess",
            "Application",
            "TechnologyComponent",
        ],
        directed: true,
        multiplicity: None,
        attributes: &[
            MetaAttributeSpec {
                name: "op",
                value_type: "enum",
                required: true,
                enum_values: &["create", "update", "delete", "link", "unlink"],
            },
            MetaAttributeSpec {
                name: "target_ref",
                value_type: "string",
                required: true,
                enum_values: &[],
            },
        ],
    },
];
