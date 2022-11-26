use askama::Template;
use axum::{
    extract::Form,
    response::{IntoResponse, Redirect},
    Extension,
};
use chrono::Duration;
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    session::{new_session_cookie_header, UserKey},
    toggl::{Workspace, WorkspaceId},
};

#[derive(Template)]
#[template(path = "signup.html")]
pub struct SignupTemplate<'a> {
    pub toggl_api_key: &'a str,
    pub workspaces: &'a [Workspace],
    pub timezones: &'a [Tz],
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignupForm {
    toggl_api_key: String,
    workspace_id: WorkspaceId,
    daily_max_hours: i64,
    daily_max_minutes: i64,
    daily_max_seconds: i64,
    timezone: String,
}

pub async fn post(
    Extension(pool): Extension<PgPool>,
    Form(form): Form<SignupForm>,
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
