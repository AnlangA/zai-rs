//! Minimal browser chat example backed by `zai-rs`.

use std::net::SocketAddr;

use axum::{
    Router,
    extract::MatchedPath,
    http::{HeaderName, HeaderValue, Method, header},
    routing::get,
};
use tower_http::{
    cors::CorsLayer,
    set_header::SetResponseHeaderLayer,
    trace::{DefaultOnResponse, TraceLayer},
};
use tracing::{Level, info};

mod server;

use server::{config::Config, routes, state::AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();

    let config = Config::from_env()?;
    let address = SocketAddr::new(config.bind_address, config.port);
    let state = AppState::new(&config)?;
    spawn_maintenance(state.sessions.clone(), state.rate_limiter.clone());
    let app = create_app(state, &config);

    let listener = tokio::net::TcpListener::bind(address).await?;
    info!(%address, "web chat listening");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn spawn_maintenance(
    sessions: std::sync::Arc<server::state::SessionStore>,
    rate_limiter: std::sync::Arc<server::state::RateLimiter>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        // `interval` ticks immediately once; skip that tick because the store
        // has just been created.
        interval.tick().await;
        loop {
            interval.tick().await;
            let removed = sessions.remove_expired();
            if removed > 0 {
                tracing::debug!(removed, "expired chat sessions removed");
            }
            rate_limiter.remove_inactive(std::time::Duration::from_secs(120));
        }
    });
}

fn create_app(state: AppState, config: &Config) -> Router {
    // Only the headers and methods used by this example are accepted. The
    // peer address comes from the TCP connection, not an untrusted proxy
    // header, and is used for rate limiting in the chat handlers.
    let cors = CorsLayer::new()
        .allow_origin(config.cors_origins.clone())
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE]);
    let trace = TraceLayer::new_for_http()
        .make_span_with(|request: &axum::http::Request<_>| {
            let route = request
                .extensions()
                .get::<MatchedPath>()
                .map_or("<unmatched>", MatchedPath::as_str);
            tracing::info_span!("http_request", method = %request.method(), route)
        })
        .on_response(DefaultOnResponse::new().level(Level::INFO));

    Router::new()
        .nest("/api", routes::api_routes())
        .nest_service("/static", routes::static_routes())
        .route("/health", get(routes::health::health_check))
        .route("/ready", get(routes::health::readiness_check))
        .route("/live", get(routes::health::liveness_check))
        .route("/", get(routes::index::index_handler))
        .with_state(state)
        .layer(trace)
        .layer(cors)
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'; object-src 'none'",
            ),
        ))
}
