use axum::{
    async_trait,
    extract::{FromRequest, RequestParts, TypedHeader},
    headers::Cookie,
    http::StatusCode,
};
use sqlx::{Acquire, Postgres};

const AXUM_SESSION_COOKIE_NAME: &str = "session";

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
    pub async fn get_user_id<'a, A>(&self, connection: A) -> anyhow::Result<Option<i32>>
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

// pub struct GetUserIdError(sqlx::Error);

// impl From<GetUserIdError> for StatusCode {
//     fn from(_: GetUserIdError) -> Self {
//         Self::INTERNAL_SERVER_ERROR
//     }
// }

// impl From<sqlx::Error> for GetUserIdError {
//     fn from(e: sqlx::Error) -> Self {
//         Self(e)
//     }
// }
