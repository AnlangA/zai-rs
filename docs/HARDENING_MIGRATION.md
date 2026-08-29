# Hardening migration notes

## Release status

The version `6.2.0` declared by `Cargo.toml` is the current release, prepared
on 2026-08-29. The previous `6.1.0` release carries the hardening and GLM-5.3
support documented below. Its `v6.1.0` tag workflow (run
`31818272628`) did not publish on its first attempts: one attempt hit the
macOS proxy-isolation test race, and the next passed quality, packaging, SBOM,
checksum, and attestation before crates.io rejected OIDC authentication
because no matching Trusted Publisher was configured. `6.1.0` was subsequently
published outside that tag workflow from main commit `376c2c1`; its registry
archive therefore does not match the workflow's attested tag artifact. The
failed attempts and immutable tag remain part of the release history.

The earlier `v6.0.1` workflow runs (`30091854390` and `30092565276`) also
remain historical evidence. The recovered step logs show that the first run's
then-current tag was not annotated. The second passed quality, packaging,
SBOM, checksum, and attestation, but failed at the same missing Trusted
Publisher boundary. `6.0.1` was never published and its tag must not be moved
or reused.

`6.2.0` adds GLM-5.3-Flash multimodal support on top of the published `6.1.0`
API.

## Additions after published 6.1.0 (ship in 6.2.0)

### GLM-5.3-Flash multimodal support

The new `GLM5_3_flash` model type (wire id `glm-5.3-flash`) supports text,
image, video, and file input through `VisionMessage`. It exposes synchronous
and asynchronous chat, function calling, structured output, and streamed tool
calls. Like `GLM5_3`, thinking cannot be disabled and `reasoning_effort`
accepts only `low`, `high`, and `max`; request validation enforces those
constraints before network I/O. File input cannot be mixed with image or video
input in one request.

The change is additive, so existing `6.1.0` consumers need no source changes.
Code adopting `GLM5_3_flash` must use `VisionMessage` even for a text-only
prompt and should use `enabled` plus `reasoning_effort = "low"` when it needs
the lightest available reasoning mode.

## Hardening and features added after the v6.0.1 candidate tag (released in 6.1.0)

The changes in this section were published in `6.1.0`; they were not part of
the existing `v6.0.1` candidate tag.

### GLM-5.3 model support

The new `GLM5_3` model type (wire id `glm-5.3`) is a text model with a 1M
context window and 128K max output. Unlike earlier GLM-5 releases it always
thinks, so request validation rejects `thinking.type = "disabled"` with
`thinking_cannot_be_disabled`, and its `reasoning_effort` is frozen to
`low` / `high` / `max` — other levels fail validation with
`reasoning_effort_not_supported`. Both checks run before any network I/O and
only affect `GLM5_3` requests; migrate legacy disabled-thinking requests to
`enabled` plus `reasoning_effort = "low"`. The per-model contract is exposed
through the new `ChatRequestModel::THINKING_DISABLE_SUPPORTED` and
`ChatRequestModel::REASONING_EFFORTS` associated constants, recorded in the
frozen chat-model registry snapshot.

### Loopback HTTP endpoints bypass system proxies

HTTP endpoints whose parsed host is `localhost`, an IPv4 loopback address, or
an IPv6 loopback address now use a dedicated `no_proxy` connection pool. This
keeps an explicitly local trust boundary local even when the application sets
`HTTP_PROXY` or `ALL_PROXY` without a matching `NO_PROXY` entry, and prevents a
proxy from receiving the SDK Authorization header.

Applications that intentionally routed loopback test endpoints through a
system proxy must instead point the SDK at the proxy explicitly as the endpoint
or change their test topology. Public HTTPS endpoints continue to use the
system-proxy-aware pool, including in a client that also has local endpoints.

### Non-2xx diagnostic bodies no longer replace HTTP semantics

The transport now retains at most 64 KiB from a non-2xx body for diagnostics.
If an error page is oversized, stalls, or disconnects after response headers,
the SDK preserves the HTTP status rather than replacing it with an SDK body
size, timeout, or network error. Consequently an idempotent 503 can now retry,
and final 401/429/5xx errors keep their HTTP category.

Only a diagnostic body that reaches EOF before the cap and deadline is parsed
as a business-error envelope. A capped, timed-out, or disconnected prefix is
not a complete JSON document: its body-derived code, request ID, and message are ignored,
retry/classification use only the HTTP status, and the returned error carries a
static payload-free message. Applications whose metrics or control flow relied
on the previous SDK-validation error or on a business code recovered from an
incomplete error page should migrate to `category()`, `code()`, and
`is_retryable()`.

### HTTP operations now have shared admission control

Every `ZaiClient` now admits at most 64 logical HTTP operations by default;
all of its cheap clones share that budget. A buffered request retains one
permit across every attempt, backoff, bounded body read, and response decode.
An SSE or file response retains its permit until the stream terminates, is
dropped, or its safety lease expires, so long-lived consumers can no longer
create unbounded active response bodies through client cloning. Terminal SSE
parser/decoder errors drop the raw response before yielding the error, even if
the caller never polls again.

An established SSE response has a five-minute configured stream-consumer base
by default. A scoped override selects `base = min(scoped, global)`; the
transport then derives `effective = max(base, sse_idle + 1s)`. Both base setters
accept `1ns..=24h`, while the effective interval can reach 24 hours plus one
second. The idle floor can therefore make a smaller scoped base a no-op. The
lease renews only when an underlying raw-stream poll actually yields a chunk; a
typed decoder can sometimes produce an item from an already-buffered raw chunk
without renewing it. The floor keeps server silence (`SseIdle`) distinct from
a raw stream that stops being advanced (`StreamConsumer`). File streams instead
reuse their existing absolute overall deadline (`Overall`). Expiry atomically
takes and drops the entire raw response body and its permit, wakes queued
admission, and leaves the retained public stream ready to yield exactly one
timeout before ending. Caller Drop still performs the same reclamation
immediately. This is observable hardening for applications that previously
kept an unpolled stream alive indefinitely.

When all permits are occupied, a new operation waits up to 30 seconds by
default. Exhausting that queue deadline returns request metadata with zero HTTP
attempts and `TimeoutPhase::Queue`; queue waiting is separate from the attempt
and overall deadlines. This can be observable in workloads that previously
started more than 64 operations at once. Use `HttpConcurrencyConfig` on
`ZaiClientBuilder` to choose an explicit limit (`1..=4096`) and queue deadline
(`0s..=24h`); zero provides fail-fast admission. A scoped
`RequestOptions::with_queue_timeout` may shorten, but never extend, that queue
deadline. The same policy exposes a configurable SSE consumer base
(`1ns..=24h`); a scoped `with_stream_consumer_timeout` can lower that base, but
the `sse_idle + 1s` floor may make the override a no-op and is applied after the
base minimum.

### Voice-list query serialization matches the wire schema

`VoiceListQuery`'s public `Serialize` implementation now emits the upstream
`voiceName` and `voiceType` names. The SDK's own `send_via` path already used
those canonical names, but generic serializers previously produced incorrect
`voice_name` / `voice_type` keys. Code that intentionally depended on that
incorrect serialized shape must update to the camel-case wire names.

### Atomic downloads have a no-clobber fallback and explicit durability limits

`FileContentRequest::send_to_via` still refuses to replace an existing target,
including when another writer wins after the initial existence check. It writes
and file-syncs a private same-directory partial before making the destination
visible. A hard link remains the preferred publication operation. When the
filesystem explicitly reports hard links as unsupported, the SDK now uses a
platform-aware no-clobber persistence fallback; other hard-link failures remain
errors. The fallback preserves the no-overwrite guarantee, although failure to
remove its old private name can leave a `.part` entry behind.

