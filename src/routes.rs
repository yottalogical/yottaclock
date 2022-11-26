use axum::{
    routing::{get, post},
    Router,
};

mod index;
mod login;
mod newproject;
mod signup;

pub fn router() -> Router {
    Router::new()
        .route("/", get(index::get))
        .route("/login/", get(login::get))
        .route("/login/", post(login::post))
        .route("/signup/", post(signup::post))
        .route("/newproject/", get(newproject::get))
}
