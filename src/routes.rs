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
        .route("/signup/", post(signup::post))
}
