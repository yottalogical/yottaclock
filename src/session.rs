use crate::errors::InternalResult;
use axum::{
    async_trait,
    extract::{Extension, FromRequestParts},
    http::{header::SET_COOKIE, request::Parts, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::{
    extract::CookieJar,
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use rand::distributions::{Alphanumeric, DistString};
use sqlx::PgPool;
use std::env;

pub static BASIC_AUTH_USERNAME: &str = "api_token";
pub static SESSION_COOKIE_NAME: &str = "session";

pub struct UserKey(pub i64);

fn to_internal_server_error<E>(_: E) -> Response {
    StatusCode::INTERNAL_SERVER_ERROR.into_response()
}

#[async_trait]
impl<S> FromRequestParts<S> for UserKey
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Extension(pool) = Extension::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;

        let cookie_jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(to_internal_server_error)?;

        let session_token = cookie_jar
            .get(SESSION_COOKIE_NAME)
            .map(|cookie| cookie.value());

        let basic_authorization: Option<TypedHeader<Authorization<Basic>>> =
            match TypedHeader::from_request_parts(parts, state).await {
                Ok(api_token) => Some(api_token),
                Err(rejection) => {
                    if rejection.is_missing() {
                        None
                    } else {
                        return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                    }
                }
            };

        let user_key =
            if let Some(TypedHeader(Authorization(basic_authorization))) = basic_authorization {
                if basic_authorization.username() == BASIC_AUTH_USERNAME {
                    let api_token = basic_authorization.password();

                    sqlx::query!(
                        "SELECT user_key FROM users WHERE toggl_api_key = $1",
                        api_token,
                    )
                    .fetch_optional(&pool)
                    .await
                    .map_err(to_internal_server_error)?
                    .map(|s| s.user_key)
                    .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?
                } else {
                    return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                }
            } else if let Some(session_token) = session_token {
                sqlx::query!(
                    "SELECT user_key FROM session_tokens WHERE token = $1",
                    session_token,
                )
                .fetch_optional(&pool)
                .await
                .map_err(to_internal_server_error)?
                .map(|s| s.user_key)
                .ok_or_else(|| StatusCode::BAD_REQUEST.into_response())?
            } else {
                return Err(Redirect::to("/login/").into_response());
            };

        Ok(Self(user_key))
    }
}

pub async fn new_session_cookie_header(
    user_key: UserKey,
    pool: &PgPool,
) -> InternalResult<(HeaderName, HeaderValue)> {
    let session_token = Alphanumeric.sample_string(&mut rand::thread_rng(), 64);

    sqlx::query!(
        "INSERT INTO session_tokens(token, user_key) VALUES ($1, $2)",
        session_token,
        user_key.0,
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
