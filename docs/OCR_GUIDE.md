# OCR (光学字符识别) 使用指南

本文档介绍如何使用 zai-rs 的 OCR 功能进行手写文字识别。

## 功能概述

OCR (Optical Character Recognition) 功能支持：
- ✅ 手写文字识别
- ✅ 多语言支持（中文、英文、日文等 20+ 种语言）
- ✅ 置信度返回
- ✅ 文字位置信息
- ✅ 支持多种图片格式（PNG、JPG、JPEG、BMP）

## 快速开始

### 基础用法

```rust
use zai_rs::model::ocr::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // 设置 API Key
    let key = std::env::var("ZHIPU_API_KEY")?;

    // 创建 OCR 请求
    let response = OcrRequest::new(key)
        .with_file_path("path/to/image.png")
        .with_tool_type(OcrToolType::HandWrite)
        .with_language_type(OcrLanguageType::ChnEng)
        .send()
        .await?;

    // 处理结果
    println!("识别结果: {:?}", response);
    Ok(())
}
```

### 完整示例

```rust
use zai_rs::model::ocr::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let key = std::env::var("ZHIPU_API_KEY").expect("Please set ZHIPU_API_KEY");
    let file_path = "data/ocr_example.png";

    println!("=== OCR 手写识别示例 ===\n");

    // 构建并发送 OCR 请求
    let response = OcrRequest::new(key)
        .with_file_path(file_path)
        .with_tool_type(OcrToolType::HandWrite)
        .with_language_type(OcrLanguageType::ChnEng)
        .with_probability(true)  // 返回置信度
        .send()
        .await?;

    println!("任务 ID: {:?}", response.task_id);
    println!("状态: {:?}", response.status);
    println!("识别结果数量: {:?}\n", response.words_result_num);

    // 遍历识别结果
    if let Some(results) = response.words_result {
        for (idx, item) in results.iter().enumerate() {
            println!("--- 文字块 {} ---", idx + 1);

            // 识别的文字
            if let Some(text) = &item.words {
                println!("文字: {}", text);
            }

            // 位置信息
            if let Some(location) = &item.location {
                println!(
                    "位置: left={}, top={}, width={}, height={}",
                    location.left.unwrap_or(0),
                    location.top.unwrap_or(0),
                    location.width.unwrap_or(0),
                    location.height.unwrap_or(0)
                );
            }

            // 置信度
            if let Some(prob) = &item.probability {
                println!(
                    "置信度: 平均={:.2}, 方差={:.2}, 最小={:.2}",
                    prob.average.unwrap_or(0.0),
                    prob.variance.unwrap_or(0.0),
                    prob.min.unwrap_or(0.0)
                );
            }

            println!();
        }
    }

    println!("=== 识别完成 ===");
    Ok(())
}
```

## API 参考

### OcrRequest

OCR 请求构建器。

#### 方法

| 方法 | 参数 | 说明 |
|------|------|------|
| `new(key)` | `String` | 创建新的 OCR 请求 |
| `with_file_path(path)` | `impl Into<String>` | 设置图片文件路径 |
| `with_tool_type(tool_type)` | `OcrToolType` | 设置识别工具类型 |
| `with_language_type(lang)` | `OcrLanguageType` | 设置识别语言 |
| `with_probability(enabled)` | `bool` | 是否返回置信度 |
| `with_request_id(id)` | `impl Into<String>` | 设置请求 ID |
| `with_user_id(id)` | `impl Into<String>` | 设置用户 ID |
| `send()` | - | 发送请求并获取响应 |

### OcrToolType

识别工具类型。

| 值 | 说明 |
|----|------|
| `HandWrite` | 手写文字识别 |

### OcrLanguageType

支持的语言类型。

| 值 | 说明 |
|----|------|
| `Auto` | 自动检测 |
| `ChnEng` | 中英文混合（默认） |
| `Eng` | 英语 |
| `Jap` | 日语 |
| `Kor` | 韩语 |
| `Fre` | 法语 |
| `Spa` | 西班牙语 |
| `Por` | 葡萄牙语 |
| `Ger` | 德语 |
| `Ita` | 意大利语 |
| `Rus` | 俄语 |
| `Dan` | 丹麦语 |
| `Dut` | 荷兰语 |
| `Mal` | 马来语 |
| `Swe` | 瑞典语 |
| `Ind` | 印尼语 |
| `Pol` | 波兰语 |
| `Rom` | 罗马尼亚语 |
| `Tur` | 土耳其语 |
| `Gre` | 希腊语 |
| `Hun` | 匈牙利语 |
| `Tha` | 泰语 |
| `Vie` | 越南语 |
| `Ara` | 阿拉伯语 |
| `Hin` | 印地语 |

### OcrResponse

OCR 识别响应。

