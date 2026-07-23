# 全面优化路线图

更新日期：2026-07-21  
适用版本：`zai-rs 0.6.x`，其中明确标注的破坏性调整进入 `0.7`

## 目标与原则

本路线图覆盖正确性、安全、传输性能、Realtime、公共 API、测试、发布和文档。
实施时遵守以下边界：

- 先修错误结果、数据风险和发布假绿，再做性能与结构重构。
- `0.6.x` 默认保持源码兼容；需要改变公共类型或模块路径的工作集中到 `0.7`。
- 性能修改必须先有可复现基准，完成后保留回归门槛。
- 默认测试不依赖真实 API key 或外网，网络行为使用本地脚本化服务验证。
- 日志、错误和测试报告不得包含凭据、用户正文、完整 URL 查询或文件内容。

## 当前基线

2026-07-21 审计时已验证：

- Rust `1.88.0`（项目 MSRV）能够构建 workspace。
- `cargo check --workspace --all-features --all-targets --locked` 通过。
- `cargo test --workspace --all-features --tests --locked`：693 项通过。
- `cargo test -p zai-rs --no-default-features --tests --locked`：580 项通过。
- Rustdoc：73 个正向示例和 9 个 `compile_fail` 示例通过。
- all-features Clippy、四个可选 feature 的独立 check、严格 Rustdoc 构建通过。
- crates.io 包为 266 个文件、约 376 KiB 压缩后体积；媒体夹具未进入发布包。

首批已完成的修复：

- 修复 `mcp` all-targets 测试误用字符串模型而导致的编译阻断。
- 修复 Coding Plan Usage 合法 `code: 0` 被统一传输层误判为错误。
- 修复完成态异步聊天任务在 `AsyncTaskResult::status()` 丢失状态。
- 为 MCP 工具发现增加与工具调用一致的超时保护。
- 修复重定向 hop 反复重置 per-attempt deadline；累计延迟回归测试证明仅真正重试会
  获得新 deadline。
- 将五条异步上传路径统一切换到异步文件 metadata 预检，避免在 Tokio worker 上执行
  同步文件系统查询。
- 修正 `mcp_vision` 示例格式，并增加 LF `.gitattributes` 约束。
- 修复 Gitleaks 自定义配置未继承默认规则的发布假绿；完整历史复扫通过，运行时
  canary 检测通过。历史测试占位值仅使用 fingerprint 级豁免。

## 优先级定义

- **P0**：错误结果、凭据风险、数据破坏或发布门禁失效；立即阻断发布。
- **P1**：会影响生产可靠性、资源上限或主要公共能力；当前主线完成。
- **P2**：结构、效率和治理增强；在 P1 稳定后迭代。

## M0：恢复并锁定绿色基线

状态：进行中，预计 0.5–1 人日。

- [x] 修复 all-features/all-targets 编译失败。
- [x] 修复 `code: 0` 与统一异步任务状态。
- [x] 恢复真实 Gitleaks 默认规则并复扫历史。
- [x] 修复 Windows fuzz 源文件换行门禁。
- [x] 修复 redirect hop 重置 per-attempt deadline，并补累计延迟回归测试。
- [ ] 运行最终全量 fmt/check/test/clippy/doc/package 安全检查。

完成标准：工作树中的所有新增改动通过与 CI 等价的本地检查；计划之外没有未解释的
警告或测试跳过。

## M1：HTTP 正确性、超时与错误上下文

优先级：P1，预计 5–8 人日。

1. **拆分超时策略**

   将当前单一 `request_timeout` 拆分为 connect、attempt、overall、SSE handshake、
   SSE idle，并支持受控的 per-request override。取消不可提高的 60 秒上限，保留安全
   默认值和上界校验。

   验收：慢速 100 MiB 本地上传/下载能够通过；REST attempt 与 SSE idle 相互独立；
   paused-time 测试覆盖 deadline、backoff 和边界竞争。

2. **保留完整错误元数据**

   为最终错误保留 HTTP status、原始业务码、`Retry-After`、安全的 request id、attempt
   数和 timeout phase。未知业务码按 HTTP 401/429/5xx 回退分类，而不是丢失状态。

   验收：带未知或文本业务码的 401/429/503 仍正确归类；错误显示和 tracing 不泄露
   header、URL、正文或凭据。

3. **公开可达的请求策略**

   用受控 `RequestOptions` 替代当前公开但不可使用的 `RetryOverride`。POST 只有调用者
   明确声明幂等时才允许重放；SSE 继续禁止自动重放。

   验收：编译级和 mock-server 测试证明 opt-in 会改变 attempt 数，默认行为不变。

4. **补齐正向传输状态机测试**

   增加 GET retry、`Retry-After`、慢 body、redirect 累计 deadline、重试后 MIME/业务
   错误等端到端测试。

## M2：有界流式 I/O 与异步文件路径

优先级：P1，预计 6–10 人日。

1. **真正的流式下载**

   新增 `Stream<Item = ZaiResult<Bytes>>` 路径；`send_to_via` 将受限 chunk 直接写入
   同目录临时文件，完成 fsync 后原子发布。保留缓冲便利 API，并在兼容窗口后修正
   `ByteStream = Vec<u8>` 的误导命名。

   验收：100 MiB 下载额外常驻内存为 O(chunk)；超限、取消和断流都清理临时文件；
   目标文件永不被覆盖；慢消费者产生背压。

2. **SSE 增量、有界解析**

   将 `Vec<Vec<u8>>` 批量产出改成逐事件状态机；分别限制 incomplete line、单事件
   行数、pending 数和 pending 总字节，并补 lone CR/BOM 兼容。

   验收：32 MiB 极小事件恶意输入内存有界；超限只返回一次错误；fuzz 与分配/吞吐
   benchmark 覆盖大量空行、分片和单 chunk 多事件。

