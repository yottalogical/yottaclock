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
    let user = sqlx::query!(
        "SELECT user_id FROM users WHERE toggl_api_key = $1",
        form.toggl_api_key
    )
    .fetch_optional(&pool)
    .await?;

    let user_id: i32 = if let Some(user) = user {
        user.user_id
    } else {
        sqlx::query!(
            "INSERT INTO users(toggl_api_key) VALUES ($1) RETURNING user_id",
            form.toggl_api_key
        )
        .fetch_one(&pool)
        .await?
        .user_id
    };

    Ok((
        [new_session_cookie_header(user_id, &pool).await?],
        Redirect::to("/"),
    ))
}
