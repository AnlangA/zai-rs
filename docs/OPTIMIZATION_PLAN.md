# 全面优化路线图

更新日期：2026-07-24
适用范围：`zai-rs 6.0.1` 发布线；后续破坏性调整进入 `7.0`

## 目标与原则

本路线图覆盖正确性、安全、传输性能、Realtime、公共 API、测试、发布和文档。

- 先处理错误结果、凭据风险、数据破坏和发布假绿，再处理性能与结构。
- 可观察行为变化必须进入合适的版本，并在
  [安全加固迁移说明](HARDENING_MIGRATION.md) 中披露；不以“安全修复”为由隐藏兼容影响。
- 性能结论必须有可复现基准；没有数据的改动只记为结构或有界性改进。
- 默认测试不使用真实 API key 或外网，网络状态机由本机脚本化服务验证。
- 日志、错误和测试报告不得包含凭据、用户正文、完整 URL 查询或文件内容。

## 2026-07-24 发布验证基线

当前工作树已经通过：

- Rust `1.88.0`（MSRV）：workspace all-features/all-targets、no-default、四个独立
  optional feature 和全部六个二元 feature 组合。
- Rust stable `1.97.1`：all-features/no-default check 与 Clippy
  `-D warnings`，以及 fuzz workspace Clippy。
- 测试：all-features 774 项、no-default 636 项，均为 0 失败；其中真实回环
  HTTP/WebSocket 集成测试在允许绑定本机端口的环境运行。
- Rustdoc：75 个正向示例和 10 个 `compile_fail` 示例；严格
  `RUSTDOCFLAGS="-D warnings"` 文档构建通过。
- 所有 workspace examples 构建通过；mdBook `0.5.4` build/test 通过。
- crates.io dry-run：275 个文件，约 2.0 MiB 未压缩、437.0 KiB 压缩；仓库工具配置、
  fuzz、CI 和上游快照未进入包。
- 发布证据链实测：CycloneDX 1.5 全目标 SBOM（298 个组件）、SHA-256 校验、artifact
  路径和 attestation 输入一致；生成 SBOM 前后 crate 字节不变。
- workflow/Dependabot YAML 可解析，外部 Actions 均固定 40 位 commit SHA；
  `bash -n`、浏览器 JavaScript 语法和 `git diff --check` 通过。

本机未重复安装 `shellcheck`、`cargo-audit`、`cargo-deny`、Gitleaks 和 Lychee；
CI 使用固定版本执行这些门禁。真实 API smoke test 仍只允许在受保护环境按需运行。

## 状态总览

| 里程碑 | 当前状态 | 剩余重点 |
| --- | --- | --- |
| M0 绿色基线 | 本轮完成 | 等 CI 在 Linux/Windows/macOS/nightly 再验证 |
| M1 HTTP/错误 | 本轮完成 | 等跨平台 CI 与真实 API smoke test 再验证 |
| M2 流式 I/O | 核心完成 | crash durability、跨文件系统发布、并发预算、RSS 长压 |
| M3 Realtime | 核心可靠性完成 | 有序优先级调度、公开配置/注入、压测 |
| M4 API/工具/MCP | 核心完成 | 7.0 保留未知枚举原值 |
| M5 7.0 架构 | 未开始 | registry、pagination、模块/feature 收敛 |
| M6 治理/发布 | 大部完成 | 长压 RSS/p95/p99 趋势、外部发布配置 |

## M0：恢复并锁定绿色基线

状态：本轮完成。

- [x] 修复 all-features/all-targets 编译失败和 Coding Plan `code: 0` 误判。
- [x] 修复异步任务状态、redirect attempt deadline、MCP discovery timeout。
- [x] 所有异步上传入口使用异步 metadata 预检。
- [x] Gitleaks 配置继承默认规则并保留 canary 门禁。
- [x] fmt/check/test/clippy/doc/examples/mdBook/package 最终门禁通过。

