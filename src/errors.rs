use std::str::FromStr;

use axum::{
    http::{header::InvalidHeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use chrono_tz::Tz;
use tokio::task::JoinError;
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum InternalError {
    #[error("Fatal sqlx error: {0}")]
    FatalSqlxError(#[from] sqlx::Error),

    #[error("Fatal askama error: {0}")]
    FatalAskamaError(#[from] askama::Error),

    #[error("Fatal reqwest error: {0}")]
    FatalReqwestError(#[from] reqwest::Error),

    #[error("Fatal JoinError: {0}")]
    FatalJoinError(#[from] JoinError),

    #[error("Fatal InvalidHeaderValue error: {0}")]
    FatalInvalidHeaderValueError(#[from] InvalidHeaderValue),

    #[error("Fatal UnrecognizedTimezone error: {0}")]
    FatalUnrecognizedTimezoneError(<Tz as FromStr>::Err),
}

pub type InternalResult<T, E = InternalError> = Result<T, E>;

impl IntoResponse for InternalError {
    fn into_response(self) -> Response {
        error!("{}", self);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
