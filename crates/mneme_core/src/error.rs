use thiserror::Error;

#[derive(Debug, Error)]
pub enum MnemeError {
    #[error("storage error: {message}")]
    Storage { message: String },
    #[error("not found: {message}")]
    NotFound { message: String },
    #[error("invalid input: {message}")]
    InvalidInput { message: String },
    #[error("conflict: {message}")]
    Conflict { message: String },
    #[error("not implemented: {message}")]
    NotImplemented { message: String },
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
        Self::InvalidInput {
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::NotImplemented {
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