#### 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `task_id` | `Option<String>` | 任务 ID |
| `status` | `Option<String>` | 任务状态 |
| `message` | `Option<String>` | 状态消息 |
| `words_result_num` | `Option<i32>` | 识别结果数量 |
| `words_result` | `Option<Vec<WordsResultItem>>` | 识别结果列表 |

### WordsResultItem

单个文字块的识别结果。

#### 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `words` | `Option<String>` | 识别的文字内容 |
| `location` | `Option<Location>` | 文字位置信息 |
| `probability` | `Option<Probability>` | 置信度信息 |

### Location

文字在图片中的位置。

#### 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `left` | `Option<u32>` | 左边距（像素） |
| `top` | `Option<u32>` | 上边距（像素） |
| `width` | `Option<u32>` | 宽度（像素） |
| `height` | `Option<u32>` | 高度（像素） |

### Probability

识别置信度。

#### 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `average` | `Option<f64>` | 平均置信度（0-1） |
| `variance` | `Option<f64>` | 置信度方差 |
| `min` | `Option<f64>` | 最小置信度 |

## 文件要求

### 支持的格式
- PNG
- JPG / JPEG
- BMP

### 文件大小限制
- 最大 **8MB**

### 图片建议
- 分辨率：建议至少 500x500 像素
- 对比度：文字和背景对比度要高
- 清晰度：图片清晰，避免模糊
- 光照：均匀光照，避免阴影

## 错误处理

```rust
use zai_rs::model::ocr::*;

match OcrRequest::new(key)
    .with_file_path("image.png")
    .send()
    .await
{
    Ok(response) => {
        println!("识别成功: {:?}", response);
    },
    Err(e) => {
        eprintln!("识别失败: {:?}", e);
        match e {
            ZaiError::FileError { code, message } => {
                eprintln!("文件错误 [{}]: {}", code, message);
            },
            ZaiError::ApiError { code, message } => {
                eprintln!("API 错误 [{}]: {}", code, message);
            },
            _ => {
                eprintln!("其他错误: {:?}", e);
            }
        }
    }
}
```

## 常见错误

| 错误 | 原因 | 解决方案 |
|------|------|----------|
| 文件不存在 | 文件路径错误 | 检查文件路径是否正确 |
| 文件过大 | 超过 8MB 限制 | 压缩图片或使用更小的图片 |
| 格式不支持 | 文件格式不是 PNG/JPG/BMP | 转换图片格式 |
| 识别失败 | 图片质量太差 | 提高图片质量 |

## 最佳实践

### 1. 批量处理

```rust
use std::path::Path;

async fn process_images(dir: &str, key: String) -> Result<(), Box<dyn std::error::Error>> {
    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let path = entry?.path();
        if path.extension().and_then(|s| s.to_str()) == Some("png") {
            let response = OcrRequest::new(key.clone())
                .with_file_path(path.to_str().unwrap())
                .send()
                .await?;

            println!("识别 {}: {:?}", path.display(), response.words_result_num);
        }
    }

    Ok(())
}
```

### 2. 结果持久化

```rust
use std::fs::File;
use std::io::Write;

fn save_result(result: &OcrResponse, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(output_path)?;

    writeln!(file, "OCR 识别结果")?;
    writeln!(file, "任务 ID: {:?}", result.task_id)?;
    writeln!(file, "状态: {:?}", result.status)?;

    if let Some(results) = &result.words_result {
        for (idx, item) in results.iter().enumerate() {
            writeln!(file, "\n--- 文字块 {} ---", idx + 1)?;
            if let Some(text) = &item.words {
                writeln!(file, "文字: {}", text)?;
            }
        }
    }

    Ok(())
}
```

### 3. 结合其他 API

```rust
// OCR + 翻译
async fn ocr_and_translate(image_path: &str, key: String) -> Result<(), Box<dyn std::error::Error>> {
    // 1. OCR 识别
    let ocr_response = OcrRequest::new(key.clone())
        .with_file_path(image_path)
        .send()
        .await?;

    // 2. 提取文字
    let text = ocr_response
        .words_result
        .and_then(|results| results.first())
        .and_then(|item| item.words.clone())
        .unwrap_or_default();

    // 3. 调用翻译 API
    let model = GLM4_5_flash {};
    let messages = TextMessage::user(format!("请将以下文字翻译成英文：{}", text));

    let client = ChatCompletion::new(model, messages, key);
    let translation = client.send().await?;

    println!("翻译结果: {:?}", translation);
    Ok(())
}
```

## 相关链接

- [智谱 AI OCR API 文档](https://docs.bigmodel.cn/)
- [示例代码](../examples/ocr.rs)
- [错误处理指南](ERROR_HANDLING.md)

---

**最后更新**: 2025-02
