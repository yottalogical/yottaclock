use askama::Template;
use axum::{
    extract::Form,
    response::{Html, IntoResponse, Redirect},
    Extension,
};
use chrono::Duration;
use chrono_tz::{Tz, TZ_VARIANTS};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    session::{new_session_cookie_header, UserKey},
    toggl::{get_workspaces, Workspace, WorkspaceId},
};

#[derive(Template)]
#[template(path = "signup_step1.html")]
pub struct SignupTemplateStep1 {}

pub async fn get() -> InternalResult<impl IntoResponse> {
    let template = SignupTemplateStep1 {};
    Ok(Html(template.render()?))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignupFormStep1 {
    toggl_api_key: String,
}

#[derive(Template)]
#[template(path = "signup_step2.html")]
pub struct SignupTemplateStep2<'a> {
    toggl_api_key: &'a str,
    workspaces: &'a [Workspace],
    timezones: &'a [Tz],
}

pub async fn post_step1(
    Extension(client): Extension<Client>,
    Form(form): Form<SignupFormStep1>,
) -> InternalResult<impl IntoResponse> {
    let workspaces = get_workspaces(&form.toggl_api_key, client).await?;

    let template = SignupTemplateStep2 {
        toggl_api_key: &form.toggl_api_key,
        workspaces: &workspaces,
        timezones: &TZ_VARIANTS,
    };

    Ok(Html(template.render()?).into_response())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignupFormStep2 {
    toggl_api_key: String,
    workspace_id: WorkspaceId,
    daily_max_hours: i64,
    daily_max_minutes: i64,
    daily_max_seconds: i64,
    timezone: String,
}

pub async fn post_step2(
    Extension(pool): Extension<PgPool>,
    Form(form): Form<SignupFormStep2>,
) -> InternalResult<impl IntoResponse> {
    let daily_max = Duration::hours(form.daily_max_hours)
        + Duration::minutes(form.daily_max_minutes)
        + Duration::seconds(form.daily_max_seconds);

    // TODO: Check that fields going into database (API key, workspace_id, timezone) are valid

    let user_key = UserKey(
        sqlx::query!(
            "INSERT INTO users(toggl_api_key, workspace_id, daily_max, timezone)
            VALUES ($1, $2, $3, $4)
            RETURNING user_key",
            form.toggl_api_key,
            form.workspace_id.0,
            daily_max.num_seconds(),
            form.timezone,
        )
        .fetch_one(&pool)
        .await?
        .user_key,
    );

    Ok((
        [new_session_cookie_header(user_key, &pool).await?],
        Redirect::to("/"),
    )
        .into_response())
}
