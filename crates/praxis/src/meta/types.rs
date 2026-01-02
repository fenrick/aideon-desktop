//! Canonical meta-model document types for Praxis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaModelDocument {
    pub version: String,
    pub description: Option<String>,
    pub types: Vec<MetaType>,
    pub relationships: Vec<MetaRelationship>,
    pub validation: Option<MetaValidationRules>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaType {
    pub id: String,
    pub label: Option<String>,
    pub category: Option<String>,
    pub extends: Option<String>,
    #[serde(default)]
    pub attributes: Vec<MetaAttribute>,
    #[serde(rename = "effectTypes", default)]
    pub effect_types: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetaAttribute {
    pub name: String,
    #[serde(rename = "type")]
    pub value_type: MetaAttributeKind,
    #[serde(default)]
    pub required: bool,
    #[serde(rename = "enum", default)]
    pub enum_values: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MetaAttributeKind {
    String,
    Text,
    Number,
    Boolean,
    Enum,
    Datetime,
    Blob,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaRelationship {
    pub id: String,
    pub label: Option<String>,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub directed: Option<bool>,
    pub multiplicity: Option<MetaMultiplicity>,
    #[serde(default)]
    pub attributes: Vec<MetaAttribute>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaMultiplicity {
    pub from: String,
    pub to: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaValidationRules {
    pub attributes: Option<MetaAttributeRules>,
    pub relationships: Option<HashMap<String, MetaRelationshipValidation>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaAttributeRules {
    pub string: Option<MetaStringRule>,
    pub text: Option<MetaStringRule>,
    #[serde(rename = "enum")]
    pub enum_rule: Option<MetaEnumRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaStringRule {
    #[serde(rename = "maxLength")]
    pub max_length: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaEnumRule {
    #[serde(rename = "caseSensitive")]
    pub case_sensitive: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaRelationshipValidation {
    #[serde(rename = "allowSelf")]
    pub allow_self: Option<bool>,
    #[serde(rename = "allowDuplicate")]
    pub allow_duplicate: Option<bool>,
}
