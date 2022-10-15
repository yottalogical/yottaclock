use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub struct InternalError(anyhow::Error);

pub type InternalResult<T> = std::result::Result<T, InternalError>;

impl From<anyhow::Error> for InternalError {
    fn from(error: anyhow::Error) -> Self {
        Self(error)
    }
}

// impl<E> From<E> for InternalError
// where
//     E: std::error::Error + Send + Sync + 'static,
// {
//     fn from(error: E) -> Self {
//         Self(anyhow::Error::new(error))
//     }
// }

impl IntoResponse for InternalError {
    fn into_response(self) -> Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
