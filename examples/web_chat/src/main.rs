//! Modern AI Chat Web Application
//!
//! A production-quality web chat interface with streaming capabilities,
//! perfect markdown rendering, and industry-leading user experience.

use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
    LatencyUnit,
};
use tracing::{info, Level};

mod server;
use server::{config::Config, routes, state::AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("🚀 Starting Modern AI Chat Application");

    // Load configuration
    let config = Config::from_env()?;
    info!("📋 Configuration loaded: {:?}", config);

    // Initialize application state
    let state = AppState::new(config.clone());

    // Build the application router
    let app = create_app(state, config);

    // Create socket address
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("🌐 Server listening on http://{}", addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Creates the main application router with all routes and middleware
fn create_app(state: AppState, config: Config) -> Router {
    // Configure CORS
    let cors_layer = CorsLayer::new()
        .allow_origin(config.cors_origins.iter().map(|s| s.parse().unwrap()).collect::<Vec<_>>())
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
        .allow_credentials(true);

    // Configure request tracing
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(LatencyUnit::Millis),
        );

    // Build the router
    Router::new()
        // API routes
        .nest("/api", routes::api_routes())
        // Static file serving
        .nest_service("/static", routes::static_routes())
        // Health check
        .route("/health", get(routes::health::health_check))
        // Main page
        .route("/", get(routes::index::index_handler))
        // Add state and middleware
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(cors_layer)
                .layer(trace_layer),
        )
}