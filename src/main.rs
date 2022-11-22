#![forbid(unsafe_code)]

use axum::{
    extract::Extension,
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;
use tracing::info;

mod errors;
mod routes;
mod session;
mod toggl;

#[tokio::main]
async fn main() {
    let _ = dotenv();

    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL environmental variable");

    let pool = PgPoolOptions::new()
        .connect(&database_url)
        .await
        .expect("Could not create database connection pool");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Could not run database migrations");

    let app = Router::new()
        .route("/", get(routes::index::get))
        .route("/login/", get(routes::login::get))
        .route("/login/", post(routes::login::post))
        .layer(TraceLayer::new_for_http())
        .layer(Extension(pool));

    info!("Starting hyper server");
    axum::Server::bind(&SocketAddr::from(([0, 0, 0, 0], 8000)))
        .serve(app.into_make_service())
        .await
        .expect("Error running hyper server");
}