完成标准仍是：计划之外没有未解释的警告、测试跳过或发布包内容。

## M1：HTTP 正确性、超时与错误上下文

状态：本轮完成。

已完成：

- 请求超时保持 60 秒安全默认值，可显式提高到 24 小时；connect timeout 最长 10 秒。
- attempt/overall budget 覆盖 redirect、backoff 和 body 消费，不会在 redirect hop 重置。
- `Retry-After` 支持 delta-seconds 与 IMF-fixdate，并与本地 backoff 取更保守值。
- 未知业务码遇到 HTTP 401/403、429、5xx 时返回 `HttpBusinessError`，保留限长、
  canonical、credential-redacted 的原始业务码，并按 HTTP 恢复语义分类。
- 外部错误、MCP 错误和后加的 error context 都经过限长与脱敏。
- GET retry、redirect deadline、慢 body、错误 envelope 和文件流重试有端到端测试。
- 公开 `RequestOptions` 通过共享连接池的 scoped `ZaiClient` handle 覆盖 attempt、
  overall、SSE handshake/idle 和较低的尝试次数；timeout 可按慢请求放宽但受
  24h/72h hard cap，尝试次数受全局上限。
- `RetryOverride::AssumeIdempotent` 现在公开可达；只有显式断言后 POST 才可 retry
  或跟随同源 307/308，SSE 即使带断言也始终不重放。
- SSE handshake 与 idle timeout 使用不同诊断消息，不再误报为普通 attempt/overall。
- `RequestErrorMetadata` 在透明的 `ZaiError::Request` 包装中结构化保留安全
  request id、实际 attempt 数、timeout phase 和最终 `Retry-After`；原有错误
  分类、错误码、消息和 retry 判断继续透传，默认 `Display`/`Debug` 不显示 request id。

## M2：有界流式 I/O 与异步文件路径

状态：核心完成，耐久性与性能验收未闭环。

已完成：

- `FileContentRequest::stream_via` 返回 pull-based `Bytes` chunk；总响应限制 128 MiB，
  慢消费者产生背压。
- 瞬时错误只在首个可见 chunk 前重试；交付字节后断流直接报错，避免重复可见内容。
- `send_to_via` 写入同目录私有 partial，文件 `fsync` 后以 no-clobber hard link
  发布；并发写者只有一个成功，已有目标永不覆盖，常规错误/取消会清理 partial。
- SSE 使用单事件增量状态机，限制单事件 32 MiB 和 4096 条 data line，覆盖 BOM、
  lone CR、任意分片和一次性终止错误；旧公开 helper 已从分片 O(N²) 修为摊销 O(N)。
- 三个 fuzz target 均有小型 seed corpus，PR/push 运行 45 秒，定时任务运行 10 分钟。

剩余：

- 当前 hard-link 发布要求目标文件系统支持 hard link，且尚未 fsync 父目录；设计
  可移植 no-clobber fallback 和明确的掉电恢复协议。
- `Drop` 中的 best-effort 同步 partial 删除在慢 NFS/FUSE 上可能阻塞 runtime worker；
  评估安全的后台清理与进程重启 scavenger。
- 持有但不 poll 的响应 stream 只能在下次 poll 或 Drop 时释放连接；评估独立 deadline
  驱动是否值得额外 task。
- 增加 SDK 级 `max_in_flight` / queue timeout，并补 100 MiB RSS、SSE 吞吐和分配基准。

## M3：Realtime 双工与背压

状态：核心可靠性完成，调度和配置仍是 P1/P2。

已完成：

- 内建 WebSocket 拆出独立 writer；session 继续读取 heartbeat/事件，不因应用 send
  阻塞 30 秒。
- session 队列和 writer backlog 都有字节预算；消息上限 8 MiB，出站 frame 手工分片
  为最多 2 MiB。
- RFC Pong 使用独立、latest-value 合并控制路径，可插入 continuation frame 之间；
  10 秒绝对 deadline 包含排队时间，单个数据 frame 也最多阻塞 10 秒。
