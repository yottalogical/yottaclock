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
use chrono::Duration;
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
    pub daily_max: HumanDuration,
    pub percentage: i64,
    pub goals: Vec<Goal>,
}

pub async fn get(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Extension(client): Extension<Client>,
) -> InternalResult<impl IntoResponse> {
    let record = sqlx::query!(
        "SELECT daily_max FROM users WHERE user_key = $1",
        user_key.0
    )
    .fetch_one(&pool)
    .await?;

    let daily_max = Duration::seconds(record.daily_max);

    if let Some((goals, total_debt)) = calculate_goals(user_key, pool, client).await? {
        let template = Index {
            total_debt: HumanDuration(total_debt),
            daily_max: HumanDuration(daily_max),
            percentage: 100 - (total_debt.num_seconds() * 100 / daily_max.num_seconds()),
            goals: goals.into_iter().map(|g| g.into()).collect(),
        };

        Ok(Html(template.render()?).into_response())
    } else {
        Ok(Redirect::to("/project/new/").into_response())
    }
}