The complete-file operation now accepts only `200 OK` without a
`Content-Range` header. An unsolicited `206 Partial Content`, `204 No Content`,
or ranged-looking `200` fails before any bytes become visible and never
publishes a destination. A genuine `200` with an empty body remains a valid
empty file. An unranged `200` JSON business-error envelope is still decoded
after the header integrity check, so its business code and retry policy are not
hidden by the file MIME contract. Every other 2xx status, and any response with
`Content-Range`, fails from headers before its body is polled. Applications that
intentionally need range downloads must use a range-aware endpoint rather than
treating this complete-object API as one.

An SSE handshake likewise succeeds only for `200 OK` without a
`Content-Range` header and with the existing `text/event-stream` MIME check. A
terminal marker inside an unsolicited `206`, a `204 No Content`, or a
ranged-looking `200` can no longer establish a stream and turn an incomplete
response into typed success. Invalid 2xx handshake responses fail from status
and headers before their body is polled, so a slow or malicious body cannot
delay or override the protocol error. Complete non-2xx responses retain
bounded business-error projection, sensitive header-value redaction, request
metadata, and `Retry-After` handling; incomplete diagnostics use status only.
Streaming POST requests remain non-replayable.

Buffered JSON and binary operations now enforce the success statuses frozen in
the route contract instead of accepting every `2xx` response. The current
contract declares `200 OK` for all such operations; an unexpected
`201`/`202`/`204`/`206`/other `2xx`, or a `Content-Range` header on a complete
response, fails from status and headers before the body is polled and is never
retried. This prevents a partial binary or a syntactically valid response under
an undocumented status from becoming typed success.

Redirect policy failures are now preserved as the original static
`SDK_VALIDATION` diagnostic. Cross-origin targets, TLS downgrades, malformed
locations, and hop-limit violations are not downgraded to a generic `3xx`
error, are never replayed, and never include the rejected `Location` value in
the diagnostic. When a permitted redirect is followed but its target fails
before returning headers, request metadata retains the most recent redirect's
bounded request ID and valid `Retry-After` instead of stale data from an older
retry response.

### Ambiguous and malformed business-error diagnostics fail closed

The transport now distinguishes a clean JSON response, a recognized business
error, an ambiguous envelope containing duplicate reserved fields, and
malformed JSON. A
duplicate top-level `error`, `code`, `message`, or `request_id`, or duplicate
`code`/`message` directly inside `error`, can no longer fall through to a typed
success response that ignores those fields. An ambiguous HTTP 2xx returns a static protocol
error; non-2xx retry and classification use only the HTTP status, never one of
the conflicting values. The diagnostic does not include the response body.
Malformed complete non-2xx JSON follows the same status-only rule; valid 2xx
payloads still proceed to the endpoint's typed decoder. Provider-controlled
composite `code` values are consumed without building a full JSON tree;
oversized numeric/string literals are replaced by fixed non-numeric sentinels,
and excessive nesting is rejected before Serde scratch can grow with depth.
This prevents the bounded response body from expanding into an unbounded
diagnostic object.
The legacy envelope projection remains frozen as an internal test oracle; the
stricter probe is the production transport boundary.

Relative destinations are converted to one lexical absolute path before parent
creation or network I/O. A process-wide working-directory change during a long
download therefore cannot redirect later stages, although this does not
canonicalize or pin symlinks.

Before creating parent directories, Unix builds now capture the destination's
lexical parent chain through the first directory that already exists. After
publication they fsync the immediate parent and each newly created ancestor up
through that anchor, deepest first. With a stable namespace, a successful
return therefore covers the completed file, its destination entry, and every
directory entry introduced by this operation. This does not canonicalize or
pin symlinks, and another process replacing path components concurrently is
outside the durability contract. If any directory-chain sync fails, the call
returns `SDK_IO` in a published-but-durability-unconfirmed state. Publication
is not rolled back: the complete destination already exists and should be
inspected rather than blindly retried.

Stable Rust does not expose a portable documented directory-sync contract on
Windows or other non-Unix targets. Those builds still file-sync before
no-clobber publication, but a successful return does not promise that the new
directory entry survives a sudden power loss.

Cancellation can race with a publication operation already dispatched to the
blocking filesystem pool. The destination may therefore appear after the
future is cancelled; when present it contains the complete synced file and was
not created by replacing an existing path. Callers should reconcile the target
instead of deleting or blindly retrying it, while the SDK makes a best-effort
attempt to clean the private `.part` name.

Partial-file Drop cleanup no longer sends every filesystem removal through the
async runtime worker. After closing the SDK-owned file handle, it may defer the
removal to Tokio's blocking pool. A process-wide budget permits at most eight
queued or running deferred cleanup jobs. When that budget is full, no runtime is
active, or an armed path guard cannot be created, Drop attempts the removal
synchronously rather than creating an unbounded backlog. Queued-task shutdown
and scheduling failure retain guarded cleanup ownership and release the budget.
All of these paths remain best-effort: cleanup errors are not returned from
Drop, and the SDK deliberately performs no startup directory scan or scavenging.
Applications that require residue recovery should reconcile their own download
directory and its private `.zai-dl-*.part` names under an application-specific
retention policy.

### Public SSE parsing is bounded

`SseEventParser::try_push` and `try_finish` are the new fallible APIs. They use
the same per-event limits as production streams: 32 MiB and 4096 `data:` lines.
On a violation they return `ZaiError` and release all parser-owned buffers.
Large but valid comment or unknown-field lines also release oversized scratch
capacity once consumed, rather than pinning that allocation for the remaining
lifetime of a long-lived stream.

The historical `push` and `finish` methods keep their `Vec` return types but
now enforce those limits too. Because they cannot report an error, a violation
resets the parser and returns an empty vector; completed events accumulated in
that same call are discarded. Callers that need to distinguish an incomplete
event from rejected input must migrate to the `try_*` methods. This is an
intentional resource-exhaustion hardening and an observable behavior change for
previously accepted oversized events.

### Chat streams tolerate additive tool-call kinds

The optional `type` field of a streaming tool call still decodes the documented
`"function"` value as `StreamToolCallType::Function`. A future string value now
decodes as `None` while preserving the call id, index, and incremental function
payload, so one additive discriminator cannot terminate the SSE stream and hide
later text or `[DONE]`. Missing and `null` values remain `None`; non-string
values are still malformed and fail deserialization. Because the unknown raw
string is not retained, applications that need to interpret a new tool kind
must upgrade the SDK rather than treating `None` as a known kind.

### Optional response markers tolerate additive strings

Optional discriminator fields on batch lists, file lists/objects/deletes,
embedding responses/items, web-search intents, and image content-filter stages
now treat an unknown future string as `None` while preserving the remaining
documented payload, including a generated image URL and filter severity. Known
values, missing fields, and `null` keep their previous meanings; numbers,
objects, arrays, and other malformed wire types still fail deserialization.

This tolerance is intentionally field-specific. Direct deserialization of the
public marker enums remains strict, and their serialization is unchanged. The
existing non-empty response invariants also remain in force: a top-level
response whose only non-null field is an unknown marker is still rejected,
while a response carrying a documented data or intent container remains
usable. Because the unknown raw string is not retained, applications that need
to interpret a new marker must upgrade the SDK rather than assigning meaning
to `None`.

### Moderation preserves every current disposition

The provider's current moderation contract adds `BLOCK` for a general
violation that should be intercepted without necessarily ending the
conversation, and `HIGH` for high-risk content whose input should also be
withdrawn. Both now deserialize to distinct `RiskLevel` variants instead of
collapsing into `RiskLevel::Unknown`. `RiskLevel` was already non-exhaustive,
so downstream matches must retain their wildcard arm for later service values.

### Knowledge success envelopes now follow the frozen business contract

Data-bearing Knowledge and Document responses now deserialize successfully
only when the envelope contains business code `200` and a present, non-null
`data` field; an empty array or object remains valid data. Delete, update, and
re-embed operation responses require code `200` but do not invent a `data`
requirement that is absent from their upstream response shape. The public
fields remain optional for source compatibility and caller-side construction,
but HTTP-200 bodies with a missing code, a non-success code, or missing/null
required data now fail instead of becoming partial typed success.

