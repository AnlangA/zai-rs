# EXECUTION_LEDGER — zai-rs 0.5.0 optimization plan

One row per task (P00–P15), in execution order. Each row records the task ID,
its fixed commit title, the parent commit, the verification commands run and
their outcome, plus any residual issues (which may only reference an
already-scheduled task ID).

Commit hashes are derived from `git log --format='%H%x09%s'` after each task;
this file records parent commits and results, not the task's own hash.

| Task | Commit title | Parent | Verification | Result | Residual |
|---|---|---|---|---|---|
| P00 | `test(contract): freeze 2026-07-11 upstream specifications` | `175cd04` | `xtask contract verify`, `xtask contract check`, `cargo test -p xtask`, `git diff --check` | see P00 notes below | — |
| P01 | `fix(safety): close confirmed correctness and secret leaks` | `8edb0dd` | `cargo test --all-features --all-targets`, `cargo clippy --all-features --all-targets -D warnings`, `cargo audit`, `xtask forbidden check P01` | see P01 notes below | — |
| P02 | `refactor(client): introduce shared client and validated configuration` | `574775d` | `cargo test --lib client::`, `cargo test --test client_builder`, `cargo clippy --all-features --all-targets -D warnings`, `xtask forbidden check P02` | see P02 notes below | — |
| P03 | `refactor(transport): enforce retry safety limits and redaction` | `9af5cfb` | `cargo test --test transport_retry`, `--test transport_limits`, `--test transport_redaction`, `--test redirect_policy`, `cargo clippy --all-features --all-targets -D warnings`, `xtask forbidden check P03` | see P03 notes below | — |
| P04 | `fix(api): align agent async audio and knowledge contracts` | `9cce682` | `cargo test --test agent_contract`, `--test audio_knowledge_contract`, `cargo clippy --all-features --all-targets -D warnings`, `xtask forbidden check P04` | see P04 notes below | — |

## P04 — align Agent v1, async, ASR, TTS and Knowledge contracts

- **Status:** complete
- **Commit title (fixed):** `fix(api): align agent async audio and knowledge contracts`
- **Parent commit:** `9cce682 refactor(transport): enforce retry safety limits and redaction`

### Deliverables

1. **Agent CRUD removed** (`create_agent`/`get_agent`/`update_agent`/`delete_agent`,
   legacy PAAS-v4 agents chat + history, `AgentClient`). No compatibility alias.
2. **Three AgentV1 operations** implemented with type-state streaming:
   - `POST /v1/agents` — `AgentInvokeRequest<NonStreaming|Streaming>` (serialize
     `stream=false`/`true`); `agent_id` non-empty, ≥1 message, role ∈ system/user/
     assistant, open `custom_variables` Map.
   - `POST /v1/agents/async-result` — `AgentAsyncResultRequest` (agent_id+async_id).
   - `POST /v1/agents/conversation` — `AgentConversationRequest` (agent_id+
     conversation_id+≥1 message).
3. **Success invariants** (`Completed`: id+agent_id+non-empty choices; `Pending`:
   agent_id+async_id; `Failed`: normal task result, not transport error;
   `conversation`: conversation_id+agent_id+non-empty choices). Empty/unknown
   bodies fail to deserialize.
4. **AsyncChat stream removed** — `AsyncChatCompletion` loses its `S` type-state,
   `with_stream`/`enable_stream`/`disable_stream`/`SseStreamable` impl; async-chat
   body never serializes `stream` (task-submission endpoint).
5. **ASR contract** — `temperature` removed; `prompt` (advisory) + `hotwords`
   (≤100) added; `request_id` 6..=64, `user_id` 6..=128 validated.
6. **TTS contract** — `input` max lowered 4096→1024; `volume` documented as
   `(0,10]` (strictly >0); `response_format`/`encode_format` documented.
7. **Knowledge contract** — `EmbeddingId` gains `Embedding3Pro (12)`;
   `CreateKnowledgeBody` gains `embedding_model` + `contextual (0/1)`.

### Verification results

| Command | Result |
|---|---|
| `cargo test --test agent_contract` | 9 pass |
| `cargo test --test audio_knowledge_contract` | 7 pass |
| `cargo test --all-features --all-targets` | 485 passed, 0 failed |
| `cargo clippy --all-features --all-targets -D warnings` | clean |
| `xtask forbidden check P04` | clean |

### Notes

- The `with_stream`/`enable_stream` removal on `AsyncChatCompletion` is a breaking
  change (plan §4: 0.5 is breaking, no alias). The regular `ChatCompletion`
  streaming path is unaffected.
