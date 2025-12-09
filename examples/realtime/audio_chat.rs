//! 精简音频聊天示例 - 流式输出版

use base64::Engine;
use std::env;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use zai_rs::real_time::*;

// 流式事件处理器
struct SimpleAudioHandler {
    is_receiving_text: Arc<AtomicBool>,
    is_receiving_audio: Arc<AtomicBool>,
}

impl SimpleAudioHandler {
    fn new() -> Self {
        Self {
            is_receiving_text: Arc::new(AtomicBool::new(false)),
            is_receiving_audio: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl EventHandler for SimpleAudioHandler {
    fn on_error(&mut self, event: ErrorEvent) {
        eprintln!("错误: {}", event.error.message);
    }

    fn on_session_created(&mut self, event: SessionCreatedEvent) {
        println!("会话已创建: {}", event.session.id);
    }

    fn on_response_text_delta(&mut self, event: ResponseTextDeltaEvent) {
        // 标记正在接收文本
        if !self.is_receiving_text.load(Ordering::Relaxed) {
            self.is_receiving_text.store(true, Ordering::Relaxed);
            print!("AI回复: ");
        }

        // 直接输出文本，实现流式效果
        print!("{}", event.delta);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }

    fn on_response_text_done(&mut self, _event: ResponseTextDoneEvent) {
        // 重置文本接收状态
        self.is_receiving_text.store(false, Ordering::Relaxed);
        println!(); // 添加换行
    }

    fn on_response_audio_delta(&mut self, event: ResponseAudioDeltaEvent) {
        if let Ok(_bytes) = base64::engine::general_purpose::STANDARD.decode(&event.delta) {
            // 标记正在接收音频
            if !self.is_receiving_audio.load(Ordering::Relaxed) {
                self.is_receiving_audio.store(true, Ordering::Relaxed);
                print!("\n音频流: ");
            }

            print!("·");
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
    }

    fn on_response_done(&mut self, event: ResponseDoneEvent) {
        println!("\n响应完成");
        if let Some(usage) = &event.response.usage {
            println!(
                "Token使用: {} (输入:{}, 输出:{})",
                usage.total_tokens, usage.input_tokens, usage.output_tokens
            );
        }
    }

    fn on_unknown_event(&mut self, event: serde_json::Value) {
        if let Some(event_type) = event.get("type").and_then(|v| v.as_str()) {
            match event_type {
                "input_audio_buffer.committed" => {
                    println!("\n音频缓冲区已提交，开始处理...");
                }
                "response.created" => {
                    println!("\nAI正在生成回复...");
                }
                "response.audio.done" => {
                    self.is_receiving_audio.store(false, Ordering::Relaxed);
                    print!("\n");
                }
                "conversation.item.input_audio_transcription.completed" => {
                    if let Some(transcript) = event.get("transcript").and_then(|v| v.as_str()) {
                        println!("\n语音识别: {}", transcript);
                    }
                }
                _ => {}
            }
        }
    }

    // 以下为必需实现但留空的方法
    fn on_session_updated(&mut self, _event: SessionUpdatedEvent) {}
    fn on_response_audio_done(&mut self, _event: ResponseAudioDoneEvent) {}
    fn on_heartbeat(&mut self, _event: HeartbeatEvent) {}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let api_key = env::var("ZHIPU_API_KEY").unwrap_or_else(|_| {
        eprintln!("请设置 ZHIPU_API_KEY 环境变量");
        std::process::exit(1);
    });

    // 检查音频文件是否存在
    let audio_file_path = "data/test.wav";
    if !fs::metadata(audio_file_path).is_ok() {
        eprintln!("错误: 音频文件 {} 不存在", audio_file_path);
        eprintln!("请先准备一个 WAV 格式的音频文件");
        std::process::exit(1);
    }

    // 会话配置 - 添加了必要的配置以触发AI回复
    let session_config = SessionConfig {
        model: Some("glm-realtime".to_string()),
        modalities: Some(vec!["text".to_string(), "audio".to_string()]),
        instructions: Some("你是一个友好的助手，请简洁地回答用户的问题。".to_string()),
        voice: Some("tongtong".to_string()),
        input_audio_format: "wav".to_string(),
        output_audio_format: "pcm".to_string(),
        input_audio_noise_reduction: Some(NoiseReductionConfig {
            reduction_type: NoiseReductionType::FarField,
        }),
        turn_detection: Some(VadConfig {
            vad_type: VadType::ServerVad,
            create_response: Some(true), // 关键配置：自动创建回复
            interrupt_response: Some(true),
            prefix_padding_ms: Some(300),
            silence_duration_ms: Some(500),
            threshold: Some(0.5),
        }),
        temperature: Some(0.7),
        max_response_output_tokens: Some("200".to_string()),
        tools: Some(vec![]),
        beta_fields: Some(BetaFields {
            chat_mode: ChatMode::Audio.to_string(),
            tts_source: None,
            auto_search: Some(false),
            greeting_config: None,
        }),
    };

    println!("连接到 GLM-Realtime API...");
    let audio_data = fs::read(audio_file_path)?;
    println!("准备发送音频数据 (大小: {} 字节)...", audio_data.len());

    let mut client = RealtimeClient::new(&api_key).with_event_handler(SimpleAudioHandler::new());

    match client.connect(GLMRealtime, session_config).await {
        Ok(_) => {
            println!("连接成功，发送音频数据...");
            client.send_audio(&audio_data).await?;

            // 给服务器一些时间处理音频
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            if let Err(e) = client.listen_for_events().await {
                eprintln!("监听错误: {}", e);
            }

            println!("\n对话完成");
        }
        Err(e) => {
            eprintln!("连接失败: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}
