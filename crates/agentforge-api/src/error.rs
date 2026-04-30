use axum::{http::StatusCode, response::{IntoResponse, Json, Response}};
use agentforge_core::AgentForgeError;
use serde_json::json;

/// Axum-compatible error type that serializes to JSON.
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::NOT_FOUND, code: "NOT_FOUND", message: msg.into() }
    }
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: "BAD_REQUEST", message: msg.into() }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, code: "INTERNAL_ERROR", message: msg.into() }
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::CONFLICT, code: "CONFLICT", message: msg.into() }
    }
    pub fn unprocessable(msg: impl Into<String>) -> Self {
        Self { status: StatusCode::UNPROCESSABLE_ENTITY, code: "UNPROCESSABLE_ENTITY", message: msg.into() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = json!({ "error": { "code": self.code, "message": self.message } });
        (self.status, Json(body)).into_response()
    }
}

impl From<AgentForgeError> for ApiError {
    fn from(e: AgentForgeError) -> Self {
        match &e {
            AgentForgeError::NotFound { .. } => ApiError::not_found(e.to_string()),
            AgentForgeError::ValidationError(_) | AgentForgeError::ParseError(_) => {
                ApiError::bad_request(e.to_string())
            }
            AgentForgeError::CircularBiasError { .. } => ApiError::bad_request(e.to_string()),
            AgentForgeError::ScoreGateFailed { .. }
            | AgentForgeError::RegressionGateFailed { .. }
            | AgentForgeError::StabilityGateFailed { .. } => ApiError::unprocessable(e.to_string()),
            _ => {
                tracing::error!(error = %e, "Internal server error");
                ApiError::internal("An unexpected error occurred")
            }
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
