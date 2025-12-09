//! 直接的音频聊天示例
//!
//! 这个示例展示了如何使用 GLM-Realtime API 进行音频对话：
//! 1. 连接到 GLM-Realtime API
//! 2. 读取并发送音频文件 (data/春.wav)
//! 3. 接收并显示 AI 的文字回复

use std::env;
use std::fs;
use std::time::Duration;
use tokio::time::sleep;

use zai_rs::real_time::*;

/// 简单的事件处理器，用于捕获AI的响应
struct DirectAudioEventHandler {
    // 这里我们不存储状态，而是直接打印响应
}

impl DirectAudioEventHandler {
    fn new() -> Self {
        Self {}
    }
}

impl EventHandler for DirectAudioEventHandler {
    fn on_error(&mut self, event: ErrorEvent) {
        eprintln!("错误: {}", event.error.message);
    }

    fn on_session_created(&mut self, event: SessionCreatedEvent) {
        println!("会话已创建: {}", event.session.id);
    }

    fn on_session_updated(&mut self, event: SessionUpdatedEvent) {
        println!("会话已更新: {}", event.session.id);
    }

    fn on_response_text_delta(&mut self, event: ResponseTextDeltaEvent) {
        // 直接打印响应文本
        print!("{}", event.delta);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }

    fn on_response_text_done(&mut self, _event: ResponseTextDoneEvent) {
        println!("\n=== 文本响应完成 ===");
    }

    fn on_response_audio_delta(&mut self, _event: ResponseAudioDeltaEvent) {
        // 在实际应用中，这里会处理音频输出
    }

    fn on_response_audio_done(&mut self, _event: ResponseAudioDoneEvent) {
        // 音频流完成
    }

    fn on_response_done(&mut self, _event: ResponseDoneEvent) {
        println!("\n=== 响应完成 ===");
        println!("音频聊天示例成功完成！");
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
                        println!("=== 音频转录 ===");
                        println!("{}", transcript);
                    }
                }
                _ => {
                    // 忽略其他未知事件
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();

    // 从环境变量获取 API 密钥
    let api_key = env::var("ZHIPU_API_KEY").expect("请设置 ZHIPU_API_KEY 环境变量");

    // 读取音频文件
    let audio_file_path = "data/春.wav";
    println!("正在读取音频文件: {}", audio_file_path);

    let audio_data = match fs::read(audio_file_path) {
        Ok(data) => {
            println!("成功读取音频文件，大小: {} 字节", data.len());
            data
        }
        Err(e) => {
            eprintln!("读取音频文件失败 {}: {}", audio_file_path, e);
            return Err(e.into());
        }
    };

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

    // 连接API并发送数据的完整流程
    println!("正在连接到 GLM-Realtime API...");

    // 使用重试机制
    let mut retry_count = 0;
    let max_retries = 3;

    while retry_count < max_retries {
        // 每次重试创建新的客户端和事件处理器
        let event_handler = DirectAudioEventHandler::new();
        let mut client = RealtimeClient::new(&api_key).with_event_handler(event_handler);

        match client.connect(GLMRealtime, session_config.clone()).await {
            Ok(_) => {
                println!("连接成功！开始发送音频数据...");

                // 发送音频数据
                if let Err(e) = client.send_audio(&audio_data) {
                    eprintln!("发送音频失败: {}", e);
                    retry_count += 1;
                    continue;
                }
                println!("音频数据发送完成！");

                // 提交音频缓冲区
                if let Err(e) = client.commit_audio_buffer() {
                    eprintln!("提交音频缓冲区失败: {}", e);
                    retry_count += 1;
                    continue;
                }
                println!("音频缓冲区提交完成！");

                // 请求生成响应
                if let Err(e) = client.create_response() {
                    eprintln!("创建响应失败: {}", e);
                    retry_count += 1;
                    continue;
                }
                println!("响应请求已发送！");

                // 开始监听响应，设置60秒超时
                println!("开始监听AI响应（最多60秒）...");
                println!("=== AI回复 ===");

                // 使用tokio::select!来实现超时控制
                tokio::select! {
                    // 监听事件
                    result = client.listen_for_events() => {
                        match result {
                            Ok(_) => {
                                println!("\n事件监听完成");
                            }
                            Err(e) => {
                                eprintln!("\n监听事件时出错: {}", e);
                            }
                        }
                    }
                    // 超时控制
                    _ = sleep(Duration::from_secs(60)) => {
                        println!("\n等待响应超时（60秒）");
                    }
                }

                // 完成
                return Ok(());
            }
            Err(e) => {
                eprintln!("连接失败 (尝试 {}/{}): {}", retry_count + 1, max_retries, e);
                retry_count += 1;

                if retry_count < max_retries {
                    println!("等待 5 秒后重试...");
                    sleep(Duration::from_secs(5)).await;
                } else {
                    println!("已达到最大重试次数，退出。");
                    return Err(e.into());
                }
            }
        }
    }

    Ok(())
}
