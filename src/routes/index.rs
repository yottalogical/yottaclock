use crate::{
    errors::InternalResult,
    session::UserKey,
    toggl::{self, calculate_goals},
};
use askama::Template;
use axum::{
    extract::Extension,
    response::{Html, IntoResponse, Redirect},
};
use reqwest::Client;
use sqlx::PgPool;

use crate::human_duration::HumanDuration;

pub struct Goal {
    pub name: String,
    pub time: HumanDuration,
}

impl From<toggl::Goal> for Goal {
    fn from(other: toggl::Goal) -> Self {
        Self {
            name: other.name,
            time: HumanDuration(other.time),
        }
    }
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct Index {
    pub total_debt: HumanDuration,
    pub goals: Vec<Goal>,
}

pub async fn get(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Extension(client): Extension<Client>,
) -> InternalResult<impl IntoResponse> {
    Ok(
        if let Some((goals, total_debt)) = calculate_goals(user_key, pool, client).await? {
            let template = Index {
                total_debt: HumanDuration(total_debt),
                goals: goals.into_iter().map(|g| g.into()).collect(),
            };

            Html(template.render()?).into_response()
        } else {
            Redirect::to("/project/new/").into_response()
        },
    )
}
