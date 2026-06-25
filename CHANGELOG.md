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

#### Further optimization pass (non-breaking)
- SSE event parser rewritten from a per-line `drain`+`collect` (O(n²) memmove
  per chunk — the hottest path in the crate, since every streaming token flows
  through it) to an in-place scan with a single trailing drain (O(n) per chunk).
- `parse_typed_response` now deserializes straight from the raw response bytes
  via `serde_json::from_slice` instead of collecting the body to a `String`
  first — one fewer full-body allocation and UTF-8 validation pass per typed
  response.
- `ToolCallCache` key canonicalization gained a fast path: when no JSON object
  key needs whitespace trimming (the common case), the arguments are serialized
  directly instead of deep-cloning and rebuilding the whole tree. Keys are also
  order-independent (sorted), so `{"a":1,"b":2}` and `{"b":2,"a":1}` now collide.
- `ToolExecutor::execute` no longer builds (deep-clone + re-serialize) a cache
  key when caching is disabled.
- Dropped a redundant `Arc::clone` of the HTTP config on every
  POST/GET/PUT/DELETE — the `HttpClient` trait already returns an owned `Arc`.
- **Fixed a panic** in `calculate_retry_delay`: a user-supplied `base` near
  `Duration::MAX` overflowed `base * 2^attempt` *before* the `.min(max)` clamp
  ran. It now uses `checked_mul` and falls back to `max` on overflow. `add_jitter`,
  WAV-header sizing, and the tool-executor backoff delay were switched to
  saturating/checked arithmetic to eliminate silent truncation and overflow.
- Realtime swallowed errors are now observable: a client event that fails to
  serialize and a slow-consumer `Lagged` gap in the event/audio streams now emit
  `warn!` lines instead of being silently dropped.
- Gated `uuid` under the `realtime` feature (only used to mint realtime event
  ids), trimming one crate from the default chat/embeddings build.
- Broad mechanical clippy pass (redundant closures → fn items, unnested
  or-patterns) applied across the crate.
- Added regression tests for the retry loop (end-to-end via a mock server), the
  SSE parser on split/DONE/CRLF boundaries, the business-error-code band edges
  (1306/1307 gap, 1499), cache-key canonicalization, and the cache-disabled
  execution path.

#### Code-hygiene pass (no `unwrap`/`expect`, no redundant qualification)
- **Production code is now free of every panicking form** (`unwrap`/`expect`/
  `panic!`/`unreachable!`/`todo!`/`unimplemented!`). Replacements: the secret-
  masking regex statics resolve via `.ok()`/`filter_map` (`Option<Regex>`) instead
  of `.expect()`; the Beijing-offset helper returns `Option<FixedOffset>`; the
  tool-call semaphore `acquire().expect()` → `.ok()` (held as `Option<Permit>`);
  the schema-cache `RwLock` recovers from poison via `into_inner` instead of
  `.unwrap()`; doc examples use `?`/`unwrap_or` instead of `.unwrap()`. (Test
  code still uses `unwrap`, which is idiomatic there.)
- Removed redundant fully-qualified paths across the crate: where a name is
  imported via `use`, callers now use the short name instead of re-qualifying
  from the root (e.g. `std::sync::Arc` → `Arc`). The one retained full path is
  `std::io::Error`, which must stay qualified to avoid shadowing the
  `thiserror::Error` derive macro in the same module.
- Dropped the unused `serde` feature from the (now `realtime`-gated) `uuid` dep,
  and corrected the stale `bytes` dep comment (it serves the default HTTP
  send/parse path, not just realtime).
- The cache fast-path now documents its reliance on serde_json's default
  (BTreeMap, sorted) key ordering, and `test_cache_collides_on_reordered_keys`
  pins the user-facing cache-hit behavior.

### Planned for a later release (deferred from this pass)
- Typed response enums (`FinishReason`/`Role`) replacing the stringly-typed
  `Option<String>` fields — deferred: it touches the streaming response shape
  and examples; a non-breaking typed-accessor approach is recommended first.
- Hide `reqwest::Response` behind an opaque `ZaiResponse` (decouples the public
  API from the `reqwest` major version) — highest blast-radius change; warrants
  its own pass.
- `ApiKey` newtype (replacing bare `api_key: String` fields) and removal of the
  dead `ZaiConfig.reqwest` field.
- Realtime error surfacing is partially landed: serialize failures and
  `Lagged` gaps now `warn!`. The remaining (optional, opt-in `realtime`-only)
  step is exposing `Lagged` as a stream item / `Result` variant rather than a
  log line.
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
