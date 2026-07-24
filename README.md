# zai-rs

中文 | [English](README.en.md)

一个简洁、类型安全的 Zhipu AI Rust SDK。专注提升 Rust 开发者的接入效率：更少样板代码、更一致的错误处理、可读的请求/响应类型，以及开箱即用的示例。

当前仓库版本为 `6.0.1`。从旧版升级时，请先阅读
[0.6 迁移指南](docs/MIGRATING-0.6.md)；从 `0.6.0` 升级到 `6.0.1` 时还必须阅读
[6.0.1 安全加固迁移说明](docs/HARDENING_MIGRATION.md)。

## 快速开始

1. 准备环境
   - Rust 1.88+（edition 2024）
   - 设置环境变量：`ZHIPU_API_KEY="<your_api_key>"`
2. 构建
   - `cargo build`
3. 运行示例（examples/ 目录内）
   - `cargo run --example chat_loop`

更多设计和维护说明见 [架构说明](docs/ARCHITECTURE.md)，完整使用文档见 [docs/](docs/README.md)。

## 支持的模型

### 文本模型

| 模型 | 结构体 | Thinking | ReasoningEffort | Async | ToolStream |
|------|--------|----------|-----------------|-------|------------|
| glm-5.2 | `GLM5_2` | ✓ | ✓ | ✓ | ✓ |
| glm-5.1 | `GLM5_1` | ✓ | ✗ | ✓ | ✓ |
| glm-5.1-highspeed | `GLM5_1_highspeed` | ✓ | ✗ | ✓ | ✓ |
| glm-5 | `GLM5` | ✓ | ✗ | ✓ | ✓ |
| glm-5-turbo | `GLM5_turbo` | ✓ | ✗ | ✓ | ✓ |
| glm-4.7 | `GLM4_7` | ✓ | ✗ | ✓ | ✓ |
| glm-4.7-flash | `GLM4_7_flash` | ✓ | ✗ | ✗ | ✗ |
| glm-4.7-flashx | `GLM4_7_flashx` | ✓ | ✗ | ✗ | ✗ |
| glm-4.6 | `GLM4_6` | ✓ | ✗ | ✓ | ✓ |
| glm-4.5-air | `GLM4_5_air` | ✓ | ✗ | ✓ | ✗ |
| glm-4.5-airx | `GLM4_5_airx` | ✓ | ✗ | ✓ | ✗ |
| glm-4.5-flash | `GLM4_5_flash` | ✓ | ✗ | ✓ | ✗ |
| glm-4-flash-250414 | `GLM4_flash_250414` | ✗ | ✗ | ✓ | ✗ |
| glm-4-flashx-250414 | `GLM4_flashx_250414` | ✗ | ✗ | ✓ | ✗ |

### 文本视觉模型

| 模型 | 结构体 |
|------|--------|
| autoglm-phone | `autoglm_phone` |
| glm-5v-turbo | `GLM5V_turbo` |
| glm-4.6v | `GLM4_6v` |
| glm-4.6v-flash | `GLM4_6v_flash` |
| glm-4.6v-flashx | `GLM4_6v_flashx` |
| glm-4v-flash | `GLM4v_flash` |
| glm-4.1v-thinking-flash | `GLM4_1v_thinking_flash` |
| glm-4.1v-thinking-flashx | `GLM4_1v_thinking_flashx` |

### 语音处理模型

| 模型 | 结构体 | 能力 |
|------|--------|------|
| glm-4-voice | `GLM4_voice` | HTTP 语音聊天 |
| glm-asr-2512 | `GlmAsr` | 语音转文字（完整响应 / SSE） |
| glm-tts | `GlmTts` | 文本转语音（完整响应 / SSE） |

### 图像模型

| 模型 | 结构体 |
|------|--------|
| glm-image | `GlmImage` |
| cogview-4-250304 | `CogView4_250304` |
| cogview-4 | `CogView4` |
| cogview-3-flash | `CogView3Flash` |

### 实时模型（`realtime` feature）

| 模型 | 结构体 |
|------|--------|
| glm-realtime-flash | `GLM_realtime_flash` |
| glm-realtime-air | `GLM_realtime_air` |

`RealtimeClient::session` 通过密封的模型 trait 只接受上述两个 Realtime
模型，因此错误模型会在编译期被拒绝。`GLM4_voice` 是 HTTP 语音聊天模型，
不能用于 Realtime WebSocket；无可用操作能力的旧 Realtime marker 已在 0.6
删除。实时音频完整示例见 `examples/realtime_audio.rs`：

```bash
cargo run --example realtime_audio --features realtime
```

## 示例（examples/）

### 常用示例

