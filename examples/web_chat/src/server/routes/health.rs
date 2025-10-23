//! Health check endpoints

use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
    pub uptime: u64,
    pub services: HealthServices,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthServices {
    pub database: String,
    pub api: String,
    pub sessions: SessionHealth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionHealth {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub status: String,
}

/// Health check handler
pub async fn health_check(
    axum::extract::State(state): axum::extract::State<crate::server::state::AppState>,
) -> Result<Json<HealthResponse>, StatusCode> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .as_secs();

    let session_stats = state.sessions.stats();

    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp: now,
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime: now, // In a real app, you'd track actual startup time
        services: HealthServices {
            database: "healthy".to_string(), // Would check actual DB connection
            api: "healthy".to_string(),
            sessions: SessionHealth {
                total_sessions: session_stats.total_sessions,
                activeSessions: session_stats.active_sessions,
                status: if session_stats.total_sessions > 0 {
                    "operational"
                } else {
                    "no_sessions"
                }.to_string(),
            },
        },
    };

    Ok(Json(response))
}

/// Readiness check handler
pub async fn readiness_check() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "ready",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

/// Liveness check handler
pub async fn liveness_check() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "alive",
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}