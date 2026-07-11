//! Chat API routes

use std::{convert::Infallible, sync::Arc, time::Instant};

use axum::{
    extract::State,
    response::{sse::Event, Sse},
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self, Stream};
use tokio::sync::mpsc;
use validator::Validate;

use crate::server::{
    error::{AppError, AppResult},
    models::*,
    state::AppState,
};

/// Create chat routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/send", post(send_message))
        .route("/stream", post(stream_message))
        .route("/history/{session_id}", get(get_history))
        .route("/clear/{session_id}", post(clear_history))
}

/// Send a regular chat message
pub async fn send_message(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> AppResult<Json<ChatResponse>> {
    let start_time = Instant::now();

    // Validate request
    request.validate().map_err(AppError::from)?;

    // Rate limiting check
    let client_ip = "127.0.0.1"; // In real app, extract from request
    if !state.rate_limiter.is_allowed(client_ip, 10, 60)? {
        return Err(AppError::RateLimitExceeded);
    }

    // Get or create session
    let session_id = state.sessions.get_or_create(request.session_id.clone())?;

    // Get session and add user message
    let mut session = state.sessions.get(&session_id)?;
    let user_message = zai_rs::model::TextMessage::user(&request.message);
    session.add_message(user_message.clone());

    // Build chat completion client (P05: the ZaiClient owns the key and is
    // supplied to `.send_via` at send time; the builder no longer takes a key).
    let messages = session.get_recent_messages(50); // Keep last 50 messages for context

    let client = crate::server::models::ChatCompletionBuilder::new()
        .messages(messages)
        .temperature(request.get_temperature())
        .top_p(request.get_top_p())
        .with_thinking(request.is_think_mode())
        .build()?;

    // Get AI response
    let response = client
        .send_via(&state.zai_client)
        .await
        .map_err(AppError::from)?;
    let ai_text = crate::server::models::chat_utils::extract_text_from_response(&response)
        .unwrap_or_else(|| "抱歉，我现在无法回复。".to_string());

    // Add AI response to session
    let assistant_message = zai_rs::model::TextMessage::assistant(&ai_text);
    session.add_message(assistant_message);
    state.sessions.update(&session_id, session)?;

    // Calculate processing time
    let processing_time = start_time.elapsed().as_millis() as u64;

    // Build response
    let chat_response = ChatResponse {
        reply: ai_text,
        session_id,
        metadata: ResponseMetadata {
            model: "GLM4_6".to_string(),
            think_mode: request.is_think_mode(),
            parameters: GenerationParameters {
                temperature: request.get_temperature(),
                top_p: request.get_top_p(),
                max_tokens: request.get_max_tokens(),
            },
            timestamp: chrono::Utc::now().to_rfc3339(),
            processing_time_ms: processing_time,
        },
        usage: response.usage().map(|usage| UsageStats {
            prompt_tokens: usage.prompt_tokens.unwrap_or_default(),
            completion_tokens: usage.completion_tokens.unwrap_or_default(),
            total_tokens: usage.total_tokens.unwrap_or_default(),
            estimated_cost: None,
        }),
    };

    Ok(Json(chat_response))
}

/// Stream a chat message with Server-Sent Events
pub async fn stream_message(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> AppResult<Sse<impl Stream<Item = Result<Event, Infallible>>>> {
    let start_time = Instant::now();

    // Validate request
    request.validate().map_err(AppError::from)?;

    // Rate limiting check
    let client_ip = "127.0.0.1"; // In real app, extract from request
    if !state.rate_limiter.is_allowed(client_ip, 10, 60)? {
        return Err(AppError::RateLimitExceeded);
    }

    // Get or create session
    let session_id = state.sessions.get_or_create(request.session_id.clone())?;
    let session_id_clone = session_id.clone();

    // Get session and add user message
    let mut session = state.sessions.get(&session_id)?;
    let user_message = zai_rs::model::TextMessage::user(&request.message);
    session.add_message(user_message.clone());

    // Build chat completion client (P05: the ZaiClient owns the key and is
    // supplied via `.send_via`; P08: streaming was removed temporarily, so the
    // SSE stream now emits the full response as a single content chunk plus a
    // terminal `done` chunk instead of incrementally per-stream-delta).
    let messages = session.get_recent_messages(50);

    let client = crate::server::models::ChatCompletionBuilder::new()
        .messages(messages)
        .temperature(request.get_temperature())
        .top_p(request.get_top_p())
        .with_thinking(request.is_think_mode())
        .build()?;

    // Create channel for SSE chunks
    let (tx, mut rx) = mpsc::channel::<StreamChunk>(100);

    // Spawn send task: fetch the full response, then push it as a single chunk.
    let state_clone = state.clone();
    let request_clone = request.clone();
    let zai_client = state.zai_client.clone();

    tokio::spawn(async move {
        let send_result = client.send_via(&zai_client).await;

        match send_result {
            Ok(response) => {
                // Emit the full text as one content chunk.
                let content =
                    crate::server::models::chat_utils::extract_text_from_response(&response)
                        .unwrap_or_else(|| "抱歉，我现在无法回复。".to_string());

                let content_chunk = StreamChunk {
                    content: content.clone(),
                    session_id: session_id_clone.clone(),
                    done: false,
                    metadata: Some(StreamMetadata {
                        finish_reason: response
                            .choices()
                            .and_then(|c| c.first())
                            .and_then(|choice| choice.finish_reason.clone()),
                        model: response.model.clone(),
                        has_reasoning: request_clone.is_think_mode(),
                    }),
                    usage: None,
                };
                let _ = tx.send(content_chunk).await;

                // Send final completion chunk
                let final_chunk = StreamChunk {
                    content: String::new(),
                    session_id: session_id_clone.clone(),
                    done: true,
                    metadata: Some(StreamMetadata {
                        finish_reason: Some("stop".to_string()),
                        model: Some("GLM4_6".to_string()),
                        has_reasoning: request_clone.is_think_mode(),
                    }),
                    usage: None,
                };
                let _ = tx.send(final_chunk).await;

                // Update session with complete response
                let assistant_message = zai_rs::model::TextMessage::assistant(&content);
                if let Err(e) = state_clone
                    .sessions
                    .add_message(&session_id_clone, assistant_message)
                {
                    tracing::error!("Failed to update session after response: {}", e);
                }
            },
            Err(e) => {
                tracing::error!("Chat request error: {}", e);
                // Send error chunk
                let error_chunk = StreamChunk {
                    content: "抱歉，响应出现错误。".to_string(),
                    session_id: session_id_clone.clone(),
                    done: true,
                    metadata: Some(StreamMetadata {
                        finish_reason: Some("error".to_string()),
                        model: None,
                        has_reasoning: false,
                    }),
                    usage: None,
                };
                let _ = tx.send(error_chunk).await;
            },
        }
    });

    // Convert channel receiver to SSE stream
    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Some(chunk) => {
                let json = serde_json::to_string(&chunk).unwrap_or_default();
                Some((Ok(Event::default().data(json)), rx))
            },
            None => None,
        }
    });

    Ok(Sse::new(stream))
}

