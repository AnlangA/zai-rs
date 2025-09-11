//! # ZAI-RS: 智谱AI Rust SDK
//!
//! `zai-rs` 是一个为智谱AI（Zhipu AI）API提供完整支持的Rust SDK。
//! 它提供了类型安全的API客户端，支持多种AI功能包括聊天、图像生成、
//! 语音识别、语音合成等。
//!
//! ## 主要功能
//!
//! - **聊天完成** - 支持文本、视觉和语音聊天
//! - **图像生成** - 文本到图像生成
//! - **语音识别** - 音频转文本
//! - **语音合成** - 文本转语音
//! - **工具调用** - 函数调用和工具集成
//! - **文件管理** - 文件上传和管理
//! - **流式响应** - 支持Server-Sent Events流式响应
//!
//! ## 模块结构
//!
//! - [`client`] - HTTP客户端和网络通信
//! - [`model`] - 数据模型和API请求/响应类型
//! - [`file`] - 文件管理功能
//! - [`toolkits`] - 工具调用和函数执行框架
//!
//! ## 快速开始
//!
//! ```rust,no_run
//! use zai_rs::model::*;
//! use zai_rs::client::http::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let model = GLM4_5_flash {};
//!     let key = std::env::var("ZHIPU_API_KEY").unwrap();
//!     let client = ChatCompletion::new(model, TextMessage::user("你好"), key);
//!     let resp = client.post().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## 特性
//!
//! - **类型安全** - 编译时类型检查，防止运行时错误
//! - **异步支持** - 基于Tokio的异步API
//! - **流式处理** - 支持流式响应处理
//! - **工具集成** - 强大的工具调用框架
//! - **模型验证** - 内置数据验证和错误处理

pub mod client;
pub mod file;
pub mod model;
pub mod tool;
pub mod toolkits;
