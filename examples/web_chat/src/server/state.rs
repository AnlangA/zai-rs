//! Concurrent in-memory sessions and per-peer rate limiting.

use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::Serialize;
use tokio::sync::{Mutex, OwnedMutexGuard};
use uuid::Uuid;
use zai_rs::{client::ZaiClient, model::TextMessage};

use crate::server::{
    config::Config,
    error::{AppError, AppResult},
};

#[derive(Clone)]
pub struct AppState {
    pub zai_client: ZaiClient,
    pub sessions: Arc<SessionStore>,
    pub rate_limiter: Arc<RateLimiter>,
    started_at: Instant,
}

impl AppState {
    pub fn new(config: &Config) -> AppResult<Self> {
        Ok(Self {
            zai_client: ZaiClient::builder(&config.api_key).build()?,
            sessions: Arc::new(SessionStore::new(
                config.session_timeout_secs,
                config.max_messages_per_session,
            )),
            rate_limiter: Arc::new(RateLimiter::default()),
            started_at: Instant::now(),
        })
    }

    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}

pub struct SessionStore {
    sessions: DashMap<String, ChatSession>,
    turn_locks: DashMap<String, Arc<Mutex<()>>>,
    timeout: Duration,
    max_messages: usize,
}

/// Exclusive access to one conversation while a chat turn is in flight.
///
/// The guard is intentionally held until the upstream request (including an
/// SSE response body) finishes or is dropped. This keeps concurrent requests
/// for the same session in request order without serializing unrelated users.
pub struct SessionTurn {
    session_id: String,
    _guard: OwnedMutexGuard<()>,
}

impl SessionTurn {
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl SessionStore {
    pub fn new(timeout_secs: u64, max_messages: usize) -> Self {
        Self {
            sessions: DashMap::new(),
            turn_locks: DashMap::new(),
            timeout: Duration::from_secs(timeout_secs),
            max_messages,
        }
    }

    /// Lock a requested conversation, recreating it if its idle timeout has
    /// elapsed. The caller must retain the returned guard for the entire turn.
    pub async fn start_turn(&self, requested_id: Option<&str>) -> SessionTurn {
        let id = requested_id
            .map(str::to_owned)
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let (_, guard) = self.acquire_turn_lock(&id).await;
        let expired = self
            .sessions
            .get(&id)
            .is_some_and(|session| self.is_expired(&session));
        if expired {
            self.sessions.remove(&id);
        }
        self.sessions
            .entry(id.clone())
            .or_insert_with(ChatSession::new);
        SessionTurn {
            session_id: id,
            _guard: guard,
        }
    }

    /// Lock an existing, unexpired conversation for a consistent read or
    /// mutation. Failed lookups do not leave an unbounded lock-map entry.
    pub async fn lock_existing(&self, session_id: &str) -> AppResult<SessionTurn> {
        let (lock, guard) = self.acquire_turn_lock(session_id).await;
        let result = match self.sessions.get(session_id) {
            None => Err(AppError::SessionNotFound(session_id.to_owned())),
            Some(session) if self.is_expired(&session) => {
                drop(session);
                self.sessions.remove(session_id);
                Err(AppError::SessionExpired(session_id.to_owned()))
            },
            Some(_) => Ok(SessionTurn {
                session_id: session_id.to_owned(),
                _guard: guard,
            }),
        };
        if result.is_err() {
            self.unregister_turn_lock(session_id, &lock);
        }
        result
    }

    /// Persist a user turn and return a bounded context snapshot. Callers hold
    /// the turn guard while the upstream request is active, so a failed or
    /// cancelled request retains the user's submitted message but never a
    /// partial assistant response.
    pub fn append_user_and_recent(
        &self,
        turn: &SessionTurn,
        message: TextMessage,
        context_messages: usize,
    ) -> AppResult<Vec<TextMessage>> {
        let session_id = turn.session_id();
        let mut session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::SessionNotFound(session_id.to_owned()))?;
        session.push(message, self.max_messages);
        Ok(session.recent(context_messages))
    }

