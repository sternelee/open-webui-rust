use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    InternalServerError(String),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User already exists")]
    UserAlreadyExists,

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("External service error: {0}")]
    ExternalServiceError(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Redis pool error: {0}")]
    RedisPool(String),

    #[error("Request timeout: {0}")]
    Timeout(String),

    #[error("Too many requests: {0}")]
    TooManyRequests(String),
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub detail: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::Database(ref e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Database error".to_string(),
                )
            }
            AppError::Redis(ref e) => {
                tracing::error!("Redis error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Redis error".to_string())
            }
            AppError::Auth(ref e) => (StatusCode::UNAUTHORIZED, e.clone()),
            AppError::Validation(ref e) => (StatusCode::BAD_REQUEST, e.clone()),
            AppError::NotFound(ref e) => (StatusCode::NOT_FOUND, e.clone()),
            AppError::Unauthorized(ref e) => (StatusCode::UNAUTHORIZED, e.clone()),
            AppError::Forbidden(ref e) => (StatusCode::FORBIDDEN, e.clone()),
            AppError::BadRequest(ref e) => (StatusCode::BAD_REQUEST, e.clone()),
            AppError::InternalServerError(ref e) => {
                tracing::error!("Internal server error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.clone())
            }
            AppError::Jwt(ref e) => {
                tracing::error!("JWT error: {:?}", e);
                (StatusCode::UNAUTHORIZED, "Invalid token".to_string())
            }
            AppError::InvalidCredentials => {
                (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string())
            }
            AppError::UserAlreadyExists => {
                (StatusCode::BAD_REQUEST, "User already exists".to_string())
            }
            AppError::Conflict(ref e) => (StatusCode::CONFLICT, e.clone()),
            AppError::Io(ref e) => {
                tracing::error!("IO error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "IO error".to_string())
            }
            AppError::NotImplemented(ref e) => (StatusCode::NOT_IMPLEMENTED, e.clone()),
            AppError::ExternalServiceError(ref e) => {
                tracing::error!("External service error: {:?}", e);
                (StatusCode::BAD_GATEWAY, e.clone())
            }
            AppError::Internal(ref e) => {
                tracing::error!("Internal error: {:?}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, e.clone())
            }
            AppError::Http(ref e) => {
                tracing::error!("HTTP error: {:?}", e);
                (StatusCode::BAD_GATEWAY, "HTTP request failed".to_string())
            }
            AppError::RedisPool(ref e) => {
                tracing::error!("Redis pool error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Redis pool error".to_string(),
                )
            }
            AppError::Timeout(ref e) => {
                tracing::error!("Request timeout: {:?}", e);
                (StatusCode::GATEWAY_TIMEOUT, e.clone())
            }
            AppError::TooManyRequests(ref e) => (StatusCode::TOO_MANY_REQUESTS, e.clone()),
        };

        let body = json!({
            "detail": error_message
        });

        (status, Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

// Implement From for redis pool errors
impl From<deadpool_redis::PoolError> for AppError {
    fn from(err: deadpool_redis::PoolError) -> Self {
        AppError::RedisPool(err.to_string())
    }
}
