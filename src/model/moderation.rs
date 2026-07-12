//! # Content Moderation Module
//!
//! This module provides content moderation functionality for analyzing text,
//! image, audio, and video content for safety risks.
//!
//! ## Features
//!
//! - **Multi-format support** - Text, image, audio, and video content
//!   moderation
//! - **Risk detection** - Identifies pornographic, violent, and illegal content
//! - **Structured results** - Detailed risk level and type information
//! - **Validation** - Input validation using the validator crate
//!
//! ## Examples
//!
//! ```no_run
//! use zai_rs::model::moderation::*;
//! use zai_rs::ZaiClient;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ZaiClient::from_env()?;
//! // Text moderation
//! let moderation = Moderation::new_text("Content to review");
//! let _result = moderation.send_via(&client).await?;
//!
//! // Multimedia moderation
//! let moderation = Moderation::new_multimedia(
//!     MediaType::Image,
//!     "https://example.com/image.jpg"
//! );
//! let _result = moderation.send_via(&client).await?;
//! # Ok(())
//! # }
//! ```

/// Request builder and client for content moderation.
mod data;
/// Supported moderation model ids.
mod models;

// Re-export the client-facing builder and all moderation wire types.
pub use data::Moderation;
pub use models::*;
