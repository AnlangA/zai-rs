# 全面优化路线图

更新日期：2026-08-11
适用范围：未发布的 `zai-rs 6.0.1` 候选版及其 tag 之后的工作树；后续破坏性调整进入 `7.0`

## 目标与原则

本路线图覆盖正确性、安全、传输性能、Realtime、公共 API、测试、发布和文档。

- 先处理错误结果、凭据风险、数据破坏和发布假绿，再处理性能与结构。
- 可观察行为变化必须进入合适的版本，并在
  [安全加固迁移说明](HARDENING_MIGRATION.md) 中披露；不以“安全修复”为由隐藏兼容影响。
- 性能结论必须有可复现基准；没有数据的改动只记为结构或有界性改进。
- 默认测试不使用真实 API key 或外网，网络状态机由本机脚本化服务验证。
- SDK 生成的 validation 文本、明确标注为脱敏的 `Debug` 实现和测试报告不得主动写入
  凭据、用户正文、完整 URL query 或文件内容；`ZaiError` 的 `Display` / `Debug` /
  `message()` / `compact()` 可能包含 provider message，显式 `raw()` payload 也仍是敏感
  应用数据。默认日志只记录安全的结构化分类，不直接渲染完整错误。

## 版本与发布事实

- `Cargo.toml` 仍声明 `6.0.1`，但该版本是未发布候选版。
- `v6.0.1` 的两次发布 workflow（runs `30091854390` 和
  `30092565276`）均以 `failure` 结束。原始 step 日志已确认：首次运行的当时
  tag 不是 annotated tag；第二次已通过质量、打包、SBOM、校验与两份
  attestation，但 crates.io 因没有 `AnlangA/zai-rs` Trusted Publishing 配置而
  拒绝 OIDC 鉴权。
- `cargo search` 和 `cargo info` 验证 crates.io 最新版仍为 `0.6.0`；该
  legacy 版本的 API 与当前工作树存在差异。
- 当前工作树已偏离既有 `v6.0.1` tag。不能移动或复用该 tag；下一次
  正式发布必须先把 Cargo 版本提升到高于 `6.0.1`，再创建新的 annotated tag。

## 2026-07-24 候选版时点的验证记录

以下是 2026-07-24 当时候选工作树的历史记录，不是对 2026-08-11
当前工作树的验证，也不能作为 `6.0.1` 已发布的证据。当时的候选工作树通过：

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
- 发布证据链实测：CycloneDX 1.5 全目标平台、all-feature SBOM（298 个组件）、
  SHA-256 校验、artifact 路径和 attestation 输入一致；生成 SBOM 前后 crate 字节不变。
- workflow/Dependabot YAML 可解析，外部 Actions 均固定 40 位 commit SHA；
  `bash -n`、浏览器 JavaScript 语法和 `git diff --check` 通过。

本机历史记录未重复安装 `shellcheck`、`cargo-audit`、`cargo-deny`、Gitleaks 和 Lychee。
CI bootstrap 清单中的 Rust/安全/文档工具精确 pin；ShellCheck 与 Node 使用
`ubuntu-24.04` runner 的预装版本，可能随 runner image 更新。真实 API smoke test 仍只允许
在受保护环境按需运行。

## 2026-08-10 至 2026-08-11 增量加固（未发布）

本批改动必须以高于 `6.0.1` 的版本发布；当前未修改 crate 版本，也未宣称已经发布。

- HTTP loopback endpoint 使用独立的 `no_proxy` 连接池。即使下游进程设置了
  `HTTP_PROXY` / `ALL_PROXY` 且清空 `NO_PROXY`，本地明文请求及其 Authorization
  header 也不会被系统代理转发；混合 public/local 配置仍只对 loopback 选直连池。
- 非 2xx 响应只保留 64 KiB 诊断前缀。超大、停滞或中途断开的错误页不再掩盖
  401/429/5xx 状态和 retry 语义；只有在 cap/deadline 前读到 EOF 的完整诊断才解析
  business envelope。被截断、超时或中断的前缀不参与 body-derived
  code/request-id/message 投影，
  retry 与分类只看 HTTP status，并返回不含 body 的静态消息；flat `0` / `200` success
  code 也不会覆盖非成功 HTTP status。
- 服务错误中的递归 JSON object/array、组合 credential 字段、string/object key
  凭据、一层额外 JSON-string 编码、单引号对象片段、未加引号值及 EOF 截断字段均会
  fail closed 脱敏；无敏感字段的常见路径避免重复全量复制。结构化输出明确只用于
  安全诊断，不承诺保留 JSON 空白、member 顺序或重复 member。
- 公开 `SseEventParser` 新增 fallible `try_push` / `try_finish`，并与生产流统一执行
  32 MiB / 4096 `data:` line 限制。旧 `push` / `finish` 也已限界，超限时释放全部
  parser buffer 并返回空结果；该可观察加固已记录在迁移说明。
- `prelude` 公开主路径补齐 `ApiFamily`、`HttpConcurrencyConfig`、`RequestOptions`、
  `RetryOverride`；HTTP transport fluent API 补齐 compression 开关和校验型
  `try_build()`。
- Chat 的 stream-independent builder 已提升到通用 type-state impl，
  `.enable_stream().with_temperature(...).add_message(...)` 等自然链式顺序可用；
  StreamOn 同时提供公开 `validate()` 预检，capability bounds 保持不变。
- source-only Network/JSON/Realtime error 的 `.context()` 不再静默丢失操作信息；新增
  `ZaiError::Context` 透明包装并保持分类、重试、code、metadata 和 compact 前缀。
- Chat SSE 对未来字符串 tool-call type 降级为 `None` 并继续读取后续文本与 `[DONE]`；
  Realtime session 则只对经过完整 shape 复验的候选嵌套枚举发出
  `UnsupportedKnown`；live-session 的 known/unknown fast path 与兼容 fallback 都先递归
  拒绝重复键，音频格式和 malformed sibling 仍 fail closed。
- Batch/File/Embedding 的可选 object marker、web-search intent 与图片过滤阶段
  对未来字符串降级为 `None`，保留仍可理解的列表、文件、向量、搜索结果与
  生成图片 payload；非字符串坏类型及公开 enum 的直接反序列化继续严格。
- 2026-08-11 重新拉取官方 OpenAPI 后，53 个 path 与 59 个 operation 均未漂移；
  moderation 新增的 `BLOCK` / `HIGH` 处置语义已进入 `RiskLevel`，不再都压成
  `Unknown`。Realtime AsyncAPI 与 2026-07-11 冻结快照逐字节一致。
