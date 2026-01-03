use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::{MnemeError, MnemeResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Id(pub [u8; 16]);

impl Id {
    pub fn new() -> Self {
        Self(*Uuid::new_v4().as_bytes())
    }

    pub fn from_uuid_str(value: &str) -> MnemeResult<Self> {
        let uuid = Uuid::parse_str(value)
            .map_err(|err| MnemeError::invalid(format!("invalid uuid '{value}': {err}")))?;
        Ok(Self(*uuid.as_bytes()))
    }

    pub fn from_ulid_str(value: &str) -> MnemeResult<Self> {
        let ulid = ulid::Ulid::from_string(value)
            .map_err(|err| MnemeError::invalid(format!("invalid ulid '{value}': {err}")))?;
        Ok(Self(ulid.to_bytes()))
    }

    pub fn to_uuid_string(self) -> String {
        Uuid::from_bytes(self.0).to_string()
    }

    pub fn to_ulid_string(self) -> String {
        ulid::Ulid::from_bytes(self.0).to_string()
    }

    pub fn as_bytes(self) -> [u8; 16] {
        self.0
    }

    pub fn as_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let uuid = Uuid::from_bytes(self.0);
        write!(f, "{uuid}")
    }
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_uuid_string())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if let Ok(uuid) = Uuid::parse_str(&value) {
            return Ok(Id::from_bytes(*uuid.as_bytes()));
        }
        if let Ok(ulid) = ulid::Ulid::from_string(&value) {
            return Ok(Id::from_bytes(ulid.to_bytes()));
        }
        Err(serde::de::Error::custom("invalid Id string"))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct PartitionId(pub Id);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ActorId(pub Id);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct OpId(pub Id);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ScenarioId(pub Id);

macro_rules! id_wrapper_serde {
    ($name:ident) => {
        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Ok(Self(Id::deserialize(deserializer)?))
            }
        }
    };
}

id_wrapper_serde!(PartitionId);
id_wrapper_serde!(ActorId);
id_wrapper_serde!(OpId);
id_wrapper_serde!(ScenarioId);

#[cfg(test)]
mod tests {
    use super::Id;

    #[test]
    fn id_roundtrips_uuid_and_ulid() {
        let id = Id::new();
        let uuid = id.to_uuid_string();
        let ulid = id.to_ulid_string();
        let from_uuid = Id::from_uuid_str(&uuid).expect("uuid parse");
        let from_ulid = Id::from_ulid_str(&ulid).expect("ulid parse");
        assert_eq!(from_uuid.as_bytes(), id.as_bytes());
        assert_eq!(from_ulid.as_bytes(), id.as_bytes());
    }

    #[test]
    fn id_rejects_invalid_strings() {
        assert!(Id::from_uuid_str("not-a-uuid").is_err());
        assert!(Id::from_ulid_str("not-a-ulid").is_err());
    }
}
