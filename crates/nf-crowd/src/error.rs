use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrowdError {
    #[error("submission rejected: {reason}")]
    Rejected { reason: String },

    #[error("contributor suspended: {reason}")]
    Suspended { reason: String },

    #[error("duplicate submission: matches existing entity {entity_id}")]
    Duplicate { entity_id: String },

    #[error("invalid source: {0}")]
    InvalidSource(String),

    #[error("rate limited: {0}")]
    RateLimited(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("store error: {0}")]
    Store(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type CrowdResult<T> = Result<T, CrowdError>;
