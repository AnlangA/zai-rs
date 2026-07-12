# OCR 使用指南

OCR API 用于识别本地图片中的手写文字。请求采用 `multipart/form-data`，通过共享的
`ZaiClient` 发送。

## 快速开始

```rust,ignore
use zai_rs::{
    ZaiClient,
    model::ocr::{OcrLanguageType, OcrRequest, OcrToolType},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZaiClient::from_env()?;
    let response = OcrRequest::new()
        .with_file_path("image.png")
        .with_tool_type(OcrToolType::HandWrite)
        .with_language_type(OcrLanguageType::ChnEng)
        .with_probability(true)
        .send_via(&client)
        .await?;

    for item in response.words_result.unwrap_or_default() {
        if let Some(text) = item.words {
            println!("{text}");
        }
    }
    Ok(())
}
```

不要为每张图片重新创建 client。`ZaiClient::clone()` 成本很低，且会共享连接池、
端点配置和传输策略。

## 请求选项

| 方法 | 说明 |
|------|------|
| `OcrRequest::new()` | 创建空请求；不接收 API key |
| `with_file_path(path)` | 设置必填的本地图片路径 |
| `with_tool_type(OcrToolType::HandWrite)` | 选择手写识别；未设置时也默认使用该值 |
| `with_language_type(language)` | 设置识别语言；未设置时由服务端决定默认行为 |
| `with_probability(bool)` | 请求返回置信度统计 |
| `with_request_id(id)` | 设置客户端请求 ID |
| `with_user_id(id)` | 设置 6 到 128 字符的终端用户 ID |
| `validate()` | 同步检查参数与本地文件；适合非异步预检 |
| `send_via(&client)` | 异步校验文件、上传并解析响应 |

`OcrLanguageType` 包含 `Auto`、`ChnEng`、`Eng`、`Jap`、`Kor`、`Fre`、
`Spa`、`Por`、`Ger`、`Ita`、`Rus`、`Dan`、`Dut`、`Mal`、`Swe`、`Ind`、
`Pol`、`Rom`、`Tur`、`Gre`、`Hun`、`Tha`、`Vie`、`Ara` 和 `Hin`。

## 文件约束

- 路径必须指向普通文件。
- 扩展名必须为 `.png`、`.jpg`、`.jpeg` 或 `.bmp`（大小写不敏感）。
- 文件大小不得超过 8 MiB。
- 扩展名用于选择 multipart MIME type；伪造扩展名仍可能被服务端拒绝。

`send_via` 使用异步文件元数据和读取操作，避免在 Tokio executor 上执行阻塞文件
探测。请求在网络发送前完成本地校验，因此路径或格式错误不会消耗 API 调用。

## 读取响应

`OcrResponse` 按冻结 OpenAPI 契约解码。`task_id`、`message`、`status` 和
`words_result_num` 必须存在；`status` 只能是 `succeeded` 或 `failed`。
`words_result` 可省略：

| 字段 | 含义 |
|------|------|
| `task_id` | 服务端任务 ID |
| `status` | 任务状态 |
| `message` | 状态或错误描述 |
| `words_result_num` | 识别结果数量 |
| `words_result` | 文字块列表 |

每个 `WordsResultItem` 都包含 `words`、完整矩形 `location`，以及完整的
`probability`（`average`、`variance`、`min`）。只有结果列表本身需要模式匹配：

```rust,ignore
if let Some(items) = response.words_result {
    for item in items {
        println!("{}", item.words);
        println!(
            "left={}, top={}, width={}, height={}",
            item.location.left,
            item.location.top,
            item.location.width,
            item.location.height,
        );
    }
}
```

## 错误处理

本地路径、文件大小和扩展名问题返回 `ZaiError::FileError`；缺少必填路径或
`user_id` 长度不合法返回 `ZaiError::ApiError`。网络、认证和服务端业务错误沿用
SDK 的统一错误类型：

```rust,ignore
use zai_rs::client::error::ZaiError;

match request.send_via(&client).await {
    Ok(response) => println!("识别到 {:?} 个文字块", response.words_result_num),
    Err(ZaiError::FileError { code, message }) => {
        tracing::warn!(code, %message, "OCR 文件不可用");
    },
    Err(error) => tracing::error!(error = %error, "OCR 请求失败"),
}
```

更多恢复与日志建议见[错误处理指南](ERROR_HANDLING.md)。
