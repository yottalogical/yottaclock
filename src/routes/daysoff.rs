use std::fmt::{self, Display, Formatter};

use askama::Template;
use axum::{
    debug_handler,
    extract::Extension,
    response::{Html, IntoResponse},
};
use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    session::UserKey,
    toggl::{get_user_projects, Project, ProjectId, WorkspaceId},
};

#[derive(Deserialize)]
#[serde(transparent)]
pub struct DayOffKey(pub i64);

impl Display for DayOffKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

struct DayOff<'a> {
    pub key: DayOffKey,
    pub date: NaiveDate,
    pub projects: Vec<&'a Project>,
}

#[derive(Template)]
#[template(path = "daysoff.html")]
pub struct DaysOffTemplate<'a> {
    days_off: &'a [DayOff<'a>],
}

#[debug_handler]
pub async fn get(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Extension(client): Extension<Client>,
) -> InternalResult<impl IntoResponse> {
    // TODO: Parallelize the await points in this function

    let user_record = sqlx::query!(
        "SELECT toggl_api_key, workspace_id FROM users WHERE user_key = $1",
        user_key.0,
    )
    .fetch_one(&pool)
    .await?;

    let day_off_records = sqlx::query!(
        "SELECT day_off_key, day_off
        FROM days_off
        WHERE user_key = $1",
        user_key.0,
    )
    .fetch_all(&pool)
    .await?;

    let project_ids_to_projects = get_user_projects(
        &user_record.toggl_api_key,
        WorkspaceId(user_record.workspace_id),
        &client,
        user_key,
        &pool,
    )
    .await?;

    let mut days_off = Vec::new();
    for day_off_record in day_off_records {
        let day_off_key = DayOffKey(day_off_record.day_off_key);

        let project_records = sqlx::query!(
            "SELECT project_id
            FROM projects
            INNER JOIN days_off_to_projects
            ON projects.project_key = days_off_to_projects.project_key
            WHERE day_off_key = $1",
            day_off_key.0,
        )
        .fetch_all(&pool)
        .await?;

        let mut projects = Vec::new();
        for project_record in project_records {
            let project_id = ProjectId(project_record.project_id);

            if let Some(project) = project_ids_to_projects.get(&project_id) {
                projects.push(project);
            }
        }
        projects.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

        days_off.push(DayOff {
            key: day_off_key,
            date: day_off_record.day_off,
            projects,
        })
    }
    days_off.sort_by_key(|day_off| day_off.date);

    let template = DaysOffTemplate {
        days_off: &days_off,
    };

    Ok(Html(template.render()?))
}
