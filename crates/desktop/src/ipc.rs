//! Shared IPC error types for the Tauri host.
//!
//! M0 contract discipline requires a stable, machine-readable error envelope
//! across all commands (see `docs/ROADMAP.md` and `docs/CONTRACTS-AND-SCHEMAS.md`).

use serde::Deserialize;
use serde::Serialize;
use serde_json::{Value, json};
use std::fmt;

/// Stable error envelope returned by host commands.
///
/// - `code` is a stable, machine-readable identifier (snake_case).
/// - `message` is user-facing and may change between releases.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostError {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

impl std::error::Error for HostError {}

impl HostError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new("invalid_input", message)
    }

    pub fn invalid_time(message: impl Into<String>) -> Self {
        Self::new("invalid_time", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new("internal_error", message)
    }
}

/// Canonical IPC request envelope.
///
/// Matches the host design doc contract:
/// `{ requestId: "uuid", payload: { ... } }`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcRequest<T> {
    pub request_id: String,
    pub payload: T,
}

/// Payload used when a command requires a payload object but has no inputs.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmptyPayload {}

/// Canonical IPC response envelope.
///
/// Matches the host design doc contract:
/// `{ requestId, status, result?, error? }`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcResponse<T> {
    pub request_id: String,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<IpcError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub code: &'static str,
    pub message: String,
    pub details: Value,
}

impl From<HostError> for IpcError {
    fn from(value: HostError) -> Self {
        Self {
            code: value.code,
            message: value.message,
            details: json!({}),
        }
    }
}

impl<T> IpcResponse<T> {
    pub fn ok(request_id: impl Into<String>, result: T) -> Self {
        Self {
            request_id: request_id.into(),
            status: "ok",
            result: Some(result),
            error: None,
        }
    }

    pub fn err(request_id: impl Into<String>, error: impl Into<IpcError>) -> Self {
        Self {
            request_id: request_id.into(),
            status: "error",
            result: None,
            error: Some(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ipc_request_deserializes_camel_case_envelope() {
        let raw = json!({
            "requestId": "req-1",
            "payload": {}
        });
        let decoded: IpcRequest<EmptyPayload> =
            serde_json::from_value(raw).expect("decode IpcRequest");
        assert_eq!(decoded.request_id, "req-1");
    }

    #[test]
    fn ipc_response_ok_serializes_camel_case_envelope() {
        let response = IpcResponse::ok("req-1", json!({ "value": 1 }));
        let value = serde_json::to_value(&response).expect("serialize IpcResponse");
        assert_eq!(
            value,
            json!({
                "requestId": "req-1",
                "status": "ok",
                "result": { "value": 1 }
            })
        );
    }

    #[test]
    fn ipc_response_err_serializes_stable_error_envelope() {
        let response: IpcResponse<()> = IpcResponse::err("req-2", HostError::invalid_input("bad"));
        let value = serde_json::to_value(&response).expect("serialize IpcResponse");
        assert_eq!(
            value,
            json!({
                "requestId": "req-2",
                "status": "error",
                "error": {
                    "code": "invalid_input",
                    "message": "bad",
                    "details": {}
                }
            })
        );
    }
}
