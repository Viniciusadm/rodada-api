use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    Configuration(String),
    Conflict(String),
    Forbidden(String),
    Internal(String),
    NotFound(String),
    Persistence(String),
    Unauthorized(String),
    Validation(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(message) => write!(f, "configuration error: {message}"),
            Self::Conflict(message) => write!(f, "conflict: {message}"),
            Self::Forbidden(message) => write!(f, "forbidden: {message}"),
            Self::Internal(message) => write!(f, "internal error: {message}"),
            Self::NotFound(message) => write!(f, "not found: {message}"),
            Self::Persistence(message) => write!(f, "persistence error: {message}"),
            Self::Unauthorized(message) => write!(f, "unauthorized: {message}"),
            Self::Validation(message) => write!(f, "validation error: {message}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        Self::Persistence(value.to_string())
    }
}

impl From<crate::domain::DomainValidationError> for AppError {
    fn from(value: crate::domain::DomainValidationError) -> Self {
        Self::Validation(format!("{value:?}"))
    }
}
