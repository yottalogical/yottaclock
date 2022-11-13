use crate::{errors::InternalResult, session::SessionToken, templates};
use askama::Template;
use axum::{
    response::{Html, IntoResponse},
    Extension,
};
use sqlx::PgPool;

pub async fn get(
    session_token: SessionToken,
    Extension(pool): Extension<PgPool>,
) -> InternalResult<impl IntoResponse> {
    let user_id = session_token.get_user_id(&pool).await?;

    let template = templates::Index { user_id };
    Ok(Html(template.render()?))
}