| 示例 | 描述 |
|------|------|
| `chat_text` | 基础文本对话 |
| `chat_loop` | 多轮对话循环 |
| `chat_coding_plan` | 编程辅助对话（coding 专属端点） |
| `coding_plan_usage` | Coding Plan 余量 / 额度查询 |
| `chat_vision` | 视觉模型对话（图片/视频） |
| `chat_voice` | 语音模型对话 |
| `async_chat_text` | 异步对话任务提交与轮询 |
| `glm45_thinking_mode` | 深度思考模式 |
| `glm52_reasoning_effort` | GLM-5.2 推理深度控制（reasoning_effort） |
| `function_call` | 函数调用 |
| `function_call_with_toolkits` | 工具集调用 |
| `mcp` | 统一 MCP 搜索、阅读、仓库与视觉能力 |
| `mcp_web_search` | Web Search MCP 完整参数调用 |
| `mcp_web_reader` | Web Reader MCP 全部读取选项 |
| `mcp_zread` | ZRead MCP 的搜索、目录和文件读取 |
| `mcp_vision` | Vision MCP 的全部 8 个视觉工具 |
| `translation_bot` | 翻译机器人 |
| `ocr` | OCR 手写文字识别 |
| `gen_image` | 图像生成 |
| `gen_video` | 视频生成 |
| `text_to_audio` | 文本转语音 |
| `audio_to_text` | 语音转文字 |
| `voice_clone` | 音色复刻 |
| `embedding` | 文本嵌入 |
| `files_upload` | 文件上传 |
| `file_parser_demo` | 提交文件解析任务并轮询结果 |
| `knowledge_create` | 知识库创建 |
| `web_search` | 网络搜索 |
| `batches_create` | 批处理任务创建 |
| `batches_cancel` | 批处理任务取消 |
| `agent_invoke` | Agent v1 类型安全调用与异步结果轮询 |
| `assistant` | 助手调用（单条消息） |
| `application` | LLM 应用调用（文本输入） |
| `batches_list` / `batches_retrieve` | 批处理任务列表 / 详情检索 |
| `files_list` / `files_content` / `files_delete` | 文件列表 / 内容下载 / 删除 |
| `knowledge_list` / `knowledge_update` / `knowledge_delete` / `knowledge_capacity` | 知识库列表 / 编辑 / 删除 / 容量查询 |
| `knowledge_retrieve` | 知识库详情检索 |
| `knowledge_document_list` / `knowledge_document_detail` / `knowledge_document_delete` | 文档列表 / 详情 / 删除 |
| `knowledge_document_upload_file` / `knowledge_document_upload_url` | 上传文件 / URL 文档到知识库 |
| `knowledge_document_reembedding` / `knowledge_document_image_list` | 文档重新向量化 / 提取图片清单 |
| `rerank` | 候选段落重排序 |
| `tokenizer` | 文本分词计数 |
| `simple_moderation` | 内容安全审核 |
| `voice_list` / `voice_delete` | 音色列表 / 删除 |
| `realtime_audio` | 实时音频会话（`realtime` feature） |

### 运行方式

```bash
# Windows PowerShell
$Env:ZHIPU_API_KEY = "<your_api_key>"
cargo run --example chat_loop

# macOS/Linux
export ZHIPU_API_KEY="<your_api_key>"
cargo run --example chat_loop
```

## API 覆盖度

### 模型 API
- [x] POST 对话补全（同步/异步）
- [x] GLM-5.2 / GLM-5.1 / GLM-5 / GLM-4.7 / GLM-4.6 / GLM-4.5 系列支持
- [x] 思考模式（Thinking Mode），支持 clear_thinking 保留式思考
- [x] 推理深度控制（Reasoning Effort，GLM-5.2+：max/xhigh/high/medium/low/minimal/none）
- [x] 类型安全的 SSE 聊天流（`enable_stream().stream_via(&client)`）
- [x] 图像生成
- [x] 视频生成（异步）
- [x] 语音转文本（完整响应 / 类型安全 SSE）
- [x] 文本转语音（完整音频 / 类型安全 PCM SSE）
- [x] 音色复刻/列表/删除
- [x] 文本嵌入/重排序/分词
- [x] OCR 手写识别

### 工具 API
- [x] POST 网络搜索
- [x] POST 内容安全
- [x] POST 文件解析
- [x] GET 解析结果

### 文件 API
- [x] GET 文件列表
- [x] POST 上传文件
- [x] DELETE 删除文件
- [x] GET 文件内容

### 批处理 API
- [x] GET 列出批处理任务
- [x] POST 创建批处理任务
- [x] GET 检索批处理任务
- [x] POST 取消批处理任务

### 知识库 API
- [x] GET 知识库列表
- [x] POST 创建知识库
- [x] GET 知识库详情
- [x] PUT 编辑知识库
- [x] DELETE 删除知识库
- [x] GET 知识库使用量
- [x] GET 文档列表
- [x] POST 上传文件文档
- [x] POST 上传 URL 文档
- [x] GET 文档详情
- [x] DELETE 删除文档
- [x] POST 重新向量化

### Coding Plan API
- [x] POST 编程辅助对话（`/api/coding/paas/v4`，专属端点）
- [x] GET 余量 / 额度查询（`/api/monitor/usage/quota/limit`，5 小时窗口 + 每周窗口）