    pub fn append_assistant(
        &self,
        turn: &SessionTurn,
        message: TextMessage,
        think_mode: bool,
        total_tokens: u32,
    ) -> AppResult<()> {
        let session_id = turn.session_id();
        let mut session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::SessionNotFound(session_id.to_owned()))?;
        session.push(message, self.max_messages);
        session.think_mode = think_mode;
        session.total_tokens = session.total_tokens.saturating_add(u64::from(total_tokens));
        Ok(())
    }

    pub fn snapshot(&self, turn: &SessionTurn) -> AppResult<ChatSession> {
        let session_id = turn.session_id();
        self.sessions
            .get(session_id)
            .map(|session| session.clone())
            .ok_or_else(|| AppError::SessionNotFound(session_id.to_owned()))
    }

    pub fn clear(&self, turn: &SessionTurn) -> AppResult<()> {
        let session_id = turn.session_id();
        let mut session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| AppError::SessionNotFound(session_id.to_owned()))?;
        session.clear();
        Ok(())
    }

    pub fn stats(&self) -> SessionStats {
        let mut total = 0;
        let mut active = 0;
        for entry in &self.sessions {
            let idle = entry.last_activity_instant.elapsed();
            if idle <= self.timeout {
                total += 1;
                if idle <= Duration::from_secs(5 * 60) {
                    active += 1;
                }
            }
        }
        SessionStats {
            total_sessions: total,
            active_sessions: active,
        }
    }

    pub fn remove_expired(&self) -> usize {
        let candidates: Vec<_> = self
            .sessions
            .iter()
            .filter(|session| self.is_expired(session))
            .map(|session| session.key().clone())
            .collect();
        let mut removed = 0;

        for session_id in candidates {
            // Register (or reuse) the same lock used by request handlers. A
            // non-blocking acquisition ensures maintenance never waits for a
            // slow upstream response and never removes an active session.
            let lock = self
                .turn_locks
                .entry(session_id.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone();
            let Ok(_guard) = lock.clone().try_lock_owned() else {
                continue;
            };
            if !self.is_registered_lock(&session_id, &lock) {
                continue;
            }
            let still_expired = self
                .sessions
                .get(&session_id)
                .is_some_and(|session| self.is_expired(&session));
            if still_expired && self.sessions.remove(&session_id).is_some() {
                removed += 1;
                self.unregister_turn_lock(&session_id, &lock);
            }
        }

        removed
    }

    fn is_expired(&self, session: &ChatSession) -> bool {
        session.last_activity_instant.elapsed() > self.timeout
    }

    async fn acquire_turn_lock(&self, session_id: &str) -> (Arc<Mutex<()>>, OwnedMutexGuard<()>) {
        loop {
            let lock = self
                .turn_locks
                .entry(session_id.to_owned())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone();
            let guard = lock.clone().lock_owned().await;
            // Maintenance can retire an unlocked entry. Re-check after the
            // await so a waiter never proceeds on a detached mutex while a
            // newer request uses a replacement lock for the same session.
            if self.is_registered_lock(session_id, &lock) {
                return (lock, guard);
            }
        }
    }

    fn is_registered_lock(&self, session_id: &str, expected: &Arc<Mutex<()>>) -> bool {
        self.turn_locks
            .get(session_id)
            .is_some_and(|current| Arc::ptr_eq(current.value(), expected))
    }

    fn unregister_turn_lock(&self, session_id: &str, expected: &Arc<Mutex<()>>) {
        self.turn_locks
            .remove_if(session_id, |_, current| Arc::ptr_eq(current, expected));
    }

    #[cfg(test)]
    pub(crate) fn snapshot_for_test(&self, session_id: &str) -> ChatSession {
        self.sessions
            .get(session_id)
            .expect("test session exists")
            .clone()
    }
}

#[derive(Clone, Debug)]
pub struct ChatSession {
    pub messages: Vec<TextMessage>,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub think_mode: bool,
    pub total_tokens: u64,
    last_activity_instant: Instant,
}

impl ChatSession {
    fn new() -> Self {
        let now = Utc::now();
        Self {
            messages: Vec::new(),
            created_at: now,
            last_activity: now,
            think_mode: false,
            total_tokens: 0,
            last_activity_instant: Instant::now(),
        }
    }

    fn push(&mut self, message: TextMessage, max_messages: usize) {
        self.messages.push(message);
        let excess = self.messages.len().saturating_sub(max_messages);
        if excess > 0 {
            self.messages.drain(..excess);
        }
        self.touch();
    }

