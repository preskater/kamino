//! Error type shared by every service, rendered as a consistent JSON envelope.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::{Map, Value};

pub type ApiResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    BadRequest(String),

    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    Conflict(String),

    #[error("validation failed")]
    Validation(Map<String, Value>),

    #[error("internal server error")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn not_found(resource: &str, id: impl std::fmt::Display) -> Self {
        AppError::NotFound(format!("{resource} {id} not found"))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::NotFound("resource not found".into()),
            sqlx::Error::Database(db) => {
                let code = db.code().map(|code| code.into_owned());
                match code.as_deref() {
                    Some("23505") => AppError::Conflict(format!(
                        "a record with the same unique value already exists ({})",
                        db.constraint().unwrap_or("unknown constraint")
                    )),
                    Some("23503") => AppError::BadRequest(
                        "referenced record does not exist or is still in use".into(),
                    ),
                    Some("23514") => AppError::BadRequest(format!(
                        "value violates constraint ({})",
                        db.constraint().unwrap_or("unknown constraint")
                    )),
                    Some("22P02") | Some("22007") | Some("22008") => {
                        AppError::BadRequest("invalid value syntax".into())
                    }
                    _ => AppError::Internal(anyhow::Error::new(sqlx::Error::Database(db))),
                }
            }
            other => AppError::Internal(anyhow::Error::new(other)),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Internal(anyhow::Error::new(err))
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let mut details = Map::new();
        for (field, problems) in errors.field_errors() {
            let messages = problems
                .iter()
                .map(|problem| {
                    problem
                        .message
                        .clone()
                        .map(|message| message.into_owned())
                        .unwrap_or_else(|| problem.code.clone().into_owned())
                })
                .collect::<Vec<_>>();
            details.insert(field.to_string(), Value::from(messages));
        }
        AppError::Validation(details)
    }
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match self {
            AppError::BadRequest(message) => {
                (StatusCode::BAD_REQUEST, "bad_request", message, None)
            }
            AppError::NotFound(message) => (StatusCode::NOT_FOUND, "not_found", message, None),
            AppError::Conflict(message) => (StatusCode::CONFLICT, "conflict", message, None),
            AppError::Validation(details) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "one or more fields are invalid".to_string(),
                Some(Value::Object(details)),
            ),
            AppError::Internal(err) => {
                tracing::error!(error = ?err, "unhandled internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "internal server error".to_string(),
                    None,
                )
            }
        };

        let body = ErrorEnvelope {
            error: ErrorBody {
                code,
                message,
                details,
            },
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_missing_row_to_not_found() {
        let error = AppError::from(sqlx::Error::RowNotFound);
        assert!(matches!(error, AppError::NotFound(_)));
    }

    #[test]
    fn validation_errors_become_details() {
        let mut errors = validator::ValidationErrors::new();
        let mut error = validator::ValidationError::new("length");
        error.message = Some("too short".into());
        errors.add("name", error);

        let mapped = AppError::from(errors);
        assert!(matches!(mapped, AppError::Validation(_)));
    }
}
