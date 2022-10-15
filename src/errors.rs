use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum InternalError {
    #[error("Fatal sqlx error: {0}")]
    FatalSqlxError(#[from] sqlx::Error),

    #[error("Fatal askama error: {0}")]
    FatalAskamaError(#[from] askama::Error),
}

pub type InternalResult<T, E = InternalError> = Result<T, E>;

impl IntoResponse for InternalError {
    fn into_response(self) -> Response {
        error!("{}", self);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