    fn recent(&self, count: usize) -> Vec<TextMessage> {
        let mut start = self.messages.len().saturating_sub(count);
        while matches!(
            self.messages.get(start),
            Some(TextMessage::Assistant { .. } | TextMessage::Tool { .. })
        ) {
            start += 1;
        }
        self.messages[start..].to_vec()
    }

    fn clear(&mut self) {
        self.messages.clear();
        self.think_mode = false;
        self.total_tokens = 0;
        self.touch();
    }

    fn touch(&mut self) {
        self.last_activity = Utc::now();
        self.last_activity_instant = Instant::now();
    }
}

#[derive(Debug, Serialize)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub active_sessions: usize,
}

#[derive(Default)]
pub struct RateLimiter {
    requests: DashMap<IpAddr, RateWindow>,
}

struct RateWindow {
    count: u32,
    started_at: Instant,
}

impl RateLimiter {
    pub fn is_allowed(&self, peer: IpAddr, limit: u32, window: Duration) -> bool {
        let now = Instant::now();
        let mut entry = self.requests.entry(peer).or_insert_with(|| RateWindow {
            count: 0,
            started_at: now,
        });
        if now.duration_since(entry.started_at) >= window {
            entry.count = 0;
            entry.started_at = now;
        }
        if entry.count >= limit {
            return false;
        }
        entry.count += 1;
        true
    }

    pub fn remove_inactive(&self, max_idle: Duration) {
        self.requests
            .retain(|_, window| window.started_at.elapsed() <= max_idle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn session_history_is_bounded() {
        let store = SessionStore::new(60, 2);
        let turn = store.start_turn(None).await;
        for message in ["one", "two", "three"] {
            store
                .append_user_and_recent(&turn, TextMessage::user(message), 10)
                .unwrap();
        }
        let session = store.snapshot(&turn).unwrap();
        assert_eq!(session.messages.len(), 2);
    }

    #[tokio::test]
    async fn active_turn_prevents_concurrent_access_and_expiration() {
        let store = SessionStore::new(1, 10);
        let turn = store.start_turn(Some("session-1")).await;
        let lock = store.turn_locks.get(turn.session_id()).unwrap().clone();
        assert!(lock.try_lock().is_err());
        assert!(
            futures_util::FutureExt::now_or_never(store.start_turn(Some("session-1"))).is_none()
        );

        store
            .sessions
            .get_mut(turn.session_id())
            .unwrap()
            .last_activity_instant = Instant::now() - Duration::from_secs(2);
        assert_eq!(store.remove_expired(), 0);
        store
            .append_assistant(&turn, TextMessage::assistant("answer"), false, 0)
            .unwrap();
    }

    #[tokio::test]
    async fn clearing_history_resets_conversation_metadata() {
        let store = SessionStore::new(60, 10);
        let turn = store.start_turn(None).await;
        store
            .append_user_and_recent(&turn, TextMessage::user("question"), 10)
            .unwrap();
        store
            .append_assistant(&turn, TextMessage::assistant("answer"), true, 12)
            .unwrap();

        store.clear(&turn).unwrap();
        let session = store.snapshot(&turn).unwrap();
        assert!(session.messages.is_empty());
        assert!(!session.think_mode);
        assert_eq!(session.total_tokens, 0);
    }

    #[test]
    fn limiter_resets_after_its_window() {
        let limiter = RateLimiter::default();
        let peer = IpAddr::from([127, 0, 0, 1]);
        assert!(limiter.is_allowed(peer, 1, Duration::ZERO));
        assert!(limiter.is_allowed(peer, 1, Duration::ZERO));
    }

    #[tokio::test]
    async fn maintenance_removes_expired_sessions_and_turn_locks() {
        let store = SessionStore::new(1, 10);
        let turn = store.start_turn(None).await;
        let id = turn.session_id().to_owned();
        drop(turn);
        store.sessions.get_mut(&id).unwrap().last_activity_instant =
            Instant::now() - Duration::from_secs(2);
        assert_eq!(store.remove_expired(), 1);
        assert!(!store.sessions.contains_key(&id));
        assert!(!store.turn_locks.contains_key(&id));
    }
}
