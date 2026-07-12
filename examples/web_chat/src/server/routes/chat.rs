//! Chat request, SSE delivery, history, and reset handlers.

use std::{net::SocketAddr, pin::Pin, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{ConnectInfo, Path, State, rejection::JsonRejection},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post},
};
use futures_util::{
    FutureExt, StreamExt,
    future::BoxFuture,
    stream::{self, Stream},
};
use serde::Serialize;
use validator::Validate;
use zai_rs::{
    ZaiResult,
    model::{ChatStreamResponse, TextMessage},
};

use crate::server::{
    error::{AppError, AppResult},
    models::{
        self, ChatModelRequest, ChatRequest, ChatResponse, ResponseMetadata, StreamChunk,
        StreamMetadata, UsageStats,
    },
    state::{AppState, SessionStore, SessionTurn},
};

const CONTEXT_MESSAGES: usize = 50;
const REQUESTS_PER_MINUTE: u32 = 10;
/// A defensive ceiling on the assembled answer retained until `[DONE]`.
const MAX_STREAMED_RESPONSE_BYTES: usize = 1024 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/send", post(send_message))
        .route("/stream", post(stream_message))
        .route("/history/{session_id}", get(get_history))
        .route("/clear/{session_id}", post(clear_history))
}

pub async fn send_message(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    payload: Result<Json<ChatRequest>, JsonRejection>,
) -> AppResult<Json<ChatResponse>> {
    let request = parse_json_request(payload)?;
    let started_at = std::time::Instant::now();
    let (turn, completion) = prepare_completion(&state, peer, &request).await?;
    let session_id = turn.session_id().to_owned();
    let response = completion.send_via(&state.zai_client).await?;
    let content = models::response_text(&response).ok_or(AppError::InvalidUpstreamResponse)?;
    let usage = response.usage().map(UsageStats::from);
    let total_tokens = usage.as_ref().map_or(0, |usage| usage.total_tokens);
    state.sessions.append_assistant(
        &turn,
        TextMessage::assistant(&content),
        request.thinking_enabled(),
        total_tokens,
    )?;

    Ok(Json(ChatResponse {
        reply: content,
        session_id,
        metadata: ResponseMetadata {
            model: response
                .model
                .clone()
                .unwrap_or_else(|| "glm-4.6".to_owned()),
            think_mode: request.thinking_enabled(),
            temperature: request.temperature(),
            max_tokens: request.max_tokens(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            processing_time_ms: u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
        },
        usage,
    }))
}

/// Relay typed upstream chunks one at a time, preserving downstream
/// backpressure. Dropping the HTTP response also drops the upstream stream, so
/// a disconnected browser cannot leave a detached request running.
pub async fn stream_message(
    State(state): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    payload: Result<Json<ChatRequest>, JsonRejection>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, axum::Error>>>> {
    let request = parse_json_request(payload)?;
    let (turn, completion) = prepare_completion(&state, peer, &request).await?;
    let zai_client = state.zai_client.clone();
    let think_mode = request.thinking_enabled();
    let sessions = state.sessions.clone();
    let startup = async move { completion.enable_stream().stream_via(&zai_client).await }.boxed();
    let events = relay_stream(startup, sessions, turn, think_mode);

    Ok(Sse::new(events).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(10))
            .text("keep-alive"),
    ))
}

