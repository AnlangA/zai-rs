# Migrating to zai-rs 0.5

This guide covers the breaking changes introduced in zai-rs 0.5.0 and how to
update your code.

## Overview

0.5 is a **breaking release**. The per-request `key`/`url`/`EndpointConfig`/
`HttpClientConfig` fields are removed from every request type. Instead, a single
`ZaiClient`
holds credentials, transport, and validated endpoints. All requests are sent via
`request.send_via(&client)`.

## 1. Create one ZaiClient

**Before (0.4):**
```rust,ignore
let key = std::env::var("ZHIPU_API_KEY")?;
let client = ChatCompletion::new(model, msg, key);
let resp = client.send().await?;
```

**After (0.5):**
```rust,ignore
use zai_rs::client::ZaiClient;
use zai_rs::model::*;

let client = ZaiClient::from_env()?;
let request = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"));
let resp = request.send_via(&client).await?;
```

## 2. Per-request key/config constructors removed

Every `*Request::new(key, ...)` now takes no `key`:

| 0.4 | 0.5 |
|---|---|
| `ChatCompletion::new(model, msg, key)` | `ChatCompletion::new(model, msg)` |
| `FileUploadRequest::new(key, purpose, path)` | `FileUploadRequest::new(purpose, path)` |
| `EmbeddingRequest::new(key, model, input)` | `EmbeddingRequest::new(model, input)` |
| `CreateBatchRequest::new(key, file_id, ep)` | `CreateBatchRequest::new(file_id, ep)` |

## 3. `with_base_url` / `with_endpoint_config` / `with_http_config` removed

These are replaced by `ZaiClient::builder(api_key).endpoint(ApiFamily::PaasV4, base).build()?`
and `HttpTransportConfig`.

## 4. Async chat no longer accepts `stream`

The async-chat endpoint is a task-submission interface. The `AsyncChatCompletion`
type-state (`StreamOn`/`StreamOff`), `enable_stream()`, and `with_stream()` are
removed. Async chat cannot be streamed. Starting in `0.6`, use the regular chat
streaming API:

```rust,ignore
let mut stream = ChatCompletion::new(GLM5_2 {}, TextMessage::user("hello"))
    .enable_stream()
    .stream_via(&client)
    .await?;
while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    // Consume chunk.choices[*].delta here.
}
```

## 5. Agent API rewritten

The 0.4 `/paas/v4/agents` CRUD is removed. The official Agent v1 contract is:

```rust,ignore
use zai_rs::agent::*;

let req = AgentInvokeRequest::<NonStreaming>::builder(AgentId::GeneralTranslation)
    .message(AgentMessage::user("hello"))
    .build()?;
// req carries the body; send via a ZaiClient in a future task.
```

## 6. Feature changes

| 0.4 | 0.5 |
|---|---|
| `tool-validation` enables `jsonschema` | Removed; use `toolkits` |
| (no `toolkits` feature) | `toolkits` enables JSON-Schema validation for local tool execution |
| `rmcp-kits = ["dep:rmcp"]` | `rmcp-kits = ["toolkits", "dep:rmcp"]` |

## 7. SSE streaming

The old `stream_for_each`, `stream_sse_for_each`, and `sse_stream` helpers are
not part of `0.5`. The `0.5.1` low-level `prepare_stream_via` escape hatch is
removed in `0.6` because it exposed credentials. Upgrade to `0.6` and use
`ChatCompletion::enable_stream().stream_via(&client)`, which keeps credentials
inside `ZaiClient` and yields typed `ChatStreamResponse` values. `ChatStream`
provides an inherent `next().await`, so no stream-extension dependency or trait
import is required.

## 8. Realtime protocol

The realtime model and event alignment is a `0.6` change. See the
[0.6 migration guide](MIGRATING-0.6.md) before updating realtime applications.

## 9. Public module paths are semantic

Implementation file names are no longer public API. Import from the capability
module itself:

```rust,ignore
// Before
use zai_rs::model::ocr::request::OcrToolType;
use zai_rs::knowledge::create::CreateKnowledgeRequest;

// After
use zai_rs::model::ocr::OcrToolType;
use zai_rs::knowledge::CreateKnowledgeRequest;
```

The empty `client.services()` facade hierarchy was removed. It exposed names
such as `ChatService` without implementing operations, while real requests used
`send_via`. `ZaiClient` plus `Request::send_via` is now the only HTTP dispatch
model.

## Summary

| Change | Impact |
|---|---|
| `ZaiClient` replaces per-request keys | Breaking — update all constructors |
| `send_via(&client)` replaces `send()` | Breaking — update all send calls |
| `with_base_url` etc. removed | Breaking — use `ZaiClient::builder().endpoint()` |
| Agent API rewritten | Breaking — use AgentInvokeRequest |
| Async chat stream removed | Breaking — use regular chat `stream_via` |
| Realtime model ids updated | Use `GLM_realtime_flash` or `GLM_realtime_air` |
| `toolkits` feature added | Additive — opt-in |
| `data`/`request`/`response` submodules hidden | Breaking — import from the capability module |
| Empty `client.services()` facades removed | Breaking — use `request.send_via(&client)` |
