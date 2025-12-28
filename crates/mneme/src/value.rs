use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{Id, ValidTime};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Layer {
    Plan = 10,
    Actual = 20,
}

impl Layer {
    pub fn default_actual() -> Self {
        Self::Actual
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EntityKind {
    Node = 1,
    Edge = 2,
}

impl EntityKind {
    pub fn as_i16(self) -> i16 {
        self as i16
    }

    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(EntityKind::Node),
            2 => Some(EntityKind::Edge),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ValueType {
    Str = 1,
    I64 = 2,
    F64 = 3,
    Bool = 4,
    Time = 5,
    Ref = 6,
    Blob = 7,
    Json = 8,
}

impl ValueType {
    pub fn as_i16(self) -> i16 {
        self as i16
    }

    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(ValueType::Str),
            2 => Some(ValueType::I64),
            3 => Some(ValueType::F64),
            4 => Some(ValueType::Bool),
            5 => Some(ValueType::Time),
            6 => Some(ValueType::Ref),
            7 => Some(ValueType::Blob),
            8 => Some(ValueType::Json),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MergePolicy {
    Lww = 1,
    Mv = 2,
    OrSet = 3,
    Counter = 4,
    Text = 5,
}

impl MergePolicy {
    pub fn as_i16(self) -> i16 {
        self as i16
    }

    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            1 => Some(MergePolicy::Lww),
            2 => Some(MergePolicy::Mv),
            3 => Some(MergePolicy::OrSet),
            4 => Some(MergePolicy::Counter),
            5 => Some(MergePolicy::Text),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Str(String),
    I64(i64),
    F64(f64),
    Bool(bool),
    Time(ValidTime),
    Ref(Id),
    Blob(Vec<u8>),
    Json(JsonValue),
}

impl Value {
    pub fn value_type(&self) -> ValueType {
        match self {
            Value::Str(_) => ValueType::Str,
            Value::I64(_) => ValueType::I64,
            Value::F64(_) => ValueType::F64,
            Value::Bool(_) => ValueType::Bool,
            Value::Time(_) => ValueType::Time,
            Value::Ref(_) => ValueType::Ref,
            Value::Blob(_) => ValueType::Blob,
            Value::Json(_) => ValueType::Json,
        }
    }
}
