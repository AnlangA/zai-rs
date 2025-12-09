//! Example of real-time audio chat with GLM-Realtime
//!
//! This example demonstrates how to:
//! - Connect to the GLM-Realtime API
//! - Configure a session for audio conversation
//! - Send audio data to the model
//! - Receive and process audio responses
//! - Handle various server events

use std::env;
use std::io::{self, Read};
use std::process::Command;
use std::time::Duration;
use tokio::time::sleep;

use zai_rs::real_time::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();

    // Get API key from environment variable
    let api_key = env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY environment variable");

    // Create a custom event handler to log events
    let mut event_handler = AudioChatEventHandler::new();

    // Create a real-time client
    let mut client = RealtimeClient::new(api_key).with_event_handler(&mut event_handler);

    // Configure the session for audio chat
    let session_config = SessionConfig {
        model: Some("glm-realtime-flash".to_string()),
        modalities: Some(vec!["text".to_string(), "audio".to_string()]),
        instructions: Some(
            "你是一个友好的人工智能助手。请用自然、友好的语调与用户对话。".to_string(),
        ),
        voice: Some("tongtong".to_string()),
        input_audio_format: "wav".to_string(),
        output_audio_format: "pcm".to_string(),
        input_audio_noise_reduction: Some(NoiseReductionConfig {
            reduction_type: NoiseReductionType::NearField,
        }),
        turn_detection: Some(VadConfig {
            vad_type: VadType::ServerVad,
            create_response: Some(true),
            interrupt_response: Some(true),
            prefix_padding_ms: Some(300),
            silence_duration_ms: Some(500),
            threshold: Some(0.5),
        }),
        temperature: Some(0.7),
        max_response_output_tokens: Some("inf".to_string()),
        tools: None,
        beta_fields: Some(BetaFields {
            chat_mode: ChatMode::Audio.to_string(),
            tts_source: Some("e2e".to_string()),
            auto_search: Some(false),
            greeting_config: Some(GreetingConfig {
                enable: Some(true),
                content: Some("你好，我是小智，很高兴为你服务！".to_string()),
            }),
        }),
    };

    println!("Connecting to GLM-Realtime...");

    // Connect to the real-time API
    client.connect(GLMRealtimeFlash, session_config).await?;

    println!("Connected! Starting audio chat...");

    // Start listening for events in a separate task
    let mut client_clone = client.clone();
    tokio::spawn(async move {
        if let Err(e) = client_clone.listen_for_events().await {
            eprintln!("Error listening for events: {}", e);
        }
    });

    // Main interaction loop
    println!("You can now start speaking. Press Ctrl+C to exit.");

    // For this example, we'll simulate audio input with predefined audio files
    // In a real application, you would capture audio from a microphone
    let audio_files = vec![
        "audio_samples/hello.wav",
        "audio_samples/how_are_you.wav",
        "audio_samples/tell_joke.wav",
    ];

    for audio_file in audio_files {
        // Check if the file exists
        if std::path::Path::new(audio_file).exists() {
            println!("Playing audio from: {}", audio_file);

            // Read the audio file
            let audio_data = match std::fs::read(audio_file) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Failed to read audio file {}: {}", audio_file, e);
                    continue;
                }
            };

            // Send audio data to the model
            if let Err(e) = client.send_audio(&audio_data) {
                eprintln!("Failed to send audio: {}", e);
                continue;
            }

            // Wait for a response
            sleep(Duration::from_secs(2)).await;

            // For demonstration, we'll play the received audio
            // In a real application, you would handle the audio output
            play_audio_output(&event_handler.received_audio).await;
            event_handler.clear_audio_buffer();
        } else {
            println!("Audio file not found: {}", audio_file);
            println!("In a real application, you would capture audio from a microphone here.");

            // Simulate audio input with a text message instead
            println!("Simulating audio input: '你好，请介绍一下自己'");
            let text_item = RealtimeConversationItem {
                id: None,
                item_type: ItemType::Message,
                object: "realtime.item".to_string(),
                status: Some(ItemStatus::Completed),
                role: Some(ItemRole::User),
                content: Some(vec![ContentPart {
                    content_type: ContentType::InputText,
                    text: Some("你好，请介绍一下自己".to_string()),
                    audio: None,
                    transcript: None,
                }]),
                name: None,
                arguments: None,
                output: None,
            };

            if let Err(e) = client.create_conversation_item(text_item) {
                eprintln!("Failed to create conversation item: {}", e);
                continue;
            }

            // Trigger a response
            if let Err(e) = client.create_response() {
                eprintln!("Failed to create response: {}", e);
                continue;
            }

            // Wait for a response
            sleep(Duration::from_secs(3)).await;

            // Print the text response
            if let Some(text) = &event_handler.received_text {
                println!("Model response: {}", text);
                event_handler.clear_text_buffer();
            }

            // Simulate playing audio
            println!("[Simulated audio playback]");
            sleep(Duration::from_secs(1)).await;
        }
    }

    println!("Audio chat example completed.");
    Ok(())
}

