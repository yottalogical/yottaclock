use askama::Template;
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
};
use chrono::NaiveDate;
use reqwest::Client;
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    human_duration::HumanDuration,
    session::UserKey,
    toggl::{get_user_projects, ProjectId, WhichWeekdays, WorkspaceId},
};

struct Project {
    pub id: ProjectId,
    pub name: String,
    pub starting_date: NaiveDate,
    pub daily_goal: HumanDuration,
    pub weekdays: WhichWeekdays,
}

#[derive(Template)]
#[template(path = "projects.html")]
pub struct ProjectsTemplate<'a> {
    projects: &'a [Project],
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

    let mut projects: Vec<Project> = get_user_projects(
        &record.toggl_api_key,
        WorkspaceId(record.workspace_id),
        &client,
        user_key,
        &pool,
    )
    .await?
    .into_iter()
    .map(|(project_id, project)| Project {
        id: project_id,
        name: project.name,
        starting_date: project.starting_date,
        daily_goal: HumanDuration(project.daily_goal),
        weekdays: project.weekdays,
    })
    .collect();
    projects.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

    let template = ProjectsTemplate {
        projects: &projects,
    };

    Ok(Html(template.render()?))
}
