//! Example of real-time video chat with GLM-Realtime using trait-based WebSocket client
//!
//! This example demonstrates how to:
//! - Connect to the GLM-Realtime API using the new trait-based WebSocket client
//! - Configure a session for video chat
//! - Send video frames and audio to the model
//! - Receive and process video-aware responses
//! - Handle function calling during video conversations
//!
//! The example showcases the trait-based approach where models implement the WebSocketClient
//! trait to define their connection parameters.

use base64;
use base64::Engine;
use std::env;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;

use zai_rs::real_time::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();

    // Get API key from environment variable
    let api_key =
        env::var("ZHIPU_API_KEY").expect("Please set the ZHIPU_API_KEY environment variable");

    // Create a custom event handler to log events
    let event_handler = VideoChatEventHandler::new();

    // Create a real-time client using the new trait-based WebSocket client
    let mut client = RealtimeClient::new(api_key).with_event_handler(event_handler.clone());
    let mut event_handler = event_handler.clone();

    // The GLMRealtime model now implements the WebSocketClient trait, which defines
    // WebSocket URL and connection parameters. This allows for trait-based
    // configuration of WebSocket connections for different models.

    // Configure the session for video chat
    let session_config = SessionConfig {
        model: Some("glm-realtime".to_string()),
        modalities: Some(vec!["text".to_string(), "audio".to_string()]),
        instructions: Some(
            "你是一个智能视频助手。请根据用户展示的视觉内容提供相关的回答和建议。".to_string(),
        ),
        voice: Some("tongtong".to_string()),
        input_audio_format: "wav".to_string(),
        output_audio_format: "pcm".to_string(),
        input_audio_noise_reduction: Some(NoiseReductionConfig {
            reduction_type: NoiseReductionType::FarField,
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
        tools: Some(vec![Tool {
            tool_type: "function".to_string(),
            name: "analyze_image".to_string(),
            description: "Analyze the image in the video frame and provide details".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "analysis_type": {
                        "type": "string",
                        "description": "Type of analysis to perform (object, scene, text, etc.)"
                    }
                }
            }),
        }]),
        beta_fields: Some(BetaFields {
            chat_mode: ChatMode::VideoPassive.to_string(),
            tts_source: Some("e2e".to_string()),
            auto_search: Some(true),
            greeting_config: Some(GreetingConfig {
                enable: Some(true),
                content: Some(
                    "你好！我是小智，可以看懂你展示的内容，请开始我们的视频对话吧！".to_string(),
                ),
            }),
        }),
    };

    println!("Connecting to GLM-Realtime for video chat...");

    // Connect to the real-time API
    client.connect(GLMRealtime, session_config).await?;

    println!("Connected! Starting video chat...");

    // Start listening for events in a separate task
    let mut client_clone = client.clone();
    tokio::spawn(async move {
        if let Err(e) = client_clone.listen_for_events().await {
            eprintln!("Error listening for events: {}", e);
        }
    });

    // Main interaction loop
    println!("You can now start sending video frames and audio. Press Ctrl+C to exit.");

    // For this example, we'll simulate video frames with image files
    // In a real application, you would capture video from a camera
    let video_frames = vec![
        "video_samples/frame1.jpg",
        "video_samples/frame2.jpg",
        "video_samples/frame3.jpg",
    ];

    // Simulate a video conversation
    for (i, frame_path) in video_frames.iter().enumerate() {
        // Check if the frame file exists
        if std::path::Path::new(frame_path).exists() {
            println!("Sending video frame: {}", frame_path);

            // Read and encode the image as base64
            let image_data = match fs::read(frame_path) {
                Ok(data) => data,
                Err(e) => {
                    eprintln!("Failed to read image file {}: {}", frame_path, e);
                    continue;
                }
            };

            let image_base64 = base64::engine::general_purpose::STANDARD.encode(&image_data);

            // Send the video frame
            if let Err(e) = client.send_video_frame(&image_base64) {
                eprintln!("Failed to send video frame: {}", e);
                continue;
            }

            // Simulate asking a question about the frame
            let questions = vec![
                "请描述这张图片中的内容",
                "这张图片的主要特点是什么？",
                "你能识别图片中的物体吗？",
            ];

            if i < questions.len() {
                let question = questions[i];
                println!("Asking: {}", question);

                // Create a text message with the question
                let text_item = RealtimeConversationItem {
                    id: None,
                    item_type: ItemType::Message,
                    object: "realtime.item".to_string(),
                    status: Some(ItemStatus::Completed),
                    role: Some(ItemRole::User),
                    content: Some(vec![ContentPart {
                        content_type: ContentType::InputText,
                        text: Some(question.to_string()),
                        audio: None,
                        transcript: None,
                    }]),
                    name: None,
                    arguments: None,
                    output: None,
                };

                // Send the question
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
        } else {
            println!("Image file not found: {}", frame_path);
            println!("In a real application, you would capture video frames from a camera here.");

            // Just send a text message as fallback
            println!("Simulating video input and text question: '请看看我展示的这个物体'");

            let text_item = RealtimeConversationItem {
                id: None,
                item_type: ItemType::Message,
                object: "realtime.item".to_string(),
                status: Some(ItemStatus::Completed),
                role: Some(ItemRole::User),
                content: Some(vec![ContentPart {
                    content_type: ContentType::InputText,
                    text: Some("请看看我展示的这个物体，它是什么？".to_string()),
                    audio: None,
                    transcript: None,
                }]),
                name: None,
                arguments: None,
                output: None,
            };

            // Send the question
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

        // Wait before sending the next frame
        sleep(Duration::from_secs(1)).await;
    }

    println!("Video chat example completed.");
    Ok(())
}

/// Custom event handler for video chat
#[derive(Clone)]
struct VideoChatEventHandler {
    /// Received text from the model
    pub received_text: Option<String>,
    /// Function calls from the model
    pub function_calls: Vec<String>,
}

impl VideoChatEventHandler {
    fn new() -> Self {
        Self {
            received_text: None,
            function_calls: Vec::new(),
        }
    }

    #[allow(dead_code)]
    fn clear_text_buffer(&mut self) {
        self.received_text = None;
    }
}

impl EventHandler for VideoChatEventHandler {
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

    fn on_response_audio_delta(&mut self, _event: ResponseAudioDeltaEvent) {
        // In a real application, you would handle audio output
        println!("[Received audio chunk]");
    }

    fn on_response_audio_done(&mut self, _event: ResponseAudioDoneEvent) {
        // Audio stream is done
        println!("[Audio stream completed]");
    }

    fn on_response_done(&mut self, _event: ResponseDoneEvent) {
        println!("Response completed");
    }

    fn on_heartbeat(&mut self, _event: HeartbeatEvent) {
        // Ignore heartbeats
    }

    fn on_unknown_event(&mut self, event: serde_json::Value) {
        // Check if this is a function call event
        if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
            match event_type {
                "response.function_call_arguments.done" => {
                    if let Some(args) = event.get("arguments").and_then(|v| v.as_str()) {
                        println!("Function call arguments: {}", args);
                        self.function_calls.push(args.to_string());
                    }
                }
                "response.function_call.simple_browser" => {
                    println!("Browser function call triggered");
                    if let Some(session) = event.get("session") {
                        println!("Session info: {}", session);
                    }
                }
                _ => {
                    println!("Unknown event type: {}", event_type);
                }
            }
        } else {
            println!("Unknown event: {}", event);
        }
    }
}

