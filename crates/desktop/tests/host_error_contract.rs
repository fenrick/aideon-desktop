use aideon_desktop_lib::HostError;

#[test]
fn host_error_serializes_as_stable_envelope() {
    let error = HostError::new("validation_failed", "Bad input");
    let json = serde_json::to_string(&error).expect("serialize");
    assert!(json.contains("\"code\""));
    assert!(json.contains("\"message\""));
    assert!(json.contains("validation_failed"));
    assert!(json.contains("Bad input"));
}