fn relay_stream<S>(
    startup: BoxFuture<'static, ZaiResult<S>>,
    sessions: Arc<SessionStore>,
    turn: SessionTurn,
    think_mode: bool,
) -> impl Stream<Item = Result<Event, axum::Error>>
where
    S: Stream<Item = ZaiResult<ChatStreamResponse>> + Send + 'static,
{
    let session_id = turn.session_id().to_owned();
    let state = RelayState {
        startup: Some(startup),
        upstream: None,
        sessions,
        turn,
        session_id,
        think_mode,
        content: String::new(),
        finish_reason: None,
        model: None,
        usage: None,
        has_reasoning: false,
        finished: false,
    };

    stream::unfold(state, |mut state| async move {
        if state.finished {
            return None;
        }

        if state.upstream.is_none() {
            let Some(startup) = state.startup.take() else {
                tracing::error!("chat relay entered an invalid startup state");
                state.finished = true;
                let event = stream_event(stream_error_chunk(
                    state.session_id.clone(),
                    "The chat stream could not be started.",
                ));
                return Some((event, state));
            };
            match startup.await {
                Ok(upstream) => state.upstream = Some(Box::pin(upstream)),
                Err(error) => {
                    tracing::warn!(upstream_code = ?error.code(), "upstream chat request failed");
                    state.finished = true;
                    let event = stream_event(stream_error_chunk(
                        state.session_id.clone(),
                        "The chat request failed. Please try again.",
                    ));
                    return Some((event, state));
                },
            }
        }

        let Some(upstream) = state.upstream.as_mut() else {
            tracing::error!("chat relay entered an invalid streaming state");
            state.finished = true;
            let event = stream_event(stream_error_chunk(
                state.session_id.clone(),
                "The chat stream could not be continued.",
            ));
            return Some((event, state));
        };
        let next = upstream.next().await;
        let event = match next {
            Some(Ok(chunk)) => match state.accept_chunk(chunk) {
                Ok(chunk) => stream_event(chunk),
                Err(message) => {
                    tracing::warn!(%message, "streamed response rejected");
                    state.finished = true;
                    stream_event(stream_error_chunk(state.session_id.clone(), message))
                },
            },
            Some(Err(error)) => {
                tracing::warn!(upstream_code = ?error.code(), "upstream chat stream failed");
                state.finished = true;
                stream_event(stream_error_chunk(
                    state.session_id.clone(),
                    "The chat stream failed. Please try again.",
                ))
            },
            None => {
                // `ChatStream` reaches `None` only after validating the
                // provider's `[DONE]` marker. Until this point the assistant
                // text exists only in this response-local buffer.
                state.finished = true;
                stream_event(state.commit_completed_response())
            },
        };
        Some((event, state))
    })
}

struct RelayState<S> {
    startup: Option<BoxFuture<'static, ZaiResult<S>>>,
    upstream: Option<Pin<Box<S>>>,
    sessions: Arc<SessionStore>,
    turn: SessionTurn,
    session_id: String,
    think_mode: bool,
    content: String,
    finish_reason: Option<String>,
    model: Option<String>,
    usage: Option<UsageStats>,
    has_reasoning: bool,
    finished: bool,
}

impl<S> RelayState<S> {
    fn accept_chunk(&mut self, chunk: ChatStreamResponse) -> Result<StreamChunk, &'static str> {
        let choice = chunk
            .choices
            .iter()
            .find(|choice| choice.index.unwrap_or_default() == 0)
            .or_else(|| chunk.choices.first());
        let delta = choice
            .and_then(|choice| choice.delta.as_ref())
            .and_then(|delta| delta.content.as_deref())
            .unwrap_or_default();
        let new_len = self
            .content
            .len()
            .checked_add(delta.len())
            .ok_or("The streamed response exceeded the server limit.")?;
        if new_len > MAX_STREAMED_RESPONSE_BYTES {
            return Err("The streamed response exceeded the server limit.");
        }
        self.content.push_str(delta);

        let chunk_has_reasoning = choice
            .and_then(|choice| choice.delta.as_ref())
            .and_then(|delta| delta.reasoning_content.as_deref())
            .is_some_and(|reasoning| !reasoning.is_empty());
        self.has_reasoning |= chunk_has_reasoning;
        if let Some(finish_reason) = choice.and_then(|choice| choice.finish_reason.clone()) {
            self.finish_reason = Some(finish_reason);
        }
        if let Some(model) = chunk.model.clone() {
            self.model = Some(model);
        }
        if let Some(usage) = chunk.usage.as_ref().map(UsageStats::from) {
            self.usage = Some(usage);
        }

