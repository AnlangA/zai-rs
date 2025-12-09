//! Application state management

use crate::server::{config::Config, error::AppResult};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use zai_rs::model::{TextMessage, chat_models::GLM4_6};

/// Application state shared across all requests
#[derive(Clone)]
pub struct AppState {
    /// Application configuration
    pub config: Config,

    /// Session store for managing chat sessions
    pub sessions: Arc<SessionStore>,

    /// Rate limiter for API requests
    pub rate_limiter: Arc<RateLimiter>,
}

impl AppState {
    /// Create a new application state
    pub fn new(config: Config) -> Self {
        Self {
            sessions: Arc::new(SessionStore::new(config.session_timeout)),
            rate_limiter: Arc::new(RateLimiter::new()),
            config,
        }
    }
}

/// Session store for managing chat conversations
pub struct SessionStore {
    /// Map of session IDs to chat sessions
    sessions: DashMap<String, ChatSession>,

    /// Session timeout in seconds
    timeout: u64,
}

impl SessionStore {
    /// Create a new session store
    pub fn new(timeout: u64) -> Self {
        Self {
            sessions: DashMap::new(),
            timeout,
        }
    }

    /// Get or create a session
    pub fn get_or_create(&self, session_id: Option<String>) -> AppResult<String> {
        let session_id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        if !self.sessions.contains_key(&session_id) {
            let session = ChatSession::new();
            self.sessions.insert(session_id.clone(), session);
        }

        Ok(session_id)
    }

    /// Get a session by ID
    pub fn get(&self, session_id: &str) -> AppResult<ChatSession> {
        self.sessions
            .get(session_id)
            .map(|session| session.clone())
            .ok_or_else(|| crate::server::error::AppError::SessionNotFound(session_id.to_string()))
    }

    /// Update a session
    pub fn update(&self, session_id: &str, session: ChatSession) -> AppResult<()> {
        self.sessions.insert(session_id.to_string(), session);
        Ok(())
    }

    /// Remove expired sessions
    pub async fn cleanup_expired(&self) {
        let now = Utc::now();
        let expired_keys: Vec<String> = self
            .sessions
            .iter()
            .filter(|entry| {
                let session = entry.value();
                now.signed_duration_since(session.last_activity)
                    .num_seconds()
                    > self.timeout as i64
            })
            .map(|entry| entry.key().clone())
            .collect();

        for key in expired_keys {
            self.sessions.remove(&key);
        }
    }

    /// Get session statistics
    pub fn stats(&self) -> SessionStats {
        SessionStats {
            total_sessions: self.sessions.len(),
            activeSessions: self
                .sessions
                .iter()
                .filter(|entry| {
                    let session = entry.value();
                    Utc::now()
                        .signed_duration_since(session.last_activity)
                        .num_seconds()
                        < 300 // 5 minutes
                })
                .count(),
        }
    }
}

/// Chat session containing conversation history
#[derive(Clone, Debug)]
pub struct ChatSession {
    /// Unique session identifier
    pub id: String,

    /// Conversation messages
    pub messages: Vec<TextMessage>,

    /// Session creation time
    pub created_at: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Session metadata
    pub metadata: SessionMetadata,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct SessionMetadata {
    /// User agent string
    pub user_agent: Option<String>,

    /// Client IP address
    pub client_ip: Option<String>,

    /// Preferred language
    pub language: Option<String>,

    /// Think mode enabled
    pub think_mode: bool,

    /// Total tokens used
    pub total_tokens: u64,

    /// Number of requests made
    pub request_count: u64,
}

impl ChatSession {
    /// Create a new chat session
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            messages: Vec::new(),
            created_at: Utc::now(),
            last_activity: Utc::now(),
            metadata: SessionMetadata {
                user_agent: None,
                client_ip: None,
                language: None,
                think_mode: false,
                total_tokens: 0,
                request_count: 0,
            },
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, message: TextMessage) {
        self.messages.push(message);
        self.last_activity = Utc::now();
    }

    /// Get recent messages for context
    pub fn get_recent_messages(&self, count: usize) -> Vec<TextMessage> {
        let start = self.messages.len().saturating_sub(count);
        self.messages[start..].to_vec()
    }

    /// Update metadata
    pub fn update_metadata<F>(&mut self, f: F)
    where
        F: FnOnce(&mut SessionMetadata),
    {
        f(&mut self.metadata);
        self.last_activity = Utc::now();
    }
}

/// Session statistics
#[derive(Debug, Serialize)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub activeSessions: usize,
}

/// Rate limiter for API requests
pub struct RateLimiter {
    /// Request counts per IP address
    requests: DashMap<String, RateLimitData>,
}

#[derive(Clone, Debug)]
struct RateLimitData {
    /// Number of requests in the current window
    count: u64,

    /// Window start time
    window_start: DateTime<Utc>,

    /// Total requests (for statistics)
    total_requests: u64,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new() -> Self {
        Self {
            requests: DashMap::new(),
        }
    }

    /// Check if a request is allowed
    pub fn is_allowed(&self, ip: &str, limit: u64, window_seconds: u64) -> AppResult<bool> {
        let now = Utc::now();
        let mut entry = self
            .requests
            .entry(ip.to_string())
            .or_insert_with(|| RateLimitData {
                count: 0,
                window_start: now,
                total_requests: 0,
            });

        // Check if we need to reset the window
        let window_duration = chrono::Duration::seconds(window_seconds as i64);
        if now.signed_duration_since(entry.window_start) > window_duration {
            entry.count = 0;
            entry.window_start = now;
        }

        // Check rate limit
        if entry.count >= limit {
            return Ok(false);
        }

        // Increment counters
        entry.count += 1;
        entry.total_requests += 1;

        Ok(true)
    }

    /// Get rate limit statistics
    pub fn stats(&self) -> RateLimitStats {
        let total_ips = self.requests.len();
        let total_requests: u64 = self
            .requests
            .iter()
            .map(|entry| entry.value().total_requests)
            .sum();

        RateLimitStats {
            total_ips,
            total_requests,
            average_requests_per_ip: if total_ips > 0 {
                total_requests / total_ips as u64
            } else {
                0
            },
        }
    }
}

/// Rate limiter statistics
#[derive(Debug, Serialize)]
pub struct RateLimitStats {
    pub total_ips: usize,
    pub total_requests: u64,
    pub average_requests_per_ip: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_store() {
        let store = SessionStore::new(3600);
        let session_id = store.get_or_create(None).unwrap();

        assert!(!session_id.is_empty());

        let session = store.get(&session_id).unwrap();
        assert_eq!(session.id, session_id);
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new();
        let ip = "127.0.0.1";

        // Should allow first request
        assert!(limiter.is_allowed(ip, 5, 60).unwrap());

        // Should allow up to limit
        for _ in 1..5 {
            assert!(limiter.is_allowed(ip, 5, 60).unwrap());
        }

        // Should deny when limit exceeded
        assert!(!limiter.is_allowed(ip, 5, 60).unwrap());
    }
}
