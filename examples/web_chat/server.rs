//! Simple Web-based AI Chat Application
//!
//! A simplified web chat application using the zai-rs crate.

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, Sse, sse::Event},
    routing::{get, post},
};
use env_logger;
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::Infallible, sync::Arc};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, services::ServeDir};
use uuid::Uuid;
use zai_rs::model::*;

/// Session data for maintaining conversation context
type SessionStore = Arc<RwLock<HashMap<String, ChatSession>>>;

#[derive(Clone)]
struct ChatSession {
    messages: Vec<TextMessage>,
}

/// Request payload for chat messages
#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    session_id: Option<String>,
}

/// Response payload for chat messages
#[derive(Serialize)]
struct ChatResponse {
    reply: String,
    session_id: String,
}

/// Streaming response chunk
#[derive(Serialize, Clone)]
struct StreamChunk {
    content: String,
    session_id: String,
    done: bool,
}

/// Extract text content from AI response
fn extract_text_from_content(v: &serde_json::Value) -> Option<String> {
    v.as_str().map(|s| s.to_string())
}

/// Initialize a new chat session
fn create_new_session() -> ChatSession {
    ChatSession {
        messages: Vec::new(),
    }
}

/// Build ChatCompletion client with messages
fn build_client(messages: &[TextMessage], api_key: &str) -> ChatCompletion<GLM4_6, TextMessage> {
    let model = GLM4_6 {};

    if messages.is_empty() {
        return ChatCompletion::new(model, TextMessage::user("你好"), api_key.to_string())
            .with_temperature(0.7)
            .with_top_p(0.9)
            .with_coding_plan();
    }

    let mut client = ChatCompletion::new(model, messages[0].clone(), api_key.to_string())
        .with_temperature(0.7)
        .with_top_p(0.9)
        .with_coding_plan();

    for msg in messages.iter().skip(1) {
        client = client.add_messages(msg.clone());
    }

    client
}

/// Handle regular chat requests
async fn chat_handler(
    State(sessions): State<SessionStore>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    let api_key = std::env::var("ZHIPU_API_KEY").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_id = request
        .session_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let mut sessions_guard = sessions.write().await;
    let session = sessions_guard
        .entry(session_id.clone())
        .or_insert_with(create_new_session);

    // Add user message to session
    session.messages.push(TextMessage::user(&request.message));

    // Build client with current messages
    let client = build_client(&session.messages, &api_key);

    // Get AI response
    match client.send().await {
        Ok(body) => {
            let ai_text = body
                .choices()
                .and_then(|cs| cs.get(0))
                .and_then(|c| c.message().content())
                .and_then(|v| extract_text_from_content(v))
                .unwrap_or_else(|| "抱歉，我现在无法回复。".to_string());

            // Add AI response to session
            session.messages.push(TextMessage::assistant(&ai_text));

            Ok(Json(ChatResponse {
                reply: ai_text,
                session_id,
            }))
        }
        Err(e) => {
            eprintln!("Chat API error: {:?}", e);
            Ok(Json(ChatResponse {
                reply: "服务器内部错误，请稍后重试。".to_string(),
                session_id,
            }))
        }
    }
}

/// Handle streaming chat requests
async fn chat_stream_handler(
    State(sessions): State<SessionStore>,
    Json(request): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    let api_key = std::env::var("ZHIPU_API_KEY").unwrap_or_else(|_| "demo_key".to_string());

    let session_id = request
        .session_id
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let session_id_clone = session_id.clone();

    let mut sessions_guard = sessions.write().await;
    let session = sessions_guard
        .entry(session_id.clone())
        .or_insert_with(create_new_session);

    // Add user message to session
    session.messages.push(TextMessage::user(&request.message));

    // Build client with current messages
    let client = build_client(&session.messages, &api_key);
    let mut streaming_client = client.enable_stream();

    let (tx, rx) = tokio::sync::mpsc::channel::<StreamChunk>(1);

    // Spawn streaming task
    let sessions_clone = sessions.clone();
    eprintln!("🚀 开始流式响应，会话ID: {}", session_id_clone);

    tokio::spawn(async move {
        let accumulated_response = Arc::new(RwLock::new(String::new()));
        let accumulated_clone = accumulated_response.clone();

        if let Err(_) = streaming_client
            .stream_for_each(|chunk: ChatStreamResponse| {
                let tx = tx.clone();
                let session_id = session_id_clone.clone();
                let acc_ref = accumulated_clone.clone();
                async move {
                    if let Some(choice) = chunk.choices.get(0) {
                        if let Some(delta) = &choice.delta {
                            if let Some(content) = &delta.content {
                                // 累积响应内容
                                {
                                    let mut acc = acc_ref.write().await;
                                    acc.push_str(content);
                                    eprintln!(
                                        "📝 收到流式数据块 ({} chars): {:?}",
                                        content.len(),
                                        content
                                    );
                                }

                                let stream_chunk = StreamChunk {
                                    content: content.clone(),
                                    session_id: session_id,
                                    done: false,
                                };
                                if let Err(_) = tx.send(stream_chunk).await {
                                    eprintln!("❌ 发送流式数据块失败");
                                } else {
                                    eprintln!("✅ 流式数据块已发送");
                                }
                            }
                        }
                    }
                    Ok(())
                }
            })
            .await
        {
            // Send error chunk if streaming fails
            let error_chunk = StreamChunk {
                content: "抱歉，流式响应出现错误。".to_string(),
                session_id: session_id_clone.clone(),
                done: true,
            };
            let _ = tx.send(error_chunk).await;
        } else {
            // Send final completion chunk
            let final_chunk = StreamChunk {
                content: String::new(),
                session_id: session_id_clone.clone(),
                done: true,
            };
            let _ = tx.send(final_chunk).await;

            // Update session with complete response
            let final_response = {
                let acc = accumulated_response.read().await;
                acc.clone()
            };

            let mut sessions_guard = sessions_clone.write().await;
            if let Some(session) = sessions_guard.get_mut(&session_id_clone) {
                session
                    .messages
                    .push(TextMessage::assistant(&final_response));
            }
        }
    });

    // Convert channel receiver to SSE stream
    let stream = stream::unfold(rx, |mut rx| async {
        match rx.recv().await {
            Some(chunk) => {
                let json = serde_json::to_string(&chunk).unwrap_or_default();
                eprintln!("📤 发送SSE事件: {} chars, done: {}", json.len(), chunk.done);
                Some((Ok(Event::default().data(json)), rx))
            }
            None => {
                eprintln!("🔚 SSE流结束");
                None
            }
        }
    });

    Ok(Sse::new(stream))
}

/// Serve the main HTML page
async fn index_handler() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

/// Start the web server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Check for API key
    std::env::var("ZHIPU_API_KEY").expect("请先在环境变量中设置 ZHIPU_API_KEY");

    // Initialize session store
    let sessions: SessionStore = Arc::new(RwLock::new(HashMap::new()));

    // Build the router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/chat", post(chat_handler))
        .route("/api/chat/stream", post(chat_stream_handler))
        .nest_service("/static", ServeDir::new("examples/web_chat/static"))
        .layer(CorsLayer::permissive())
        .with_state(sessions);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    println!("🚀 AI Chat Server is running on http://localhost:3000");

    axum::serve(listener, app).await?;

    Ok(())
}
