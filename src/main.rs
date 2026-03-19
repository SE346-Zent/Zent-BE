use axum::{response::IntoResponse, routing::post, Json, Router};
use serde::Deserialize;

pub mod entities;
pub mod extractor;
pub mod handlers;
pub mod model;
pub mod services;
pub mod state;

#[derive(Deserialize)]
struct User {
    name: String,
}

async fn hello_user(Json(user): Json<User>) -> impl IntoResponse {
    format!("hello {}", user.name)
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/hello", post(hello_user));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
