# zai-rs

[中文文档](README.md) | English

A concise, type-safe Rust SDK for Zhipu AI (BigModel / Z.ai). It focuses on developer ergonomics for Rust users: less boilerplate, consistent error handling, readable request/response types, and ready-to-run examples.

The current repository version is `0.6.0`. When upgrading from an older
release, read the [0.6 migration guide](docs/MIGRATING-0.6.md) first. When
evaluating the unreleased hardening worktree, also read the
[hardening migration notes](docs/HARDENING_MIGRATION.md).

## Quick start

1. Prepare the environment
   - Rust 1.88+ (edition 2024)
   - Set the environment variable: `ZHIPU_API_KEY="<your_api_key>"`
2. Build
   - `cargo build`
3. Run an example (from the `examples/` directory)
   - `cargo run --example chat_loop`

See [Architecture](docs/ARCHITECTURE.md) for design and maintenance notes, and
[docs/](docs/README.md) for the full user documentation.

## Supported models

### Text models

| Model | Struct | Thinking | ReasoningEffort | Async | ToolStream |
|-------|--------|----------|-----------------|-------|------------|
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

### Text & vision models

| Model | Struct |
|-------|--------|
| autoglm-phone | `autoglm_phone` |
| glm-5v-turbo | `GLM5V_turbo` |
| glm-4.6v | `GLM4_6v` |
| glm-4.6v-flash | `GLM4_6v_flash` |
| glm-4.6v-flashx | `GLM4_6v_flashx` |
| glm-4v-flash | `GLM4v_flash` |
| glm-4.1v-thinking-flash | `GLM4_1v_thinking_flash` |
| glm-4.1v-thinking-flashx | `GLM4_1v_thinking_flashx` |

### Audio models

| Model | Struct | Capability |
|-------|--------|------------|
| glm-4-voice | `GLM4_voice` | HTTP voice chat |
| glm-asr-2512 | `GlmAsr` | Speech-to-text (full response / SSE) |
| glm-tts | `GlmTts` | Text-to-speech (full response / SSE) |

### Image models

| Model | Struct |
|-------|--------|
| glm-image | `GlmImage` |
| cogview-4-250304 | `CogView4_250304` |
| cogview-4 | `CogView4` |
| cogview-3-flash | `CogView3Flash` |

### Realtime models (`realtime` feature)

| Model | Struct |
|-------|--------|
| glm-realtime-flash | `GLM_realtime_flash` |
| glm-realtime-air | `GLM_realtime_air` |

Through a sealed model trait, `RealtimeClient::session` accepts only the two
Realtime models above, so an invalid model is rejected at compile time.
`GLM4_voice` is an HTTP voice-chat model and cannot be used with the Realtime
WebSocket; the legacy Realtime markers without usable operations were removed
in 0.6. A full realtime audio example lives in `examples/realtime_audio.rs`:

```bash
cargo run --example realtime_audio --features realtime
```

## Examples (examples/)

### Frequently used examples