        Ok(StreamChunk {
            content: delta.to_owned(),
            session_id: self.session_id.clone(),
            done: false,
            error: None,
            metadata: Some(StreamMetadata {
                finish_reason: choice.and_then(|choice| choice.finish_reason.clone()),
                model: chunk.model,
                has_reasoning: chunk_has_reasoning,
            }),
            usage: chunk.usage.as_ref().map(UsageStats::from),
        })
    }

    fn commit_completed_response(&self) -> StreamChunk {
        if self.content.trim().is_empty() {
            tracing::warn!("upstream stream contained no assistant text");
            return stream_error_chunk(
                self.session_id.clone(),
                "The provider returned an empty response.",
            );
        }

        let total_tokens = self.usage.as_ref().map_or(0, |usage| usage.total_tokens);
        if let Err(error) = self.sessions.append_assistant(
            &self.turn,
            TextMessage::assistant(&self.content),
            self.think_mode,
            total_tokens,
        ) {
            tracing::warn!(
                error_code = error.status_and_code().1,
                "could not persist completed assistant response"
            );
            return stream_error_chunk(
                self.session_id.clone(),
                "The completed response could not be saved.",
            );
        }

        StreamChunk {
            content: String::new(),
            session_id: self.session_id.clone(),
            done: true,
            error: None,
            metadata: Some(StreamMetadata {
                finish_reason: self.finish_reason.clone(),
                model: self.model.clone(),
                has_reasoning: self.has_reasoning,
            }),
            usage: self.usage.clone(),
        }
    }
}

fn stream_event(chunk: StreamChunk) -> Result<Event, axum::Error> {
    Event::default().json_data(chunk)
}

fn stream_error_chunk(session_id: String, message: impl Into<String>) -> StreamChunk {
    StreamChunk {
        content: String::new(),
        session_id,
        done: true,
        error: Some(message.into()),
        metadata: Some(StreamMetadata {
            finish_reason: Some("error".to_owned()),
            model: None,
            has_reasoning: false,
        }),
        usage: None,
    }
}

fn parse_json_request(payload: Result<Json<ChatRequest>, JsonRejection>) -> AppResult<ChatRequest> {
    payload
        .map(|Json(request)| request)
        .map_err(|rejection| match rejection.status() {
            axum::http::StatusCode::PAYLOAD_TOO_LARGE => AppError::PayloadTooLarge,
            axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE => AppError::UnsupportedMediaType,
            _ => AppError::InvalidJson,
        })
}

async fn prepare_completion(
    state: &AppState,
    peer: SocketAddr,
    request: &ChatRequest,
) -> AppResult<(SessionTurn, ChatModelRequest)> {
    request.validate()?;
    if !state
        .rate_limiter
        .is_allowed(peer.ip(), REQUESTS_PER_MINUTE, Duration::from_secs(60))
    {
        return Err(AppError::RateLimitExceeded);
    }
    let turn = state.sessions.start_turn(request.session_id()).await;
    let messages = state.sessions.append_user_and_recent(
        &turn,
        TextMessage::user(request.message()),
        CONTEXT_MESSAGES,
    )?;
    let completion = models::build_completion(&messages, request)?;
    Ok((turn, completion))
}

#[derive(Serialize)]
pub struct HistoryResponse {
    session_id: String,
    messages: Vec<HistoryMessage>,
    created_at: String,
    last_activity: String,
    think_mode: bool,
    total_tokens: u64,
}

#[derive(Serialize)]
pub struct HistoryMessage {
    role: &'static str,
    content: String,
}

