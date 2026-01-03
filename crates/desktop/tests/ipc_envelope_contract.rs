use aideon_desktop_lib::{HostError, IpcRequest, IpcResponse};

#[test]
fn ipc_request_deserializes_with_camelcase_fields() {
    let json = r#"{"requestId":"req-1","payload":{"task":"frontend"}}"#;
    let request: IpcRequest<serde_json::Value> = serde_json::from_str(json).expect("deserialize");
    assert_eq!(request.request_id, "req-1");
    assert_eq!(request.payload["task"], "frontend");
}

#[test]
fn ipc_response_serializes_ok_shape() {
    let response = IpcResponse::ok("req-2", serde_json::json!({"value": 1}));
    let json = serde_json::to_string(&response).expect("serialize");
    assert!(json.contains("\"requestId\""));
    assert!(json.contains("\"status\":\"ok\""));
    assert!(json.contains("\"result\""));
    assert!(!json.contains("\"error\""));
}

#[test]
fn ipc_response_serializes_error_shape_with_details() {
    let error = HostError::new("validation_failed", "Bad input");
    let response: IpcResponse<serde_json::Value> = IpcResponse::err("req-3", error);
    let json = serde_json::to_string(&response).expect("serialize");
    assert!(json.contains("\"requestId\""));
    assert!(json.contains("\"status\":\"error\""));
    assert!(!json.contains("\"result\""));
    assert!(json.contains("\"error\""));
    assert!(json.contains("\"details\""));
}