- shutdown 可中断发送；writer 失败会向 reader/session 传播；close 有边界且等待 future
  被取消后仍可继续 join，重复 close 保留同一终态。
- audio → commit → create → cancel 等应用命令保持严格 FIFO，避免 cancel 越过其对应
  create；burst Ping、分片、deadline、permit 和关闭竞态都有回归测试。
- 单会话 preparation semaphore 在 audio/video base64 与 JSON 序列化前准入；事件在
  等待精确字节预算前释放，WAV header+PCM 直接流入 base64 目标，不再分配完整 WAV
  中间 `Vec`。

剩余：

- cancel/commit 若要跨越媒体 backlog，必须实现带单调序号的 typed ordered barrier；
  不能以“高优先级”为由越过更早的 audio、commit 或 create。当前选择正确 FIFO。
- 引入 `RealtimeTransportConfig`（connect/write/pong/close/idle/queue/frame），并设计只在
  `session.update` 前允许的安全首次连接重试。
- 为公开 `RealtimeTransport` 提供稳定注入入口，或在 7.0 收为内部抽象。
- 增加 20 ms 音频帧长压、慢/停读对端和任务失败组合的 RSS/p95/p99 基准。

## M4：响应兼容、工具副作用与能力完整性

状态：大部完成。

已完成：

- Agent/异步聊天响应先显式判别 union，再允许响应叶节点接受 provider 新字段；
  空对象、矛盾状态和错误嵌套形状仍拒绝。
- `TaskStatus::Unknown` 前向兼容，显示和再次序列化稳定。
- 工具默认 `CachePolicy::Never` / `RetryPolicy::Never`；只有 executor 全局开启且工具
  分别声明 `Pure` / `Idempotent` 时才缓存或重试，TTL 使用单调时钟。
- 纯工具同注册代际/规范化参数的并发 cache miss 使用取消安全 singleflight；热缓存
  保持无等待 fast path。等待者取消会清理 gate，clear/按工具 invalidate 通过 epoch
  fence 阻止旧执行回填；按工具失效不影响无关工具，失败仍不缓存、不共享。
- 目录工具保留安全的旧 registry API，并新增绑定本地 handler 与可信 effect policy
  的 `ToolRegistration`；JSON 不能提权，解析、schema 校验、重复/冲突预检与整批提交
  采用两阶段流程，不会留下半批注册。
- Agent v1 非流式调用、异步结果轮询和会话续接都有生产 route 与
  `send_via(&ZaiClient)`。
- Vision MCP 默认不再隐式运行 `npx`；本地运行时/下载必须显式同意。子进程清空继承
  环境后只传 allowlist，discovery/call/close 均有边界，外部错误输出限长脱敏。

剩余：

- 7.0 将开放字符串枚举改为可保留原始值的类型；当前 `TaskStatus::Unknown` 会丢失
  provider 的未知字符串，部分 Agent 枚举仍是 closed enum。

## M5：7.0 架构与公共 API 收敛

状态：未开始，P2/破坏性。

- 用单一声明式 model registry 生成模型 ID、类型、消息绑定、能力 marker、schema 分组
  和文档快照。
- 抽取 cursor/page pagination primitive 与 query helper。
- 收敛 `model` 实现子模块暴露，澄清 `services::tools` 与 `crate::tool` 边界；旧路径先
  重导出并 deprecate。
- 重新命名 feature，区分 tool executor 与 JSON Schema validation，并评估默认依赖。
- 建立 `cargo-semver-checks` 基线；所有破坏性调整只在 7.0 合并。

## M6：CI、发布、安全与性能治理

状态：大部完成。

已完成：

- CI 覆盖 no-default、每个 optional feature、depth-2 feature powerset、stable/nightly、
  MSRV 1.88、Windows 2025 和 macOS 15。
