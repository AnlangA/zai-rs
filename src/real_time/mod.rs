//! # Real-time Audio and Video Communication Module
//!
//! This module provides functionality for real-time audio and video conversations with GLM models.
//! It implements the GLM-Realtime API which enables low-latency voice and video interactions
//! through WebSocket connections.
//!
//! ## Features
//!
//! - **Real-time Audio Chat**: Bidirectional audio conversations with AI models
//! - **Video Chat Support**: Video input with AI understanding and response
//! - **Function Calling**: Tool integration during real-time conversations
//! - **Voice Activity Detection**: Both client and server-side VAD support
//! - **Audio Processing**: Input audio noise reduction and format handling
//!
//! ## Components
//!
//! - [`client`] - WebSocket client for real-time communication
//! - [`types`] - Data structures for events and conversations
//! - [`models`] - Real-time model definitions
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use zai_rs::real_time::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let api_key = std::env::var("ZHIPU_API_KEY")?;
//!
//!     // Create a real-time client
//!     let mut client = RealtimeClient::new(api_key);
//!
//!     // Configure the session
//!     let session_config = SessionConfig {
//!         model: "glm-realtime-flash".to_string(),
//!         modalities: vec!["text".to_string(), "audio".to_string()],
//!         voice: "tongtong".to_string(),
//!         input_audio_format: "wav".to_string(),
//!         output_audio_format: "pcm".to_string(),
//!         // ... other configuration options
//!         ..Default::default()
//!     };
//!
//!     // Connect and start a session
//!     client.connect(session_config).await?;
//!
//!     // Handle events...
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Event Flow
//!
//! The real-time API follows an event-driven model where:
//!
//! 1. Client sends configuration and input events
//! 2. Server processes and responds with output events
//! 3. Both sides maintain a persistent WebSocket connection
//!
//! For more detailed information about the API, see the [GLM-Realtime documentation](https://docs.bigmodel.cn/cn/guide/models/sound-and-video/glm-realtime).

pub mod client;
pub mod models;
pub mod types;

// Re-export the main types for convenience
pub use client::RealtimeClient;
pub use models::{GLMRealtime, RealtimeModel};
pub use types::*;
