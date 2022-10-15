use askama::Template;
use axum::{response::Html, Extension};
use sqlx::PgPool;

use crate::{errors::InternalResult, session::SessionToken, templates};

pub async fn index(
    session_token: SessionToken,
    pool: Extension<PgPool>,
) -> InternalResult<Html<String>> {
    let Extension(pool) = pool;
    let user_id = session_token.get_user_id(&pool).await?;

    let template = templates::Index { user_id };
    Ok(Html(template.render()?))
}
