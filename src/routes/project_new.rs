use askama::Template;
use axum::{
    extract::{Extension, Form},
    response::{Html, IntoResponse, Redirect},
};
use chrono::{Duration, NaiveDate};
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    session::UserKey,
    toggl::{get_toggl_projects, ProjectId, TogglProject, WorkspaceId},
};

#[derive(Template)]
#[template(path = "project_new.html")]
pub struct NewProjectTemplate<'a> {
    projects: &'a [TogglProject],
}

pub async fn get(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Extension(client): Extension<Client>,
) -> InternalResult<impl IntoResponse> {
    let record = sqlx::query!(
        "SELECT toggl_api_key, workspace_id FROM users WHERE user_key = $1",
        user_key.0,
    )
    .fetch_one(&pool)
    .await?;

    let toggl_projects = get_toggl_projects(
        &record.toggl_api_key,
        WorkspaceId(record.workspace_id),
        &client,
    )
    .await?;

    let records = sqlx::query!(
        "SELECT project_id FROM projects WHERE user_key = $1",
        user_key.0,
    )
    .fetch_all(&pool)
    .await?;

    // Remove Toggl projects that the user has already created projects for
    let toggl_projects_not_yet_created: Vec<TogglProject> = toggl_projects
        .into_iter()
        .filter(|toggl_project| {
            (&records)
                .into_iter()
                .find(|user_project| toggl_project.id == ProjectId(user_project.project_id))
                .is_none()
        })
        .collect();

    let template = NewProjectTemplate {
        projects: &toggl_projects_not_yet_created,
    };

    Ok(Html(template.render()?))
}

#[derive(Debug, Deserialize)]
pub struct NewProjectForm {
    project_id: ProjectId,
    starting_date: NaiveDate,
    daily_goal_hours: i64,
    daily_goal_minutes: i64,
    daily_goal_seconds: i64,
    monday: Option<String>,
    tuesday: Option<String>,
    wednesday: Option<String>,
    thursday: Option<String>,
    friday: Option<String>,
    saturday: Option<String>,
    sunday: Option<String>,
}

pub async fn post(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<NewProjectForm>,
) -> InternalResult<impl IntoResponse> {
    let daily_goal = Duration::hours(form.daily_goal_hours)
        + Duration::minutes(form.daily_goal_minutes)
        + Duration::seconds(form.daily_goal_seconds);

    sqlx::query!(
        "INSERT INTO projects(user_key, project_id, starting_date, daily_goal,
            monday, tuesday, wednesday, thursday, friday, saturday, sunday)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        user_key.0,
        form.project_id.0,
        form.starting_date,
        daily_goal.num_seconds(),
        form.monday.is_some(),
        form.tuesday.is_some(),
        form.wednesday.is_some(),
        form.thursday.is_some(),
        form.friday.is_some(),
        form.saturday.is_some(),
        form.sunday.is_some(),
    )
    .execute(&pool)
    .await?;

    Ok(Redirect::to("/"))
}
