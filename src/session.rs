use crate::errors::InternalResult;
use axum::{
    async_trait,
    extract::{FromRequest, RequestParts, TypedHeader},
    headers::{Cookie, HeaderName, HeaderValue},
    http::{header::SET_COOKIE, StatusCode},
};
use rand::distributions::{Alphanumeric, DistString};
use sqlx::{Acquire, PgPool, Postgres};
use std::env;

pub static AXUM_SESSION_COOKIE_NAME: &str = "session";

pub struct SessionToken(Option<String>);

#[async_trait]
impl<B> FromRequest<B> for SessionToken
where
    B: Send,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        Ok(Self(
            Option::<TypedHeader<Cookie>>::from_request(req)
                .await
                .unwrap() // Infallible
                .as_ref()
                .and_then(|c| c.get(AXUM_SESSION_COOKIE_NAME))
                .map(|c| c.into()),
        ))
    }
}

impl SessionToken {
    pub async fn get_user_id<'a, A>(&self, connection: A) -> InternalResult<Option<i32>>
    where
        A: Acquire<'a, Database = Postgres>,
    {
        if let Self(Some(token)) = self {
            let mut conn = connection.acquire().await?;

            let s = sqlx::query!("SELECT user_id FROM session_tokens WHERE token = $1", token)
                .fetch_optional(&mut *conn)
                .await?
                .map(|s| s.user_id);

            Ok(s)
        } else {
            Ok(None)
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
