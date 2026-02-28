use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("URL is not reachable: {status}")]
    UrlNotReachable { status: u16 },

    #[error("URL is dead (non-2xx response): {status}")]
    UrlDead { status: u16 },

    #[error("FEC API error: {0}")]
    FecApiError(String),

    #[error("OpenSanctions API error: {0}")]
    OpenSanctionsError(String),

    #[error("JSON parse error: {0}")]
    JsonParse(String),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Verification not applicable for source type: {0}")]
    NotApplicable(String),
}

pub type VerifyResult<T> = Result<T, VerifyError>;
