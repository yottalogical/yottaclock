use crate::{
    errors::InternalResult,
    session::UserId,
    templates::{self},
    toggl::calculate_goals,
};
use askama::Template;
use axum::{
    extract::Extension,
    response::{Html, IntoResponse, Redirect},
};
use sqlx::PgPool;

pub async fn get(
    UserId(user_id): UserId,
    Extension(pool): Extension<PgPool>,
) -> InternalResult<impl IntoResponse> {
    if let Some(user_id) = user_id {
        let template = templates::Index {
            user_id,
            goals: calculate_goals(user_id, pool),
        };

        Ok(Html(template.render()?).into_response())
    } else {
        Ok(Redirect::to("/login/").into_response())
    }
}