pub async fn get_history(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> AppResult<Json<HistoryResponse>> {
    models::ensure_valid_session_id(&session_id)?;
    let turn = state.sessions.lock_existing(&session_id).await?;
    let session = state.sessions.snapshot(&turn)?;
    let messages = session
        .messages
        .iter()
        .map(|message| HistoryMessage {
            role: models::message_role(message),
            content: models::message_text(message),
        })
        .collect();
    Ok(Json(HistoryResponse {
        session_id,
        messages,
        created_at: session.created_at.to_rfc3339(),
        last_activity: session.last_activity.to_rfc3339(),
        think_mode: session.think_mode,
        total_tokens: session.total_tokens,
    }))
}

#[derive(Serialize)]
pub struct ClearResponse {
    success: bool,
    session_id: String,
}

pub async fn clear_history(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> AppResult<Json<ClearResponse>> {
    models::ensure_valid_session_id(&session_id)?;
    let turn = state.sessions.lock_existing(&session_id).await?;
    state.sessions.clear(&turn)?;
    Ok(Json(ClearResponse {
        success: true,
        session_id,
    }))
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use futures_util::{StreamExt, pin_mut};
    use serde_json::json;
    use zai_rs::client::ApiFamily;

    use super::*;

    async fn session_with_user() -> (Arc<SessionStore>, SessionTurn) {
        let sessions = Arc::new(SessionStore::new(60, 10));
        let turn = sessions.start_turn(None).await;
        sessions
            .append_user_and_recent(&turn, TextMessage::user("question"), 10)
            .unwrap();
        (sessions, turn)
    }

    fn chunk(content: &str, finish_reason: Option<&str>, usage: bool) -> ChatStreamResponse {
        let usage = usage.then(|| {
            json!({
                "prompt_tokens": 2,
                "completion_tokens": 3,
                "total_tokens": 5
            })
        });
        serde_json::from_value(json!({
            "model": "glm-4.6",
            "choices": [{
                "index": 0,
                "delta": { "content": content },
                "finish_reason": finish_reason
            }],
            "usage": usage
        }))
        .unwrap()
    }

    async fn mock_upstream(
        headers: axum::http::HeaderMap,
        Json(body): Json<serde_json::Value>,
    ) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
        assert_eq!(
            headers
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer test-api-key")
        );
        assert_eq!(body.get("stream"), Some(&serde_json::Value::Bool(true)));
        let chunk = Event::default()
            .json_data(json!({
                "model": "glm-4.6",
                "choices": [{
                    "index": 0,
                    "delta": { "content": "real stream" },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 2,
                    "completion_tokens": 2,
                    "total_tokens": 4
                }
            }))
            .unwrap();
        Sse::new(stream::iter([
            Ok(chunk),
            Ok(Event::default().data("[DONE]")),
        ]))
    }

    #[tokio::test]
    async fn authenticated_typed_upstream_sse_reaches_the_relay() {
        let app = Router::new().route("/api/paas/v4/chat/completions", post(mock_upstream));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await });

        let client = zai_rs::client::ZaiClient::builder("test-api-key")
            .endpoint(ApiFamily::PaasV4, format!("http://{address}/api/paas/v4"))
            .allow_insecure_transport(true)
            .build()
            .unwrap();
        let request: ChatRequest =
            serde_json::from_value(json!({ "message": "question" })).unwrap();
        let completion =
            models::build_completion(&[TextMessage::user("question")], &request).unwrap();
        let (sessions, turn) = session_with_user().await;
        let session_id = turn.session_id().to_owned();
        let startup = async move { completion.enable_stream().stream_via(&client).await }.boxed();
        let events = relay_stream(startup, sessions.clone(), turn, false);
        pin_mut!(events);

        assert!(events.next().await.unwrap().is_ok());
        assert!(events.next().await.unwrap().is_ok());
        assert!(events.next().await.is_none());
        let turn = sessions.lock_existing(&session_id).await.unwrap();
        let session = sessions.snapshot(&turn).unwrap();
        assert_eq!(models::message_text(&session.messages[1]), "real stream");
        assert_eq!(session.total_tokens, 4);

        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
    }

    #[tokio::test]
    async fn completed_stream_is_committed_only_after_upstream_end() {
        let (sessions, turn) = session_with_user().await;
        let session_id = turn.session_id().to_owned();
        let upstream = stream::iter(vec![
            Ok::<_, zai_rs::ZaiError>(chunk("Hel", None, false)),
            Ok(chunk("lo", Some("stop"), true)),
        ]);
        let startup = async move { Ok(upstream) }.boxed();
        let events = relay_stream(startup, sessions.clone(), turn, true);
        pin_mut!(events);

        assert!(events.next().await.unwrap().is_ok());
        assert_eq!(sessions.snapshot_for_test(&session_id).messages.len(), 1);
        assert!(events.next().await.unwrap().is_ok());
        assert_eq!(sessions.snapshot_for_test(&session_id).messages.len(), 1);

        // Polling past the final upstream chunk observes `[DONE]`, commits the
        // complete answer once, and emits the terminal browser event.
        assert!(events.next().await.unwrap().is_ok());
        assert!(events.next().await.is_none());
        let turn = sessions.lock_existing(&session_id).await.unwrap();
        let session = sessions.snapshot(&turn).unwrap();
        assert_eq!(session.messages.len(), 2);
        assert_eq!(models::message_text(&session.messages[1]), "Hello");
        assert_eq!(session.total_tokens, 5);
        assert!(session.think_mode);
    }

    #[tokio::test]
    async fn upstream_error_does_not_commit_partial_answer() {
        let (sessions, turn) = session_with_user().await;
        let session_id = turn.session_id().to_owned();
        let error = zai_rs::ZaiError::ApiError {
            code: zai_rs::client::error::codes::SDK_IO,
            message: "connection lost".to_owned(),
        };
        let upstream = stream::iter(vec![Ok(chunk("partial", None, false)), Err(error)]);
        let startup = async move { Ok(upstream) }.boxed();
        let events = relay_stream(startup, sessions.clone(), turn, false);
        pin_mut!(events);

        assert!(events.next().await.unwrap().is_ok());
        assert!(events.next().await.unwrap().is_ok());
        assert!(events.next().await.is_none());
        let turn = sessions.lock_existing(&session_id).await.unwrap();
        let session = sessions.snapshot(&turn).unwrap();
        assert_eq!(session.messages.len(), 1);
        assert_eq!(models::message_text(&session.messages[0]), "question");
    }

    #[tokio::test]
    async fn blank_completed_stream_is_not_persisted() {
        let (sessions, turn) = session_with_user().await;
        let session_id = turn.session_id().to_owned();
        let upstream = stream::iter(vec![Ok::<_, zai_rs::ZaiError>(chunk(
            " \n",
            Some("stop"),
            false,
        ))]);
        let startup = async move { Ok(upstream) }.boxed();
        let events = relay_stream(startup, sessions.clone(), turn, false);
        pin_mut!(events);

        assert!(events.next().await.unwrap().is_ok());
        assert!(events.next().await.unwrap().is_ok());
        assert!(events.next().await.is_none());
        let turn = sessions.lock_existing(&session_id).await.unwrap();
        let session = sessions.snapshot(&turn).unwrap();
        assert_eq!(session.messages.len(), 1);
    }

    #[tokio::test]
    async fn dropping_downstream_does_not_commit_partial_answer() {
        let (sessions, turn) = session_with_user().await;
        let session_id = turn.session_id().to_owned();
        {
            let upstream = stream::iter(vec![Ok::<_, zai_rs::ZaiError>(chunk(
                "partial", None, false,
            ))]);
            let startup = async move { Ok(upstream) }.boxed();
            let events = relay_stream(startup, sessions.clone(), turn, false);
            pin_mut!(events);
            assert!(events.next().await.unwrap().is_ok());
            // The response body is dropped before the relay polls the upstream
            // stream to completion, matching a disconnected browser.
        }

        let turn = sessions.lock_existing(&session_id).await.unwrap();
        let session = sessions.snapshot(&turn).unwrap();
        assert_eq!(session.messages.len(), 1);
        assert_eq!(models::message_text(&session.messages[0]), "question");
    }
}
