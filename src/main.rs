use axum::{routing::get, Router};

mod routes;
mod templates;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(routes::index));

    axum::Server::bind(&"0.0.0.0:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}
