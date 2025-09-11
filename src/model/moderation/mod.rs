//! # Content Moderation Module
//!
//! This module provides content moderation functionality for analyzing text, image,
//! audio, and video content for safety risks.
//!
//! ## Features
//!
//! - **Multi-format support** - Text, image, audio, and video content moderation
//! - **Risk detection** - Identifies pornographic, violent, and illegal content
//! - **Structured results** - Detailed risk level and type information
//! - **Validation** - Input validation using the validator crate
//!
//! ## Examples
//!
//! ```rust,ignore
//! use zai_rs::model::moderation::*;
//!
//! // Text moderation
//! let moderation = Moderation::new_text("审核内容安全样例字符串。", api_key);
//! let result = moderation.send().await?;
//!
//! // Multimedia moderation
//! let moderation = Moderation::new_multimedia(
//!     MediaType::Image,
//!     "https://example.com/image.jpg",
//!     api_key
//! );
//! let result = moderation.send().await?;
//! ```

pub mod data;
pub mod models;

// Re-export main types for convenience
pub use data::Moderation;
pub use models::*;