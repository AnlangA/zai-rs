//! # Real-Time Audio and Video Module
//!
//! This module provides data structures and types for the GLM-Realtime API,
//! which enables real-time audio and video conversations with AI models.
//!
//! ## API Endpoint
//!
//! The GLM-Realtime API is built on top of WebSocket API:
//! - **API URL**: `wss://open.bigmodel.cn/api/paas/v4/realtime`
//! - **Authentication**: Use JWT or API Key in the `Authorization` header
//!
//! ## Key Features
//!
//! - **Real-time Communication** - WebSocket-based low-latency audio/video interactions
//! - **Multimodal Support** - Text, audio, and video input/output capabilities
//! - **Voice Activity Detection** - Both client-side and server-side VAD support
//! - **Function Calling** - Integration with external tools and APIs
//! - **Stream Processing** - Incremental response handling
//!
//! ## Module Structure
//!
//! - [`types`] - Core data types and shared structures
//! - [`client_events`] - Client-to-server event definitions
//! - [`server_events`] - Server-to-client event definitions
//! - [`session`] - Session configuration and management
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use zai_rs::real_time::*;
//!
//! // Create a session update event
//! let session_update = ClientEvent::SessionUpdate(SessionUpdateEvent {
//!     event_id: "session-123".to_string(),
//!     client_timestamp: 1625097600000,
//!     session: Session {
//!         model: "glm-realtime".to_string(),
//!         modalities: vec!["text".to_string(), "audio".to_string()],
//!         voice: "tongtong".to_string(),
//!         // ... other session parameters
//!         ..Default::default()
//!     },
//! });
//!
//! // Serialize to JSON for WebSocket transmission
//! let json = serde_json::to_string(&session_update)?;
//! ```

pub mod client_events;
pub mod server_events;
pub mod session;
pub mod types;

// Re-export commonly used types for convenience
pub use client_events::*;
pub use server_events::*;
pub use session::*;
pub use types::*;