### ZRAG retrieval and agent chat are public typed APIs

`zai_rs::zrag` now exposes typed multimodal retrieval and stream-only agent
chat. Retrieval validates nested filters and identifiers before network I/O and
rejects response envelopes with no known non-null field. Agent chat is a
non-replayable SSE POST; an optional continuation ID is sent only through an
operation-local sensitive `X-Session-Id` header and never enters the JSON body.

The chat decoder advances one event at a time in bounded 64 KiB input slices
and recursively rejects duplicate JSON object keys. A JSON `type=done` event is
yielded once before normal termination. An in-band `type=error`, EOF before
done, literal `[DONE]`, or malformed event yields one error and immediately
releases the raw response. Future event types and tool-result statuses retain
their original values. Their explicit raw/status/session accessors can contain
user content, tool data, or provider identifiers and must be treated as
sensitive; default `Debug` output does not render them.

Recognizable credentials and exact continuation-session echoes are removed
from HTTP/in-band error messages and from business-code, request-ID, and
`Retry-After` diagnostics. Arbitrary provider messages are still application
content, not a general-purpose redacted log format.

### Realtime sessions preserve selected future nested enum values

The session decoder now emits `ServerEvent::UnsupportedKnown { event_type,
raw }` when an otherwise valid known event contains a future string value in a
small, explicit set of nested enums. The compatible paths are
`session.updated` modalities, voice, turn-detection type, noise-reduction type,
and chat mode, plus `item.type` on conversation-item created/retrieved and
response-output-item added/done events. The original semantic JSON is retained
in `raw`, so it may contain transcripts, tool arguments, or other sensitive
application data and must not be logged without redaction.

The live-session decoder performs an allocation-light recursive duplicate-key
preflight before every strict known or unknown event decode. The raw-tree
fallback runs only after strict typed decoding fails; it rejects wrong JSON
types, missing or malformed sibling fields, and unrelated paths, and its
patched probe must still decode as the same known top-level event. Future
input/output audio-format strings remain fatal because continuing with an
unknown negotiated encoding could corrupt media. Direct
`serde_json::from_str::<ServerEvent>` also remains strict; only the live session
decoder constructs `UnsupportedKnown`. Existing exhaustive matching was
already prevented by `ServerEvent` being non-exhaustive, but applications may
now receive this additional event instead of observing the session close.

### Error diagnostics redact quoted and truncated credentials

Credential masking now recursively covers strict JSON objects/arrays, common
compound credential fields, credentials embedded in string values or object
keys, one additional JSON-string encoding layer, single-quoted provider log
fragments, unquoted malformed values, and values truncated before their closing
quote. Rendered provider errors can therefore contain more `[FILTERED]`
markers than before. Clean messages also avoid repeated no-match copies.

The redactor is a safety transform for diagnostics, not a lossless formatter.
When the input is valid JSON it may be compacted, object members may be
reordered, and duplicate members may collapse during parse/serialization.
Applications must not use its output for byte equality, signatures, or later
protocol processing.

### Source-backed errors retain operation context

`ZaiError::context` now retains context for `NetworkError`, `JsonError`, and
`RealtimeError`. These variants cannot store an editable message, so the SDK
uses the new transparent `ZaiError::Context` wrapper while preserving the
original source and its standard error chain. `code()`, `category()`,
`is_retryable()`, `raw_business_code()`, `request_metadata()`, and
`source_error()` continue to delegate to the underlying error; `compact()`
also keeps its stable `NETWORK:`, `JSON:`, or `REALTIME:` prefix.

Code that directly matched one of those three variants immediately after
calling `.context(...)` must instead inspect `error.source_error()` and retain
a wildcard arm. `Display`, `Debug`, `message()`, and the standard
`Error::source()` chain now include the retained operation, which is an
intentional diagnostic behavior change. Repeated contexts are flattened in
outer-to-inner order rather than creating an unbounded wrapper chain.

### Realtime handshake errors retain only a safe summary

An SDK-created Tungstenite HTTP handshake failure now becomes
`RealtimeErrorKind::HandshakeHttp(RealtimeHandshakeHttpContext)` before it
enters the public error chain. The SDK retains only the HTTP status, a canonical
numeric business code recovered from bounded, complete JSON, and a valid
`Retry-After` duration. Tungstenite exposes only the body tail that accompanied
the parsed headers, so the SDK trusts that JSON only when one `Content-Length`
exactly matches the tail and no `Transfer-Encoding` is present; otherwise the
HTTP status alone controls policy. Peer-controlled response headers and body bytes are
dropped and are no longer reachable through `Debug` or the standard
`Error::source()` chain. The low-level WebSocket `Debug` implementation also
uses its source's payload-free `Display` text, preventing outbound messages or
manually wrapped HTTP bodies from appearing in default diagnostics.

Handshake category and retry decisions now use the same HTTP/business policy
as the HTTP transport: authentication, quota, and validation signals fail
closed, while only documented transient statuses or business codes retry.
Non-HTTP WebSocket failures are retryable only for an explicit transient I/O
allowlist; URL, TLS/certificate, protocol, capacity, malformed-data, and closed
states are not. Code that matched an SDK handshake as
`WebSocket { source: tungstenite::Error::Http(..) }` must instead match
`HandshakeHttp(context)` and use its safe getters. Applications that require a
provider's raw diagnostic body must capture it in a separately secured proxy;
the SDK intentionally does not preserve it.

### Realtime writer scheduling is fair under feedback Pings

The built-in writer now alternates explicit control/data preferences. Pong is
normally preferred, but after a successful control write the writer switches
to data preference and yields once so a late-arriving producer can run. A peer
that immediately sends another Ping after every Pong can therefore no longer
starve application data. Completing an application message restores control
preference, so a permanent data backlog cannot starve Pong either.

This changes scheduling, not application ordering. Shutdown remains the first
choice in either preference state; RFC control frames are inserted only at
application-message frame boundaries, and complete application messages keep
their FIFO order without interleaving.

### Additive API conveniences

- The common prelude now exports `ApiFamily`, `HttpConcurrencyConfig`,
  `RequestOptions`, and `RetryOverride`.
- `zai_rs::pagination` now exposes validated `CursorPagination` and
  `PagePagination` values. They reject zero values, redact opaque cursors from
  `Debug`, and are attached through each request's `try_with_pagination` so the
  request can enforce endpoint-specific limits. File limits and assistant page
  sizes are capped at 100; batch, knowledge, and document lists add no SDK cap.
  They deliberately do not implement `Serialize`: endpoints use different
  `limit`, `size`, or `page_size` wire fields and may require additional fields.
- `HttpTransportConfig` adds `with_compression`; its builder adds `compression`
  and validated `try_build`. The existing `build` method remains available and
  client construction still validates its result.
- Stream-independent `ChatCompletion` builders remain available after
  `enable_stream()`, and streaming requests now expose the same preflight
  `validate()` entry point as non-streaming requests.
- Every typed MCP request exposes an inherent `validate()` method, and
  `SessionBuilder::validate()` checks realtime session, message-size,
  transport-policy, API-key, and JWT inputs without opening a network
  connection.
- The additive, canonical realtime policy type is
  `zai_rs::realtime::RealtimeTransportConfig`. Its twelve primary knobs cover
  the built-in connection-attempt limit, connect/write/Pong/close/idle/admission
  deadlines, outbound/writer/event/audio capacities, and maximum frame bytes;
  the exact defaults and accepted ranges are recorded in the type's API table.
  `Default` retains the previous primary timeout and capacity values but makes
  observable bounded changes: outbound admission now has a 30-second total
  deadline instead of waiting indefinitely, the default per-data-frame stall
  guard is 5 rather than 10 seconds, and a built-in session may recover from a
  transient first-connection failure with up to three attempts.
  `RealtimeClient::with_transport_config` supplies a default that is
  snapshotted when `session(...)` creates a builder; a builder-level
  `with_transport_config` replaces that snapshot for one session, whose
  effective value is available through `RealtimeSession::transport_config()`.
  Only a built-in Tungstenite session created by `SessionBuilder` consumes all
  twelve settings. Its `max_connect_attempts` defaults to 3 and accepts `1..=3`;
  one disables connection retry. Every attempt, full-jitter backoff, and valid
  `Retry-After` shares the one `connect_timeout` acquisition budget, and JWT
  mode freshly signs authorization for each attempt. A direct
  `TungsteniteTransport::connect_with_config` always performs one attempt: its
  `connect_timeout` bounds that attempt, while the attempt limit and session
  queue/event/audio settings are retained only for its config getter. An
  injected transport is already connected, so neither connection setting
  applies.
