use askama::Template;
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
};
use reqwest::Client;
use sqlx::PgPool;

use crate::{errors::InternalResult, session::UserKey};

#[derive(Template)]
#[template(path = "newproject.html")]
pub struct NewProjectTemplate {}

pub async fn get(
    user_key: UserKey,
    Extension(pool): Extension<PgPool>,
    Extension(client): Extension<Client>,
) -> InternalResult<impl IntoResponse> {
    let template = NewProjectTemplate {};

    Ok(Html(template.render()?))
}
