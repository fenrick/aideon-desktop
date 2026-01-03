//! Shared health check DTOs surfaced via host/worker IPC.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerHealth {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub timestamp_ms: u64,
}

impl WorkerHealth {
    pub fn healthy(timestamp_ms: u64) -> Self {
        Self {
            ok: true,
            message: None,
            timestamp_ms,
        }
    }

    pub fn degraded(timestamp_ms: u64, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            message: Some(message.into()),
            timestamp_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WorkerHealth;

    #[test]
    fn healthy_serializes_without_message() {
        let health = WorkerHealth::healthy(42);
        let json = serde_json::to_string(&health).expect("serialize");
        assert!(json.contains("\"ok\":true"));
        assert!(!json.contains("message"));
    }

    #[test]
    fn degraded_serializes_message() {
        let health = WorkerHealth::degraded(99, "offline");
        let json = serde_json::to_string(&health).expect("serialize");
        assert!(json.contains("\"message\":\"offline\""));
        let back: WorkerHealth = serde_json::from_str(&json).expect("roundtrip");
        assert!(!back.ok);
        assert_eq!(back.timestamp_ms, 99);
    }

    #[test]
    fn deserializes_expected_contract_shape() {
        // Contract is camelCase; message omitted when healthy
        let payload = r#"{"ok":true,"timestampMs":1234}"#;
        let back: WorkerHealth = serde_json::from_str(payload).expect("deserialize");
        assert!(back.ok);
        assert_eq!(back.timestamp_ms, 1234);
        assert!(back.message.is_none());
    }
}
