use thiserror::Error;

#[derive(Debug, Error)]
pub enum NfError {
    #[error("missing source: {0}")]
    MissingSource(String),

    #[error("invalid entity: {0}")]
    InvalidEntity(String),

    #[error("invalid relationship: {0}")]
    InvalidRelationship(String),

    #[error("schema violation: {0}")]
    SchemaViolation(String),

    #[error("source verification failed: {0}")]
    SourceVerificationFailed(String),

    #[error("duplicate entity: {0}")]
    DuplicateEntity(String),

    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("url parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type NfResult<T> = Result<T, NfError>;
