mod assets;
mod handlers;
mod protocol;
mod session;
mod state;

use axum::{extract::State, routing::get, Router};
use axum_embed::ServeEmbed;
use std::net::SocketAddr;
use tracing::info;

use crate::assets::Assets;
use crate::state::AppState;

async fn debug_sessions(State(state): State<AppState>) -> String {
    format!("Active sessions: {}", state.session_count())
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

    // Create application state
    let state = AppState::new();

    // Create embedded asset server with SPA fallback
    // First param: index file for "/" route, Second: fallback behavior for unknown paths
    let serve_assets = ServeEmbed::<Assets>::with_parameters(
        Some("index.html".to_owned()),
        axum_embed::FallbackBehavior::Ok,
        None,
    );

    // Build router
    let app = Router::new()
        .route("/ws", get(handlers::ws_handler))
        .route("/debug/sessions", get(debug_sessions))
        .fallback_service(serve_assets)
        .with_state(state);

    // Bind and serve
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Relay server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
