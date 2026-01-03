//! Shared IPC error types for the Tauri host.
//!
//! M0 contract discipline requires a stable, machine-readable error envelope
//! across all commands (see `docs/ROADMAP.md` and `docs/CONTRACTS-AND-SCHEMAS.md`).

use serde::Serialize;
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
