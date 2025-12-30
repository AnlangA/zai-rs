//! Static file serving routes

use axum::{
    extract::Path,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::path::PathBuf;
use tower_http::services::ServeDir;

/// Serve static files from the static directory
pub async fn static_files(Path(file_path): Path<PathBuf>) -> impl IntoResponse {
    let static_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("static");

    // Check if the file exists in the static directory
    let file_path = static_dir.join(&file_path);
    if !file_path.exists() {
        return (StatusCode::NOT_FOUND, "File not found").into_response();
    }

    // Serve the file
    let content = tokio::fs::read(&file_path).await;
    match content {
        Ok(content) => {
            // Determine the MIME type based on file extension
            let mime_type = mime_guess::from_path(&file_path)
                .first_or_octet_stream()
                .to_string();

            match Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime_type)
                .body(axum::body::Body::from(content))
            {
                Ok(response) => response.into_response(),
                Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to build response").into_response(),
            }
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read file").into_response(),
    }
}

/// Create a service for serving static files
pub fn static_service() -> ServeDir {
    let static_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("static");

    if !static_dir.exists() {
        tracing::warn!("Static directory does not exist: {:?}", static_dir);
    }

    ServeDir::new(static_dir)
}
