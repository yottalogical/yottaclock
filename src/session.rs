use crate::errors::InternalResult;
use axum::{
    async_trait,
    body::BoxBody,
    extract::{Extension, FromRequest, RequestParts, TypedHeader},
    headers::{Cookie, HeaderName, HeaderValue},
    http::{header::SET_COOKIE, Response, StatusCode},
    response::{IntoResponse, Redirect},
};
use rand::distributions::{Alphanumeric, DistString};
use sqlx::PgPool;
use std::env;

pub static SESSION_COOKIE_NAME: &str = "session";

pub struct UserId(pub i64);

fn to_internal_server_error<E>(_: E) -> Response<BoxBody> {
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

#[async_trait]
impl<B> FromRequest<B> for UserId
where
    B: Send,
{
    type Rejection = Response<BoxBody>;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let typed_headers = Option::<TypedHeader<Cookie>>::from_request(req)
            .await
            .map_err(to_internal_server_error)?;

        let token = typed_headers
            .as_ref()
            .and_then(|c| c.get(SESSION_COOKIE_NAME))
            .ok_or_else(|| Redirect::to("/login/").into_response())?;

        let Extension(pool) = Extension::from_request(req)
            .await
            .map_err(to_internal_server_error)?;

        let user_id = sqlx::query!("SELECT user_id FROM session_tokens WHERE token = $1", token)
            .fetch_optional(&pool)
            .await
            .map_err(to_internal_server_error)?
            .map(|s| s.user_id)
            .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?;

        Ok(Self(user_id))
    }
}

pub async fn new_session_cookie_header(
    user_id: UserId,
    pool: &PgPool,
) -> InternalResult<(HeaderName, HeaderValue)> {
    let session_token = Alphanumeric.sample_string(&mut rand::thread_rng(), 64);

    sqlx::query!(
        "INSERT INTO session_tokens(token, user_id) VALUES ($1, $2)",
        session_token,
        user_id.0,
    )
    .execute(pool)
    .await?;

    let secure = if env::var_os("YOTTACLOCK_INSECURE_COOKIES").is_some() {
        ""
    } else {
        "Secure; "
    };

    Ok((
        SET_COOKIE,
        format!(
            "{}={}; Max-Age=2592000; Path=/; {}HttpOnly; SameSite=Strict",
            SESSION_COOKIE_NAME, session_token, secure,
        )
        .parse()?,
    ))
}
