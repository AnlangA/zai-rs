# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Comprehensive optimization pass in progress. See the project plan for the full
breakdown; highlights:

### Build
- `cargo fmt` config trimmed to stable-only options so the CI formatting check
  is actually enforced (the nightly-only options it used were silently ignored).
- Added `[package.metadata.docs.rs]` so docs.rs builds with all features and
  feature-gated items are badged.
- Expanded the publish `exclude` list to cover nested example build dirs and the
  8 MiB `data/` example media (tarball 9.0 MiB → 1.1 MiB).
- **Default build slimmed ~32% (210 vs 307 crates):** `realtime` (WebSocket)
  and `tool-validation` (jsonschema argument validation) are now opt-in Cargo
  features, both off by default. Trimming `tokio` from `["full"]` to the
  features actually used. (`reqwest` was already rustls-only — no native-tls /
  quinn in the build graph.)
- Dropped the direct `parking_lot` dependency (the single `RwLock` is now
  `std::sync::RwLock` behind the `tool-validation` feature).

### Documentation
- Added standard rustdoc to all public API items.
- Introduced this `CHANGELOG.md`.

### Tooling
- Tightened CI parity checks; replaced `tracing` usage in examples with plain
  `println!`.

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

### Pending (breaking, targeted at `0.3.0`)
- Typed response enums (`FinishReason`/`Role`), `#[non_exhaustive]` across the
  public API, hide `reqwest::Response` behind an opaque type, `ApiKey` newtype,
  rename the destructive `add_user` method.

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

[Unreleased]: https://github.com/AnlangA/zai-rs/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/AnlangA/zai-rs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/AnlangA/zai-rs/releases/tag/v0.2.0
