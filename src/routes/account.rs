use std::str::FromStr;

use askama::Template;
use axum::{
    debug_handler,
    extract::{Extension, Form},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
};
use chrono::Duration;
use chrono_tz::{Tz, TZ_VARIANTS};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    errors::{InternalError, InternalResult},
    human_duration::hours_minutes_seconds,
    session::UserKey,
};

#[derive(Template)]
#[template(path = "account.html")]
struct AccountTemplate<'a> {
    daily_max_hours: i64,
    daily_max_minutes: i64,
    daily_max_seconds: i64,
    user_timezone: &'a Tz,
    timezones: &'a [Tz],
}

#[debug_handler]
pub async fn get(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
) -> InternalResult<impl IntoResponse> {
    let record = sqlx::query!(
        "SELECT daily_max, timezone FROM users WHERE user_key = $1",
        user_key.0,
    )
    .fetch_one(&pool)
    .await?;

    let timezone: Tz = record
        .timezone
        .parse()
        .map_err(InternalError::UnrecognizedTimezone)?;

    let (daily_max_hours, daily_max_minutes, daily_max_seconds) =
        hours_minutes_seconds(Duration::seconds(record.daily_max));

    let template = AccountTemplate {
        daily_max_hours,
        daily_max_minutes,
        daily_max_seconds,
        user_timezone: &timezone,
        timezones: &TZ_VARIANTS,
    };

    Ok(Html(template.render()?))
}

#[derive(Debug, Deserialize)]
pub struct AccountForm {
    daily_max_hours: i64,
    daily_max_minutes: i64,
    daily_max_seconds: i64,
    timezone: String,
}

#[debug_handler]
pub async fn post(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<AccountForm>,
) -> InternalResult<impl IntoResponse> {
    let daily_max = Duration::hours(form.daily_max_hours)
        + Duration::minutes(form.daily_max_minutes)
        + Duration::seconds(form.daily_max_seconds);

    let positive_daily_max = daily_max >= Duration::zero();
    let timezone: Result<Tz, <Tz as FromStr>::Err> = form.timezone.parse();

    Ok(
        if let (true, Ok(timezone)) = (positive_daily_max, timezone) {
            sqlx::query!(
                "UPDATE users SET daily_max = $1, timezone = $2 WHERE user_key = $3",
                daily_max.num_seconds(),
                timezone.to_string(),
                user_key.0,
            )
            .execute(&pool)
            .await?;

            Redirect::to("/").into_response()
        } else {
            StatusCode::BAD_REQUEST.into_response()
        },
    )
}
