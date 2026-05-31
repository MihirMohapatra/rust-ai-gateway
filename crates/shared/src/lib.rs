pub mod models;
pub mod providers;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Database error: {0}")]
    DbError(#[from] sqlx::Error),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl GatewayError {
    pub fn status_code(&self) -> u16 {
        match self {
            Self::AuthError(_) => 401,
            Self::RateLimited => 429,
            Self::BadRequest(_) => 400,
            Self::ProviderError(_) => 502,
            Self::DbError(_) | Self::Internal(_) => 500,
        }
    }
}