- A nonzero outbound admission deadline covers the single preparation permit,
  media expansion and JSON serialization, exact byte-budget admission, and
  command-channel admission, with checks between stages. Zero makes every
  contended admission fail fast. Configuration cannot raise the fixed 8 MiB
  serialized-message/end-to-end session byte budget, the direct transport's
  8 MiB writer budget, the 4 MiB raw-media limit, or the single concurrent
  preparation. Built-in sessions use the smaller outbound/writer message
  capacity across the complete pipeline, and each accepted command retains
  its byte/count permits until the socket writer finishes. A command therefore
  cannot succeed at the public admission boundary and later terminate the
  session merely because a private second queue is full.
  Built-in confirmed writes and ordinary-send guards derive as `write + 1s`;
  only the injected path adds a `write + 2s` outer initial-update guard. Writer
  join derives as `close + 1s`, and the session/injected close guard as
  `close + 2s`. A data frame no longer directly shares the Pong
  deadline: its stall guard is `min(5s, pong / 2)`, while Pong retains its own
  absolute deadline including queue time.
- `SessionBuilder::build_with_transport` remains an additive entry point for an
  already-connected, already-authenticated `RealtimeTransport`. The SDK does
  not validate, use, or pass its API key, JWT, Authorization header, or
  configured URL on this path. It validates and enforces the effective
  session-owned outbound admission/queue, event/audio buffers, idle and message
  limits, plus write/close-derived outer guards, but never passes the policy
  object to the injected transport. Connect, Pong, frame, and built-in writer
  settings apply only to the SDK's Tungstenite transport. The injected transport
  receives the complete application `session.update`, so it must protect that
  payload and redact its own logs and errors. The method waits for
  `send_confirmed`; buffered transports must override that method rather than
  merely enqueue the message. Existing three-method implementations remain
  source-compatible because `send_confirmed` defaults to `send`. The old
  `RealtimeClient::new(...).session(...).build()` path remains available and
  uses the default bounded retry policy. `TungsteniteTransport::connect(...)`
  remains a single direct attempt using `Default`; explicit direct connections
  may use `connect_with_config`. Built-in retries cover only retryable network
  or handshake failures before the first `session.update`. Once that write
  begins, its outcome can be ambiguous and the SDK never replays it.
  The built-in Tungstenite write buffer is bounded at twice the configured
  frame limit (4 MiB by default), and oversized inbound audio is rejected by
  encoded length before allocating a decoded buffer.

## Hardening prepared for the unpublished 6.0.1 candidate

This section records behavior changes prepared for the unpublished
`zai-rs 6.0.1` candidate. Because the first four changes below alter observable
runtime behavior, they ultimately shipped in `6.1.0`, a version greater than
`6.0.1`, rather than being presented as an ordinary `0.6.x` patch release.

## Vision MCP now requires explicit runtime consent

`McpClient` no longer downloads and executes an npm package merely because a
Vision capability is called. Choose one of these paths:

```rust,ignore
use zai_rs::mcp::{McpClient, VisionMcpCommand};

# fn build() -> zai_rs::ZaiResult<McpClient> {
let runtime = VisionMcpCommand::new("/opt/zai/vision-mcp")?.arg("--stdio");
let client = McpClient::new("test.12345678901234567890")?
    .with_vision_mcp_command(runtime);
# Ok(client)
# }
```

For local development, callers may explicitly restore the historical
convenience behavior with `.with_vision_npx_download()`. That path requires
Node.js 22+ and permits `npx` to resolve the pinned top-level package; its
transitive npm dependency graph is not covered by `Cargo.lock`.

The child process now receives a minimal runtime environment plus the Z.ai
credential, region, and optional model override. If a custom wrapper needs
additional configuration, put it in the reviewed wrapper rather than relying
on inherited application secrets.

## Tool caching and retries require a per-tool declaration

Executor-wide cache and retry settings are upper bounds. Existing tools default
to neither behavior until their effect policy is declared:

```rust,ignore
use zai_rs::toolkits::{
    CachePolicy, FunctionTool, RetryPolicy, ToolExecutionPolicy,
};

# fn build() -> zai_rs::toolkits::error::ToolResult<FunctionTool> {
let tool = FunctionTool::builder("lookup", "deterministic lookup")
    .execution_policy(ToolExecutionPolicy::new(
        CachePolicy::Pure,
        RetryPolicy::Idempotent,
    ))
    .handler(|arguments| async move { Ok(arguments) })
    .build()?;
# Ok(tool)
# }
```

Use `CachePolicy::Pure` only when the result has no observable side effect and
is stable for the cache lifetime. Use `RetryPolicy::Idempotent` only when a
complete duplicate invocation cannot repeat or corrupt an effect. Enabling
cache or retries globally without these declarations intentionally does
nothing for that tool.

Concurrent misses for the same pure tool registration and canonical arguments
are now collapsed into one successful execution. Failures are not cached or
shared: the next waiter may execute the handler. `clear_cache()` and per-tool
invalidation fence off results that were already in flight, so an older
execution cannot repopulate the cache after the invalidation point.

The executor now moves its owned JSON arguments into a non-retryable or final
attempt. It clones them only when an idempotent future retry must retain the
original value. This preserves retry/cache behavior while removing a
payload-sized deep clone from the safe default `RetryPolicy::Never` path.

Directory-loaded tools have the same trusted policy path. The historical
`add_functions_from_dir_with_registry` API remains available and always uses
`Never`/`Never`, even when its JSON contains policy-looking fields. To opt in,
bind the local handler and policy explicitly:

```rust,ignore
use std::{collections::HashMap, sync::Arc};
use zai_rs::toolkits::{
    CachePolicy, RetryPolicy, ToolExecutionPolicy, ToolHandler,
    ToolRegistration, executor::ToolExecutor,
};

let handler: ToolHandler =
    Arc::new(|arguments| Box::pin(async move { Ok(arguments) }));
let registrations = HashMap::from([(
    "lookup".to_string(),
    ToolRegistration::new(handler).with_execution_policy(
        ToolExecutionPolicy::new(CachePolicy::Pure, RetryPolicy::Never),
    ),
)]);

let executor = ToolExecutor::builder().enable_cache().build();
executor.add_functions_from_dir_with_registrations(
    "./tool-specs",
    &registrations,
    true,
)?;
```

Every JSON file and selected schema is validated before the registry changes.
Duplicate names in the directory or conflicts with an existing tool reject the
whole batch. In strict mode, a specification without a local registration also
rejects the batch; non-strict mode skips it. Extra local registrations without
a matching file are ignored.

## Unknown business codes preserve HTTP recovery semantics

An unrecognized business code paired with HTTP 401/403, 429, or 5xx now becomes
`ZaiError::HttpBusinessError` instead of discarding the actionable HTTP
classification in `ZaiError::Unknown`.

- `category()` and `is_retryable()` use the HTTP status.
- `code()` returns that HTTP status.
- `raw_business_code()` returns the bounded, canonicalized,
  credential-redacted wire code for explicit diagnostics.
- A recognized business code still takes precedence over the HTTP status.

This changes variant matching, metrics keyed by `code()`, and retry decisions.
Applications should prefer the category helpers and retain a wildcard arm when
matching the non-exhaustive error enum.

