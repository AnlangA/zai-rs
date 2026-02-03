//! # Real-time API Module
//!
//! Provides real-time audio and video communication capabilities for the Zhipu AI API.
//! This module is designed for interactive applications that require low-latency
//! audio and video streaming.
//!
//! ## Features
//!
//! - **Audio Streaming** - Real-time audio input and output
//! - **Video Streaming** - Real-time video input and output
//! - **Low Latency** - Optimized for minimal communication delay
//! - **Interactive Communication** - Support for bidirectional audio/video calls
//!
//! ## Usage
//!
//! ```rust,ignore
//! use zai_rs::realTime::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = RealTimeClient::new(api_key);
//!
//!     // Start an audio session
//!     let session = client
//!         .audio_session()
//!         .model(RealTimeModel::Glm4Voice)
//!         .build()
//!         .await?;
//!
//!     // Send audio data
//!     session.send_audio(audio_bytes).await?;
//!
//!     // Receive responses
//!     while let Some(event) = session.next_event().await? {
//!         match event {
//!             RealTimeEvent::Audio(data) => {
//!                 // Handle audio response
//!             },
//!             RealTimeEvent::Text(text) => {
//!                 // Handle transcription
//!             },
//!             _ => {},
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod models;
pub mod session;
pub mod types;

pub use client::*;
pub use models::*;
pub use session::*;
pub use types::*;
