use askama::Template;
use axum::{http::StatusCode, response::Html, Extension};
use sqlx::PgPool;

use crate::session::SessionToken;
use crate::templates;

pub async fn index(
    session_token: SessionToken,
    pool: Extension<PgPool>,
) -> Result<Html<String>, StatusCode> {
    let Extension(pool) = pool;
    let user_id = session_token.get_user_id(&pool).await?;

    let template = templates::Index { user_id };
    Ok(Html(template.render().unwrap()))
}
