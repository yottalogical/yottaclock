use axum::{
    routing::{get, post},
    Router,
};

mod account;
mod api;
mod dayoff_delete;
mod dayoff_new;
mod daysoff;
mod index;
mod login;
mod project_delete;
mod project_new;
mod projects;
mod signup;

pub fn router() -> Router {
    Router::new()
        .route("/", get(index::get))
        .route("/login/", get(login::get))
        .route("/login/", post(login::post))
        .route("/signup/", post(signup::post))
        .route("/projects/", get(projects::get))
        .route("/project/new/", get(project_new::get))
        .route("/project/new/", post(project_new::post))
        .route("/project/delete/", post(project_delete::post))
        .route("/account/", get(account::get))
        .route("/account/", post(account::post))
        .route("/daysoff/", get(daysoff::get))
        .route("/dayoff/new/", get(dayoff_new::get))
        .route("/dayoff/new/", post(dayoff_new::post))
        .route("/dayoff/delete/", post(dayoff_delete::post))
        .route("/api/v1/status/", get(api::v1::status::get))
}
