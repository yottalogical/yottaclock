use askama::Template;
use axum::{
    extract::Form,
    response::{Html, IntoResponse, Redirect},
    Extension,
};
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    session::{new_session_cookie_header, UserKey},
};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    unrecognized_api_token: bool,
}

pub async fn get() -> InternalResult<impl IntoResponse> {
    let template = LoginTemplate {
        unrecognized_api_token: false,
    };

    Ok(Html(template.render()?))
}

#[derive(Deserialize)]
pub struct LoginForm {
    toggl_api_key: String,
}

pub async fn post(
    Extension(pool): Extension<PgPool>,
    Form(form): Form<LoginForm>,
) -> InternalResult<impl IntoResponse> {
    let user = sqlx::query!(
        "SELECT user_key
        FROM users
        WHERE toggl_api_key = $1",
        form.toggl_api_key
    )
    .fetch_optional(&pool)
    .await?;

    Ok(if let Some(user) = user {
        (
            [new_session_cookie_header(UserKey(user.user_key), &pool).await?],
            Redirect::to("/"),
        )
            .into_response()
    } else {
        let template = LoginTemplate {
            unrecognized_api_token: true,
        };

        Html(template.render()?).into_response()
    })
}
