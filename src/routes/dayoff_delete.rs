use axum::{
    extract::{Extension, Form},
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{errors::InternalResult, session::UserKey};

use super::daysoff::DayOffKey;

#[derive(Deserialize)]
pub struct DayOffDeleteForm {
    day_off_key: DayOffKey,
}

pub async fn post(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Form(form): Form<DayOffDeleteForm>,
) -> InternalResult<impl IntoResponse> {
    sqlx::query!(
        "DELETE FROM days_off
        WHERE day_off_key = $1
        AND user_key = $2",
        form.day_off_key.0,
        user_key.0,
    )
    .execute(&pool)
    .await?;

    Ok(Redirect::to("/daysoff/"))
}
