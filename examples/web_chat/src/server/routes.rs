//! Route composition for the web chat server.

use std::path::PathBuf;

use axum::Router;

pub mod chat;
pub mod health;
pub mod index;

pub fn api_routes() -> Router<crate::server::state::AppState> {
    Router::new().nest("/chat", chat::routes())
}

pub fn static_routes() -> tower_http::services::ServeDir {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("static");
    tower_http::services::ServeDir::new(directory)
}