## HTTP failures now carry request diagnostics

Every failure after a request enters the HTTP transport is now transparently
wrapped in `ZaiError::Request`. Stable helpers such as `code()`, `category()`,
`is_retryable()`, and `compact()` continue to delegate to the original error.
Code that directly matches variants such as `AuthError` or `RateLimitError`
must instead match on `error.source_error()` and retain a wildcard arm.

`request_metadata()` exposes the number of attempts, a bounded provider request ID,
the final valid `Retry-After` hint, and the timeout phase (attempt, overall,
SSE handshake, SSE idle, admission queue, or stream consumer). These values
remain absent from default
`Display`, `Debug`, and `compact()` output to avoid accidental disclosure.
The request ID remains provider-controlled application data and should be read
only under an explicit logging policy.

## Additive capabilities

- Agent v1 non-streaming invocation, async-result polling, and conversation
  continuation now provide `send_via(&ZaiClient)`.
- HTTP failures provide bounded, structured request diagnostics through
  `request_metadata()` while preserving their original classification helpers.
- `FileContentRequest::stream_via` exposes bounded `Bytes` chunks;
  `send_to_via` writes chunks to a private same-directory partial file and
  publishes without replacing an existing destination.
- `HttpTransportConfig::request_timeout` retains its 60-second default but may
  be raised explicitly up to 24 hours for intentionally slow transfers.
- `RequestOptions` can be attached to a cheap cloned `ZaiClient` handle to
  shorten the admission-queue deadline, set attempt/overall and SSE
  handshake/idle deadlines, lower the configured consumer base for selected
  requests, lower the global attempt cap, or explicitly assert idempotency. SSE
  requests remain non-replayable regardless of that assertion. The SSE idle
  deadline is absolute from the latest transport chunk and is not restarted by
  a consumer pause; a chunk already buffered before the deadline is still
  delivered first. The separate consumer lease slides only when an underlying
  raw-stream poll actually yields a chunk; producing a typed item from a
  buffered raw chunk need not renew it. Its configured base is the minimum of
  the scoped and global values, followed by the `sse_idle + 1s` effective floor;
  the floor can make an override a no-op. Base setters stop at 24 hours, while
  the effective interval can reach 24 hours plus one second. File streams use
  the absolute overall deadline instead.
- Realtime audio/video preparation is admitted before WAV/base64/JSON
  expansion. A stack WAV header plus PCM, raw PCM, or JPEG is base64-encoded
  directly into one exactly sized final JSON buffer. The public `ClientEvent`
  wire remains byte-identical, while the intermediate base64 string and JSON
  reallocations are removed.
- ASR `with_file_base64` keeps its public String-based builder contract but
  transfers that allocation into immutable `Bytes`. Validation reads standard
  Base64 to EOF through a fixed 8 KiB scratch buffer, retaining only the first
  12 decoded bytes and the decoded length, so malformed tails and the 25 MiB
  limit keep their prior ordering without allocating the decoded payload.
  Multipart retries share the same encoded allocation and preserve the exact
  text-field wire metadata and ordering.

---

## 发布状态

`Cargo.toml` 声明的 `6.2.0` 是 2026-08-29 准备的当前发布版。此前的
`6.1.0` 包含下文记录的安全加固与 GLM-5.3 支持。
`v6.1.0` tag workflow（run `31818272628`）最初没有完成发布：一次运行在
macOS 的 proxy-isolation 测试竞态处失败，下一次已通过质量、打包、SBOM、
校验和与 attestation，但 crates.io 因缺少匹配的 Trusted Publisher 配置而拒绝
OIDC 鉴权。随后 `6.1.0` 从 main commit `376c2c1` 在该 tag workflow 之外发布；
因此 registry archive 与 workflow 已证明的 tag 制品并不一致。失败尝试与不可移动的
tag 仍作为发布历史保留。

更早的两次 `v6.0.1` workflow（`30091854390` 和 `30092565276`）同样保留为
历史证据。已取回的 step 日志证明：首次运行的当时 tag 不是 annotated tag；
第二次已通过质量、打包、SBOM、校验和与 attestation，但在缺少 Trusted
Publisher 的边界失败。`6.0.1` 从未发布，其 tag 不得移动或复用。

`6.2.0` 在已发布的 `6.1.0` API 基础上新增 GLM-5.3-Flash 多模态支持。

## 6.1.0 之后新增并计划随 6.2.0 发布的内容

### GLM-5.3-Flash 多模态支持

新增的 `GLM5_3_flash` 模型类型（wire id `glm-5.3-flash`）通过
`VisionMessage` 支持文本、图片、视频与文件输入，并提供同步/异步对话、
Function Calling、结构化输出与流式工具调用。与 `GLM5_3` 一样，它不允许
关闭 thinking，`reasoning_effort` 仅接受 `low`、`high` 与 `max`；请求校验会在
网络 I/O 前执行这些约束。同一请求中，文件输入不能与图片或视频输入混用。

该变更是纯新增能力，现有 `6.1.0` 用户无需修改源码。采用
`GLM5_3_flash` 的代码即使只发送文本，也必须使用 `VisionMessage`；需要最轻量
推理模式时，应使用 `enabled` 并设置 `reasoning_effort = "low"`。

## v6.0.1 候选 tag 之后、已随 6.1.0 发布的加固

本节改动已随 `6.1.0` 发布，但未包含在既有 `v6.0.1` 候选 tag 中。

- host 为 `localhost`、IPv4 loopback 或 IPv6 loopback 的 HTTP endpoint 现在使用
  独立 `no_proxy` 连接池。即使应用设置 `HTTP_PROXY` / `ALL_PROXY` 且没有配置
  `NO_PROXY`，本地请求和 Authorization header 也不会进入系统代理。确实需要代理
  本地测试流量的应用，应把代理显式配置为 SDK endpoint 或调整测试拓扑；public HTTPS
  仍使用支持系统代理的连接池。
- 非 2xx body 只保留最多 64 KiB 诊断前缀。错误页超大、停滞或在 header 后断开时，
  SDK 继续保留 HTTP status，而不再改报 SDK body-size、timeout 或 network error；因此
  幂等 503 可能新增一次正确 retry，最终 401/429/5xx 也保留其分类。只有在 cap/deadline
  前读到 EOF 的完整诊断才会解析 business envelope；被截断、超时或中断的前缀不会贡献
  body-derived code、request ID 或 message，retry/分类只按 HTTP status，并返回不含 body
  的静态消息。
  依赖旧错误类型或不完整前缀业务码的代码应迁移到 `category()`、`code()` 与
  `is_retryable()`。
- 每个 `ZaiClient` 默认最多准入 64 个逻辑 HTTP 操作，所有 clone 共享同一预算。
  buffered 请求在全部 attempt/backoff、限界 body read 与 response decode 期间持有
  permit；SSE/文件响应则持有到 stream 结束、调用方 Drop 或安全 lease 到期。终止型
  SSE parser/decoder 错误会在返回 error item 前主动丢弃 raw response，不依赖调用方
  再次 poll。SSE consumer 的 configured base 默认 5 分钟，先取
  `base = min(scoped, global)`，再取 `effective = max(base, sse_idle + 1s)`。两个 base
  setter 都接受 `1ns..=24h`；idle floor 可能使更小的 scoped base 不改变 effective，且
  effective 最大可达 24 小时加 1 秒。只有底层 raw-stream poll 实际取得 chunk 才续期，
  typed decoder 从已缓冲 raw chunk 产出 item 不一定续期。floor 保证服务端静默报告
  `SseIdle`，raw stream 未被推进则报告 `StreamConsumer`；文件流复用既有 absolute
  overall deadline 并报告 `Overall`。到期时
  SDK 原子取得并丢弃完整 raw body 与 permit、唤醒排队请求，仍被调用方持有的公开 stream
  随后只返回一次 timeout 再结束；调用方 Drop 仍立即执行相同回收。这会让过去无限期持有
  未 poll stream 的应用观察到新 timeout。预算用尽后默认排队 30 秒；超时错误的 request
  metadata 为零次 HTTP
  attempt 与 `TimeoutPhase::Queue`，且排队不消耗 attempt/overall deadline。此前同时
  启动超过 64 个操作的应用可能观察到新超时，应通过 `ZaiClientBuilder::concurrency`
  显式设置 `HttpConcurrencyConfig`（并发 `1..=4096`、排队 `0s..=24h`、SSE consumer
  base `1ns..=24h`）。零排队时间提供 fail-fast 准入；scoped
  `RequestOptions::with_queue_timeout` 只能缩短全局 queue deadline。scoped
  `with_stream_consumer_timeout` 只能降低 global base，随后应用的 `sse_idle + 1s` floor
  可能使该 override 不改变 effective lease。