| Example | Description |
|---------|-------------|
| `chat_text` | Basic text chat |
| `chat_loop` | Multi-turn conversation loop |
| `chat_coding_plan` | Coding assistant chat (dedicated coding endpoint) |
| `coding_plan_usage` | Coding Plan quota / remaining-quota query |
| `chat_vision` | Vision model chat (images / video) |
| `chat_voice` | Voice model chat |
| `async_chat_text` | Async chat task submission and polling |
| `glm45_thinking_mode` | Deep thinking mode |
| `glm52_reasoning_effort` | GLM-5.2 reasoning depth control (`reasoning_effort`) |
| `function_call` | Function calling |
| `function_call_with_toolkits` | Tool calling with the toolkits framework |
| `mcp` | Unified MCP search, reader, repository, and vision capabilities |
| `mcp_web_search` | Web Search MCP with full parameters |
| `mcp_web_reader` | Web Reader MCP with all reading options |
| `mcp_zread` | ZRead MCP search, directory, and file reading |
| `mcp_vision` | All 8 Vision MCP tools |
| `translation_bot` | Translation bot |
| `ocr` | OCR handwriting recognition |
| `gen_image` | Image generation |
| `gen_video` | Video generation |
| `text_to_audio` | Text-to-speech |
| `audio_to_text` | Speech-to-text |
| `voice_clone` | Voice cloning |
| `embedding` | Text embeddings |
| `files_upload` | File upload |
| `file_parser_demo` | Submit a file-parsing task and poll the result |
| `knowledge_create` | Knowledge base creation |
| `web_search` | Web search |
| `batches_create` | Batch task creation |
| `batches_cancel` | Batch task cancellation |
| `agent_invoke` | Type-safe Agent v1 invocation and async-result polling |
| `assistant` | Assistant invocation (single message) |
| `application` | LLM application invocation (text input) |
| `batches_list` / `batches_retrieve` | Batch task listing / retrieval |
| `files_list` / `files_content` / `files_delete` | File listing / content download / deletion |
| `knowledge_list` / `knowledge_update` / `knowledge_delete` / `knowledge_capacity` | Knowledge base listing / editing / deletion / capacity query |
| `knowledge_retrieve` | Knowledge base detail retrieval |
| `knowledge_document_list` / `knowledge_document_detail` / `knowledge_document_delete` | Document listing / detail / deletion |
| `knowledge_document_upload_file` / `knowledge_document_upload_url` | Upload file / URL documents to a knowledge base |
| `knowledge_document_reembedding` / `knowledge_document_image_list` | Document re-embedding / extracted-image listing |
| `rerank` | Candidate passage reranking |
| `tokenizer` | Text token counting |
| `simple_moderation` | Content moderation |
| `voice_list` / `voice_delete` | Voice listing / deletion |
| `realtime_audio` | Realtime audio session (`realtime` feature) |

### How to run

```bash
# Windows PowerShell
$Env:ZHIPU_API_KEY = "<your_api_key>"
cargo run --example chat_loop

# macOS/Linux
export ZHIPU_API_KEY="<your_api_key>"
cargo run --example chat_loop
```

## API coverage

### Model APIs
- [x] POST chat completions (sync/async)
- [x] GLM-5.2 / GLM-5.1 / GLM-5 / GLM-4.7 / GLM-4.6 / GLM-4.5 series support
- [x] Thinking Mode, with `clear_thinking` preserved-thinking support
- [x] Reasoning depth control (Reasoning Effort, GLM-5.2+: max/xhigh/high/medium/low/minimal/none)
- [x] Type-safe SSE chat streaming (`enable_stream().stream_via(&client)`)
- [x] Image generation
- [x] Video generation (async)
- [x] Speech-to-text (full response / type-safe SSE)
- [x] Text-to-speech (full audio / type-safe PCM SSE)
- [x] Voice clone / list / delete
- [x] Text embeddings / rerank / tokenizer
- [x] OCR handwriting recognition

### Tool APIs
- [x] POST web search
- [x] POST content moderation
- [x] POST file parsing
- [x] GET parsing result

### File APIs
- [x] GET file list
- [x] POST upload file
- [x] DELETE delete file
- [x] GET file content

### Batch APIs
- [x] GET list batch tasks
- [x] POST create batch task
- [x] GET retrieve batch task
- [x] POST cancel batch task

### Knowledge APIs
- [x] GET knowledge base list
- [x] POST create knowledge base
- [x] GET knowledge base detail
- [x] PUT edit knowledge base
- [x] DELETE delete knowledge base
- [x] GET knowledge base usage
- [x] GET document list
- [x] POST upload file document
- [x] POST upload URL document
- [x] GET document detail
- [x] DELETE delete document
- [x] POST re-embed document

### Coding Plan API
- [x] POST coding assistant chat (`/api/coding/paas/v4`, dedicated endpoint)
- [x] GET quota / remaining-quota query (`/api/monitor/usage/quota/limit`, 5-hour window + weekly window)