```rust,no_run
use zai_rs::{ZaiClient, usage::CodingPlanUsageRequest};

# async fn go(key: String) -> zai_rs::ZaiResult<()> {
let client = ZaiClient::builder(key).build()?;
let resp = CodingPlanUsageRequest::new().send_via(&client).await?;
if let Some(window) = resp.summary().time_limit() {
    println!("5h 余量: {}/{}", window.remaining, window.quota);
}
# Ok(())
# }
```

### MCP API

开启 `mcp` feature 后，可以直接使用统一 MCP API：

```rust,no_run
use zai_rs::mcp::{
    McpClient, SearchContentSize, SearchRecency, WebSearchRequest,
};

# async fn go() -> zai_rs::ZaiResult<()> {
let client = McpClient::from_env()?;
let request = WebSearchRequest::new("Rust rmcp 2.2.0")
    .domain("docs.rs")
    .recency(SearchRecency::OneMonth)
    .content_size(SearchContentSize::High);
let result = client.web_search_with(request).await?;
println!("{:#?}", result.results);
client.close().await?;
# Ok(())
# }
```

用户无需选择 MCP 服务或传输方式；SDK 会根据调用的能力自动路由、按需连接并复用连接。
所有工具都有强类型请求 API，用户无需构造模板 JSON：

- 搜索与阅读：`web_search[_with]`、`read_web_page[_with]`
- 开源仓库：`search_repo[_with]`、`repo_structure[_with]`、`read_repo_file[_with]`
- 视觉工具：`ui_to_artifact[_with]`、`extract_text[_with]`、
  `diagnose_error[_with]`、`understand_diagram[_with]`、
  `analyze_visualization[_with]`、`compare_ui[_with]`、
  `analyze_image[_with]`、`analyze_video[_with]`

搜索返回 `WebSearchResponse`，网页读取返回 `WebReaderResponse`，仓库和视觉工具返回
可直接显示或通过 `into_text()` 获取内容的 `McpTextResponse`。

完整示例见 `examples/mcp.rs`。中国区可设置 `ZHIPU_API_KEY` 或
`Z_AI_API_KEY`；国际区设置 `Z_AI_API_KEY`，并将 `Z_AI_MODE=ZAI`。Vision MCP
默认不会下载或启动外部代码；生产环境应通过 `with_vision_mcp_command` 指定已审计、
预安装的本地运行时。`with_vision_npx_download` 会显式恢复通过 Node.js 22+ 和
`npx` 下载固定顶层版本 `@z_ai/mcp-server@0.1.2` 的便利行为，但 npm 的传递依赖不受
`Cargo.lock`/`cargo-deny` 约束。两种模式下子进程都只继承最小运行环境以及
`Z_AI_API_KEY`、`Z_AI_MODE` 和可选模型变量，不会继承其他应用令牌。底层使用
`rmcp 2.2.0`。默认行为变化及工具执行策略迁移见
[`docs/HARDENING_MIGRATION.md`](docs/HARDENING_MIGRATION.md)。

每个 MCP 都有独立的完整功能示例：

```bash
cargo run --example mcp_web_search --features mcp -- "Rust rmcp 2.2.0" docs.rs
cargo run --example mcp_web_reader --features mcp -- https://docs.rs/rmcp/2.2.0/rmcp/
cargo run --example mcp_zread --features mcp -- modelcontextprotocol/rust-sdk CallToolResult crates/rmcp/src README.md
cargo run --example mcp_vision --features mcp -- source.png video.mp4 actual.png
```

`mcp_zread` 会运行全部 3 个 ZRead 工具，`mcp_vision` 会运行全部 8 个 Vision
工具，并在示例代码中显式选择 `npx` 下载。Vision 示例允许省略最后一个对比图片
参数，此时会使用同一图片测试 UI 差异检查；完整执行会产生多次视觉模型调用。

### API 结构

公开 API 按能力组织，内部文件布局不属于 API：

- `zai_rs::model::<capability>`：模型请求与响应，例如 `model::ocr::OcrRequest`
- `zai_rs::file`、`batches`、`knowledge`、`agent`、`usage`：扁平导出各能力类型
- `zai_rs::tool::<capability>`：Web 搜索与文件解析工具
- `zai_rs::mcp`：统一 MCP 客户端（`mcp` feature）
- `zai_rs::toolkits`：始终可用的自定义工具执行框架；`toolkits` feature 额外启用完整 JSON Schema 校验

不再通过 `data`、`request`、`response`、`model` 等实现模块导入类型。所有 HTTP
请求统一使用 `request.send_via(&client)`；原先没有业务方法的
`client.services()` 空门面已删除。

### 实时 API
- [x] WebSocket 类型定义
- [x] 会话管理与强类型事件/音频流（`RealtimeClient` / `SessionBuilder`）
- [x] Bearer / JWT 双鉴权
- [x] 完整 client/server 事件（`ClientEvent` / `ServerEvent`）
