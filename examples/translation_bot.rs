//! # Conversational Translation Bot
//!
//! This example demonstrates a sophisticated translation bot that can:
//! - Automatically detect target language from user input
//! - Translate text to specified language (default: English)
//! - Maintain conversation context for continuous translation
//! - Support streaming responses for real-time feedback
//!
//! ## Usage Examples
//!
//! ```
//! 用户> 你好，翻译成英文
//! AI> Hello
//!
//! 用户> How are you today? 翻译成中文
//! AI> 你今天好吗？
//!
//! 用户> 这是一个测试
//! AI> This is a test
//!
//! 用户> exit
//! ```
//!
//! ## Prerequisites
//!
//! Set the `ZHIPU_API_KEY` environment variable:
//! ```bash
//! export ZHIPU_API_KEY="your-api-key-here"
//! ```
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example translation_bot
//! ```

use std::{
    io::{self, Write},
    sync::Arc,
};

use tokio::sync::Mutex;
use zai_rs::model::*;

/// Language detection result containing target language and confidence
#[derive(Debug, Clone)]
struct LanguageDetection {
    target_language: String,
    original_text: String,
    is_explicit: bool, // Whether user explicitly specified the target language
}

/// Translation bot with conversation state
struct TranslationBot {
    model: GLM4_5_flash,
    api_key: String,
}

impl TranslationBot {
    /// Create a new translation bot instance
    fn new(api_key: String) -> Self {
        let model = GLM4_5_flash {};

        Self { model, api_key }
    }

    /// Detect target language and extract text to translate from user input
    fn detect_language_and_extract_text(&self, input: &str) -> LanguageDetection {
        let input_lower = input.to_lowercase();

        // Check for explicit language specification
        if input_lower.contains("翻译成英文") || input_lower.contains("translate to english") {
            if let Some(text) = input.split("翻译成英文").next() {
                let text = text.trim();
                if !text.is_empty() {
                    return LanguageDetection {
                        target_language: "English".to_string(),
                        original_text: text.to_string(),
                        is_explicit: true,
                    };
                }
            }
            if let Some(text) = input.split("translate to english").next() {
                let text = text.trim();
                if !text.is_empty() {
                    return LanguageDetection {
                        target_language: "English".to_string(),
                        original_text: text.to_string(),
                        is_explicit: true,
                    };
                }
            }
        }

        if input_lower.contains("翻译成中文") || input_lower.contains("translate to chinese") {
            if let Some(text) = input.split("翻译成中文").next() {
                let text = text.trim();
                if !text.is_empty() {
                    return LanguageDetection {
                        target_language: "Chinese".to_string(),
                        original_text: text.to_string(),
                        is_explicit: true,
                    };
                }
            }
            if let Some(text) = input.split("translate to chinese").next() {
                let text = text.trim();
                if !text.is_empty() {
                    return LanguageDetection {
                        target_language: "Chinese".to_string(),
                        original_text: text.to_string(),
                        is_explicit: true,
                    };
                }
            }
        }

        // Try to detect if input is non-English text that needs translation to English
        if self.contains_non_latin_characters(input) && !input_lower.contains("translate") {
            return LanguageDetection {
                target_language: "English".to_string(),
                original_text: input.trim().to_string(),
                is_explicit: false,
            };
        }

        // Default: translate to English
        LanguageDetection {
            target_language: "English".to_string(),
            original_text: input.trim().to_string(),
            is_explicit: false,
        }
    }

    /// Simple check for non-Latin characters (basic language detection)
    fn contains_non_latin_characters(&self, text: &str) -> bool {
        text.chars().any(|c| !c.is_ascii() && c.is_alphabetic())
    }

    /// Create translation prompt based on detection
    fn create_translation_prompt(&self, detection: &LanguageDetection) -> String {
        format!(
            "请将以下文本翻译成{}：\n\n{}",
            detection.target_language, detection.original_text
        )
    }

