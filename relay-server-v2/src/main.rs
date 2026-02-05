mod protocol;
mod session;

use axum::{routing::get, Router};
use std::net::SocketAddr;
use tracing::info;

async fn root() -> &'static str {
    "Relay Server v2"
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get port from environment variable or use default
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a valid number");

    // Build router
    let app = Router::new().route("/", get(root));

    // Bind and serve
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Relay server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
