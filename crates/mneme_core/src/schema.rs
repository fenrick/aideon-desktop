use serde::{Deserialize, Serialize};

use crate::{EntityKind, Id, MergePolicy, Value, ValueType};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeDef {
    pub type_id: Id,
    pub applies_to: EntityKind,
    pub label: String,
    pub is_abstract: bool,
    pub parent_type_id: Option<Id>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldDef {
    pub field_id: Id,
    pub label: String,
    pub value_type: ValueType,
    pub cardinality_multi: bool,
    pub merge_policy: MergePolicy,
    pub is_indexed: bool,
    #[serde(default)]
    pub disallow_overlap: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeFieldDef {
    pub type_id: Id,
    pub field_id: Id,
    pub is_required: bool,
    pub default_value: Option<Value>,
    pub override_default: bool,
    pub tighten_required: bool,
    pub disallow_overlap: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeTypeRule {
    pub edge_type_id: Id,
    pub allowed_src_type_ids: Vec<Id>,
    pub allowed_dst_type_ids: Vec<Id>,
    pub semantic_direction: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetamodelBatch {
    pub types: Vec<TypeDef>,
    pub fields: Vec<FieldDef>,
    pub type_fields: Vec<TypeFieldDef>,
    pub edge_type_rules: Vec<EdgeTypeRule>,
    pub metamodel_version: Option<String>,
    pub metamodel_source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub schema_version_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectiveField {
    pub field_id: Id,
    pub value_type: ValueType,
    pub cardinality_multi: bool,
    pub merge_policy: MergePolicy,
    pub is_required: bool,
    pub default_value: Option<Value>,
    pub is_indexed: bool,
    pub disallow_overlap: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EffectiveSchema {
    pub type_id: Id,
    pub applies_to: EntityKind,
    pub fields: Vec<EffectiveField>,
}