- SDK 生成的 Realtime HTTP 握手错误只保留 status、从唯一 `Content-Length` 可证明完整且
  无 `Transfer-Encoding` 的限长 JSON 中取得的 business code，以及有效 `Retry-After`；
  Tungstenite 的不完整 body tail 不参与策略，原始 response headers/body 不进入公开
  Debug/source chain。公开 retry
  投影与内建连接重试共用业务码优先规则和瞬时 I/O allowlist。
- typed MCP request 与 Realtime `SessionBuilder` 现在都有公开、零网络的
  `validate()` 预检入口。
- Realtime 新增 additive `SessionBuilder::build_with_transport`，接收已连接、已认证的
  transport，且不会把 SDK API key、JWT、Authorization header 或配置 URL 传给它。
  SDK 首个应用消息 `session.update` 必须 confirmed 后 build 才返回；常规 send、close、入站
  message/idle 继续受 session 级硬边界保护。`send_confirmed` 带有委托到 `send` 的默认
  实现，既有只实现 `send` / `recv` / `close` 的 transport 保持源码兼容，buffered
  transport 则必须覆写确认语义。
- Realtime 新增规范路径 `zai_rs::realtime::RealtimeTransportConfig`，集中校验 12 个
  timeout/queue/buffer/frame knob；`Default` 沿用旧主要数值，同时以默认 30 秒
  outbound admission 总期限取代无限等待，并把单 data-frame stall 从 10 秒收紧到
  5 秒；内建 builder 还会在 `session.update` 前默认最多尝试连接 3 次。client 默认由
  session builder 快照且可按单会话覆盖。内建与 injected
  transport 的适用边界、固定 byte/media cap 和派生 deadline 均已形成公开契约；只有
  builder 创建的内建会话消费全部 12 项，直接 connect 始终单次且只消费 wire 侧设置，
  其余值保留供 getter 检查。内建所有尝试、退避和有效 `Retry-After` 共享
  `connect_timeout` 总预算，每次尝试刷新 JWT；进入 initial update 后不再重放。
- Realtime 内建 writer 以显式 Control/Data 偏好轮转消除双向饥饿：反馈式 Ping 在每次
  Pong 后补入下一帧时，late-arriving data 仍会因 data 偏好与主动 yield 有界推进；持续
  data backlog 下 Pong 也会推进。shutdown 保持最高优先级，应用消息仍按 FIFO 完整写出。
- HTTP 新增共享逻辑请求准入：默认 64 个 in-flight、30 秒 queue timeout；buffered
  请求跨全部 retry/backoff 持有 permit，SSE/文件 stream 则持有到结束、Drop 或安全
  lease 到期。SSE configured base 默认 5 分钟，先取 scoped/global 较小值，再应用
  `effective=max(base,sse_idle+1s)` floor；文件复用 absolute overall；
  到期原子回收完整 response body 与 permit。`HttpConcurrencyConfig` 提供受检全局策略，
  scoped `RequestOptions` 只能缩短 queue deadline 或降低 consumer base，后者可能因
  idle floor 而不改变 effective；queue timeout 独立报告 `TimeoutPhase::Queue` 与零次
  HTTP attempt。
- 文件下载的 no-clobber 发布在 hard link 明确不受支持时使用平台相关 fallback；相对
  目标在创建目录前固定为 absolute path。Unix 会预存 lexical parent chain，并在发布后
  从直接父目录 deepest-first fsync 每个新建祖先直到首个预存 anchor；稳定 namespace 下
  成功覆盖文件、目标目录项与本次新建的每级目录项。目录链同步失败返回 `SDK_IO`，但不
  回滚完整目标，即 published-but-durability-unconfirmed。Windows 不承诺目录项掉电存活，
  并发替换 path component 也不在该 lexical 协议保证内。
- 完整文件 GET 只接受不带 `Content-Range` 的 `200 OK`；意外的 `206` / `204` 或
  ranged-looking `200` 在首字节前 fail closed，不能再把部分内容或无内容响应原子发布成
  “成功”的完整文件。合法的 `200` 空文件仍保持可用。
- SSE handshake 只接受不带 `Content-Range` 的 `200 OK`，并继续要求
  `text/event-stream`；`206` 即使包含合法 `[DONE]` / typed `done`、`204` 或
  ranged-looking `200` 也不能建立 typed stream。无效 2xx 仅凭 status/header、在 poll
  body 前失败；非 2xx 仍保留有界错误投影、敏感值脱敏、request metadata、
  `Retry-After` 与不重放语义。
- buffered JSON/binary 响应严格匹配 frozen route 的 `success_statuses`；当前 59 个操作均为
  `200`。未声明的任意其他 `2xx` 以及完整响应上的 `Content-Range` 会在 poll body 前以
  静态协议错误失败、不进入重试；route registry 逐项与 frozen contract 对照，gated-body
  回归覆盖 `201` / `202` / `204` / `206` / `299`、partial audio 与 ranged-looking `200`。
- buffered 与 file redirect 对 cross-origin、TLS downgrade、非法 `Location`、hop-limit
  的拒绝直接保留静态 policy error；不再吞掉拒绝原因后降级成泛化 `3xx`，且诊断不回显
  `Location`。允许的 buffered redirect 若目标在返回 headers 前失败，会保留最近 3xx 的
  request ID / `Retry-After`，而不是沿用更早 retry response 的 stale metadata。
- partial Drop 清理改为进程级最多 8 个 queued/running blocking job；无 runtime、预算
  饱和或无法建立 guarded path 时同步兜底，不建立无界队列。所有删除仍是 best-effort；
  SDK 不做 startup scavenger，残留 `.zai-dl-*.part` 由应用 retention policy 协调。
- 公开 `pagination::{CursorPagination, PagePagination}` 已统一非零值、opaque cursor
  校验/`Debug` 脱敏，并通过 request 的 `try_with_pagination` 执行具体 endpoint 上限。
  类型不直接实现 `Serialize`，避免把 `limit`、`size`、`page_size` 等不同 wire schema
  错当成一个通用对象。
