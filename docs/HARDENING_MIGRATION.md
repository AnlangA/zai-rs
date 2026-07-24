# Unreleased hardening migration notes

This document records behavior changes introduced by the current optimization
worktree. The package version has not been changed. Because the first three
changes below alter observable runtime behavior, release maintainers should
either ship them in `0.7` or call them out explicitly as a security exception;
they should not be hidden in an ordinary `0.6.x` patch release.

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

## Additive capabilities

- Agent v1 non-streaming invocation, async-result polling, and conversation
  continuation now provide `send_via(&ZaiClient)`.
- `FileContentRequest::stream_via` exposes bounded `Bytes` chunks;
  `send_to_via` writes chunks to a private same-directory partial file and
  publishes without replacing an existing destination.
- `HttpTransportConfig::request_timeout` retains its 60-second default but may
  be raised explicitly up to 24 hours for intentionally slow transfers.
- `RequestOptions` can be attached to a cheap cloned `ZaiClient` handle to set
  attempt/overall and SSE handshake/idle deadlines for selected requests,
  lower the global attempt cap, or explicitly assert idempotency. SSE requests
  remain non-replayable regardless of that assertion. The SSE idle deadline is
  absolute from the latest transport chunk and is not restarted by a consumer
  pause; a chunk already buffered before the deadline is still delivered first.
- Realtime audio/video preparation is admitted before WAV/base64/JSON
  expansion. WAV input is encoded directly from a stack header plus PCM into
  the base64 destination instead of retaining a second full WAV buffer.

---

# 未发布安全加固迁移说明

当前工作树尚未修改 crate 版本。Vision MCP 的默认执行行为、工具缓存/重试的默认
资格，以及未知业务码的错误分类均发生了可观察变化；发布维护者应将其放入 `0.7`，
或作为安全例外在 `0.6.x` 发布说明中显式披露，不能作为无提示的补丁行为变化。

- Vision MCP 默认不再通过 `npx` 下载或执行代码。生产环境使用
  `with_vision_mcp_command` 指向已审计的本地运行时；本地开发若接受 npm 供应链风险，
  可显式调用 `with_vision_npx_download`。
- 全局 `enable_cache()` / `retries()` 只是上限。每个工具还必须通过
  `ToolExecutionPolicy` 分别声明 `CachePolicy::Pure` 或
  `RetryPolicy::Idempotent`；未知或有副作用的工具默认不缓存、不重试。同一纯工具
  注册与规范化参数的并发 cache miss 只执行一次成功调用；失败不缓存也不共享。
  `clear_cache` 与按工具失效会阻止此前已在运行的旧结果重新写回。
- 旧目录 API `add_functions_from_dir_with_registry` 始终使用安全的
  `Never`/`Never`，JSON 中即使出现 policy 字段也不会提权。需要 opt-in 时，应用通过
  `ToolRegistration::with_execution_policy` 把本地 handler 与可信策略绑定，再调用
  `add_functions_from_dir_with_registrations`。目录会先全量解析、校验 schema 和检查
  重复/已注册冲突，再提交整批；strict 模式下缺少本地 registration 也会整批失败，
  非 strict 模式则跳过，未被文件引用的额外 registration 会忽略。
- 未知业务码若伴随 HTTP 401/403、429 或 5xx，现在返回
  `HttpBusinessError`：`code()` 是 HTTP 状态，`raw_business_code()` 才是限长并脱敏
  的 wire 业务码，分类和可重试性按 HTTP 状态决定；已知业务码仍优先。
- Agent v1 非流式调用、异步结果轮询与会话续接的 `send_via`，以及文件内容流、
  no-clobber 文件发布和最高 24 小时的显式请求超时属于新增 API 能力。文件发布依赖
  目标文件系统支持 hard link；文件本身会 `fsync`，当前不承诺父目录项的掉电持久性。
- `RequestOptions` 可挂到共享连接池的轻量 `ZaiClient` clone，为特定请求配置
  attempt/overall、SSE handshake/idle、较低尝试次数或显式幂等断言；SSE 始终不重放。
  SSE idle 从最近 transport chunk 起按绝对时间计算，不会因调用方暂停 poll 而重置；
  deadline 前已经缓冲的 chunk 仍优先交付。
  Realtime 音视频现在在 WAV/base64/JSON 扩张前取得单会话准入，WAV 编码也不再保留
  一份完整中间容器。
