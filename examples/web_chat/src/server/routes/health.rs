//! Liveness, readiness, and process health endpoints.

use axum::{Json, extract::State};
use serde::Serialize;

use crate::server::state::{AppState, SessionStats};

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
    timestamp: String,
    version: &'static str,
    uptime_seconds: u64,
    sessions: SessionStats,
}

pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.uptime().as_secs(),
        sessions: state.sessions.stats(),
    })
}

#[derive(Serialize)]
pub struct ProbeResponse {
    status: &'static str,
}

pub async fn readiness_check() -> Json<ProbeResponse> {
    Json(ProbeResponse { status: "ready" })
}

pub async fn liveness_check() -> Json<ProbeResponse> {
    Json(ProbeResponse { status: "alive" })
}
