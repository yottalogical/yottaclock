use askama::Template;
use axum::response::{Html, IntoResponse};

use crate::templates;

pub async fn index() -> impl IntoResponse {
    let template = templates::Index {};

    Html(template.render().unwrap())
}