- `cargo-audit`、`cargo-deny`、Gitleaks 和 fuzz 工具固定版本；root/fuzz lockfile 与
  license/source policy 都进入门禁。
- 新增 `SECURITY.md`、CODEOWNERS、Cargo/Actions Dependabot 分组和私有漏洞报告流程；
  GitHub private vulnerability reporting 已确认启用。
- 发布 workflow 要求 annotated tag 与 Cargo version 一致，经受保护 environment 后用
  crates.io Trusted Publishing/OIDC 临时 token 发布。
- 发布产出 `.crate`、全目标 CycloneDX 1.5 SBOM、SHA-256 和 GitHub provenance/SBOM
  attestation；Actions 固定到完整 commit SHA。
- 文档增加安全迁移说明和发布清单；中英文 README 暴露迁移入口。
- `cargo-llvm-cov 0.8.7` 实测 workspace all-features tests 基线为 region 83.14%、
  function 76.11%、line 82.92%；CI 分别设 83.10%/76.10%/82.90% floor，阈值只允许
  上调。
- 独立 CI job 使用 Rust 1.97.1 与固定 `cargo-semver-checks 0.49.0`，在 `6.0.1`
  发布前对 crates.io `0.6.0` 基线运行 all-features 公共 API 检查；当时
  196 pass、57 skip、无 break。后续检查会自动使用最新 registry 版本。
- Criterion `0.8.2` 基准覆盖 SSE 不同分片、1/64 KiB 脱敏、静态/动态 endpoint、
  tool cache hit/miss 和 Realtime 20 ms/64 KiB PCM→WAV→base64。PR 的 all-targets
  门禁只编译基准；每周和手动 workflow 运行完整测量并上传 `target/criterion`，共享
  runner 不设置脆弱的 wall-clock 阈值。

剩余：

- 补下载/100 MiB RSS、慢对端和长压的 p95/p99 趋势；先固定专用 runner 与噪声模型，
  再讨论统计回归阈值。
- 仓库外发布配置与法律信息见下一节。

## 6.0.1 发布确认

发布维护者在 2026-07-24 确认：

1. `LICENSE` 保留现有 `Copyright (c) 2025 Model Context Protocol` 归属。
2. 新版本选择为 `6.0.1`；前三项可观察行为变化已在
   [安全加固迁移说明](HARDENING_MIGRATION.md) 中显式披露，并通过新的主版本线表达。
3. GitHub `crates-io` environment 已创建且仅允许 `v6.0.1` 标签部署。crates.io
   Trusted Publisher 必须精确匹配 `AnlangA/zai-rs`、`release.yml` 和 environment
   `crates-io`；该私有配置由正式 tag workflow 的 OIDC 认证作最终验证。

## 每批改动的统一验收

```bash
cargo fmt --all -- --check
cargo fmt --manifest-path fuzz/Cargo.toml -- --check
cargo check --workspace --all-features --all-targets --locked
cargo check -p zai-rs --no-default-features --all-targets --locked
cargo test --workspace --all-features --tests --locked
cargo test -p zai-rs --no-default-features --tests --locked
cargo test --workspace --all-features --doc --locked
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
cargo clippy --manifest-path fuzz/Cargo.toml --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
mdbook build
mdbook test -L target/debug/deps
cargo publish --dry-run --locked -p zai-rs --all-features
```

涉及 feature、fuzz、coverage、semver 或发布的批次，还必须运行对应专项门禁。
涉及热点实现的批次还应运行
`cargo bench --locked -p zai-rs --features realtime,toolkits --bench hot_paths -- --test`
进行轻量 smoke；完整性能报告由定时或手动 benchmark workflow 生成。

## 进度维护规则

- 每个 PR 只承担一个可独立回滚的主题，并引用本路线图条目。
- 完成性能条目时记录基准环境、前后数据和回归阈值。
- 若上游协议变化影响优先级，先更新冻结契约与路线图，再修改实现。