- Linux 100 MiB 文件下载改用独立 release 子进程的 `VmHWM` 增量门禁；两次本机结果为
  972 KiB / 1148 KiB（上限 32 MiB），完整文件与无 partial 同时验证。SSE 另有七组
  allocator census；`try_push` 空结果不再预分配返回 `Vec`，8 MiB / 64 B 分片从
  131,077 次 allocation 降为 5 次，4096 行事件进一步从 4100 次降为 3 次。另一个
  business-error census 同时锁定顶层/嵌套 reserved `code` 的 2 MiB numeric array，
  以及 32 MiB numeric literal、普通/转义 string 与深嵌套 code 仍为常数级保留；raw
  literal 在 Serde 建立 number/string/nesting scratch 前即受限，不会被 probe 展开成
  巨型 `Value` 或临时存储。
- Realtime outbound media 由 stack WAV header/raw PCM/JPEG 直接 base64 写入精确容量的
  final JSON；640 B、64 KiB、4 MiB 的 WAV/PCM16/PCM24/JPEG 普通测试门禁锁定 wire 与
  public Serde 逐字节相等、最多 2 次 allocation 且 0 reallocation。4 MiB 路径从约
  16.8 MiB allocation traffic 降至约 5.6 MiB。
- ToolExecutor 在默认不可重试与最后一次 attempt 直接 move 自有 JSON input，只在确有
  未来幂等 retry 时 clone；65,536 字符串的普通隔离分配门禁拒绝 payload-sized copy，
  三次 retry 回归同时证明每次值一致且最终 attempt 接管原值。
- Realtime 新增满 64 条应用队列、阻塞第三方 wire task 的 paused-clock injected-contract
  stress，证明 inbound 双向推进、broadcast lag 可见、permit 全归还、16 轮
  media/control barrier 严格 FIFO 与有界 close；另有 capacity=1 的内建私有 adapter
  回归和 blocked-sink writer 回归，分别锁定 end-to-end permit 转移与完成后归还。
- Realtime 另有默认 ignored 的真实时钟 soak：本地 5 秒、每周 release 300 秒，
  用 20 ms PCM 产生器和慢/停读对端记录 admission/inter-arrival/Pong RTT
  p50/p95/p99/max 与 Linux `VmHWM`；机器可读 JSONL 作为独立 artifact，硬断言只覆盖
  queue/byte 上界与归还、FIFO、反馈 Ping 下进展、失败传播和有界 teardown。
- 33 个公开 model marker 的类型名、wire ID 与能力投影已由私有声明生成：
  23 个 Chat/Vision/Voice、8 个 Image/Video/ASR/TTS/VoiceClone HTTP endpoint marker，
  以及 2 个跨 feature 的 Realtime marker。checked-in snapshot 冻结各族投影，公共
  类型路径、Serde 与 sealed capability 保持不变。

本批回归覆盖：恶意 system proxy、超大 401/429/503 和 SSE handshake、截断 quota
JSON、flat success code、dense numeric success JSON、quoted/truncated/encoded credential、公开与生产 SSE parser
的资源/分片边界、source-only error context、完整 operation contract projection，以及
Chat/MCP/Realtime 的公开 validate、Chat/Realtime 前向兼容、Realtime 安全握手摘要、
Realtime transport 注入与 HTTP admission 边界、Knowledge 完整 success envelope，
以及文件发布、取消竞态和有界
deferred cleanup；新增资源门禁还覆盖 SSE allocation、100 MiB 下载 RSS、Realtime
满队列顺序、model registry 投影，以及 ZRAG retrieve/chat 的 schema、validation、
operation secret、逐事件 backpressure 与终止语义。2026-08-11 最终本机门禁结果：

- workspace all-features/all-targets：1052 项通过、2 项按设计 ignored（其中 root crate
  1035 项通过、2 项 ignored），另有 13 个 libtest benchmark smoke、7 组独立 SSE
  allocation census 与 6 组 business-error probe census；no-default：838 项通过、
  1 项 ignored，另有同样 13 组 deterministic census；
  ignored 项恰好是只在 release/weekly 精确运行的 Linux 100 MiB RSS gate 与
  feature-gated Realtime soak（no-default 下只有前者）；
- rustdoc：80 个正向示例和 11 个 `compile_fail` 示例，严格 `-D warnings` 通过；
- workspace all-target/all-feature 与 no-default Clippy、MSRV 1.88 all-feature check、
  root/fuzz fmt、fuzz Clippy 和 `git diff --check` 全部通过；
- mdBook build/test 通过；实际 `.crate` 通过不超过 400 个 regular files、2 MiB gzip、
  8 MiB tar 的内容策略与必需合同/允许路径检查；精确文件数和字节数不写入被打包文档，
  避免证据自引用。`cargo publish --dry-run` 完成验证且未上传；
- 相对 `v6.0.1` 的 stable semver-checks 在 default features 与 all features
  时均为 223 pass、30 skip，无 source-semver violation。

这些是当前工作树的本机证据，不替代跨平台 CI、真实 API smoke test 或正式发布。

## 状态总览

| 里程碑 | 当前状态 | 剩余重点 |
| --- | --- | --- |
| M0 绿色基线 | 本轮完成 | 等 CI 在 Linux/Windows/macOS/nightly 再验证 |
| M1 HTTP/错误 | 本轮完成 | 等跨平台 CI 与真实 API smoke test 再验证 |
| M2 流式 I/O | 资源上界与核心可靠性完成 | Windows durability、跨 runner 长期趋势 |
| M3 Realtime | writer 公平、满队列顺序、公开配置与长压证据完成 | 跨周/跨 runner 趋势积累 |
| M4 API/工具/MCP | 核心完成 | 7.0 保留未知枚举原值 |
| M5 7.0 架构 | pagination 与全部当前 marker registry foundation 已提前完成 | 模块/feature 收敛 |
| M6 治理/发布 | 工程与失败根因审计完成，发布未闭环 | crates.io 配置、新版本/tag、发布后验证、趋势积累 |

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
- buffered JSON 的 business-error probe 改为 streaming top-level visitor：未知的大型
  embedding/rerank 数组用 `IgnoredAny` 跳过，只保留 error/code/message/request-id，且
  final response 缓存首次 probe 与 fallback code 结果，typed `json()` / `bytes()`
  不再二次扫描。同一约 2 MiB、1,048,576 数字的成功 payload 探测为
  0 allocation / 0 reallocation / 0 byte。
  reserved `code` 本身也采用 bounded scalar/shape visitor：顶层与嵌套约 2 MiB numeric
  array 均只保留空 array sentinel，32 MiB numeric literal、普通或转义 string 在完整
  解码前只保留固定 sentinel；超过 128 层的 JSON 在进入 RawValue 前归为 malformed，
  避免 provider 可控字段在 32 MiB body 上放大为数百 MiB `Value` 或 scratch。
  内部 probe 还会把顶层 reserved fields 与直接 `error` 内 code/message 的重复标为
  ambiguous：2xx 静态 fail closed，非 2xx 只按 status 分类/重试，冲突 body 不进诊断；
  完整但 malformed 的非 2xx JSON 同样不贡献业务码或正文诊断。