- `VoiceListQuery` 的公开 `Serialize` 现在输出 upstream 规定的 `voiceName` /
  `voiceType`。SDK 自身 `send_via` 原本已经使用这两个名称，但通用 serializer 过去会
  错误地产生 `voice_name` / `voice_type`；依赖错误旧形状的代码需改用 camelCase wire key。
- `FileContentRequest::send_to_via` 仍以同目录私有 partial、文件 `sync_all` 和最终
  no-clobber 操作保证已有目标不被替换。hard link 是首选；仅当文件系统明确返回
  `Unsupported` 时，SDK 才改用平台相关的安全 no-clobber persistence fallback，其他
  hard-link 错误继续返回。fallback 不会覆盖并发创建的目标，但若旧私有名称清理失败，
  可能遗留 `.part`。相对目标在创建父目录和网络 I/O 前一次性转为 lexical absolute
  path，因此下载期间的进程级 CWD 变化不会让不同阶段落到不同目录；这不等于
  canonicalize 或固定 symlink。
- Unix 会在创建父目录前记录 lexical parent chain，直到首个已经存在的目录。发布后从
  目标的直接父目录开始，按 deepest-first 顺序 fsync 每个新建祖先及该预存 anchor；在
  namespace 稳定时，成功表示完整文件、目标目录项和本次新建的每级目录项均已同步。
  该协议不 canonicalize 或固定 symlink，其他进程并发替换 path component 不在保证范围。
  任一目录同步失败都会返回 `SDK_IO`，但完整目标已经存在且不会回滚，即
  published-but-durability-unconfirmed；调用方应先检查/协调该路径而不是盲目重试。
  Windows/其他 non-Unix 在 stable Rust 下没有可移植、有文档保证的 directory sync；
  文件内容仍在发布前同步，但成功不承诺目录项掉电存活。取消也可能与已经派发到
  blocking pool 的发布操作竞态：future 被取消后仍可能出现完整且未覆盖旧目标的
  destination。调用方应协调该路径而不是盲删或盲重试；SDK 会 best-effort 清理私有
  `.part`。
- partial Drop cleanup 会先关闭 SDK 持有的文件句柄；若当前存在 Tokio runtime 且取得
  进程级预算，则把删除延后到 blocking pool。预算最多覆盖 8 个“已排队 + 正在运行”的
  cleanup job；预算饱和、没有 runtime 或无法建立 armed path guard 时，会同步尝试删除，
  而不是建立无界 backlog。queued task 在 runtime shutdown 时被丢弃或调度失败，仍由
  guard best-effort 清理并释放预算。所有 cleanup 错误都不会从 Drop 返回；SDK 刻意不在
  startup 扫描或 scavenger 应用目录，异常终止/删除失败留下的 `.zai-dl-*.part` 应由应用
  按自身 retention policy 协调。
- 完整文件下载现在只接受不带 `Content-Range` 的 `200 OK`。服务端意外返回的
  `206 Partial Content`、`204 No Content` 或带 range 语义的 `200` 会在交付字节前失败，
  `send_to_via` 不会发布截断/伪空目标；真正的 `200` 空 body 仍表示合法空文件。只有不带
  `Content-Range` 的 `200` JSON business-error envelope 会在 header 完整性检查后继续按
  业务码解码/重试；其他 2xx 或任何带 `Content-Range` 的响应均在 poll body 前失败。
- SSE 握手同样只接受不带 `Content-Range`、且 MIME 为 `text/event-stream` 的 `200 OK`。
  即使 `206` body 含合法终止标记，也不能再把部分响应建立成 typed stream；`204` 与带
  range 语义的 `200` 也会仅凭 status/header、在 poll body 前失败，因此慢速或恶意 body
  无法延迟或覆盖协议错误。完整非 2xx 响应仍保留有界 business-error 投影、敏感 header
  value 脱敏、request metadata 与 `Retry-After`；不完整诊断只按 status 处理，SSE POST
  仍永不自动重放。
- buffered JSON 与 binary 操作现在严格执行冻结 route contract 声明的成功状态，而不是
  接受任意 `2xx`。当前所有此类操作均声明 `200 OK`；意外的
  `201` / `202` / `204` / `206` / 其他 `2xx`，或完整响应上的 `Content-Range`，会仅凭
  status/header 在 poll body 前失败且不重试，不能再把部分 binary 或未声明状态下的合法
  JSON 变成 typed success。
- cross-origin、TLS downgrade、非法 `Location` 与 hop-limit 等 redirect policy 拒绝现在
  保留静态 `SDK_VALIDATION`，不会降级成泛化 `3xx`，不会重放，也不会把被拒绝的
  `Location` 值写入诊断。允许的 redirect 若在目标返回 headers 前失败，错误元数据会保留
  最近一次 redirect 的限长 request ID 与有效 `Retry-After`，不会沿用更早 retry response。
- transport 会把 JSON business envelope 分为 clean、recognized error、reserved-field
  ambiguous 与 malformed 四类。顶层 `error` / `code` / `message` / `request_id`，或直接嵌套
  `error.code` / `error.message` 出现重复时，不再回退到可能忽略这些字段的 typed success；
  ambiguous 2xx 返回不含 body 的静态协议错误，非 2xx 的 retry/分类只看 HTTP status。旧投影
  兼容矩阵继续作为内部测试 oracle，生产 transport boundary 只使用严格 probe。完整但
  malformed 的非 2xx JSON 同样只按 status 且不回显 body；合法 2xx 仍交给 endpoint typed
  decoder。provider 可控的 composite `code` 会流式消费而不构造完整树，超长 numeric/string
  literal 只保留固定非数字 sentinel，病理深度在 Serde scratch 增长前即拒绝，避免有界
  body 放大成无界诊断对象。
- 公开 `SseEventParser::try_push` / `try_finish` 使用与生产流一致的单事件 32 MiB、
  4096 条 `data:` line 限制，超限会返回错误并释放 parser buffer。旧 `push` /
  `finish` 保留原返回类型但也执行限界；因其不能返回错误，超限时会 reset 并返回空
  `Vec`，同一次调用中此前累计的完成事件也会丢弃。需要区分“尚无完整事件”和“输入
  被拒绝”的调用方必须迁移到 `try_*`。
  合法但超大的 comment/未知字段行在消费后也会释放异常 scratch capacity，不再让
  长连接一直钉住接近事件上限的临时分配。
- Chat streaming tool call 的可选 `type` 遇到未来字符串时会降级为 `None`，同时保留
  call id、index 和增量 function 内容，因此不会再因一个新增 discriminator 截断后续
  文本或 `[DONE]`。已知 `"function"`、缺失/`null` 的语义不变；数字、对象等非字符串
  仍视为 malformed。未知原字符串不会保留，需要理解该新类型的应用仍应升级 SDK。
