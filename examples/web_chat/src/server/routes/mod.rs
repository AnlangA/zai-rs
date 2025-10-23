//! Route definitions for the web chat application

pub mod chat;
pub mod health;
pub mod index;
pub mod static_files;

use axum::Router;

/// Create all API routes
pub fn api_routes() -> Router<crate::server::state::AppState> {
    Router::new()
        .nest("/chat", chat::routes())
        .nest("/sessions", sessions::routes())
}

/// Session routes
pub mod sessions {
    use axum::Router;
    
    pub fn routes() -> Router<crate::server::state::AppState> {
        Router::new()
    }
}

/// Static file routes
pub fn static_routes() -> tower_http::services::ServeDir {
    tower_http::services::ServeDir::new("examples/web_chat/static")
}