- 公开 `RequestOptions` 通过共享连接池的 scoped `ZaiClient` handle 覆盖 attempt、
  overall、SSE handshake/idle、较低的尝试次数和只能收紧的 admission queue
  deadline；timeout 可按慢请求放宽但受 24h/72h hard cap，尝试次数受全局上限。
- `RetryOverride::AssumeIdempotent` 现在公开可达；只有显式断言后 POST 才可 retry
  或跟随同源 307/308，SSE 即使带断言也始终不重放。
- SSE handshake 与 idle timeout 使用不同诊断消息，不再误报为普通 attempt/overall。
- `RequestErrorMetadata` 在透明的 `ZaiError::Request` 包装中结构化保留安全
  request id、实际 attempt 数、timeout phase 和最终 `Retry-After`；原有错误
  分类、错误码、消息和 retry 判断继续透传，默认 `Display`/`Debug` 不显示 request id。
- 回环 HTTP endpoint 永远使用 proxy-free pool，系统代理不能把本地信任边界变成远端
  hop；非 2xx 错误页只保留有界诊断前缀，完整 envelope 才能贡献业务码，不完整前缀
  只保留 HTTP status 的 retry/分类语义。
- provider 错误消息中的严格 JSON、日志式单引号和截断 credential 字段统一脱敏。
- `ZaiError::context` 对 Network/JSON/Realtime 这类 source-only 变体使用透明
  `Context` 包装保留操作信息；分类、重试、code、request metadata 与原始 source
  继续透传，重复 context 扁平化。

## M2：有界流式 I/O 与异步文件路径

状态：核心完成，耐久性与性能验收未闭环。

已完成：

- `FileContentRequest::stream_via` 返回 pull-based `Bytes` chunk；总响应限制 128 MiB，
  慢消费者产生背压。
- 完整对象下载只接受不带 `Content-Range` 的 `200 OK`，拒绝会导致静默截断的
  unsolicited `206`、`204` 与 ranged-looking `200`；合法 `200` 空文件不受影响。只有不带
  `Content-Range` 的 `200` JSON business-error envelope 会在 header 完整性检查后保留业务码
  与 retry 语义；其他 2xx 或任何带 `Content-Range` 的响应均在 poll body 前失败。
- SSE 建流只接受不带 `Content-Range` 的 `200 OK`，且继续校验 `text/event-stream`；
  chat、音频与 ZRAG decoder 均覆盖 `206` 携合法终止标记、`204`、ranged-looking `200`
  的拒绝，以及正常 `200` 回归。无效 2xx 会在 poll body 前立即失败，只有非 2xx 才读取
  有界诊断 body 并投影业务错误。
- 普通 buffered JSON 与 binary 返回同样执行 route 声明的精确成功状态及无
  `Content-Range` 完整响应约束；错误状态只基于 header 即时拒绝，不读取慢 body，也不因
  GET 的 retry budget 自动重放。
- 瞬时错误只在首个可见 chunk 前重试；交付字节后断流直接报错，避免重复可见内容。
- `send_to_via` 在创建目录前把相对目标固定为 absolute path，再写入同目录私有
  partial 并执行 file sync。发布首选 no-clobber hard link；文件系统明确不支持时使用
  平台相关 no-clobber fallback，并发写者仍只有一个成功、已有目标永不覆盖。
- Unix 在创建 parent 前记录 lexical chain，发布后按直接父目录到首个预存 anchor 的
  deepest-first 顺序同步；稳定 namespace 下，本次 `create_dir_all` 新建的每级目录项都
  纳入成功契约。任一 sync 失败返回 `SDK_IO`，但完整目标已发布且不回滚，该
  published-but-durability-unconfirmed 状态已纳入公开契约和回归测试。并发替换 path
  component 不在 lexical 协议保证内。
- Linux 以独立 release 子进程执行 100 MiB `send_to_via` RSS gate：父进程持有 loopback
  server 并用固定 64 KiB buffer 校验完整文件，子进程只执行下载并从 `/proc/<pid>/status`
  记录前后 `VmHWM`。增量上限为 32 MiB；JSON 同时记录 elapsed/throughput 但不设耗时
  阈值。测试默认 ignored，只由手工或每周 benchmark workflow 精确运行。
  workflow 会校验唯一 `zai-rs.file-download-rss.v1` JSON 行并把 JSONL/完整日志作为
  带 `run_id` / `run_attempt` 的独立 artifact 保存，供跨周趋势分析。
- partial Drop 会先关闭 SDK 文件句柄，再以全局预算把最多 8 个 queued/running 删除
  延后到 Tokio blocking pool；无 runtime、预算饱和或 guard 构造失败时同步兜底，避免
  无界 cleanup backlog。删除错误不从 Drop 传播；为避免启动时扫描/误删应用目录，SDK
  明确不实现 startup scavenger，异常残留交由应用自己的 retention policy 处理。
- SSE 使用单事件增量状态机，限制单事件 32 MiB 和 4096 条 data line，覆盖 BOM、
  lone CR、任意分片和一次性终止错误；旧公开 helper 已从分片 O(N²) 修为摊销 O(N)。
- 同一事件的 `data:` 字段直接追加到单一连续 payload buffer，并用独立 line counter
  区分“没有 data field”和“空 data field”；避免逐行 `Vec` 所有权和派发时整事件 join
  copy，同时保持 CR/LF、空行、32 MiB / 4096 行边界语义。
- 已完整消费的超大 comment/未知字段行不再通过 `Vec::clear()` 把接近 32 MiB 的 scratch
  capacity 钉在长连接中；超过两个 parse slice 的临时 buffer 会立即降容，parser 随后仍
  可正常复用。
- 公开 parser 的 `try_push` / `try_finish` 暴露限界错误；旧 Vec-returning 方法也
  fail closed 并在超限时清空 retained state，fuzz target 直接覆盖 fallible API。