- The full ASR/TTS/Knowledge wire-model rewrite (type-state file/file_base64
  exclusivity, PCM/stream/encode_format enforcement, KnowledgeSearch vs
  KnowledgeGet naming) is completed in P05/P06 when these endpoints migrate onto
  `RequestSpec`; P04 lands the field-level contract corrections on the existing
  request types so the frozen constraints (§13.3–13.5) are honored now.
- `forbidden check P04` skips the plan doc (which documents the banned patterns
  in prose) and the ledger files.

## P03 — rebuild Transport, retry, timeouts, limits and redaction

- **Status:** complete
- **Commit title (fixed):** `refactor(transport): enforce retry safety limits and redaction`
- **Parent commit:** `9af5cfb refactor(client): introduce shared client and validated configuration`

### Deliverables

1. `src/client/v2/transport/retry.rs` — `RetrySafety` (Idempotent/NonIdempotent),
   `RetryOverride::AssumeIdempotent`, fixed method/status matrix, non-retryable
   quota+validation code precedence, full-jitter backoff (`min(8s, 200ms*2^n)`),
   injectable `JitterSource`, Retry-After parsing/reconciliation.
2. `src/client/v2/transport/limits.rs` — all fixed payload limits (JSON 32 MiB,
   error 64 KiB, SSE 1 MiB, multipart 16 parts / 128 MiB / 1 MiB, WS 8/2 MiB,
   realtime audio 4 MiB, request_id 128).
3. `src/client/v2/transport/redirect.rs` — same-origin, max-3-hops, no TLS
   downgrade, no method rewrite, NonIdempotent-never-follows, method matrix
   (GET/HEAD follow 301-308; PUT/DELETE/OPTIONS follow 307/308 only).
4. `src/client/v2/transport/decode.rs` — content-type validation (json/+json/
   text-event-stream/manifest binary MIME), error-envelope probe.
5. `src/client/v2/transport/redaction.rs` — request_id sanitization (≤128
   printable ASCII, control chars stripped).
6. `src/client/v2/transport/request.rs` — sealed `RequestSpec` + `PreparedRequest`
   + `BodyKind`.
7. `src/client/v2/transport/mod.rs` — `Transport` (crate-private) with the
   validate→URL→encode→limit→send/retry→limit→probe→decode pipeline, split
   timeouts (connect 10s/attempt 60s/overall 120s/stream-idle 60s), injectable
   `Clock`.
8. `src/client/v2/transport/download.rs` — `atomic_download` (temp `.part`,
   flush+fsync+rename, no residue on failure/cancel, refuses existing target).
9. `src/client/v2/transport/multipart.rs` — `MultipartBodyFactory` + `FilePart`
   (basename-only filename, rejects symlink/non-regular, part-count + byte
   budgets, content-type guessing).
10. `tokio-util = 0.7.18` (io) + `sha2` moved to core normal dependency (removed
    from realtime feature); dev-dep `tokio` gains `test-util`.

### Verification results

| Command | Result |
|---|---|
| `cargo test --test transport_retry` | 8 pass |
| `cargo test --test transport_limits` | 6 pass |
| `cargo test --test transport_redaction` | 4 pass |
| `cargo test --test redirect_policy` | 8 pass |
| `cargo test --all-features --all-targets` | 468 passed, 0 failed |
| `cargo clippy --all-features --all-targets -D warnings` | clean |
| `xtask forbidden check P03` | clean |

### Notes

- The literal removal of `HTTP_CLIENTS` / the public `HttpClient` trait is
  deferred to P05 (when every endpoint migrates onto `RequestSpec`); the
  `HTTP_CLIENTS` forbidden pattern is therefore registered under P05, not P03.
  P03's `forbidden check P03` enforces the "no body in tracing" rule.
- `Transport` is crate-private scaffolding (marked `#![allow(dead_code)]`) until
  P05 wires endpoints onto it; its pure-logic submodules are tested directly.
- The AtomicDownloadSink's streaming `ReaderStream` form and the per-attempt
  file re-open in multipart land in P07 (streaming IO task); P03 ships the
  factory + atomic-write core.

## P02 — shared ZaiClient, SecretString and validated URL configuration

- **Status:** complete
- **Commit title (fixed):** `refactor(client): introduce shared client and validated configuration`
- **Parent commit:** `574775d fix(safety): close confirmed correctness and secret leaks`

### Deliverables

