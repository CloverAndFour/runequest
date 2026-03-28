//! Application error types.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, RunequestError>;

#[derive(Debug, Error)]
pub enum RunequestError {
    #[error("User '{0}' not found")]
    UserNotFound(String),

    #[error("User '{0}' already exists")]
    UserAlreadyExists(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Invalid username: {0}")]
    InvalidUsername(String),

    #[error("Adventure '{0}' not found")]
    AdventureNotFound(String),

    #[error("Invalid game state: {0}")]
    InvalidGameState(String),

    #[error("Invalid tool call: {0}")]
    InvalidToolCall(String),

    #[error("LLM error: {0}")]
    LlmError(String),

    #[error("LLM rate limited, retry after {retry_after_secs}s")]
    LlmRateLimited { retry_after_secs: u64 },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