- 独立 `sse_allocations` census 覆盖 64 KiB / 8 MiB 单行 payload 与 64 B / 1 KiB /
  64 KiB transport chunk，并增加 4096 行、仍可被 JSON decoder 接受的最大行数事件；
  使用 allocator 计数而非 wall-clock 作为硬门禁。它发现并
  修复了 `try_push` 空结果路径每次预分配返回 `Vec` 的问题：最坏的 8 MiB / 64 B
  分片从 131,077 次 allocation 降为 5 次；连续 event buffer 又把 4096 行事件从
  4100 次 allocation 降为 3 次，并让单行场景降至 4 / 5 次。七组场景均由宽松的常数/
  对数增长及 payload-byte budget 防回归。
- 三个 fuzz target 均有小型 seed corpus，PR/push 运行 45 秒，定时任务运行 10 分钟。
- `HttpConcurrencyConfig` 为同一 client 的所有 clone 建立共享准入预算；默认
  `max_in_flight=64`、queue timeout 30 秒。buffered 请求跨重试/退避持有 permit，
  SSE/文件 stream 持有到终止、Drop 或安全 lease 到期；排队取消安全，超时以零
  attempt 的独立 `TimeoutPhase::Queue` 报告。持有但不 poll 的 SSE 默认由 5 分钟
  configured base 派生 consumer lease，只有底层 raw-stream poll 实际取得 chunk 时续期；
  typed decoder 从已缓冲 raw chunk 产出 item 不一定续期。先取
  `base=min(scoped,global)`，再取 `effective=max(base,sse_idle+1s)`；scoped override
  只能降低 base，idle floor 可能使它成为 no-op。base 最大 24 小时，effective 最大
  24 小时加 1 秒，并以 `TimeoutPhase::StreamConsumer` 报告。
  文件 stream 不另设 consumer 计时，复用既有 absolute overall deadline 并报告
  `TimeoutPhase::Overall`。reaper 原子 take/drop 整个 raw body 与 permit、唤醒消费者；
  retained stream 只 yield 一次 timeout 后终止，调用方 Drop 仍立即回收。

剩余：

- Windows/其他 non-Unix 在 stable Rust 下仍没有可移植的 directory-sync 保证；若要
  扩大跨平台掉电持久性契约，需先有可测试的平台协议。
- 在固定专用 runner 建立 SSE 吞吐/分配的跨版本长期趋势；共享 runner 继续只 gate
  确定性的资源上界，不 gate wall-clock。

## M3：Realtime 双工与背压

状态：wire writer 公平、满队列应用顺序、核心可靠性、公开配置与可重现长压证据完成。

已完成：

- 内建 WebSocket 拆出独立 writer；session 继续读取 heartbeat/事件，不因应用 send
  阻塞 30 秒。
- 内建 session 从公开准入到 writer 完成共享一个不可配置的 8 MiB 端到端字节预算；
  直接 `TungsteniteTransport` writer 另有 8 MiB 预算。消息上限固定为 8 MiB，出站
  frame 按配置在 64 KiB..=2 MiB 间手工分片（默认 2 MiB）。
- RFC Pong 使用独立、latest-value 合并控制路径，可插入 continuation frame 之间；
  默认 10 秒绝对 deadline 包含排队时间。Tungstenite 在读到 Ping 时生成
  唯一 automatic Pong，SDK control lane 只在 deadline 内 flush 该共享状态，不再
  另造一个 Pong；因此 automatic/explicit 两个发送源不会产生重复或旧世代
  payload。单个 data frame 不再直接共用 Pong
  deadline，而以 `min(5s, pong / 2)` 检测停滞，同时完整消息继续受 write deadline
  限制。
- writer 使用显式 Control/Data 偏好：成功 control 后切到 data 并主动 yield，确保对端
  每次 Pong 后立即反馈下一次 Ping 时，已排队或随后到达的 data 仍有界推进；完成 data
  message 后重新偏好 control，持续 data backlog 不会饿死 Pong。shutdown 在两态均为
  最高优先级，应用 message FIFO 与分片不交错不变量保持不变。
- shutdown 可中断发送；writer 失败会向 reader/session 传播；close 有边界且等待 future
  被取消后仍可继续 join，重复 close 保留同一终态。
- audio → commit → create → cancel 等应用命令保持严格 FIFO，避免 cancel 越过其对应
  create；burst Ping、分片、deadline、permit 和关闭竞态都有回归测试。
- paused-clock injected-contract stress 会先填满 64 条 session command queue，再让
  第三方 transport 的独立 writer
  停在显式 barrier；其间 19 条 inbound event/audio 仍精确推进，两个 8 槽 broadcast
  第 9 条均显式报告 lag，command capacity、8 MiB byte permits 与 preparation permit
  全部归还。释放 writer 后 16 轮 audio → commit → create → cancel 及 payload 保持完整
  FIFO，close 只执行一次并有界 join writer。内建路径另用真实私有 adapter 将 byte/count
  permit 转交到受阻 writer ownership，并证明第二条命令只在公开 admission 等待、不会因
  私有 writer `Full` 杀死 session；writer 释放后两类 permit 都可复用。这些测试只用确定性
  进展 watchdog，不把机器性能写成 wall-clock 阈值。
- 单会话 preparation semaphore 在 audio/video base64 与 JSON 序列化前准入；事件在
  等待精确字节预算前释放，WAV header+PCM/raw PCM/JPEG 直接流入精确容量的最终 JSON，
  不再分配完整 WAV 或 base64 中间 String，也不再扩容 JSON buffer。
- 规范路径 `zai_rs::realtime::RealtimeTransportConfig` 公开 12 个经统一校验的主
  knob：内建 connect attempt 上限，connect/write/Pong/close/idle/outbound-admission
  deadline，四类 queue/buffer capacity 和 frame bytes。`Default` 沿用旧主要数值，但新增
  默认 30 秒 outbound admission 总期限，把单 data-frame stall 默认值从 10 秒收紧到
  5 秒，并为内建会话启用默认最多 3 次首次连接尝试。client default 在创建 session
  builder 时快照，builder 可为单会话完整覆盖，session 暴露最终有效值。
  非零 outbound deadline 覆盖 preparation、media/JSON 构造、byte-budget 与 channel
  准入，零值使所有竞争准入 fail-fast。8 MiB message/内建端到端 byte cap、直接 transport
  的 8 MiB writer cap、4 MiB raw media 与单并发 preparation 仍是不可抬高的安全上限。
  内建消息数取 outbound/writer capacity 的较小值，byte/count permit 会一直跟随已接受
  command 到 socket writer 完成，消除了公开成功后私有第二层准入失败的窗口。