- Batch list、file list/object/delete、embedding response/item、web-search intent 与
  图片内容过滤阶段的可选 discriminator 遇到未来字符串时会降级为 `None`，
  同时保留其余已知 payload（包括生成图片 URL 与过滤严重度）。
  已知值、缺失/`null` 的语义不变；数字、对象、数组等非字符串仍严格报错。该宽容仅在
  对应 response 字段生效，直接反序列化公开 marker enum 仍保持严格，序列化也不变；
  只有未知顶层 marker、没有其他非空已知字段的响应仍会触发现有 empty-response 校验。
  未知原字符串不会保留，需要理解它的应用必须升级 SDK，不能自行把 `None` 当成新语义。
- moderation 当前契约新增的 `BLOCK`（一般违规、建议拦截但不一定终止本轮对话）与
  `HIGH`（高危、建议拦截并回撤输入）现在分别保留为独立 `RiskLevel` variant，不再都
  压成 `Unknown`。该 enum 原本就是 non-exhaustive；下游仍应保留 wildcard 处理未来值。
- Knowledge/Document 的 data-bearing response 现在只有在 business code 为 `200` 且
  `data` 存在、非 `null` 时才反序列化成功（空数组/空对象仍合法）；delete、update、
  re-embed 等 operation-only response 只要求 code `200`，不会虚构上游不存在的 `data`
  字段要求。公开字段为保持 source compatibility 仍是 `Option`，但缺 code、非成功 code、
  缺失/null 必需 data 的 HTTP 200 不再成为部分 typed success。
- `zai_rs::zrag` 现在提供 typed 多模态 retrieval 与仅流式 agent chat。retrieval 会在
  网络前校验嵌套 filter/ID，并拒绝没有任何已知非空字段的 response envelope。chat 是
  不重放的 SSE POST；可选续聊 ID 只通过 operation-local sensitive `X-Session-Id` header
  发送，不进入 JSON body。decoder 以 64 KiB slice 逐事件推进并递归拒绝 duplicate JSON
  key；JSON `type=done` 先交付一次再正常终止，`type=error`、done 前 EOF、literal
  `[DONE]` 或 malformed event 只产生一次错误并立即释放 raw response。未来 event type
  与 tool-result status 保留原值，但显式 raw/status/session accessor 可能含用户正文、
  工具数据或 provider ID，必须按敏感数据处理，默认 `Debug` 不渲染它们。HTTP/in-band
  error 会移除可识别凭据，以及 exact session 从 message、business code、request ID、
  `Retry-After` 的回显；任意 provider message 仍不是通用脱敏日志格式。
- Realtime session 对一组明确限定的嵌套枚举提供兼容降级：`session.updated` 的
  modalities、voice、turn-detection type、noise-reduction type、chat mode，以及
  conversation item created/retrieved 与 response output-item added/done 的 `item.type`
  若出现未来字符串，会作为 `ServerEvent::UnsupportedKnown { event_type, raw }` 交给
  订阅者并保持会话存活。live-session decoder 会在所有 strict known/unknown event 前做
  allocation-light 的递归重复 key 预检；只有 strict 失败才分配 raw tree 进入 fallback。
  fallback 仍拒绝错误 JSON 类型、缺失/损坏的同级字段和非候选路径，且 input/output
  audio format 继续 fail closed，避免未知协商格式造成媒体错配。直接
  `serde_json::from_str::<ServerEvent>` 保持严格；`raw` 可能含 transcript、tool argument
  等敏感应用数据，不能未经脱敏写日志。
- provider 错误脱敏现在递归处理严格 JSON object/array、常见组合 credential 字段、
  string value/object key 内的凭据、一层额外 JSON-string 编码、单引号日志片段、
  未加引号 malformed value，以及在 closing quote 前被截断的 credential；错误文本中
  可能比以往出现更多 `[FILTERED]`。该输出只用于安全诊断，不是 lossless transform；
  合法 JSON 可能被压缩、重排 object member，并在 parse/serialize 时合并重复 member，
  不得用于字节等值、签名或后续协议处理。
- SDK 生成的 Realtime HTTP 握手失败会在进入公开错误链前收敛为
  `RealtimeErrorKind::HandshakeHttp(RealtimeHandshakeHttpContext)`；只保留 HTTP status、
  从限长且 framing 可证明完整的 JSON 中取得的 canonical business code，以及有效
  `Retry-After`。Tungstenite 暴露的 body 只是解析响应头时同一次读取附带的 tail；只有唯一
  `Content-Length` 与 tail 长度完全一致且不存在 `Transfer-Encoding` 时才信任其中业务码，
  否则策略只按 HTTP status。原始 response headers/body 会被
  立即丢弃，无法再经 `Debug` 或标准 `Error::source()` 取得。握手分类与 retry 复用 HTTP
  transport 的业务码优先规则；非 HTTP WebSocket 只允许明确的瞬时 I/O kind 重试，URL、
  TLS/certificate、protocol、capacity、malformed-data 和 closed 状态均 fail closed。
  过去匹配 `WebSocket { source: Error::Http(..) }` 的代码应改匹配 `HandshakeHttp(context)`
  并读取安全 getter；确需保留 provider 原始诊断的应用必须在独立受控代理中采集。
- `prelude` 新增 `ApiFamily`、`HttpConcurrencyConfig`、`RequestOptions`、
  `RetryOverride`；transport config 新增 compression fluent setter 与校验型
  `try_build()`，原 `build()` 保持可用。
- `zai_rs::pagination` 新增公开 `CursorPagination` / `PagePagination`。它们统一拒绝
  零值、对 opaque cursor 的 `Debug` 脱敏，并通过具体 request 的
  `try_with_pagination` 应用 endpoint 上限：File limit 与 Assistant page size 最大 100，
  Batch/Knowledge/Document 当前不另加 SDK cap。它们刻意不直接实现 `Serialize`：不同
  接口使用 `limit`、`size` 或 `page_size`，且可能包含其他必填字段，应由 request 映射
  wire shape。
- 与 stream 无关的 `ChatCompletion` builder 在 `enable_stream()` 后仍可继续调用；
  streaming request 也新增与非流式一致的公开 preflight `validate()`。
- Realtime 策略的规范公开路径是
  `zai_rs::realtime::RealtimeTransportConfig`。12 个主配置项覆盖内建连接尝试上限、
  connect/write/Pong/close/idle/出站准入 deadline、outbound/writer/event/audio
  capacity 和最大 frame bytes；精确默认值与合法范围见该类型的 API 表。
  `Default` 沿用旧的主要 timeout/capacity 数值，但有可观察的有界加固：outbound
  admission 从无限等待改为默认 30 秒总 deadline，单个 data frame 的默认 stall guard
  从 10 秒收紧到 5 秒，内建会话还会以最多 3 次尝试恢复首次连接的瞬时失败。client 上的
  `with_transport_config` 会由 `session(...)` 创建的
  新 builder 快照；builder 同名方法会为单个会话完整替换该快照，最终值可从
  `RealtimeSession::transport_config()` 读取；`SessionBuilder::validate()` 会在零网络
  副作用下同时校验有效策略。只有 `SessionBuilder` 创建的内建 Tungstenite 会话会消费
  全部 12 项。`max_connect_attempts` 默认 3、范围 `1..=3`，设为 1 会禁用连接重试；每次
  尝试、full-jitter 退避和有效 `Retry-After` 共享同一个 `connect_timeout` 连接获取总预算，
  JWT 模式会在每次尝试前重新签发凭证。直接 `TungsteniteTransport::connect_with_config` 始终只做
  一次尝试，`connect_timeout` 只约束该次尝试，attempt 数与 session queue/event/audio
  设置仅完整保留供 config getter 检查；注入 transport 已连接，因此两项连接策略都不适用。
