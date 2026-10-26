use thiserror::Error;

#[derive(Debug, Error)]
pub enum AtelierError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Windows API error: {0}")]
    Platform(String),

    #[error("Unknown error occurred: {0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, AtelierError>;
