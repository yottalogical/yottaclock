#![forbid(unsafe_code)]

use axum::extract::Extension;
use dotenvy::dotenv;
use reqwest::Client;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

mod errors;
mod human_duration;
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

    let client = Client::builder()
        .use_rustls_tls()
        .https_only(true)
        .build()
        .expect("Could not create reqwest client");

    let app = routes::router()
        .layer(TraceLayer::new_for_http())
        .layer(Extension(pool))
        .layer(Extension(client));

    info!("Starting hyper server");
    axum::serve(
        TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], 8000)))
            .await
            .unwrap(),
        app,
    )
    .await
    .expect("Error running hyper server");
}
