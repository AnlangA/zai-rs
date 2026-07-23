# Migrating to zai-rs 0.6

This guide covers the breaking changes introduced in zai-rs 0.6.0 (from 0.5.x)
and how to update your code.

## Overview

0.6 is a **breaking release**. The headline changes are a reworked chat
builder/streaming API, a strict realtime model contract, and a cleaned-up
built-in model table. Transport (`ZaiClient` + `request.send_via(&client)`),
the error hierarchy, and the module layout from 0.5 are unchanged.

## 1. Chat builder API reworked

`ChatCompletion` (and the other chat request builders) were streamlined
(`src/model/chat/data.rs`):

| 0.5 | 0.6 |
|---|---|
| `add_messages(messages)` | `extend_messages(messages)` — accepts any `IntoIterator` |
| `add_tool(tool: Tools)` | `add_tool(tool: N::Tool)` — tool type is now per-model |
| `add_tools(Vec<Tools>)` | `add_tools(impl IntoIterator<Item = N::Tool>)` |
| `body()` / `body_mut()` | Removed — use the builder methods |
| `with_stop(String)` | `with_stop(impl Into<String>)` |

New builder methods: `clear_tools()` and `with_watermark_enabled(bool)`.

**Before (0.5):**
```rust,ignore
let request = ChatCompletion::new(GLM4_6 {}, TextMessage::user("hi"))
    .add_messages(history)
    .add_tools(vec![tool_a, tool_b]);
```

**After (0.6):**
```rust,ignore
let request = ChatCompletion::new(GLM4_6 {}, TextMessage::user("hi"))
    .extend_messages(history)
    .add_tools([tool_a, tool_b]);
```

## 2. Streaming: `prepare_stream_via` → `stream_via`

Type-state streaming remains (`enable_stream()`), but the terminal call changed
and now returns a dedicated [`ChatStream`]:

```rust,ignore
use futures_util::StreamExt;

let mut stream = ChatCompletion::new(GLM4_6 {}, TextMessage::user("hi"))
    .enable_stream()
    .stream_via(&client)
    .await?;

while let Some(chunk) = stream.next().await {
    let chunk = chunk?;
    // ...
}
```

`ChatStream` implements `futures_core::Stream` and also provides an inherent
`next()` method, so you can consume it with or without `StreamExt`. The old
`chat_stream` and `tool_stream_min` examples were removed; the same pattern is
used by the type-safe ASR/TTS streams (see `examples/audio_to_text.rs` and
`examples/text_to_audio.rs`).

## 3. Realtime: strict model contract

`RealtimeClient::session` now accepts **only** the two dedicated realtime
models through a sealed model trait; anything else is rejected at compile
time:

| Model | Struct |
|---|---|
| glm-realtime-flash | `GLM_realtime_flash` |
| glm-realtime-air | `GLM_realtime_air` |

**Breaking:** `GLM4_voice` is no longer accepted by `RealtimeClient::session`
(it is an HTTP voice-chat model — use it with the regular chat API). The old
realtime marker types that exposed no usable operations were deleted.

The realtime protocol surface (`ClientEvent` / `ServerEvent`, session audio
streams, event ids) was also heavily expanded; if you built directly on 0.5
realtime internals, expect to re-read `zai_rs::realtime`.

**Before (0.5):**
```rust,ignore
let session = RealtimeClient::new(key)
    .session(GLM4_voice {})
    .build()
    .await?;
```

**After (0.6):**
```rust,ignore
use zai_rs::model::GLM_realtime_flash;

let session = RealtimeClient::new(key)
    .session(GLM_realtime_flash {})
    .build()
    .await?;
```

## 4. Built-in model table updated

Removed chat/vision model markers (upstream deprecated them):

- `GLM4_5` (glm-4.5)
- `GLM4_5_x` (glm-4.5-X)
- `GLM4_5v` (glm-4.5v)

Capability corrections:

- `GLM4_7_flash` / `GLM4_7_flashx` no longer carry the async-task marker
  (the upstream async endpoint does not support them).

Added model markers:

| Model | Struct |
|---|---|
| glm-4-flash-250414 | `GLM4_flash_250414` |
| glm-4-flashx-250414 | `GLM4_flashx_250414` |
| glm-4v-flash | `GLM4v_flash` |
| glm-4.1v-thinking-flash | `GLM4_1v_thinking_flash` |
| glm-4.1v-thinking-flashx | `GLM4_1v_thinking_flashx` |
| glm-asr-2512 | `GlmAsr` |
| glm-tts | `GlmTts` |
| glm-image | `GlmImage` |
| cogview-4-250304 / cogview-4 / cogview-3-flash | `CogView4_250304` / `CogView4` / `CogView3Flash` |

## 5. Dependency footprint changes

These do not change zai-rs API, but can affect downstream builds that
transitively relied on zai-rs dependencies:

- `reqwest` is now built with `default-features = false` + `rustls`, HTTP/2,
  `system-proxy`, gzip, multipart, and streaming. The SDK performs its own
  JSON decode; the reqwest `json` feature and native-tls are no longer pulled
  in. If your application depended on reqwest's default TLS or `json` feature
  through zai-rs, declare reqwest (or your own HTTP client) directly.
- `tokio` from zai-rs no longer enables `rt-multi-thread` (dev-only now).
- `tokio-stream` was dropped from the `realtime` feature; `sha2` is now
  optional and only pulled in by `realtime`.
- `chrono` is built with `default-features = false` (`clock` + `std` only).

## 6. Feature flags

Feature names are unchanged (`default`, `toolkits`, `rmcp-kits`, `mcp`,
`realtime`). One clarification: the `toolkits` module itself is always
compiled; the `toolkits` feature only enables the full JSON-Schema argument
validation (via `jsonschema`).

## Summary

| Change | Impact |
|---|---|
| `add_messages` → `extend_messages`, per-model tool types | Breaking — update chat builders |
| `body()`/`body_mut()` removed | Breaking — use builder methods |
| `prepare_stream_via` → `stream_via` returning `ChatStream` | Breaking — update streaming calls |
| Realtime session restricted to `GLM_realtime_flash`/`GLM_realtime_air` | Breaking — `GLM4_voice` rejected at compile time |
| `GLM4_5`, `GLM4_5_x`, `GLM4_5v` removed | Breaking — pick a current model |
| `GLM4_7_flash(x)` async marker removed | Breaking — async task API rejects them |
| reqwest rustls/no-default-features, tokio `rt-multi-thread` dev-only | Build-level — declare your own deps if needed |
| Typed SSE for ASR/TTS, new voice/image/vision models, expanded services | Additive |
