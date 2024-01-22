use askama::Template;
use axum::{
    debug_handler,
    extract::Form,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Extension,
};
use chrono_tz::TZ_VARIANTS;
use reqwest::Client;
use serde::Deserialize;
use sqlx::PgPool;

use crate::{
    errors::InternalResult,
    session::{new_session_cookie_header, UserKey},
    toggl::get_workspaces,
};

use super::signup::SignupTemplate;

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    unrecognized_api_token: bool,
}

#[debug_handler]
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

#[debug_handler]
pub async fn post(
    Extension(pool): Extension<PgPool>,
    Extension(client): Extension<Client>,
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
        // User already exists: Log them in
        (
            [new_session_cookie_header(UserKey(user.user_key), &pool).await?],
            Redirect::to("/"),
        )
            .into_response()
    } else if let Some(workspaces) = get_workspaces(&form.toggl_api_key, client).await? {
        // User doesn't exist, but the API token works: Sign them up
        let template = SignupTemplate {
            toggl_api_key: &form.toggl_api_key,
            workspaces: &workspaces,
            timezones: &TZ_VARIANTS,
        };

        Html(template.render()?).into_response()
    } else {
        // User doesn't exist and the API token works: Respond with an error message
        let template = LoginTemplate {
            unrecognized_api_token: true,
        };

        (StatusCode::BAD_REQUEST, Html(template.render()?)).into_response()
    })
}