/// Custom event handler for audio chat
struct AudioChatEventHandler {
    /// Received text from the model
    pub received_text: Option<String>,
    /// Received audio data from the model
    pub received_audio: Vec<Vec<u8>>,
    /// Temporary buffer for audio data
    audio_buffer: Vec<u8>,
}

impl AudioChatEventHandler {
    fn new() -> Self {
        Self {
            received_text: None,
            received_audio: Vec::new(),
            audio_buffer: Vec::new(),
        }
    }

    fn clear_text_buffer(&mut self) {
        self.received_text = None;
    }

    fn clear_audio_buffer(&mut self) {
        self.received_audio.clear();
    }
}

impl EventHandler for AudioChatEventHandler {
    fn on_error(&mut self, event: ErrorEvent) {
        eprintln!("Error: {}", event.error.message);
    }

    fn on_session_created(&mut self, event: SessionCreatedEvent) {
        println!("Session created: {}", event.session.id);
    }

    fn on_session_updated(&mut self, event: SessionUpdatedEvent) {
        println!("Session updated: {}", event.session.id);
    }

    fn on_response_text_delta(&mut self, event: ResponseTextDeltaEvent) {
        if let Some(text) = &mut self.received_text {
            text.push_str(&event.delta);
        } else {
            self.received_text = Some(event.delta);
        }
    }

    fn on_response_text_done(&mut self, event: ResponseTextDoneEvent) {
        self.received_text = Some(event.text);
    }

    fn on_response_audio_delta(&mut self, event: ResponseAudioDeltaEvent) {
        // Decode the base64 audio data
        if let Ok(audio_data) = base64::decode(&event.delta) {
            self.audio_buffer.extend_from_slice(&audio_data);
        }
    }

    fn on_response_audio_done(&mut self, _event: ResponseAudioDoneEvent) {
        // When audio is done, store the complete audio buffer
        if !self.audio_buffer.is_empty() {
            self.received_audio.push(self.audio_buffer.clone());
            self.audio_buffer.clear();
        }
    }

    fn on_response_done(&mut self, event: ResponseDoneEvent) {
        println!("Response completed");
    }

    fn on_heartbeat(&mut self, _event: HeartbeatEvent) {
        // Ignore heartbeats
    }

    fn on_unknown_event(&mut self, event: serde_json::Value) {
        println!("Unknown event: {}", event);
    }
}

// Clone implementation for RealtimeClient
impl Clone for RealtimeClient {
    fn clone(&self) -> Self {
        // Note: This is a simplified clone that doesn't clone the actual WebSocket connection
        // In a real implementation, you would need a more sophisticated approach
        Self {
            api_key: self.api_key.clone(),
            websocket: None,
            event_handler: Box::new(DefaultEventHandler),
        }
    }
}

/// Simulate playing audio output
async fn play_audio_output(audio_chunks: &[Vec<u8>]) {
    for chunk in audio_chunks {
        // In a real application, you would play the audio using a library like rodio
        println!("Playing audio chunk of {} bytes", chunk.len());
        sleep(Duration::from_millis(500)).await;
    }
}

/// Fallback function for capturing audio from microphone (not implemented in this example)
async fn capture_audio_from_mic() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    println!("In a real application, this would capture audio from the microphone.");
    // In a real implementation, you would use a library like cpal or audiopus to capture audio
    // For this example, we'll return an empty buffer
    Ok(Vec::new())
}

/// Fallback function for playing audio through speakers (not implemented in this example)
async fn play_audio_through_speakers(audio_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    println!("In a real application, this would play audio through the speakers.");
    // In a real implementation, you would use a library like rodio to play audio
    sleep(Duration::from_millis(1000)).await;
    Ok(())
}
