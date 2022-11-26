use std::str::FromStr;

use askama::Template;
use axum::{
    extract::Form,
    http::StatusCode,
    response::{IntoResponse, Redirect},
    Extension,
};
use chrono::Duration;
use chrono_tz::Tz;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    session::{new_session_cookie_header, UserKey},
    toggl::{get_workspace_details, Workspace, WorkspaceId},
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
    Extension(client): Extension<Client>,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<SignupForm>,
) -> InternalResult<impl IntoResponse> {
    let daily_max = Duration::hours(form.daily_max_hours)
        + Duration::minutes(form.daily_max_minutes)
        + Duration::seconds(form.daily_max_seconds);

    // Check if the data from the form is valid
    let workspace = get_workspace_details(&form.toggl_api_key, form.workspace_id, client).await?;
    let positive_daily_max = daily_max >= Duration::seconds(0);
    let timezone: Result<Tz, <Tz as FromStr>::Err> = form.timezone.parse();

    Ok(
        if let (Some(workspace), true, Ok(timezone)) = (workspace, positive_daily_max, timezone) {
            let user_key = UserKey(
                sqlx::query!(
                    "INSERT INTO users(toggl_api_key, workspace_id, daily_max, timezone)
                    VALUES ($1, $2, $3, $4)
                    RETURNING user_key",
                    form.toggl_api_key,
                    workspace.id.0,
                    daily_max.num_seconds(),
                    timezone.to_string(),
                )
                .fetch_one(&pool)
                .await?
                .user_key,
            );

            (
                [new_session_cookie_header(user_key, &pool).await?],
                Redirect::to("/"),
            )
                .into_response()
        } else {
            StatusCode::BAD_REQUEST.into_response()
        },
    )
}
