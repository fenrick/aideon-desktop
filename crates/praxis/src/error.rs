use thiserror::Error;

/// Structured error codes emitted by the Praxis engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PraxisErrorCode {
    UnknownBranch,
    UnknownCommit,
    ConcurrencyConflict,
    ValidationFailed,
    IntegrityViolation,
    MergeConflict,
}

/// Result alias used throughout the engine.
pub type PraxisResult<T> = Result<T, PraxisError>;

/// Domain error type describing failure modes surfaced to the host.
#[derive(Debug, Error)]
pub enum PraxisError {
    #[error("unknown branch '{branch}'")]
    UnknownBranch { branch: String },

    #[error("unknown commit '{commit}'")]
    UnknownCommit { commit: String },

    #[error(
        "concurrency conflict on branch '{branch}': expected head {expected:?}, actual {actual:?}"
    )]
    ConcurrencyConflict {
        branch: String,
        expected: Option<String>,
        actual: Option<String>,
    },

    #[error("validation failed: {message}")]
    ValidationFailed { message: String },

    #[error("integrity violation: {message}")]
    IntegrityViolation { message: String },

    #[error("merge produced conflicts: {message}")]
    MergeConflict { message: String },
}

impl PraxisError {
    /// Machine-friendly error code for host serialization.
    pub fn code(&self) -> PraxisErrorCode {
        match self {
            Self::UnknownBranch { .. } => PraxisErrorCode::UnknownBranch,
            Self::UnknownCommit { .. } => PraxisErrorCode::UnknownCommit,
            Self::ConcurrencyConflict { .. } => PraxisErrorCode::ConcurrencyConflict,
            Self::ValidationFailed { .. } => PraxisErrorCode::ValidationFailed,
            Self::IntegrityViolation { .. } => PraxisErrorCode::IntegrityViolation,
            Self::MergeConflict { .. } => PraxisErrorCode::MergeConflict,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_stable() {
        assert_eq!(
            PraxisError::UnknownBranch {
                branch: "main".into()
            }
            .code(),
            PraxisErrorCode::UnknownBranch
        );
        assert_eq!(
            PraxisError::UnknownCommit {
                commit: "c1".into()
            }
            .code(),
            PraxisErrorCode::UnknownCommit
        );
        assert_eq!(
            PraxisError::ConcurrencyConflict {
                branch: "dev".into(),
                expected: Some("a".into()),
                actual: Some("b".into())
            }
            .code(),
            PraxisErrorCode::ConcurrencyConflict
        );
        assert_eq!(
            PraxisError::ValidationFailed {
                message: "x".into()
            }
            .code(),
            PraxisErrorCode::ValidationFailed
        );
        assert_eq!(
            PraxisError::IntegrityViolation {
                message: "x".into()
            }
            .code(),
            PraxisErrorCode::IntegrityViolation
        );
        assert_eq!(
            PraxisError::MergeConflict {
                message: "x".into()
            }
            .code(),
            PraxisErrorCode::MergeConflict
        );
    }
}
