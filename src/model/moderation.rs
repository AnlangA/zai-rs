//! Content-moderation requests and responses for text, image, audio, and video.
//!
//! ## Features
//!
//! - **Multi-format support** - Text, image, audio, and video content
//!   moderation
//! - **Risk detection** - Identifies pornographic, violent, and illegal content
//! - **Structured results** - Detailed risk level and type information
//! - **Validation** - Input validation before dispatch
//!
//! # Examples
//!
//! ```rust,no_run
//! use zai_rs::model::moderation::*;
//! use zai_rs::ZaiClient;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ZaiClient::from_env()?;
//! let moderation = Moderation::new_text("Content to review");
//! let _result = moderation.send_via(&client).await?;
//!
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

pub use data::Moderation;
pub use models::*;
