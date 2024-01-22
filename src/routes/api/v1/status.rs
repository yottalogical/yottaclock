use axum::{debug_handler, extract::Extension, response::IntoResponse, Json};
use chrono::Duration;
use reqwest::Client;
use serde::Serialize;
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    session::UserKey,
    toggl::{self, calculate_goals},
};

#[derive(Serialize)]
pub struct Goal {
    pub name: String,
    pub time: i64,
}

impl From<toggl::Goal> for Goal {
    fn from(other: toggl::Goal) -> Self {
        Self {
            name: other.name,
            time: other.time.num_seconds(),
        }
    }
}

#[derive(Serialize)]
pub struct ResponseBody {
    pub total_debt: i64,
    pub daily_max: i64,
    pub goals: Vec<Goal>,
}

#[debug_handler]
pub async fn get(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Extension(client): Extension<Client>,
) -> InternalResult<impl IntoResponse> {
    let record = sqlx::query!(
        "SELECT daily_max FROM users WHERE user_key = $1",
        user_key.0,
    )
    .fetch_one(&pool)
    .await?;

    let daily_max = Duration::seconds(record.daily_max);

    let response_body =
        if let Some((goals, total_debt)) = calculate_goals(user_key, pool, client).await? {
            ResponseBody {
                total_debt: total_debt.num_seconds(),
                daily_max: daily_max.num_seconds(),
                goals: goals.into_iter().map(Goal::from).collect(),
            }
        } else {
            ResponseBody {
                total_debt: 0,
                daily_max: 0,
                goals: Vec::from([]),
            }
        };

    Ok(Json(response_body).into_response())
}
