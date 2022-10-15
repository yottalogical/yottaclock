#![forbid(unsafe_code)]

use axum::{extract::Extension, routing::get, Router};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;

mod errors;
mod routes;
mod session;
mod templates;

#[tokio::main]
async fn main() {
    let _ = dotenv();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL environmental variable");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Could not create database connection pool");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Could not run database migrations");

    let app = Router::new()
        .route("/", get(routes::index))
        .layer(Extension(pool));

    println!("Server started!");
    axum::Server::bind(&SocketAddr::from(([0, 0, 0, 0], 8000)))
        .serve(app.into_make_service())
        .await
        .unwrap();
}
