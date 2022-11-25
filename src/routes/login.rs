use askama::Template;
use axum::{
    extract::Form,
    response::{Html, IntoResponse, Redirect},
    Extension,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    session::{new_session_cookie_header, UserId},
};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {}

pub async fn get() -> InternalResult<impl IntoResponse> {
    let template = LoginTemplate {};
    Ok(Html(template.render()?))
}

#[derive(Deserialize)]
pub struct LoginForm {
    toggl_api_key: String,
    workspace_id: i64,
    daily_max: i64,
    timezone: String,
}

pub async fn post(
    Extension(pool): Extension<PgPool>,
    Form(form): Form<LoginForm>,
) -> InternalResult<impl IntoResponse> {
    let user = sqlx::query!(
        "SELECT user_id FROM users WHERE toggl_api_key = $1",
        form.toggl_api_key
    )
    .fetch_optional(&pool)
    .await?;

    let user_id = UserId(if let Some(user) = user {
        user.user_id
    } else {
        sqlx::query!(
            "INSERT INTO users(toggl_api_key, workspace_id, daily_max, timezone) VALUES ($1, $2, $3, $4) RETURNING user_id",
            form.toggl_api_key,
            form.workspace_id,
            form.daily_max,
            form.timezone,
        )
        .fetch_one(&pool)
        .await?
        .user_id
    });

    Ok((
        [new_session_cookie_header(user_id, &pool).await?],
        Redirect::to("/"),
    ))
}
