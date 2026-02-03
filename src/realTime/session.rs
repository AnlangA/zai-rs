//! Real-time API session management

use std::sync::Arc;
use tokio::sync::Mutex;

use super::types::*;

/// Real-time session handle
///
/// This represents an active real-time communication session with the API.
pub struct RealTimeSession {
    pub(crate) session_id: String,
    pub(crate) config: SessionConfig,
    pub(crate) state: Arc<Mutex<SessionState>>,
    pub(crate) stats: Arc<Mutex<SessionStats>>,
}

impl RealTimeSession {
    /// Create a new real-time session
    pub(crate) fn new(session_id: String, config: SessionConfig) -> Self {
        Self {
            session_id,
            config,
            state: Arc::new(Mutex::new(SessionState::Connecting)),
            stats: Arc::new(Mutex::new(SessionStats {
                duration_seconds: 0,
                packets_sent: 0,
                packets_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                transcription_count: 0,
            })),
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the current session state
    pub async fn state(&self) -> SessionState {
        *self.state.lock().await
    }

    /// Get session statistics
    pub async fn stats(&self) -> SessionStats {
        self.stats.lock().await.clone()
    }

    /// Check if the session is active
    pub async fn is_active(&self) -> bool {
        matches!(
            *self.state.lock().await,
            SessionState::Connected
                | SessionState::Listening
                | SessionState::Processing
                | SessionState::Speaking
        )
    }

    /// Update the session state
    pub(crate) async fn update_state(&self, new_state: SessionState) {
        *self.state.lock().await = new_state;
    }

    /// Record sent audio packet
    pub(crate) async fn record_packet_sent(&self, bytes: u64) {
        let mut stats = self.stats.lock().await;
        stats.packets_sent += 1;
        stats.bytes_sent += bytes;
    }

    /// Record received audio packet
    pub(crate) async fn record_packet_received(&self, bytes: u64) {
        let mut stats = self.stats.lock().await;
        stats.packets_received += 1;
        stats.bytes_received += bytes;
    }

    /// Increment transcription count
    pub(crate) async fn increment_transcription_count(&self) {
        let mut stats = self.stats.lock().await;
        stats.transcription_count += 1;
    }
}