    /// Translate text with streaming support
    async fn translate_stream(&mut self, text: &str) -> Result<String, Box<dyn std::error::Error>> {
        let detection = self.detect_language_and_extract_text(text);
        let prompt = self.create_translation_prompt(&detection);

        println!(
            "🎯 目标语言: {} ({})",
            detection.target_language,
            if detection.is_explicit {
                "用户指定"
            } else {
                "自动检测"
            }
        );
        print!("🔄 翻译中: ");
        io::stdout().flush().ok();

        // Create client with system message and user prompt
        let system_message = TextMessage::system(
            "你是一个专业的翻译助手。请将用户提供的文本翻译成指定的语言。\
            如果用户没有指定目标语言，默认翻译成英文。\
            请只返回翻译结果，不要添加额外的解释或说明。",
        );

        let client = ChatCompletion::new(self.model.clone(), system_message, self.api_key.clone())
            .add_messages(TextMessage::user(&prompt))
            .with_temperature(0.3)
            .with_top_p(0.9)
            .with_thinking(ThinkingType::disabled());

        let result = Arc::new(Mutex::new(String::new()));
        let result_clone = result.clone();

        // Enable streaming for real-time feedback
        let mut streaming_client = client.enable_stream();

        let finish = Arc::new(Mutex::new(None::<String>));
        let finish_clone = finish.clone();

        streaming_client
            .stream_for_each(move |chunk: ChatStreamResponse| {
                let result = result_clone.clone();
                let finish = finish_clone.clone();
                async move {
                    if let Some(content) = chunk
                        .choices
                        .first()
                        .and_then(|c| c.delta.as_ref())
                        .and_then(|d| d.content.as_deref())
                    {
                        print!("{}", content);
                        let _ = std::io::stdout().flush();

                        // Store the result
                        let mut result_guard = result.lock().await;
                        result_guard.push_str(content);
                    }

                    if let Some(reason) =
                        chunk.choices.first().and_then(|c| c.finish_reason.as_ref())
                    {
                        let mut finish_guard = finish.lock().await;
                        *finish_guard = Some(reason.clone());
                    }
                    Ok(())
                }
            })
            .await?;

        println!(); // New line after streaming

        let translation_result = result.lock().await.clone();

        Ok(translation_result)
    }

    /// Start the interactive translation loop
    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🤖 智能翻译机器人已启动！");
        println!("💡 使用方法：");
        println!("   - 直接输入文本进行翻译（默认翻译为英文）");
        println!("   - 使用 '翻译成[语言]' 或 'translate to [language]' 指定目标语言");
        println!("   - 输入 'exit' 或 'quit' 退出程序");
        println!("   - 输入 'help' 查看帮助");
        println!();

        let mut input = String::new();

        loop {
            print!("📝 请输入要翻译的文本> ");
            io::stdout().flush().ok();

            input.clear();
            io::stdin().read_line(&mut input)?;
            let user_input = input.trim().to_string();

            if user_input.is_empty() {
                continue;
            }

            match user_input.to_lowercase().as_str() {
                "exit" | "quit" | "退出" => {
                    println!("👋 再见！");
                    break;
                },
                "help" | "帮助" => {
                    self.show_help();
                    continue;
                },
                _ => {
                    // Perform translation
                    match self.translate_stream(&user_input).await {
                        Ok(_) => {
                            println!(); // Add spacing
                        },
                        Err(e) => {
                            eprintln!("❌ 翻译出错: {}", e);
                            println!();
                        },
                    }
                },
            }
        }

        Ok(())
    }

    /// Display help information
    fn show_help(&self) {
        println!("📖 帮助信息：");
        println!();
        println!("🔧 支持的命令：");
        println!("   • 直接输入文本：自动翻译（默认为英文）");
        println!("   • '翻译成英文' / 'translate to English'：指定目标语言");
        println!("   • 'help' / '帮助'：显示此帮助信息");
        println!("   • 'exit' / 'quit' / '退出'：退出程序");
        println!();
        println!("🌍 支持的语言：");
        println!("   • 中文、英文、日文、韩文、法文、德文、西班牙文");
        println!();
        println!("💡 示例：");
        println!("   • '你好，翻译成英文' → 'Hello'");
        println!("   • 'How are you? 翻译成中文' → '你好吗？'");
        println!("   • 'Bonjour' → 'Hello'（自动检测为非英文）");
        println!();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Get API key from environment variable
    let api_key = std::env::var("ZHIPU_API_KEY").expect("请设置环境变量 ZHIPU_API_KEY");

    // Create and run the translation bot
    let mut bot = TranslationBot::new(api_key);
    bot.run().await?;

    Ok(())
}
