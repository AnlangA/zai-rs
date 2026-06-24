# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-06-25

Comprehensive optimization pass. **Breaking changes** are concentrated here so
users upgrade once; the rest (performance, build, tooling) ship in the same cut.

### ⚠️ Breaking
- **Default features changed:** `realtime` (WebSocket) and `tool-validation`
  (jsonschema argument validation) are now opt-in and **off by default**. If you
  relied on either, enable them explicitly: `zai-rs = { version = "0.3",
  features = ["realtime", "tool-validation"] }`. The default build is now a
  chat/embeddings-only client (~32% fewer crates).
- **`VisionMessage::add_user` / `VoiceMessage::add_user` → `add_content`.** The
  old name implied "add a user turn" but actually appended rich content, and
  silently *discarded* a `System`/`Assistant` message when called on one. The
  renamed `add_content` only appends to a `User` message and is a no-op
  otherwise (no more silent data loss).
- **`ChatBody::add_tools(tool: Tools)` → `add_tool`** (singular), to match the
  singular semantics and the `ChatCompletion::add_tool` client-layer method.
- **`#[non_exhaustive]`** added to `ZaiError` and `RealtimeErrorKind`, so adding
  error variants in a minor release is no longer a breaking change. Exhaustive
  `match`es on these in downstream code now need a `_` arm.

### Build
- `cargo fmt` config trimmed to stable-only options so the CI formatting check
  is actually enforced (the nightly-only options it used were silently ignored).
- Added `[package.metadata.docs.rs]` so docs.rs builds with all features and
  feature-gated items are badged.
- Expanded the publish `exclude` list to cover nested example build dirs and the
  8 MiB `data/` example media (tarball 9.0 MiB → 1.1 MiB).
- Trimmed `tokio` from `["full"]` to the features actually used
  (`rt-multi-thread`, `macros`, `time`, `fs`, `sync`; `net` only under
  `realtime`). `reqwest` was already rustls-only — no native-tls/quinn in the
  graph, so no change needed there.
- Dropped the direct `parking_lot` dependency (the single `RwLock` is now
  `std::sync::RwLock` behind `tool-validation`).

### Documentation
- Added standard rustdoc to all public API items.
- Documented the model-struct naming convention (echoes vendor strings; carries
  `#[allow(non_camel_case_types)]`).
- Introduced this `CHANGELOG.md`.

### Tooling
- Tightened CI parity checks; replaced `tracing` usage in examples with plain
  `println!`; added a `publish --dry-run` CI job and expanded the MSRV check to
  `--all-targets`.

### Performance / Correctness
- HTTP send path: serialize each body once (was twice), retry on cheap
  `Bytes`/`Arc<str>` handles (was a full-body `String` clone per attempt), and
  return a `Result` instead of panicking the process when `reqwest::Client`
  can't be built.
- `ErrorCategory` + `ZaiError::category()` as the single source of truth for
  client/server/retryable classification; `should_retry` delegates to
  `is_retryable()`.
- `ToolCallCache`: cheap `Arc`-shared clone (no per-call deep copy) and O(1)
  FIFO eviction (was O(n log n) per insert at capacity).
- Tool execution surfaces a panicked/cancelled task as an error message instead
  of silently dropping it, and bounds parallel tool-call concurrency.

### Planned for a later release (deferred from this pass)
- Typed response enums (`FinishReason`/`Role`) replacing the stringly-typed
  `Option<String>` fields — deferred: it touches the streaming response shape
  and examples; a non-breaking typed-accessor approach is recommended first.
- Hide `reqwest::Response` behind an opaque `ZaiResponse` (decouples the public
  API from the `reqwest` major version) — highest blast-radius change; warrants
  its own pass.
- `ApiKey` newtype (replacing bare `api_key: String` fields) and removal of the
  dead `ZaiConfig.reqwest` field.
- Surface swallowed errors in the realtime loop (`Err(_) => continue`, dropped
  `Lagged`) — contained to the now-opt-in `realtime` feature.
- Broader `#[non_exhaustive]` sweep across response structs.

## [0.2.1] - 2026-06-18

- Bumped dependencies to latest patch/minor versions.

## [0.2.0] - 2026-06-18

### Added
- Realtime (WebSocket) API with GLM-Realtime support (`realtime` module).
- GLM-5.2 model and `reasoning_effort` support.
- `usage` module for Coding Plan quota query.

### Changed
- Optimized error-code propagation and tracing observability.
- Tightened library logging to `trace`/`warn` only.
- Refactored transport layer and error handling.
- SSE parsing and streaming fixes; RMCP API updates; clippy fixes.

[Unreleased]: https://github.com/AnlangA/zai-rs/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/AnlangA/zai-rs/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/AnlangA/zai-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/AnlangA/zai-rs/releases/tag/v0.2.0
