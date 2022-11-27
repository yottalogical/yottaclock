use std::collections::HashMap;

use askama::Template;
use axum::{
    extract::{Extension, Form},
    response::{Html, IntoResponse, Redirect},
};
use chrono::NaiveDate;
use futures::future;
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    session::UserKey,
    toggl::{get_user_projects, ProjectId, WorkspaceId},
};

struct Project {
    pub id: ProjectId,
    pub name: String,
}

#[derive(Template)]
#[template(path = "dayoff_new.html")]
pub struct NewDayOffTemplate<'a> {
    projects: &'a [Project],
}

pub async fn get(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Extension(client): Extension<Client>,
) -> InternalResult<impl IntoResponse> {
    let record = sqlx::query!(
        "SELECT toggl_api_key, workspace_id
        FROM users
        WHERE user_key = $1",
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
    })
    .collect();
    projects.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

    let template = NewDayOffTemplate {
        projects: &projects,
    };

    Ok(Html(template.render()?))
}

#[derive(Debug, Deserialize)]
pub struct NewDayOffForm {
    date: NaiveDate,

    #[serde(flatten)]
    project_ids: HashMap<String, String>,
}

pub async fn post(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<NewDayOffForm>,
) -> InternalResult<impl IntoResponse> {
    let pool = &pool;

    let day_off_key = sqlx::query!(
        "INSERT INTO days_off(user_key, day_off)
        VALUES ($1, $2)
        RETURNING day_off_key",
        user_key.0,
        form.date,
    )
    .fetch_one(pool)
    .await?
    .day_off_key;

    let futures = form.project_ids.keys().map(|project_id| async move {
        let project_id = ProjectId(project_id.parse()?);

        let project_key = sqlx::query!(
            "SELECT project_key
            FROM projects
            WHERE project_id = $1
            AND user_key = $2",
            project_id.0,
            user_key.0,
        )
        .fetch_one(pool)
        .await?
        .project_key;

        sqlx::query!(
            "INSERT INTO days_off_to_projects(project_key, day_off_key)
            VALUES ($1, $2)",
            project_key,
            day_off_key,
        )
        .execute(pool)
        .await?;

        Ok(())
    });

    let future_results: Vec<InternalResult<()>> = future::join_all(futures).await;
    for future_result in future_results {
        future_result?;
    }

    Ok(Redirect::to("/"))
}
