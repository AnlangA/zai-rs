//! Simple Realtime Example
//!
//! This example demonstrates how to use the GLM-Realtime API
//! for a simple text-based conversation using WebSocket.
//!
//! Run with:
//!   export ZHIPU_API_KEY="your_api_key"
//!   cargo run --example simple_realtime

use zai_rs::client::wss::WssClient;
use zai_rs::real_time::*;
use std::io::{self, Write};

/// Realtime client configuration
struct RealtimeClientConfig {
    api_url: String,
    api_key: String,
}

/// Realtime client implementing WssClient trait
struct RealtimeClient {
    config: RealtimeClientConfig,
}

impl WssClient for RealtimeClient {
    type ApiUrl = String;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &self.config.api_url
    }

    fn api_key(&self) -> &Self::ApiKey {
        &self.config.api_key
    }
}

/// Helper function to generate a unique event ID
fn generate_event_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Helper function to get current timestamp in milliseconds
fn get_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Wait for session to be ready (session.created or session.updated)
async fn wait_for_session_ready(
    connection: &mut zai_rs::client::wss::WssConnection,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        if let Some(msg) = connection.read().await? {
            if let Ok(server_event) = serde_json::from_str::<ServerEvent>(&msg) {
                match server_event {
                    ServerEvent::SessionCreated(_) => {
                        println!("→ Session created");
                        return Ok(());
                    }
                    ServerEvent::SessionUpdated(_) => {
                        println!("→ Session updated");
                        return Ok(());
                    }
                    ServerEvent::Heartbeat(_) => {
                        // Ignore heartbeat
                    }
                    ServerEvent::Error(e) => {
                        eprintln!("→ Server error: {:?}", e);
                        return Err("Server returned an error".into());
                    }
                    _ => {
                        // Other events
                    }
                }
            }
        } else {
            return Err("Connection closed".into());
        }
    }
}

/// Process server response events until response is done
async fn process_response(
    connection: &mut zai_rs::client::wss::WssConnection,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let mut response_text = String::new();
    let mut response_done = false;

    while !response_done {
        if let Some(msg) = connection.read().await? {
            if let Ok(server_event) = serde_json::from_str::<ServerEvent>(&msg) {
                match server_event {
                    // Text delta events
                    ServerEvent::ResponseTextDelta(delta) => {
                        print!("{}", delta.delta);
                        io::stdout().flush()?;
                        response_text.push_str(&delta.delta);
                    }
                    // Audio transcript delta events
                    ServerEvent::ResponseAudioTranscriptDelta(delta) => {
                        print!("{}", delta.delta);
                        io::stdout().flush()?;
                        response_text.push_str(&delta.delta);
                    }
                    // Audio delta events (ignore audio data for text-only mode)
                    ServerEvent::ResponseAudioDelta(_delta) => {
                        // Ignore audio chunks
                    }
                    // Text done event
                    ServerEvent::ResponseTextDone(_) => {
                        // Text output completed
                    }
                    // Audio done event
                    ServerEvent::ResponseAudioDone(_) => {
                        // Audio output completed
                    }
                    // Response done event (final state)
                    ServerEvent::ResponseDone(resp) => {
                        response_done = true;
                        if let Some(total) = resp.response.usage.and_then(|u| u.total_tokens) {
                            println!("\n[Usage: {} tokens]", total);
                        }
                    }
                    // Error event
                    ServerEvent::Error(e) => {
                        eprintln!("\nError: {:?}", e);
                        return Err("Server returned an error".into());
                    }
                    // Heartbeat
                    ServerEvent::Heartbeat(_) => {
                        // Ignore
                    }
                    // Other events we can ignore
                    _ => {}
                }
            }
        } else {
            return Err("Connection closed".into());
        }
    }

    Ok(if response_text.is_empty() {
        None
    } else {
        Some(response_text)
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Get API key from environment variable
    let api_key = std::env::var("ZHIPU_API_KEY")
        .expect("Please set ZHIPU_API_KEY environment variable");

    // Create client configuration
    let config = RealtimeClientConfig {
        api_url: "wss://open.bigmodel.cn/api/paas/v4/realtime".to_string(),
        api_key,
    };

    let client = RealtimeClient { config };

    println!("Connecting to GLM-Realtime API...");
    let mut connection = client.connect().await?;
    println!("Connected successfully!\n");

    // 1. Send session.update event to configure the session
    println!("=== Setting up session ===");

    // Create session configuration - text only mode for simplicity
    let session = Session {
        model: Some("glm-realtime".to_string()),
        modalities: Some(vec!["text".to_string()]),
        voice: Some("tongtong".to_string()),
        instructions: Some("You are a helpful AI assistant. Respond concisely.".to_string()),
        input_audio_format: Some("wav".to_string()),
        output_audio_format: Some("pcm".to_string()),
        ..Default::default()
    };

    let mut session_update = SessionUpdateEvent {
        event_id: Some(generate_event_id()),
        client_timestamp: Some(get_timestamp()),
        ..Default::default()
    };
    session_update.set_session(session);

    let session_json = serde_json::to_string(&ClientEvent::SessionUpdate(session_update))?;
    connection.send(session_json).await?;
    println!("Session configuration sent\n");

    // Wait for session to be ready
    wait_for_session_ready(&mut connection).await?;
    println!("Ready for conversation!\n");

    // 2. Interactive conversation loop
    println!("=== Interactive Conversation ===");
    println!("Type your message and press Enter (or 'quit' to exit):\n");

    loop {
        // Read user input
        print!("You: ");
        io::stdout().flush()?;
        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;
        let user_input = user_input.trim();

        if user_input.is_empty() {
            continue;
        }

        if user_input.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }

        // 3. Create conversation item with user message
        let item = RealtimeConversationItem {
            item_type: ItemType::Message,
            object: "realtime.item".to_string(),
            role: Some(Role::User),
            content: Some(vec![ContentPart::InputText {
                text: Some(user_input.to_string()),
            }]),
            ..Default::default()
        };

        let mut item_create = ConversationItemCreateEvent {
            event_id: Some(generate_event_id()),
            client_timestamp: Some(get_timestamp()),
            ..Default::default()
        };
        item_create.set_item(item);

        let item_json = serde_json::to_string(&ClientEvent::ConversationItemCreate(item_create))?;
        connection.send(item_json).await?;

        // 4. Trigger response generation
        let response_create = ResponseCreateEvent {
            event_id: Some(generate_event_id()),
            client_timestamp: Some(get_timestamp()),
        };

        let response_json = serde_json::to_string(&ClientEvent::ResponseCreate(response_create))?;
        connection.send(response_json).await?;

        // 5. Process server response
        print!("AI: ");
        io::stdout().flush()?;

        process_response(&mut connection).await?;
        println!();
    }

    // Close connection
    connection.close().await?;
    println!("Connection closed.");

    Ok(())
}