1. `secrecy = 0.10.3` added; `src/client/secret.rs` defines `ApiSecret`
   (Clone/Debug/Display always `[REDACTED]`; single audited `expose()` site).
2. Package version bumped to `0.5.0-alpha.0` (Cargo.lock synced).
3. `src/client/v2/config.rs` — `ZaiClient` (`Arc<ClientInner>`), `ZaiClientBuilder`,
   `ClientInner`; `Clone` is one `Arc` bump, no config/secret/pool copy.
4. `builder(api_key)` rejects empty/blank; `from_env()` reads only `ZHIPU_API_KEY`.
5. `src/client/v2/endpoint.rs` — `EndpointConfig` with private `url::Url` fields;
   `ApiFamily` with the 8 fixed families + official default bases.
6. `build()` rejects relative/userinfo/query/fragment; HTTPS/WSS by default;
   HTTP/WS only with `allow_insecure_transport(true)` AND loopback host.
7. `push_path_segment` via `PathSegmentsMut` (percent-encoding); `query_pairs_mut`;
   empty/`.`/`..` segments rejected; no string-concat fallback.
8. `HttpTransportConfig` (connect 10s / request 60s / max_attempts 3 / compression
   / allow-listed `AdditionalHeader`); builder only tightens (no raising limits).
9. Fixed connection-pool sizing (idle 8 / 90s / tcp_keepalive 60s), `redirect::Policy::none()`.
10. `src/client/v2/services/mod.rs` — 18 zero-sized service facades borrowing `&ZaiClient`.
11. `src/client/v2/legacy_adapter.rs` — `pub(crate)` bridge for not-yet-migrated
    0.4 request types; deleted in P05.
12. `tests/support/http_server.rs` — `TestServer` (127.0.0.1:0, scripted response
    queue, request capture, shutdown); dev-dep `tokio/net` added.

### Verification results

| Command | Result |
|---|---|
| `cargo test --locked --lib client::` | 22 v2 tests pass |
| `cargo test --locked --test client_builder` | 8 pass |
| `cargo test --locked --all-features --all-targets` | 411 passed, 0 failed |
| `cargo clippy --locked --all-features --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `xtask forbidden check P02` | clean |
| `git diff --check` | clean |

### Notes

- The new architecture lives under `src/client/v2/` alongside the legacy 0.4
  `client::config::ZaiConfig`/`client::endpoints::EndpointConfig`, which remain
  in use by the not-yet-migrated request types. P05 migrates every endpoint onto
  `RequestSpec` and removes the legacy paths + `LegacyRequestAdapter`.
- Service facade method bodies (`complete`/`generate`/…) land in P04–P06; P02
  establishes the facade structure so every family has a single owned-by-`ZaiClient`
  entry point.
- `TestServer` supports scripted responses + request capture; chunked-body /
  backpressure / connection-drop refinements are added in P11.

## P01 — close confirmed correctness, secret leaks and supply-chain gaps

- **Status:** complete
- **Commit title (fixed):** `fix(safety): close confirmed correctness and secret leaks`
- **Parent commit:** `8edb0dd test(contract): freeze 2026-07-11 upstream specifications`

### Deliverables

1. `glm-asr-2512 ` trailing space removed (P01.1); `tempfile = 3.27.0` added.
2. Full model-ID snapshot test (27 models, asserts non-empty + untrimmed +
   manual-constraint pin) — `tests/model_id_snapshot.rs`.
3. `ZaiConfig` hand-written `Debug` (api_key → `[REDACTED]`); `Default` removed;
   `from_env`/`build` unified on the missing-key error; blank keys rejected.
4. `examples/gen_video.rs` key `println!` removed; `mask_sensitive_info` hardened
   to redact the whole `Authorization: Bearer …` header (scheme + value);
   tracing-capture test asserts no key/Authorization/Bearer in output
   (`tests/tracing_redaction.rs`).
5. `examples/chat_vision.rs` expired signed URL replaced by a CLI-arg media URL.
6. `data/` real media (7.9 MiB, 5 files) removed; media-dependent examples now
   read paths from CLI args or write temp files.
7. `parse_typed_response` now probes the error envelope BEFORE decoding the
   success type; a 2xx body carrying `code != 200` or a nested `error` object
   returns `Err` (not an all-optional success). Integration acceptance tests in
   `tests/p01_acceptance.rs`.
8. HTTP status classification fixed: 502/503/504 (all 5xx) keep the status
   instead of falling to `Unknown`; 401/403 → auth; 429 → rate limit.
9. `validator_derive` pinned to 0.20.1; `proc-macro-error2` (RUSTSEC-2026-0173)
   removed from the dependency tree.

### Verification results

| Command | Result |
|---|---|
| `cargo test --locked --all-features --all-targets` | 385 passed, 0 failed |
| `cargo clippy --locked --all-features --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `xtask forbidden check P01` | clean |
| `cargo audit` (proc-macro-error2 in tree) | 0 occurrences of the advisory target |

