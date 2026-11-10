use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("Invalid credentials or session")]
    Unauthorized,
    #[error("Your role cannot perform this action")]
    Forbidden,
    #[error("Resource not found")]
    NotFound,
    #[error("{0}")]
    Conflict(&'static str),
    #[error("Too many sign-in attempts. Try again in a minute.")]
    Throttled,
    #[error("Database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("Unexpected service error")]
    Internal,
}

#[derive(Serialize)]
struct Problem {
    error: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::Invalid(_) => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Throttled => StatusCode::TOO_MANY_REQUESTS,
            Self::Database(error) => {
                tracing::error!(%error, "database operation failed");
                StatusCode::INTERNAL_SERVER_ERROR
            }
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(Problem {
                error: self.to_string(),
            }),
        )
            .into_response()
    }
}

pub(crate) fn bounded(raw: &str, max: usize) -> Result<&str, ApiError> {
    let value = raw.trim();
    if value.is_empty() || value.chars().count() > max || value.contains('\0') {
        return Err(ApiError::Invalid("Text is empty, too long, or invalid"));
    }
    Ok(value)
}

pub(crate) fn email(raw: &str) -> Result<String, ApiError> {
    let raw = bounded(raw, 254)?;
    let (local, domain) = raw
        .split_once('@')
        .ok_or(ApiError::Invalid("Invalid email"))?;
    if local.is_empty()
        || domain.is_empty()
        || domain.contains('@')
        || !raw.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(ApiError::Invalid("Invalid email"));
    }
    Ok(raw.to_ascii_lowercase())
}