- confirmed/普通 send、注入路径 initial update、writer join 与 session close 不作为
  互相独立的 setter：前两类普通写保护从 write 派生 `+1s`，注入 initial 外层保护派生
  `+2s`，writer join / session close 分别从 close 派生 `+1s` / `+2s`；所有派生值与
  交叉约束在建连前统一验证。
- `SessionBuilder::build_with_transport` 为已连接、已认证的公开 transport 提供稳定
  注入入口，不向其传递或校验 SDK 凭证；`session.update` 作为 SDK 首个应用消息经
  `send_confirmed` 确认后才构造会话。注入路径的普通 send、close、入站消息与 idle
  以及 session 自有 queue/buffer 均有硬边界；connect/Pong/frame/writer 参数只适用于
  内建 Tungstenite，不会传给注入实现。新增默认方法保持既有三方法
  `RealtimeTransport` 实现源码兼容；旧 `connect` 和 `.session(...).build()` 采用
  `Default`，显式直连另有 `connect_with_config`。
- 只有 builder 的内建 Tungstenite 路径会在首个 `session.update` 前重试可恢复的连接
  失败；`max_connect_attempts` 默认 3、范围 `1..=3`。全部尝试、full-jitter 退避和有效
  `Retry-After` 共享 `connect_timeout` 绝对总预算，JWT 模式每次尝试重新签发凭证；initial update
  一旦开始写入就不重放。direct transport 始终单次，injected transport 已连接且不适用。
- SDK 生成的 HTTP 握手拒绝在进入公开错误链前压缩为只含 status、从 framing 可证明完整的
  限长 JSON 中提取的 business code 与有效 `Retry-After` 的
  `RealtimeHandshakeHttpContext`；不完整 Tungstenite tail 只按 status 分类，原始 headers/body 不进入
  `Debug` 或 source chain。公开 `is_retryable()` 与内建连接重试共享同一 HTTP/business
  判定及瞬时 I/O allowlist，TLS、URL、protocol、capacity 与 malformed-data 均 fail closed。
- Tungstenite 内部 write buffer 从无限默认值收紧为配置 frame 上限的两倍（默认且最大
  4 MiB）；入站 audio delta 在 base64 解码分配前先检查编码长度，避免超限 payload
  再额外分配完整 decoded buffer。
- 独立 `realtime_soak` 在普通矩阵中只编译且 ignored，手工/每周精确运行。
  它先确定性验证 64 条 message queue、两个展开后 4 MiB media 对 8 MiB byte
  budget 的拒绝/归还、audio → commit → create → cancel FIFO、后台失败投影与
  blocked-send close；再用真实 loopback WebSocket 验证反馈 Ping 连续存在时应用 data
  在 4096 个唯一 Pong 计数 watchdog 前推进，且每个 Ping 恰好只有一个
  回应 Pong。周跑 300 秒且保存 JSONL/失败日志；
  latency、throughput 和 RSS 只用于趋势，不在共享 runner 上写入性能阈值。

剩余：

- 累积每周 JSONL 后建立跨周基线与 runner 噪声模型；只有在固定专用 runner
  上有足够样本后才评估统计性能回归阈值。

## M4：响应兼容、工具副作用与能力完整性

状态：大部完成。

已完成：

- Agent/异步聊天响应先递归拒绝重复 JSON key，再显式判别 union 并允许响应叶节点接受
  provider 新字段；空对象、矛盾状态和错误嵌套形状仍拒绝。AsyncTask 与 ZRAG retrieve
  的 raw-preserving/custom union 入口复用同一 crate-private unique-value decoder。
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
- 所有 typed MCP request 提供无需导入 trait 的公开 `validate()`；Realtime
  `SessionBuilder::validate()` 在零网络副作用下预检 cross-field、message、API key 与 JWT。
- `VoiceListQuery::Serialize` 与真实 wire schema 统一为 `voiceName` / `voiceType`，并以
  loopback 测试锁定 Unicode/保留字符的百分号编码；SDK 手写 query 与公开 serde 契约不再漂移。
- 私有 `Operation::with_query` 直接从 typed query 的 `Serialize` 契约生成扁平标量参数，
  Batch/File/Knowledge/Document/Voice list 不再维护第二份手写字段映射；非法形状固定脱敏失败，
  原始 Unicode/保留字符只在 URL 边界编码一次。
- Chat SSE 的可选 tool-call `type` 对未来字符串会降级为 `None`，但保留 id、index 与
  function 增量；后续文本和 `[DONE]` 不再被一个新增 discriminator 截断，非字符串坏值
  仍严格拒绝。
- Realtime session 对已知事件中明确列举的未来 modality/voice/VAD/noise/chat-mode/item
  字符串发出 `UnsupportedKnown` 并保留原始 semantic JSON；所有 live-session event 在
  strict typed decode 前先以不保留 payload tree 的递归 visitor 拒绝重复键，strict 失败
  后的 patched probe 再执行同级 shape 校验，防止把 malformed event 误当兼容扩展。
  input/output audio format 继续 fatal，直接 `ServerEvent` Serde 也保持严格。
- Batch list、file list/object/delete、embedding response/item、web-search intent 与
  `ImageContentFilterInfo::role` 的可选 marker 对未来字符串只在对应 response
  字段降级为 `None`，并保留其他已知 payload；非字符串坏类型、公开 enum
  的直接 Serde 与现有 non-empty invariant 仍保持严格。
- moderation 的当前五级处置结果 `PASS` / `REVIEW` / `BLOCK` / `REJECT` / `HIGH`
  都保留为独立 `RiskLevel` 语义；未来字符串仍走非穷尽的 `Unknown` 兜底。
- Knowledge/Document 的 data-bearing envelope 统一要求 code `200` 与 non-null `data`；
  operation-only delete/update/re-embed 只要求 code `200`。公开 `Option` 字段不变，冻结
  contract 与 HTTP-200 loopback 回归共同阻止缺 code/错误 code/缺失或 null data 假成功。
