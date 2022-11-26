use axum::{
    routing::{get, post},
    Router,
};

mod index;
mod login;
pub mod signup;

pub fn router() -> Router {
    Router::new()
        .route("/", get(index::get))
        .route("/login/", get(login::get))
        .route("/login/", post(login::post))
        .route("/signup/", get(signup::get))
        .route("/signup/", post(signup::post_step1))
        .route("/signup/step2", post(signup::post_step2))
}
