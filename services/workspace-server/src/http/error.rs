use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error("{0}")]
    BadRequest(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("not found: {0}")]
    NotFound(String),

    #[error("Internal Server Error")]
    InternalServerError,
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            HttpError::BadRequest(ref m) => (StatusCode::BAD_REQUEST, m.clone()),
            HttpError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            HttpError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            HttpError::NotFound(ref m) => (StatusCode::NOT_FOUND, m.clone()),
            HttpError::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error".to_string(),
            ),
        };

        (status, msg).into_response()
    }
}