// Example of how to use the GLMRealtime model with the trait-based WebSocket client
// The GLMRealtime model implements the WebSocketClient trait, which defines
// the WebSocket URL and connection parameters.

// You can also implement custom headers for GLMRealtime by creating a wrapper:
// struct CustomGLMRealtimeWrapper {
//     model: GLMRealtime,
//     custom_headers: Vec<(String, String)>,
// }
//
// impl WebSocketClient for CustomGLMRealtimeWrapper {
//     fn websocket_url(&self) -> String {
//         self.model.websocket_url()
//     }
//
//     fn websocket_host(&self) -> &'static str {
//         self.model.websocket_host()
//     }
//
//     fn custom_headers(&self) -> Vec<(String, String)> {
//         self.custom_headers.clone()
//     }
// }
//
// This trait-based approach allows different models to define their own
// connection parameters while reusing the same WebSocket client implementation.

// Clone implementation for RealtimeClient is now in the client.rs module

/// Simulate capturing video frames from a camera
#[allow(dead_code)]
async fn capture_video_frame() -> Result<String, Box<dyn std::error::Error>> {
    println!("In a real application, this would capture a video frame from a camera.");
    // In a real implementation, you would use a library like opencv to capture video frames
    // For this example, we'll return an empty string
    Ok(String::new())
}

/// Simulate playing audio through speakers
#[allow(dead_code)]
async fn play_audio_through_speakers(_audio_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    println!("In a real application, this would play audio through speakers.");
    // In a real implementation, you would use a library like rodio to play audio
    sleep(Duration::from_millis(1000)).await;
    Ok(())
}

/// Simulate displaying video output
#[allow(dead_code)]
async fn display_video_frame(_frame_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    println!("In a real application, this would display the video frame.");
    // In a real implementation, you would use a GUI library to display the frame
    Ok(())
}
