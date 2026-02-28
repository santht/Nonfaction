use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// All API-level error types with mapping to HTTP status codes.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("internal server error: {0}")]
    Internal(String),

    #[error("store error: {0}")]
    Store(#[from] nf_store::StoreError),

    #[error("search error: {0}")]
    Search(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid UUID: {0}")]
    InvalidUuid(#[from] uuid::Error),

    #[error("unprocessable entity: {0}")]
    UnprocessableEntity(String),

    #[error("conflict: {0}")]
    Conflict(String),
}

impl From<nf_search::SearchError> for ApiError {
    fn from(e: nf_search::SearchError) -> Self {
        ApiError::Search(e.to_string())
    }
}

impl ApiError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Store(e) => match e {
                nf_store::StoreError::NotFound(_) => StatusCode::NOT_FOUND,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
            ApiError::Search(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Serialization(_) => StatusCode::BAD_REQUEST,
            ApiError::InvalidUuid(_) => StatusCode::BAD_REQUEST,
            ApiError::UnprocessableEntity(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
        }
    }

    pub fn error_code(&self) -> &'static str {
        match self {
            ApiError::NotFound(_) => "NOT_FOUND",
            ApiError::BadRequest(_) => "BAD_REQUEST",
            ApiError::Internal(_) => "INTERNAL_ERROR",
            ApiError::Store(e) => match e {
                nf_store::StoreError::NotFound(_) => "NOT_FOUND",
                _ => "STORE_ERROR",
            },
            ApiError::Search(_) => "SEARCH_ERROR",
            ApiError::Serialization(_) => "SERIALIZATION_ERROR",
            ApiError::InvalidUuid(_) => "INVALID_UUID",
            ApiError::UnprocessableEntity(_) => "UNPROCESSABLE_ENTITY",
            ApiError::Conflict(_) => "CONFLICT",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let code = self.error_code();
        let message = self.to_string();

        tracing::error!(error = %message, code = code, status = %status, "API error");

        let body = json!({
            "error": {
                "code": code,
                "message": message,
            }
        });

        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[test]
    fn test_not_found_status() {
        let err = ApiError::NotFound("entity 123".to_string());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(err.error_code(), "NOT_FOUND");
    }

    #[test]
    fn test_bad_request_status() {
        let err = ApiError::BadRequest("invalid param".to_string());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_invalid_uuid_status() {
        let bad_uuid = uuid::Uuid::parse_str("not-a-uuid");
        let err = ApiError::InvalidUuid(bad_uuid.unwrap_err());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(err.error_code(), "INVALID_UUID");
    }

    #[test]
    fn test_into_response_body() {
        let err = ApiError::NotFound("test entity".to_string());
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