/// Get chat history for a session
pub async fn get_history(
    State(state): State<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> AppResult<Json<ChatHistoryResponse>> {
    let session = state.sessions.get(&session_id)?;

    let messages: Vec<ChatMessage> = session
        .messages
        .iter()
        .enumerate()
        .map(|(index, msg)| {
            ChatMessage {
                id: format!("{}-{}", session_id, index),
                role: chat_utils::text_message_role(msg).to_string(),
                content: chat_utils::text_message_content_value(msg),
                timestamp: session.created_at.to_rfc3339(), // Simplified timestamp
            }
        })
        .collect();

    let total_messages = messages.len();

    Ok(Json(ChatHistoryResponse {
        session_id,
        messages,
        total_messages,
        metadata: ChatHistoryMetadata {
            created_at: session.created_at.to_rfc3339(),
            last_activity: session.last_activity.to_rfc3339(),
            think_mode: session.metadata.think_mode,
            total_tokens: session.metadata.total_tokens,
        },
    }))
}

/// Clear chat history for a session
pub async fn clear_history(
    State(state): State<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> AppResult<Json<ClearHistoryResponse>> {
    let mut session = state.sessions.get(&session_id)?;

    // Keep system messages if any, clear the rest
    let system_messages: Vec<zai_rs::model::TextMessage> = session
        .messages
        .into_iter()
        .filter(|msg| chat_utils::text_message_role(msg) == "system")
        .collect();

    session.messages = system_messages;
    state.sessions.update(&session_id, session)?;

    Ok(Json(ClearHistoryResponse {
        success: true,
        message: "Chat history cleared successfully".to_string(),
        session_id,
        remaining_messages: 0,
    }))
}

/// Chat history response
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatHistoryResponse {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
    pub total_messages: usize,
    pub metadata: ChatHistoryMetadata,
}

/// Individual chat message
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: serde_json::Value,
    pub timestamp: String,
}

/// Chat history metadata
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatHistoryMetadata {
    pub created_at: String,
    pub last_activity: String,
    pub think_mode: bool,
    pub total_tokens: u64,
}

/// Clear history response
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ClearHistoryResponse {
    pub success: bool,
    pub message: String,
    pub session_id: String,
    pub remaining_messages: usize,
}
