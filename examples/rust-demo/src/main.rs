use axum::{Router, routing::get};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/api/v1/users", get(users));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn root() -> &'static str {
    "Hello from Arcane Rust Demo! 🦀"
}

async fn health() -> &'static str {
    "OK"
}

async fn users() -> &'static str {
    r#"{"users": ["Alice", "Bob", "Charlie"]}"#
}