- 非零 outbound admission deadline 覆盖单并发 preparation、media 扩张与 JSON
  serialization、精确 byte-budget 和 command-channel 准入，并在阶段边界复查；零值让
  所有发生竞争的准入 fail-fast。配置不能抬高固定的 8 MiB 序列化消息/内建 session
  端到端 byte budget、直接 transport 的 8 MiB writer budget、4 MiB 原始 media 上限或
  单并发 preparation。内建 session 在完整管线使用 outbound/writer 消息 capacity 的
  较小值；一条已接受命令会持有 byte/count permit 直至 socket writer 完成，因此不会在
  公开准入已成功后仅因私有第二层队列已满而终止会话。
  内建 confirmed write 与普通 send guard 派生为 `write + 1s`；只有注入路径另以
  `write + 2s` 外层 guard 保护 initial update。writer join 为 `close + 1s`，
  session/injected close guard 为 `close + 2s`。data frame
  不再直接共用 Pong deadline：其 stall guard 是 `min(5s, pong / 2)`，Pong 则保留包含
  排队时间的独立绝对 deadline。
- 内建 writer 在 control/data 两种偏好间公平轮转：通常优先 Pong，成功写入 control 后
  切换到 data 偏好并主动 yield，使“每个 Pong 立即反馈下一次 Ping”的对端无法饿死已
  排队或随后到达的应用 data；完成一条 data message 后重新偏好 control，持续 data
  backlog 也不会饿死 Pong。shutdown 在两态都保持最高优先级，control 只插入 frame
  边界，完整应用消息仍严格 FIFO 且不互相穿插。
- `SessionBuilder::build_with_transport` 是新增的 transport 注入入口，只接受由应用
  事先完成连接和认证的 `RealtimeTransport`；该路径不校验、不使用，也不会向
  transport 传入 SDK 管理的 API key、JWT、Authorization header 或配置 URL。SDK 会
  校验并执行有效策略中的 session 自有 outbound admission/queue、event/audio buffer、
  idle、message limit 及 write/close 派生的外层 guard，但不会把 policy 对象传给注入
  transport；connect、Pong、frame 和内建 writer 设置只适用于 SDK Tungstenite。
  transport 仍会收到包含 instructions、greeting 和 tool schema 等应用内容的完整
  `session.update`，因此必须保护 payload 并脱敏自身日志/错误。SDK 发出的首个应用消息
  是 `session.update`，且必须等 `send_confirmed` 确认完整写入后才返回；
  buffered transport 不能只排队，必须覆写该方法。`send_confirmed` 默认委托给 `send`，
  因此既有只实现 `send` / `recv` / `close` 的三方法实现保持源码兼容。旧的
  `RealtimeClient::new(...).session(...).build()` 继续可用并采用默认有界连接重试策略；
  `TungsteniteTransport::connect(...)` 仍是采用 `Default` 的单次直接连接，显式直连可改用
  `connect_with_config`。内建 builder 只在首个 `session.update` 发送前重试可恢复的网络或
  握手失败；一旦开始发送，该写入结果可能不明确，SDK 不会重放。内建
  Tungstenite write buffer 上限为配置 frame 的两倍（默认 4 MiB）；超限入站 audio
  会先按编码长度拒绝，再进入 base64 decoded buffer 分配。

## 原 6.0.1 未发布候选版的安全加固

`zai-rs 6.0.1` 候选版准备了本节所列的安全加固，但它并未成功发布。
Vision MCP 的默认执行行为、工具
缓存/重试的默认资格、未知业务码的错误分类，以及 HTTP 错误的外层包装均发生了
可观察变化，因此这些改动最终随高于 `6.0.1` 的 `6.1.0` 发布，而没有作为普通
`0.6.x` 补丁发布。

- Vision MCP 默认不再通过 `npx` 下载或执行代码。生产环境使用
  `with_vision_mcp_command` 指向已审计的本地运行时；本地开发若接受 npm 供应链风险，
  可显式调用 `with_vision_npx_download`。
- 全局 `enable_cache()` / `retries()` 只是上限。每个工具还必须通过
  `ToolExecutionPolicy` 分别声明 `CachePolicy::Pure` 或
  `RetryPolicy::Idempotent`；未知或有副作用的工具默认不缓存、不重试。同一纯工具
  注册与规范化参数的并发 cache miss 只执行一次成功调用；失败不缓存也不共享。
  `clear_cache` 与按工具失效会阻止此前已在运行的旧结果重新写回。
  executor 会把自己持有的 JSON 参数直接 move 给不可重试或最后一次 attempt；只有声明
  幂等且确有未来 retry 时才 deep-clone，因此默认 `RetryPolicy::Never` 不再产生随 payload
  线性增长的额外副本，cache/retry 结果语义不变。
- 旧目录 API `add_functions_from_dir_with_registry` 始终使用安全的
  `Never`/`Never`，JSON 中即使出现 policy 字段也不会提权。需要 opt-in 时，应用通过
  `ToolRegistration::with_execution_policy` 把本地 handler 与可信策略绑定，再调用
  `add_functions_from_dir_with_registrations`。目录会先全量解析、校验 schema 和检查
  重复/已注册冲突，再提交整批；strict 模式下缺少本地 registration 也会整批失败，
  非 strict 模式则跳过，未被文件引用的额外 registration 会忽略。
- 未知业务码若伴随 HTTP 401/403、429 或 5xx，现在返回
  `HttpBusinessError`：`code()` 是 HTTP 状态，`raw_business_code()` 才是限长并脱敏
  的 wire 业务码，分类和可重试性按 HTTP 状态决定；已知业务码仍优先。
- 请求进入 HTTP 传输层后的失败现在由 `ZaiError::Request` 透明包装。
  `code()`、`category()`、`is_retryable()` 和 `compact()` 仍委托给原错误；直接匹配
  `AuthError`、`RateLimitError` 等变体的代码必须改为匹配 `error.source_error()` 并
  保留兜底分支。`request_metadata()` 提供尝试次数、限长且限制字符集的 provider
  request ID、最终有效
  `Retry-After` 和 attempt/overall/SSE handshake/SSE idle/admission queue/stream consumer
  timeout phase；默认 `Display`、`Debug` 与 `compact()` 不输出这些诊断字段。request ID
  仍属于 provider-controlled 应用数据，只应在显式日志策略允许时读取。
- Agent v1 非流式调用、异步结果轮询与会话续接的 `send_via`，以及文件内容流、
  no-clobber 文件发布和最高 24 小时的显式请求超时属于新增 API 能力。候选 tag 当时的
  文件发布仍依赖 hard link 且未同步父目录；当前工作树已经按上文加入 fallback 和
  分平台 durability 契约。
- `RequestOptions` 可挂到共享连接池的轻量 `ZaiClient` clone，为特定请求缩短
  admission queue timeout、降低 stream-consumer configured base，或配置 attempt/overall、SSE
  handshake/idle、较低尝试次数和显式幂等断言；SSE 始终不重放。
  SSE idle 从最近 transport chunk 起按绝对时间计算，不会因调用方暂停 poll 而重置；
  deadline 前已经缓冲的 chunk 仍优先交付。独立的 SSE consumer lease 只在底层
  raw-stream poll 实际取得 chunk 时滑动，typed decoder 从已缓冲 raw chunk 产出 item
  不一定续期；effective interval 在 base minimum 后应用 `sse_idle + 1s` floor，该 floor
  可能使 scoped override 不生效。base 最大 24 小时，effective 最大 24 小时加 1 秒。
  文件流则复用 absolute overall deadline。任一流 lease 到期都会整体回收 raw body 与
  共享 permit。
  Realtime 音视频现在在 WAV/base64/JSON 扩张前取得单会话准入；栈上 WAV header+PCM、
  raw PCM 或 JPEG 会直接 base64 写入一次精确分配的最终 JSON。公开 `ClientEvent` wire
  逐字节不变，同时移除了中间 base64 String 和 JSON reallocation。
  ASR `with_file_base64` 的公开 String builder 契约保持不变，内部会零拷贝接管为
  immutable `Bytes`；标准 Base64 经固定 8 KiB scratch 完整读到 EOF，只保留前 12 字节
  magic 与 decoded length，因此 malformed tail 和 25 MiB 上限仍保持原错误优先级，且
  不再分配完整 decoded payload。multipart retry 共享同一 encoded allocation，同时保留
  旧 text field 的 wire metadata、顺序和正文。
