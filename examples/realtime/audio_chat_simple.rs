//! 简单的音频聊天示例
//!
//! 这个示例展示了如何使用 GLM-Realtime API 进行音频对话：
//! 1. 连接到 GLM-Realtime API
//! 2. 读取并发送音频文件
//! 3. 接收并显示 AI 的文字回复

use std::env;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

use zai_rs::real_time::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();

    // 从环境变量获取 API 密钥
    let api_key =
        env::var("ZHIPU_API_KEY").expect("Please set the ZHIPU_API_KEY environment variable");

    // 创建共享状态
    let response_text = Arc::new(Mutex::new(String::new()));
    let response_done = Arc::new(Mutex::new(false));
    let transcription = Arc::new(Mutex::new(String::new()));

    // 克隆共享状态供事件处理器使用
    let response_text_handler = Arc::clone(&response_text);
    let response_done_handler = Arc::clone(&response_done);
    let transcription_handler = Arc::clone(&transcription);

    // 创建一个简单的事件处理器
    struct SimpleEventHandler {
        response_text: Arc<Mutex<String>>,
        response_done: Arc<Mutex<bool>>,
        transcription: Arc<Mutex<String>>,
    }

    impl SimpleEventHandler {
        fn new(
            response_text: Arc<Mutex<String>>,
            response_done: Arc<Mutex<bool>>,
            transcription: Arc<Mutex<String>>,
        ) -> Self {
            Self {
                response_text,
                response_done,
                transcription,
            }
        }
    }

    impl EventHandler for SimpleEventHandler {
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
            print!("{}", event.delta);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            // 更新响应文本
            if let Ok(mut text) = self.response_text.lock() {
                text.push_str(&event.delta);
            }
        }

        fn on_response_text_done(&mut self, event: ResponseTextDoneEvent) {
            println!("\nText response completed");

            // 更新完整的响应文本
            if let Ok(mut text) = self.response_text.lock() {
                *text = event.text;
            }
        }

        fn on_response_done(&mut self, _event: ResponseDoneEvent) {
            println!("\nResponse completed");

            // 标记响应完成
            if let Ok(mut done) = self.response_done.lock() {
                *done = true;
            }
        }

        fn on_heartbeat(&mut self, _event: HeartbeatEvent) {
            // 忽略心跳
        }

        fn on_unknown_event(&mut self, event: serde_json::Value) {
            // 检查是否是音频转录事件
            if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
                match event_type {
                    "input_audio_transcription.completed" => {
                        if let Some(transcript) = event.get("transcript").and_then(|v| v.as_str()) {
                            println!("Audio transcription: {}", transcript);

                            // 更新转录文本
                            if let Ok(mut t) = self.transcription.lock() {
                                *t = transcript.to_string();
                            }
                        }
                    }
                    _ => {
                        // 忽略其他未知事件
                    }
                }
            }
        }

        // 其他事件使用默认实现
        fn on_response_audio_delta(&mut self, _event: ResponseAudioDeltaEvent) {}
        fn on_response_audio_done(&mut self, _event: ResponseAudioDoneEvent) {}
    }

    // 创建事件处理器
    let event_handler = SimpleEventHandler::new(
        response_text_handler,
        response_done_handler,
        transcription_handler,
    );

    // 创建客户端并设置事件处理器
    let mut client = RealtimeClient::new(api_key).with_event_handler(event_handler);

    // 配置会话参数
    let session_config = SessionConfig {
        model: Some("glm-realtime".to_string()),
        modalities: Some(vec!["text".to_string(), "audio".to_string()]),
        instructions: Some(
            "你是一个智能语音助手。请根据用户的语音输入提供相关的回答和建议。".to_string(),
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
        tools: Some(vec![]),
        beta_fields: Some(BetaFields {
            chat_mode: ChatMode::Audio.to_string(),
            tts_source: Some("e2e".to_string()),
            auto_search: Some(true),
            greeting_config: Some(GreetingConfig {
                enable: Some(true),
                content: Some(
                    "你好！我是小智，可以听懂你的语音，请开始我们的语音对话吧！".to_string(),
                ),
            }),
        }),
    };

    // 连接到实时 API
    println!("Connecting to GLM-Realtime for audio chat...");
    client.connect(GLMRealtime, session_config).await?;
    println!("Connected successfully!");

    // 读取音频文件
    let audio_file_path = "data/春.wav";
    println!("Reading audio file: {}", audio_file_path);

    let audio_data = match fs::read(audio_file_path) {
        Ok(data) => {
            println!("Successfully read audio file, size: {} bytes", data.len());
            data
        }
        Err(e) => {
            eprintln!("Failed to read audio file {}: {}", audio_file_path, e);
            return Err(e.into());
        }
    };

    // 在单独的任务中监听事件
    let response_text_clone = Arc::clone(&response_text);
    let response_done_clone = Arc::clone(&response_done);

    tokio::spawn(async move {
        // 这里我们模拟监听事件，因为我们不能直接使用 client.listen_for_events()
        // 在真实应用中，您需要根据具体的WebSocket库来实现事件监听

        // 等待响应完成
        let mut attempts = 0;
        while attempts < 30 {
            // 最多等待30秒
            sleep(Duration::from_secs(1)).await;
            attempts += 1;

            if let Ok(done) = response_done_clone.lock() {
                if *done {
                    break;
                }
            }

            if attempts % 5 == 0 {
                println!("Waiting for response... ({}s elapsed)", attempts);
            }
        }
    });

    // 发送音频数据
    println!("Sending audio data...");
    client.send_audio(&audio_data)?;

    // 提交音频缓冲区
    println!("Committing audio buffer...");
    client.commit_audio_buffer()?;

    // 创建响应
    println!("Creating response...");
    client.create_response()?;

    // 等待响应完成
    println!("Waiting for AI response...");

    // 主线程也检查响应状态
    let mut attempts = 0;
    let max_attempts = 30;

    while attempts < max_attempts {
        sleep(Duration::from_secs(1)).await;
        attempts += 1;

        if let Ok(done) = response_done.lock() {
            if *done {
                break;
            }
        }

        if attempts % 5 == 0 {
            println!("Still waiting... ({}s elapsed)", attempts);
        }
    }

    // 打印最终结果
    println!("\n=== Results ===");

    if let Ok(text) = transcription.lock() {
        if !text.is_empty() {
            println!("Audio Transcription: {}", text);
        }
    }

    if let Ok(text) = response_text.lock() {
        if !text.is_empty() {
            println!("AI Response: {}", text);
        } else {
            println!("No text response received.");
        }
    }

    println!("Audio chat example completed.");
    Ok(())
}
