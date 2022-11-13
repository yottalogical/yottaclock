use crate::{errors::InternalResult, session::new_session_cookie_header, templates};
use askama::Template;
use axum::{
    extract::Form,
    response::{Html, IntoResponse, Redirect},
    Extension,
};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct Login {
    toggl_api_key: String,
}

pub async fn get() -> InternalResult<impl IntoResponse> {
    let template = templates::Login {};
    Ok(Html(template.render()?))
}

pub async fn post(
    Extension(pool): Extension<PgPool>,
    Form(form): Form<Login>,
) -> InternalResult<impl IntoResponse> {
    sqlx::query!(
        "INSERT INTO users(toggl_api_key) VALUES ($1) ON CONFLICT DO NOTHING",
        form.toggl_api_key
    )
    .execute(&pool)
    .await?;

    let user = sqlx::query!(
        "SELECT user_id, toggl_api_key FROM users WHERE toggl_api_key = $1",
        form.toggl_api_key
    )
    .fetch_one(&pool)
    .await?;

    Ok((
        [new_session_cookie_header(user.user_id, &pool).await?],
        Redirect::to("/"),
    ))
}