```rust,no_run
use zai_rs::{ZaiClient, usage::CodingPlanUsageRequest};

# async fn go(key: String) -> zai_rs::ZaiResult<()> {
let client = ZaiClient::builder(key).build()?;
let resp = CodingPlanUsageRequest::new().send_via(&client).await?;
if let Some(window) = resp.summary().time_limit() {
    println!("5h remaining: {}/{}", window.remaining, window.quota);
}
# Ok(())
# }
```

### MCP API

With the `mcp` feature enabled, the unified MCP API is available directly:

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

You never pick an MCP server or transport: the SDK routes automatically based
on the capability you call, connects on demand, and reuses connections. Every
tool has a strongly-typed request API — no template JSON to assemble:

- Search & reading: `web_search[_with]`, `read_web_page[_with]`
- Open-source repositories: `search_repo[_with]`, `repo_structure[_with]`, `read_repo_file[_with]`
- Vision tools: `ui_to_artifact[_with]`, `extract_text[_with]`,
  `diagnose_error[_with]`, `understand_diagram[_with]`,
  `analyze_visualization[_with]`, `compare_ui[_with]`,
  `analyze_image[_with]`, `analyze_video[_with]`

Search returns `WebSearchResponse`, page reading returns `WebReaderResponse`,
and repository/vision tools return `McpTextResponse`, which can be displayed
directly or unwrapped with `into_text()`.

See `examples/mcp.rs` for a complete example. For the China region set
`ZHIPU_API_KEY` or `Z_AI_API_KEY`; for the international region set
`Z_AI_API_KEY` plus `Z_AI_MODE=ZAI`. The Vision MCP does not download or start
external code by default. Production applications should use
`with_vision_mcp_command` with a reviewed, preinstalled local runtime.
`with_vision_npx_download` explicitly restores the Node.js 22+ / `npx`
convenience path for the pinned top-level `@z_ai/mcp-server@0.1.2`, but its
transitive npm dependency graph is outside `Cargo.lock` and `cargo-deny`.
In either mode, the child receives only a minimal runtime environment plus
`Z_AI_API_KEY`, `Z_AI_MODE`, and the optional model override—not unrelated
application tokens. Built on `rmcp 2.2.0`. See
[`docs/HARDENING_MIGRATION.md`](docs/HARDENING_MIGRATION.md) for the default
behavior and tool-policy migration.

Each MCP has a dedicated full-feature example:

```bash
cargo run --example mcp_web_search --features mcp -- "Rust rmcp 2.2.0" docs.rs
cargo run --example mcp_web_reader --features mcp -- https://docs.rs/rmcp/2.2.0/rmcp/
cargo run --example mcp_zread --features mcp -- modelcontextprotocol/rust-sdk CallToolResult crates/rmcp/src README.md
cargo run --example mcp_vision --features mcp -- source.png video.mp4 actual.png
```

`mcp_zread` runs all 3 ZRead tools and `mcp_vision` runs all 8 Vision tools.
The vision example explicitly opts into the `npx` download and allows omitting
the last comparison-image argument, in which case the same image is used to
test UI diffing; a full run performs several vision-model calls.

### API structure

The public API is organized by capability; the internal file layout is not
part of the API:

- `zai_rs::model::<capability>`: model requests and responses, e.g. `model::ocr::OcrRequest`
- `zai_rs::file`, `batches`, `knowledge`, `agent`, `usage`: flat exports of each capability's types
- `zai_rs::tool::<capability>`: web search and file-parsing tools
- `zai_rs::mcp`: unified MCP client (`mcp` feature)
- `zai_rs::toolkits`: always-available custom tool-execution framework; the `toolkits` feature additionally enables full JSON Schema validation

Types are no longer imported through implementation modules such as `data`,
`request`, `response`, or `model`. Every HTTP request is sent uniformly with
`request.send_via(&client)`; the old `client.services()` facade, which exposed
no business methods, has been removed.

### Realtime API
- [x] WebSocket type definitions
- [x] Session management with strongly typed event/audio streams (`RealtimeClient` / `SessionBuilder`)
- [x] Bearer / JWT dual authentication
- [x] Complete client/server events (`ClientEvent` / `ServerEvent`)
