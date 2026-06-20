//! Registry API errors

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("SType not found: {0}")]
    NotFound(String),

    #[error("Invalid SType format: {0}")]
    InvalidFormat(String),

    #[error("Schema parsing error: {0}")]
    SchemaError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for RegistryError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            RegistryError::NotFound(stype) => (
                StatusCode::NOT_FOUND,
                "E-NOT-FOUND",
                format!("SType not found: {}", stype),
            ),
            RegistryError::InvalidFormat(msg) => {
                (StatusCode::BAD_REQUEST, "E-INVALID-FORMAT", msg.clone())
            }
            RegistryError::SchemaError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "E-SCHEMA-ERROR",
                msg.clone(),
            ),
            RegistryError::IoError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "E-IO-ERROR",
                e.to_string(),
            ),
            RegistryError::Internal(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "E-INTERNAL", msg.clone())
            }
        };

        let body = Json(json!({
            "error": error_code,
            "message": message,
        }));

        (status, body).into_response()
    }
}
