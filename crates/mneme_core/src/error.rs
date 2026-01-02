use thiserror::Error;

#[derive(Debug, Error)]
pub enum MnemeError {
    #[error("storage error: {message}")]
    Storage { message: String },
    #[error("not found: {message}")]
    NotFound { message: String },
    #[error("validation error: {message}")]
    Validation { message: String },
    #[error("conflict: {message}")]
    Conflict { message: String },
    #[error("processing error: {message}")]
    Processing { message: String },
    #[error("sync error: {message}")]
    Sync { message: String },
}

impl MnemeError {
    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage {
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    pub fn processing(message: impl Into<String>) -> Self {
        Self::Processing {
            message: message.into(),
        }
    }

    pub fn sync(message: impl Into<String>) -> Self {
        Self::Sync {
            message: message.into(),
        }
    }
}

pub type MnemeResult<T> = Result<T, MnemeError>;

impl From<sea_orm::DbErr> for MnemeError {
    fn from(value: sea_orm::DbErr) -> Self {
        MnemeError::storage(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::MnemeError;

    #[test]
    fn helper_constructors_set_variants() {
        let err = MnemeError::storage("disk");
        assert!(matches!(err, MnemeError::Storage { .. }));
        let err = MnemeError::not_found("missing");
        assert!(matches!(err, MnemeError::NotFound { .. }));
        let err = MnemeError::invalid("bad");
        assert!(matches!(err, MnemeError::Validation { .. }));
        let err = MnemeError::conflict("dup");
        assert!(matches!(err, MnemeError::Conflict { .. }));
        let err = MnemeError::processing("job");
        assert!(matches!(err, MnemeError::Processing { .. }));
        let err = MnemeError::sync("sync");
        assert!(matches!(err, MnemeError::Sync { .. }));
    }
}