- ZRAG multimodal retrieve 已补齐公开的 typed request/response 与生产 route；请求在网络前
  严格校验且 `Debug` 脱敏，响应把已知字段建模为可选并忽略未来新增字段，同时拒绝不含
  任何已知非空字段的伪成功 envelope。ZRAG agent chat 也已作为 stream-only typed API
  落地：续聊 ID 只走 operation-scoped sensitive `X-Session-Id` header，JSON `type=done`
  事件先交付再终止，in-band error/提前 EOF/literal `[DONE]` 均 fail closed；未来 event type
  与 tool-result status 保留原值，默认 `Debug` 不显示 raw payload。decoder 以 64 KiB slice
  逐事件推进、递归拒绝 duplicate JSON key，并在终态立即释放 raw response；provider
  diagnostics 中 exact session 回显不会从 message、business code、request ID 或
  `Retry-After` 重新暴露。显式 raw/status/session accessor 仍返回原值，应按敏感数据处理。

剩余：

- 响应前向兼容仍需覆盖 parser/OCR/layout 状态或标签与 Agent/async-result union；
  这些语义型 discriminator 不能安全映射成已有值，应在 7.0 改为保留原始值/原始块的
  开放类型。当前 `TaskStatus::Unknown` 也会丢失 provider 的未知字符串。

## M5：7.0 架构与公共 API 收敛

状态：公开 pagination primitive 与 model registry foundation 已以 additive/私有重构
提前完成；其余为 P2/破坏性。

已完成：

- `zai_rs::pagination` 公开 `CursorPagination` / `PagePagination`，统一校验非零值并对
  opaque cursor 的 `Debug` 脱敏；Batch/File/Knowledge/Document/Assistant request 通过
  `try_with_pagination` 映射；File/Assistant 执行 100 的上限，其他路径不另加 SDK cap。
  通用类型不实现 `Serialize`，因为各接口分别使用 `limit`、`size` 或 `page_size`，且可能
  存在其他必填字段。
- 私有声明式 registry 已成为 23 个 Chat/Vision/Voice 模型的单一真源，同时生成公开
  zero-sized type、wire ID、message binding、sealed/public capability、request schema
  family 与 `MAX_TOKENS`。checked-in snapshot 冻结完整投影；公共类型名、路径、Serde 与
  trait impl 保持不变，并通过相对 `v6.0.1` 的 semver-check。
- 同一私有生成范式已覆盖 8 个 Image/Video/ASR/TTS/VoiceClone endpoint marker；
  每个 family 的 wire ID、`ModelName`、sealed/public request capability 与 snapshot 从一份
  声明生成，不暴露公开运行时 registry。
- 两个 Realtime wire marker 在 no-default 下继续公开且可 Serde；启用 `realtime`
  feature 时，同一 crate-private callback registry 才生成 sealed/public
  `RealtimeModel` 能力。正向注册断言、下游实现 compile-fail 与独立 snapshot 同时冻结该边界。
- ToolExecutor 的 owned input 在不可重试/最终 attempt 直接 move；只有仍可能执行下一次
  idempotent retry 时才保留并 clone。默认 Never 路径和最后 attempt 不再无条件 deep-clone
  大型 JSON，cache key、失败/timeout/panic 与 retry 次数语义保持不变。

剩余：

- 收敛 `model` 实现子模块暴露，澄清 `services::tools` 与 `crate::tool` 边界；旧路径先
  重导出并 deprecate。
- 重新命名 feature，区分 tool executor 与 JSON Schema validation，并评估默认依赖。
- 维护现有 `cargo-semver-checks` prior-tag 基线，并在 7.0 API 冻结后刷新；所有破坏性
  调整只在 7.0 合并。

## M6：CI、发布、安全与性能治理

状态：工程大部完成，正式发布未闭环。

已完成：

- CI 覆盖 no-default、每个 optional feature、depth-2 feature powerset、stable/nightly、
  MSRV 1.88、Windows 2025 和 macOS 15。MSRV 的每个 feature 组合会先仅编译
  production library，再编译 all-targets，避免 dev-dependency feature union 掩盖
  缺失依赖；Windows/macOS 同时运行 stable all-feature tests，并原生编译 1.88 的
  all-feature 与 no-default all-targets。
- `cargo-audit`、`cargo-deny`、Gitleaks 和 fuzz 工具固定版本；root/fuzz lockfile 与
  license/source policy 都进入门禁。
- 新增 `SECURITY.md`、CODEOWNERS、Cargo/Actions Dependabot 分组和私有漏洞报告流程；
  GitHub private vulnerability reporting 已确认启用。
- 发布 workflow 要求 annotated tag 与 Cargo version 一致，经受保护 environment 后用
  crates.io Trusted Publishing/OIDC 临时 token 发布。
- 发布 workflow 设计产出 `.crate`、覆盖所有目标平台与 crate features 的 CycloneDX 1.5 SBOM、SHA-256 和
  GitHub provenance/SBOM attestation；Actions 固定到完整 commit SHA。该设计
  不等于 `v6.0.1` 已成功发布。
- tag-triggered Release 不再提供跳过 publication 的绿色路径：attestation、crates.io
  OIDC 与 publish 任一未完成都会使 workflow 失败；打包演练由 reusable CI 的
  publish dry-run 承担。
- release evidence artifact 名包含版本、`run_id` 与 `run_attempt`；失败 job 的 rerun
  可被逐次审计，不依赖 GitHub 对 immutable artifact 的旧 attempt 清理/同名语义。
- `scripts/verify-package-contents.sh` 检查实际 `.crate` 的允许顶层、regular-file 类型、
  必需合同文件及宽松的压缩/解包大小上限；CI dry-run 与正式 release 都执行该策略，
  不依赖会随文档自身变化的精确文件数/字节快照。
- 文档增加安全迁移说明和发布清单；中英文 README 暴露迁移入口。
- `cargo-llvm-cov 0.8.7` 在 2026-07-24 候选树实测 workspace all-features tests 为
  region 83.14%、function 76.11%、line 82.92%；CI 分别设 83.10%/76.10%/82.90%
  floor，阈值只允许上调。本批未重测覆盖率，因此不把旧测量写成当前树的新基线。
- 独立 CI job 使用 Rust 1.97.1 与固定 `cargo-semver-checks 0.49.0`，从完整 git
  history 选择 HEAD 可达且不指向 HEAD 自身的最新 `v*` tag，并以 patch 规则分别检查
  明确的 default-feature 与 all-feature 公共表面；无祖先基线会 fail closed。当前相对
  本地 `v6.0.1` 的两档检查均为 223 pass、30 skip。该真实基线避免 crates.io 仍为
  `0.6.0` 时 analyzer 按跨 major 更新跳过全部检查。
