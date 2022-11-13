use crate::errors::InternalResult;
use axum::{
    async_trait,
    extract::{Extension, FromRequest, RequestParts, TypedHeader},
    headers::{Cookie, HeaderName, HeaderValue},
    http::{header::SET_COOKIE, StatusCode},
};
use rand::distributions::{Alphanumeric, DistString};
use sqlx::PgPool;
use std::env;

pub static AXUM_SESSION_COOKIE_NAME: &str = "session";

pub struct UserId(pub Option<i32>);

#[async_trait]
impl<B> FromRequest<B> for UserId
where
    B: Send,
{
    type Rejection = StatusCode;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let typed_headers = Option::<TypedHeader<Cookie>>::from_request(req)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let token = typed_headers
            .as_ref()
            .and_then(|c| c.get(AXUM_SESSION_COOKIE_NAME));

        if let Some(token) = token {
            let Extension(pool) = Extension::from_request(req)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let user_id =
                sqlx::query!("SELECT user_id FROM session_tokens WHERE token = $1", token)
                    .fetch_optional(&pool)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .map(|s| s.user_id);

            Ok(Self(user_id))
        } else {
            Ok(Self(None))
        }
    }
}

pub async fn new_session_cookie_header(
    user_id: i32,
    pool: &PgPool,
) -> InternalResult<(HeaderName, HeaderValue)> {
    let session_token = Alphanumeric.sample_string(&mut rand::thread_rng(), 64);

    sqlx::query!(
        "INSERT INTO session_tokens(token, user_id) VALUES ($1, $2)",
        session_token,
        user_id
    )
    .execute(pool)
    .await?;

    let secure = if let Some(_) = env::var_os("YOTTACLOCK_INSECURE_COOKIES") {
        ""
    } else {
        "Secure; "
    };

    Ok((
        SET_COOKIE,
        format!(
            "{}={}; Max-Age=2592000; Path=/; {}HttpOnly; SameSite=Strict",
            AXUM_SESSION_COOKIE_NAME, session_token, secure,
        )
        .parse()?,
    ))
}
