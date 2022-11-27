use axum::{
    routing::{get, post},
    Router,
};

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
}
