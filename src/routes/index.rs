use crate::{errors::InternalResult, session::UserId, templates};
use askama::Template;
use axum::response::{Html, IntoResponse};

pub async fn get(UserId(user_id): UserId) -> InternalResult<impl IntoResponse> {
    let template = templates::Index { user_id };
    Ok(Html(template.render()?))
}
