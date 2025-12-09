//! Middleware for the web_chat application

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::Instant;
use tracing::{info, warn};

/// Request logging middleware
pub async fn request_logger(req: Request, next: Next) -> Result<Response, StatusCode> {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();

    let response = next.run(req).await;

    let duration = start.elapsed();
    let status = response.status();

    if status.is_success() {
        info!(
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = duration.as_millis(),
            "Request completed"
        );
    } else {
        warn!(
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = duration.as_millis(),
            "Request completed with error"
        );
    }

    Ok(response)
}

/// CORS middleware
pub async fn cors_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    // Allow any method and origin for this example
    // In production, you should configure this properly
    let mut response = next.run(req).await;

    response.headers_mut().insert(
        "Access-Control-Allow-Origin",
        "http://localhost:3000".parse().unwrap(),
    );
    response.headers_mut().insert(
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap(),
    );
    response.headers_mut().insert(
        "Access-Control-Allow-Headers",
        "Content-Type, Authorization".parse().unwrap(),
    );

    Ok(response)
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    // This is a placeholder for rate limiting
    // In a real application, you would implement proper rate limiting here

    let response = next.run(req).await;
    Ok(response)
}
