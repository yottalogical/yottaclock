use std::{num::ParseIntError, str::FromStr};

use axum::{
    http::{header::InvalidHeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use chrono_tz::Tz;
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum InternalError {
    #[error("Fatal sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("Fatal askama error: {0}")]
    Askama(#[from] askama::Error),

    #[error("Fatal reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),

    #[error("Fatal InvalidHeaderValue error: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),

    #[error("Fatal UnrecognizedTimezone error: {0}")]
    UnrecognizedTimezone(<Tz as FromStr>::Err),

    #[error("Fatal ParseInt error: {0}")]
    ParseInt(#[from] ParseIntError),
}

pub type InternalResult<T, E = InternalError> = Result<T, E>;

impl IntoResponse for InternalError {
    fn into_response(self) -> Response {
        error!("{}", self);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
