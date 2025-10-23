//! Chat API routes

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Sse, sse::Event},
    routing::{get, post},
};
use futures::stream::{self, Stream};
use std::{convert::Infallible, sync::Arc, time::Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

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
        .route("/history/:session_id", get(get_history))
        .route("/clear/:session_id", post(clear_history))
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
    
    // Build chat completion client
    let api_key = state.config.api_key.clone();
    let messages = session.get_recent_messages(50); // Keep last 50 messages for context
    
    let client = crate::server::models::ChatCompletionBuilder::new(api_key)
        .messages(messages)
        .temperature(request.get_temperature())
        .top_p(request.get_top_p())
        .with_thinking(request.is_think_mode())
        .build()?;
    
    // Get AI response
    let response = client.send().await.map_err(AppError::from)?;
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
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
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
    
    // Build streaming chat completion client
    let api_key = state.config.api_key.clone();
    let messages = session.get_recent_messages(50);
    
    let client = crate::server::models::ChatCompletionBuilder::new(api_key)
        .messages(messages)
        .temperature(request.get_temperature())
        .top_p(request.get_top_p())
        .with_thinking(request.is_think_mode())
        .build()?;
    
    let mut streaming_client = client.enable_stream();
    
    // Create channel for streaming chunks
    let (tx, mut rx) = mpsc::channel::<StreamChunk>(100);
    
    // Spawn streaming task
    let state_clone = state.clone();
    let request_clone = request.clone();
    
    tokio::spawn(async move {
        let mut accumulated_response = String::new();
        let mut chunk_count = 0;
        
        let stream_result = streaming_client
            .stream_for_each(|chunk: zai_rs::model::ChatStreamResponse| {
                let tx = tx.clone();
                let session_id = session_id_clone.clone();
                let mut accumulated = accumulated_response.clone();
                
                async move {
                    // Extract content from chunk
                    if let Some(content) = crate::server::models::chat_utils::extract_text_from_chunk(&chunk) {
                        accumulated.push_str(&content);
                        chunk_count += 1;
                        
                        let stream_chunk = StreamChunk {
                            content: content.clone(),
                            session_id: session_id.clone(),
                            done: false,
                            metadata: Some(StreamMetadata {
                                finish_reason: chunk.choices.get(0).and_then(|c| c.finish_reason.clone()),
                                model: chunk.model.clone(),
                                has_reasoning: chunk.choices.get(0)
                                    .and_then(|c| c.delta.as_ref())
                                    .and_then(|d| d.reasoning_content.as_ref())
                                    .is_some(),
                            }),
                            usage: None, // Usage typically comes in final chunk
                        };
                        
                        if let Err(e) = tx.send(stream_chunk).await {
                            tracing::error!("Failed to send stream chunk: {}", e);
                            return Err(zai_rs::client::error_handler::ClientError::StreamingError(
                                "Channel send failed".to_string()
                            ));
                        }
                    }
                    
                    Ok(())
                }
            })
            .await;
        
        // Handle stream completion or error
        match stream_result {
            Ok(_) => {
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
                let mut sessions_guard = state_clone.sessions.sessions.write().await;
                if let Some(session) = sessions_guard.get_mut(&session_id_clone) {
                    let assistant_message = zai_rs::model::TextMessage::assistant(&accumulated_response);
                    session.add_message(assistant_message);
                }
            }
            Err(e) => {
                tracing::error!("Streaming error: {}", e);
                // Send error chunk
                let error_chunk = StreamChunk {
                    content: "抱歉，流式响应出现错误。".to_string(),
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
            }
        }
    });
    
    // Convert channel receiver to SSE stream
    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Some(chunk) => {
                let json = serde_json::to_string(&chunk).unwrap_or_default();
                Some((Ok(Event::default().data(json)), rx))
            }
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
    
    let messages: Vec<ChatMessage> = session.messages.iter().enumerate().map(|(index, msg)| {
        ChatMessage {
            id: format!("{}-{}", session_id, index),
            role: msg.role.clone(),
            content: msg.content.clone(),
            timestamp: session.created_at.to_rfc3339(), // Simplified timestamp
        }
    }).collect();
    
    Ok(Json(ChatHistoryResponse {
        session_id,
        messages,
        total_messages: messages.len(),
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
    let system_messages: Vec<zai_rs::model::TextMessage> = session.messages
        .into_iter()
        .filter(|msg| msg.role == "system")
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