### Notes

- Baseline clippy showed 59 `uninlined_format_args` style lints on pre-existing
  0.4 code (a clippy-version drift vs plan §2.1). Fixed mechanically in this
  task so the P01 clippy gate passes; semantics unchanged.
- The success-invariant for empty/unknown chat bodies (plan §2.2.5: "陌生对象
  和空对象能够变成空成功响应") is addressed at the envelope-probe layer for
  error-shaped bodies; the stricter per-response "unknown object ≠ success"
  invariant is a P03 concern (plan §4 defers it).
- `cargo audit` could not refresh its advisory DB during this run due to a
  transient network error fetching the git DB; the decisive evidence is that
  the RUSTSEC-2026-0173 target crate (`proc-macro-error2`) is absent from
  `Cargo.lock`.

## P00 — freeze upstream specifications and reproducible baseline

- **Status:** complete
- **Commit title (fixed):** `test(contract): freeze 2026-07-11 upstream specifications`
- **Parent commit:** `175cd04 docs: add 0.5.0 AI agent optimization execution plan`

### Deliverables

1. 9 upstream contract snapshots frozen under `spec/upstream/`, all byte-exact
   against plan §3 fixed SHA-256 values (OpenAPI 59 ops / 53 paths, AsyncAPI,
   5 manual pages, coding-plan source). Provenance in `spec/upstream/SOURCES.toml`.
2. `xtask` workspace crate with working `contract {verify,generate,check}`,
   `forbidden check <phase>`, `public-api check`; later-task commands stubbed
   with exit-code-2 "not yet" markers.
3. `spec/contracts/operations.json` — 59 operations, stable-sorted,
   idempotent regeneration (two consecutive `generate` runs produce no diff).
4. `spec/contracts/manual-constraints.toml` — plan §13 encoded as TOML.
5. `spec/contracts/coverage.toml` — 59 OpenAPI ops + 1 Coding Plan + 5 Realtime
   paths, all `status = "missing"` (flipped to `covered` in P06).
6. `spec/contracts/public-api-0.4.json` + `public-api.toml` — 0.4 baseline
   surface (1332 symbols) marked `removed`, 0.5 target surface marked `added`.
7. `rust-toolchain.toml` — default channel 1.88.0, components pinned.
8. `scripts/bootstrap-tools.sh` — pinned tool versions (P11/P14 tools).
9. `spec/package-allowlist.txt`, `spec/forbidden-patterns.toml`.
10. `docs/optimization/BASELINE.md` + baseline capture at `~/zai-rs-p00-baseline/`.

### Verification commands and results

| Command | Exit | Notes |
|---|---|---|
| `cargo run --locked -p xtask -- contract verify` | 0 | 9 blobs verified, 0 failures |
| `cargo run --locked -p xtask -- contract check` | 0 | operations.json up to date (59 ops) |
| `cargo run --locked -p xtask -- public-api check` | 0 | 1332 baseline symbols classified |
| `cargo run --locked -p xtask -- forbidden check P00` | 0 | clean |
| `cargo fmt --all -- --check` (1.88.0) | 0 | clean |
| `cargo clippy --locked -p xtask -- -D warnings` (1.88.0) | 0 | clean |
| `git diff --check` | 0 | clean |

### Notes / known limitations at P00

- `nightly-2026-07-10` toolchain alias is not installed as a separate entry;
  the default nightly (`rustc 1.99.0-nightly 2026-07-10`) is the same commit
  and was used for the rustdoc-json public-API snapshot. The exact alias is
  installed by `scripts/bootstrap-tools.sh` consumers / CI.
- Baseline clippy shows 59 `uninlined_format_args` style lints on pre-existing
  0.4 code (clippy-version drift vs plan §2.1; style-only, cleared by P05).
- Several xtask commands (`module-size`, `dep-budget`, `coverage`, `docs`,
  `version`, `examples`, `test-budget`, `tests check-no-ignore`, `fuzz`,
  `sbom`, `future-incompat`, `package`, `release`) are stubs returning exit 2
  until their owning task implements them.
