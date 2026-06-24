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
- Expanded the publish `exclude` list to cover nested example build dirs.

### Documentation
- Added standard rustdoc to all public API items.
- Introduced this `CHANGELOG.md`.

### Tooling
- Tightened CI parity checks; replaced `tracing` usage in examples with plain
  `println!`.

### In progress (later in this pass)
- Slim the default dependency set via gated `realtime` / `tool-validation`
  features and trimmed `tokio`/`reqwest` features.
- Remove double serialization and per-retry body clones on the HTTP send path;
  O(1) tool-result cache eviction.
- Surface swallowed errors in the realtime loop; return `Result` instead of
  panicking on HTTP client build failure.
- API hardening (targeted at a breaking `0.3.0`): typed response enums,
  `#[non_exhaustive]`, hidden `reqwest::Response`, `ApiKey` newtype.

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
