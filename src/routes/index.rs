use crate::{
    errors::InternalResult,
    session::UserKey,
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

#[cfg(test)]
mod tests;

pub struct HumanDuration(pub Duration);

impl Display for HumanDuration {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let (absolute_duration, sign) = if self.0 > Duration::seconds(0) {
            (self.0, "")
        } else {
            (-self.0, "-")
        };

        let hours = absolute_duration.num_hours();
        let minutes = absolute_duration.num_minutes() - 60 * absolute_duration.num_hours();
        let seconds = absolute_duration.num_seconds() - 60 * absolute_duration.num_minutes();

        write!(f, "{}{}:{:0>2}:{:0>2}", sign, hours, minutes, seconds)
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
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Extension(client): Extension<Client>,
) -> InternalResult<impl IntoResponse> {
    let (goals, total_debt) = calculate_goals(user_key, pool, client).await?;

    let template = Index {
        total_debt: HumanDuration(total_debt),
        goals: goals.into_iter().map(|g| g.into()).collect(),
    };

    Ok(Html(template.render()?))
}