- Criterion `0.8.2` 基准覆盖 SSE 不同分片、1/64 KiB 敏感/clean 脱敏路径、
  静态/动态 endpoint、
  tool cache hit/miss 和 Realtime 20 ms/64 KiB PCM→WAV→base64。PR 的 stable test
  门禁会执行 13 组快速 libtest benchmark smoke、七组 SSE 与一组 dense-numeric
  business-error harness-free allocation census，但不生成完整 Criterion sampling report；
  每周和手动 workflow 运行完整测量并上传 `target/criterion`，共享 runner 不设置脆弱的
  wall-clock 性能阈值。
- 同一每周/手动 workflow 以 release exact ignored 模式运行 Linux 100 MiB 文件下载
  RSS gate；机器可读 JSON 记录 `VmHWM` delta、elapsed 和 throughput，只以内存上界作为
  gate，并经 schema/单行校验后连同完整日志上传。除 5 分钟的纯存活 watchdog外，不对
  共享 runner 的 elapsed/throughput 设置 wall-clock 性能阈值。
- 同一 workflow 运行 `sse_allocations` 的七组 allocator census，硬门禁只约束
  allocation、reallocation 次数和 payload-relative bytes；workflow 还会校验恰好覆盖
  `2 payload × 3 chunk` 的六个唯一单行 JSON 组合与一个 4096 行 JSON 事件，再把
  JSONL/日志作为独立 artifact 保存。Criterion 继续负责只记录、不硬 gate 的耗时趋势。
- 同一 workflow 运行 `business_error_allocations`：顶层/嵌套 reserved `code` 的约
  2 MiB、1,048,576 元素 numeric array，以及 32 MiB numeric literal、普通/转义 code
  string 与病理深嵌套 code，均必须由 streaming
  probe 以不超过 4 次 allocation、4 次 reallocation、1024 allocated bytes 与
  1024 reallocated bytes 消费；workflow 校验精确六场景矩阵并独立上传日志与 JSONL。
- 同一 workflow 用 release exact 运行 300 秒 Realtime soak；步骤失败也会立即上传
  JSONL 与完整日志；成功路径会复验 schema 与关键硬不变量。artifact 名同时包含
  `run_id` / `run_attempt`，不会依赖 rerun 的旧 artifact 清理/同名语义。该 job 的
  30 分钟上限是纯 liveness 保护，不是性能阈值；manual 与 schedule 的并发组彼此隔离，
  不会互相取消长压证据。

剩余：

- 由仓库/发布维护者在 crates.io 配置 repository `AnlangA/zai-rs`、workflow
  `release.yml`、environment `crates-io` 的 Trusted Publisher；然后用高于
  `6.0.1` 的版本和全新 annotated tag 完成发布。
- 发布后使用 `cargo search`、`cargo info`、docs.rs 精确版本页、release evidence
  校验和 GitHub attestation 联合验证，不以 tag 存在代替发布成功。
- 积累慢对端长压的 p95/p99/RSS JSONL 历史；先固定专用 runner 与噪声模型，
  再讨论统计回归阈值。
- 仓库外发布配置与法律信息见下一节。

## 6.0.1 发布失败与恢复要求

发布维护者在 2026-07-24 确认了归属和候选版决策，但后续发布未成功：

1. `LICENSE` 保留现有 `Copyright (c) 2025 Model Context Protocol` 归属。
2. 候选版选择为 `6.0.1`；可观察行为变化已在
   [安全加固迁移说明](HARDENING_MIGRATION.md) 中显式披露，但该候选版未进入
   crates.io。
3. GitHub `crates-io` environment 已创建且仅允许 `v6.0.1` 标签部署。crates.io
   Trusted Publisher 目标是 `AnlangA/zai-rs`、`release.yml` 和 environment
   `crates-io`；第二次 run 返回“No Trusted Publishing config found”，证明该仓库外
   配置尚未存在或未精确匹配，不能视为已验证。
4. run `30091854390` 在 `Verify annotated tag` 失败；当时 `v6.0.1` ref 是
   lightweight tag。run `30092565276` 通过了 tag 验证、所有质量门禁、打包、
   SBOM、SHA-256 与两份 attestation，然后在 `Authenticate to crates.io` 失败；
   日志中的 crates.io HTTP 400 精确指向 Trusted Publisher 缺失。
5. 当前工作树已偏离 `v6.0.1` tag。恢复发布时必须保留失败 run 证据，
   提升 Cargo 版本到 `>6.0.1`，更新迁移/发布文档，并创建全新 annotated tag；
   不移动、删除或复用既有 tag。

## 每批改动的统一验收

```bash
cargo fmt --all -- --check
cargo fmt --manifest-path fuzz/Cargo.toml -- --check
cargo check --workspace --all-features --lib --locked
cargo check -p zai-rs --no-default-features --lib --locked
cargo check --workspace --all-features --all-targets --locked
cargo check -p zai-rs --no-default-features --all-targets --locked
cargo test --workspace --all-features --tests --locked
cargo test -p zai-rs --no-default-features --tests --locked
cargo test -p zai-rs --all-features --benches --locked
cargo test -p zai-rs --no-default-features --benches --locked
cargo test --workspace --all-features --doc --locked
cargo clippy --workspace --all-features --all-targets --locked -- -D warnings
cargo clippy --manifest-path fuzz/Cargo.toml --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
mdbook build
mdbook test -L target/debug/deps
./scripts/verify-package-contents.sh
cargo publish --dry-run --locked -p zai-rs --all-features
```

涉及 feature、fuzz、coverage、semver 或发布的批次，还必须运行对应专项门禁。
涉及热点实现的批次还应运行
`cargo bench --locked -p zai-rs --features realtime,toolkits --bench hot_paths -- --test`
进行轻量 smoke，并运行
`cargo bench --locked -p zai-rs --bench sse_allocations` 与
`cargo bench --locked -p zai-rs --bench business_error_allocations` 验证确定性分配上界；
完整耗时报告由定时或手动 benchmark workflow 生成。

## 进度维护规则

- 每个 PR 只承担一个可独立回滚的主题，并引用本路线图条目。
- 完成性能条目时记录基准环境、前后数据和回归阈值。
- 若上游协议变化影响优先级，先更新冻结契约与路线图，再修改实现。
