use crate::{
    errors::InternalResult,
    session::UserId,
    toggl::{self, calculate_goals},
};
use askama::Template;
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
};
use chrono::Duration;
use reqwest::Client;
use sqlx::PgPool;
use std::fmt::{self, Display, Formatter};

pub struct HumanDuration(pub Duration);

impl Display for HumanDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let hours = self.0.num_hours();
        let minutes = self.0.num_minutes() - 60 * self.0.num_hours();
        let seconds = self.0.num_seconds() - 60 * self.0.num_minutes();

        write!(f, "{}:{:0>2}:{:0>2}", hours, minutes, seconds)
    }
}

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
    UserId(user_id): UserId,
    Extension(pool): Extension<PgPool>,
    Extension(client): Extension<Client>,
) -> InternalResult<impl IntoResponse> {
    let (goals, total_debt) = calculate_goals(user_id, pool, client).await?;

    let template = Index {
        total_debt: HumanDuration(total_debt),
        goals: goals.into_iter().map(|g| g.into()).collect(),
    };

    Ok(Html(template.render()?))
}