3. **异步文件预检（已完成）**

   所有 async 上传入口改用已有 `FilePart::from_path_async`，避免网络盘 metadata 阻塞
   Tokio worker；ASR base64 校验和 multipart 构造减少整包复制。

4. **可选并发预算**

   为 SDK 增加 `max_in_flight` 和 queue timeout；文档示例默认展示有界 fan-out。

## M3：Realtime 双工与背压

优先级：P1，预计 8–13 人日。

1. 将 WebSocket sink/stream 拆成独立 writer/reader task，统一 cancellation 和一次性
   错误上报，避免最长 30 秒 send 阻塞 heartbeat 与服务端事件。
2. 将按“消息条数”的 FIFO 改为按字节预算；控制命令与媒体数据分队列，cancel、commit
   和 close 拥有更高优先级。
3. 引入 `RealtimeTransportConfig`，配置连接、写入、pong、关闭、idle、队列字节和单帧
   推荐上限；仅在发送 `session.update` 前允许安全的首次连接重试。
4. 为公开 `RealtimeTransport` 提供可用的 transport 注入入口，或在 `0.7` 将其收为内部
   抽象。

验收：writer 停滞 20–30 秒时 reader 仍处理 heartbeat；cancel 不受媒体 backlog 影响；
任一 task 失败会关闭另一侧且无悬挂；20 ms 音频帧压测下队列不持续增长。

## M4：响应兼容、工具副作用与能力完整性

优先级：P1，预计 7–12 人日。

1. **Provider 响应前向兼容**

   审核响应上的 `deny_unknown_fields`。先为 untagged union 建立显式判别，再允许叶节点
   接受新增字段；对可扩展字符串枚举使用能够保留原值的开放类型。

   验收：fixture 注入未知字段仍能解析，错误形状和 variant 混淆仍被拒绝。

2. **工具 effect policy**

   在每个工具上声明 cache、idempotency 和 retry policy；默认有副作用且不缓存、不重试，
   只有显式安全工具参与缓存/重试。TTL 使用单调时钟，并评估同 key singleflight。

   验收：全局打开 cache/retry 时副作用工具仍只执行一次；纯函数工具产生可测 cache hit。

3. **Agent 能力承诺对齐**

   当前文档宣称完整 Agent 调用，生产代码却只有 wire 类型。`0.6.x` 要么补三个 facade
   和生产 route，要么明确降级为 contract-only；优先补齐 `send_via`。

4. **MCP 运行时供应链与生命周期**

   在已完成 discovery timeout 的基础上，为 close 增加边界；允许使用预安装可执行文件
   或禁止 `npx` 自动下载，文档明确固定包版本、Node 要求和凭据传递边界。

## M5：0.7 架构与公共 API 收敛

优先级：P2/破坏性，预计 8–15 人日。

- 用单一声明式 model registry 生成模型类型、ID、消息绑定、能力 marker、schema 分组和
  文档快照，消除多处手工真源。
- 抽取 cursor/page pagination primitive 与 query helper，统一公开字段和 builder 策略。
- 收敛 `model` 的实现子模块暴露；澄清 `services::tools` 与 `crate::tool` 的领域边界，旧路径
  先兼容重导出并 deprecate。
- 重新命名 feature：区分 tool executor 与 JSON Schema validation，评估将重依赖从默认
  构建移出。
- 建立迁移指南和 `cargo-semver-checks` 基线，所有破坏性调整只在 `0.7` 合并。

## M6：CI、发布、安全与性能治理

优先级：P1/P2，可与 M1–M5 并行，预计 5–9 人日。

- Feature：each-feature 门禁已完成；继续加入 depth-2 powerset 门禁。
- Coverage：记录 all-features/no-default 基线，设置只升不降的 line/region/function 阈值。
- Platform：Linux、Windows、macOS 跑轻量 check/test；重型 security/coverage 保持 Linux。
- Security：CI 统一使用固定 `cargo-audit`；cargo-deny 覆盖 root 与 fuzz workspace；增加
  `SECURITY.md` 和最小 CODEOWNERS。
- Release：迁移 crates.io Trusted Publishing/OIDC，保留受保护 environment；发布 SBOM、
  校验和与 provenance。
- Compatibility：PR 和 release 增加固定版本 `cargo-semver-checks`。
- Fuzz：提交三个 target 的小型 seed corpus；相关 PR 跑 30–60 秒，日程任务保留长跑；
  crash 必须转确定性回归测试。
- Dependencies：增加 Dependabot/Renovate 的 Cargo 与 Actions 分组更新。
- Docs：`mcp`、`realtime`、`rmcp-kits` 的 `doc(cfg(...))` feature badge 已完成；继续保持
  中英文 README 同步。
- Benchmarks：覆盖 SSE、下载、错误脱敏、endpoint、tool cache 和 Realtime 队列；记录吞吐、
  峰值内存、分配次数和 p95/p99 延迟。

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
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
gitleaks git --redact --no-banner --config .gitleaks.toml
```

涉及 feature、fuzz、coverage、semver 或发布的批次，还必须运行对应专项门禁。真实 API
smoke test 只在受保护环境按需执行，不进入默认 PR 流程。

## 进度维护规则

- 每个 PR 只承担一个可独立回滚的优化主题，并在描述中引用本路线图条目。
- 完成条目时记录基准前后数据、测试名称和兼容性结论；没有数据的“优化”不标完成。
- 若上游协议变化使优先级改变，先更新本路线图和冻结契约，再修改实现。
