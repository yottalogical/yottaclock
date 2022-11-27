use axum::{
    extract::{Extension, Form},
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{errors::InternalResult, session::UserKey, toggl::ProjectId};

#[derive(Deserialize)]
pub struct ProjectDeleteForm {
    project_id: ProjectId,
}

pub async fn post(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<ProjectDeleteForm>,
) -> InternalResult<impl IntoResponse> {
    sqlx::query!(
        "DELETE FROM projects
        WHERE project_id = $1
        AND user_key = $2",
        form.project_id.0,
        user_key.0,
    )
    .execute(&pool)
    .await?;

    Ok(Redirect::to("/projects/"))
}
