# zai-rs 0.5.0 AI Agent 项目优化执行计划

## 1. 文档用途

本文档是交给自动编码 Agent 直接执行的工程计划。执行者必须按任务编号顺序完成工作，不得重新设计目标架构、改变兼容策略、降低验收阈值，或把本文中的实施项改写成调研项。

计划基线：

| 项目 | 固定值 |
|---|---|
| 仓库 | zai-rs |
| 基线提交 | e82be86 |
| 基线分支 | main |
| 分析日期 | 2026-07-11 |
| 当前源码版本 | 0.4.0 |
| 目标源码版本 | 0.5.0 |
| Rust edition | 2024，保持不变 |
| MSRV | Rust 1.88.0，保持不变 |
| 目标产物 | 可发布的 0.5.0 release candidate |
| 外部操作边界 | 不创建 tag，不 push，不发布 crates.io，不创建 GitHub Release |

本计划没有需要执行者再次选择的方案。遇到外部服务不可用、凭据缺失或上游固定快照无法取得时，执行者只记录失败命令、原始错误和恢复条件并停止当前任务，不自行替换数据源、放宽门禁或改用另一套协议。

## 2. 分析结论

### 2.1 当前质量基线

基线提交在传统 Rust 质量门上是绿色的，但绿色结果没有覆盖关键协议和运行时语义。

| 检查 | 基线结果 | 结论 |
|---|---:|---|
| cargo fmt --all -- --check | 通过 | 格式无问题 |
| cargo clippy --all-features --all-targets -- -D warnings | 通过 | 现有编译路径无 Clippy 警告 |
| cargo test --all-features --all-targets | 339 个单元测试、26 个集成测试通过 | 大量请求族没有真实传输测试 |
| cargo test --all-features --doc | 14 通过、90 忽略 | 公共文档示例大部分没有编译 |
| 普通 rustdoc warnings | 通过 | 没有模拟 docs.rs 的 docsrs 配置 |
| docs.rs 等价 nightly rustdoc | 失败 | redundant-explicit-links |
| cargo llvm-cov --all-features --workspace | line 51.04%，region 54.81%，function 44.34% | API 家族覆盖严重失衡 |
| cargo publish --dry-run --locked --all-features | 通过，207 files，约 1.2 MiB | 包可构建，不代表协议正确 |
| cargo audit --deny warnings | 失败 | RUSTSEC-2026-0173，经 validator_derive 0.20.0 引入 |

仓库约有 27,777 行 Rust 源码和约 615 个公开符号。42 个请求文件重复保存 EndpointConfig，约 40 个请求类型重复保存 API key、ApiBase 和 HttpClientConfig。高重复度使修复无法在一个位置生效。

### 2.2 P0 正确性问题

1. src/model/audio_to_text/model.rs 将模型 ID 写成 glm-asr-2512 后附一个空格。全部现有门禁仍然通过。
2. src/agent 实现的是 /paas/v4/agents CRUD、chat 和 history；当前官方 Agent 协议是 POST /v1/agents、POST /v1/agents/async-result、POST /v1/agents/conversation，请求和响应 wire schema 也不同。
3. async/chat/completions 被建模成可开启 SSE；官方接口是任务提交接口，不接受 stream。
4. parse_typed_response 先把 2xx body 解析成成功类型，只有成功类型解析失败后才检查错误 envelope。全字段可空的知识库响应会把 code=500 的业务失败返回为 Ok。
5. ChatCompletionResponse 使用宽松默认值，陌生对象和空对象能够变成空成功响应。
6. HTTP 502、503、504 的真实解析链会落入 Unknown，现有 retry 测试只手工构造 HttpError，因此没有发现分类断裂。
7. 所有 POST 共用自动重试路径。创建、上传、生成和工具调用会在服务端已接收请求后被重复执行。
8. VideoGenRequest 和 TextToAudioRequest 缺少强制校验的 typed send，示例直接调用底层 post 绕过校验。
9. ASR、TTS 和知识库参数相对官方文档存在明确漂移：
   - ASR 缺 prompt、hotwords、file_base64、stream、request_id 和 user_id，反而暴露无效 temperature。
   - TTS 的 input 上限错误地设为 4096，volume 允许 0，缺 PCM、stream 和 encode_format。
   - 知识库缺 Embedding-3-pro 对应的 embedding_id=12、embedding_model 和 contextual。

### 2.3 P0/P1 安全和可靠性问题

1. ZaiConfig 和 examples/web_chat 的 Config 均派生 Debug，API key 会被原文输出；web_chat 启动日志确实打印完整配置。
2. examples/gen_video.rs 直接 println API key。
3. HTTP trace 会记录完整请求与响应 body；提示词、文件内容、工具参数和个人数据均会进入日志。
4. URL 字符串拼接没有编码动态 path segment。包含 /、?、#、%、.. 的 ID 会改变请求路径。
5. 自定义 base URL 构建失败后回退到未编码字符串拼接；HTTP、WS 明文 scheme、userinfo 和 fragment 没有在配置阶段拒绝。
6. JSON body、错误 body、SSE line/event/buffer 和 WebSocket frame 都没有硬限制。
7. reqwest 的 60 秒总请求 timeout 同时作用于 SSE，会终止仍在正常活动的长流。
8. HTTP 客户端允许跨源重定向，认证头和 URL 安全策略没有统一入口。
9. data 目录跟踪约 7.9 MiB 的真实外观媒体；chat_vision 示例含已过期的签名 URL。

### 2.4 P1 架构和性能问题

1. ZaiConfig 被描述为中心配置，但发送路径从未读取其中的 reqwest client。
2. HTTP_CLIENTS 是进程级无界 DashMap，只按 timeout 和 compression 建 key，自定义 client 不参与 key。
3. 公共 HttpClient 暴露 raw get/post/put/delete，调用者可以绕过 endpoint 校验、typed decoder 和 retry safety。
4. file、ASR、OCR、文件解析和知识库上传先 tokio::fs::read 整个文件，再为重试 clone。
5. 文件下载先把完整响应装入内存；中途中断没有原子落盘和残留清理。
6. file parser 的两个结果请求串行执行；轮询使用相对 sleep，实际结束时间会超过 timeout。
7. ToolExecutor 默认开启缓存并默认重试三次，没有区分纯函数、幂等操作和有副作用操作。
8. 工具缓存 canonicalizer 会让字符串 null 与 JSON null 发生碰撞，并会 trim object key；schema cache 只用 u64 hash。
9. 工具批处理先创建全部 Tokio task，再在 task 内取得 semaphore；RMCP 调用也没有 deadline 和并发上限。
10. Realtime 使用 biased select，命令分支可饿死接收分支；错误会被静默吞掉，广播 sender 留存会让消费者永久等待。
11. rmcp 保留默认 features，toolkits 依赖常驻默认构建，Tokio 默认启用宏和多线程 runtime，依赖图大于实际功能需要。
12. ModelName: Into&lt;String&gt; 与 define_model_type! 在序列化零尺寸内置模型时产生不必要分配。

### 2.5 P1 工程化和发布问题

1. 90 个 Rustdoc 被忽略；docs 目录另有 95 个 Cargo 不会测试的 Rust fence。
2. BEST_PRACTICES、ERROR_HANDLING、ADVANCED_TOPICS 及多个模块首页使用不存在或已漂移的 API。
3. tests/integration_tests.rs 包含纯 sleep、手写 retry loop 和裸 serde_json::Value 断言，不经过 SDK。
4. agent、batches、file、knowledge、绝大多数模型 API、realtime 和 RMCP 的主要代码路径覆盖率接近 0。
5. examples/web_chat、examples/mcp/client、examples/mcp/servers、examples/mcp/list_remote_mcp_tools 不属于根 workspace，根 CI 不检查它们。
6. web_chat 锁定旧 zai-rs，存在 67 个以上 warning，并因 CORS credentials 与 Any headers 组合在启动时 panic。
7. web_chat 静态目录依赖当前工作目录，注册不存在的 service worker，并宣称存在未实现的 session、WCAG、生产级 sanitizer 等功能。
8. Cargo.toml、README 和 docs 标记 0.4.0，但 crates.io 最新公开版本仍为 0.2.0，远端没有对应 release tag。

## 3. 上游契约基线

执行过程中只使用下列固定来源。所有实现以冻结快照为准，不追随执行日期之后的上游变化。

| 来源 | URL | 固定摘要 |
|---|---|---|
| OpenAPI | https://docs.bigmodel.cn/openapi/openapi.json | SHA-256 c3754a1265e6f88dbba1404520d50773f7f7bd586fb2d676dde62d1c1bfe377e，452097 bytes，53 个 path、59 个 method/path operation |
| AsyncAPI | https://docs.bigmodel.cn/asyncapi/asyncapi.json | SHA-256 01fa9bb1c6845650d55ed7e3b18aa249774988329d7a8d6aeb20835c33620490，39553 bytes，1 个 realtime channel |
| Agent Markdown | https://docs.bigmodel.cn/api-reference/agent-api/%E6%99%BA%E8%83%BD%E4%BD%93%E5%AF%B9%E8%AF%9D.md | SHA-256 71b6e21adb0b6705d90db772e9155564809a4f8eed15928c98775f2335b4a266，16862 bytes |
| ASR Markdown | https://docs.bigmodel.cn/api-reference/%E6%A8%A1%E5%9E%8B-api/%E8%AF%AD%E9%9F%B3%E8%BD%AC%E6%96%87%E6%9C%AC.md | SHA-256 bb43aae64ebd6189193586a73a132a14147e0fec53a86b1d24ec70704cbfca18，6527 bytes |
| TTS Markdown | https://docs.bigmodel.cn/api-reference/%E6%A8%A1%E5%9E%8B-api/%E6%96%87%E6%9C%AC%E8%BD%AC%E8%AF%AD%E9%9F%B3.md | SHA-256 4563fee12a3e0868bba43837e68816cd80b43deef37e5439067d15c0b733faf4，5713 bytes |
| Knowledge Markdown | https://docs.bigmodel.cn/api-reference/%E7%9F%A5%E8%AF%86%E5%BA%93-api/%E5%88%9B%E5%BB%BA%E7%9F%A5%E8%AF%86%E5%BA%93.md | SHA-256 d8bc15e3cb478268e2d55268ea86f59d57294165b78eed931ce265341874953d，4498 bytes |
| API errors Markdown | https://docs.bigmodel.cn/cn/faq/api-code.md | SHA-256 745b2a789a34e9ec1e1b021fcbeff472bd716b95facaee28e4388990a0e8afdf，7639 bytes |
| Coding Plan Markdown | https://docs.bigmodel.cn/cn/coding-plan/extension/usage-query-plugin.md | SHA-256 d0f3e150b7b29bb63acfe9032a50f8c3975e9b752784f66a9b82d4cf7ccbceee，1973 bytes |
| Coding Plan endpoint source | https://raw.githubusercontent.com/zai-org/zai-coding-plugins/0446d0bb0bc537d97d3ab3664c4b8b9c4a0e1254/plugins/glm-plan-usage/skills/usage-query-skill/scripts/query-usage.mjs | commit 0446d0bb0bc537d97d3ab3664c4b8b9c4a0e1254，file SHA-256 9708b71af925fcf32bc9f611842e37f543ce38de4bebf09544bdf161cfdf8bae，5445 bytes |

P00 必须把上述八个 HTTP 文件和 Coding Plan endpoint source 一并提交到 spec/upstream，逐 byte 校验。manual-constraints.toml 只能编码本文第 13 节的固定约束，不从执行时网页重新归纳。

主 OpenAPI 之外，Coding Plan usage monitor 与 Realtime AsyncAPI 作为独立契约面纳入测试。契约覆盖口径固定为：

- OpenAPI：59/59 operations，其中包含三个 Agent v1 operations。
- Agent：OpenAPI 中的三个 v1 operations 额外通过 Agent 手工参考页约束测试，不增加 operation 分母。
- Realtime：握手、session update、音频、错误、关闭五条协议路径。
- Coding Plan：usage/quota/limit 一条监控路径。

## 4. 固定技术决策

| 主题 | 固定实现 |
|---|---|
| 版本策略 | 0.5.0 是一次明确的 breaking release。0.4 的公开请求/client/type 路径全部移除，不提供 Rust compatibility alias；迁移文档给出旧路径到新 service API 的固定对照。唯一兼容例外是 Cargo feature tool-validation，它在 0.5 等价转发到 toolkits 并在 0.6 删除。 |
| 核心入口 | crate root 只直接导出 ZaiClient、ZaiConfig、ZaiError、ZaiResult 和 prelude。用户只创建一次 ZaiClient。 |
| 所有权 | ZaiClient 内部持有 Arc&lt;ClientInner&gt;；ClientInner 持有 secret、validated endpoints、一个 reqwest::Client、Transport 和 policies。 |
| 请求建模 | 每个 endpoint 实现 crate-private sealed RequestSpec，声明 method、API family、path、content type、retry safety、limits、validator 和 decoder。 |
| HTTP client | 删除 HTTP_CLIENTS。ZaiClientBuilder 不接受 reqwest::Client 或 reqwest::ClientBuilder，只接受受限 HttpTransportConfig；SDK 构建唯一 client。测试替换 transport 只用 crate-private factory。 |
| 密钥 | 使用 secrecy 0.10.3 的 SecretString。所有 Secret、配置和 header 容器手写脱敏 Debug；只有构造 Authorization 时调用 expose_secret。HTTP Authorization 与 Realtime handshake auth 的 HeaderValue 构造后立即 set_sensitive(true)。 |
| URL | EndpointConfig 内部只保存 url::Url。动态 path 用 path_segments_mut().push，query 用 query_pairs_mut。默认只允许 HTTPS/WSS。测试和本地代理通过显式 allow_insecure_transport(true) 开启 HTTP/WS。 |
| 重定向 | 最多 3 次、只允许同源、禁止 TLS 降级。跨源响应直接返回 Protocol 错误，认证头不发送到新 origin。 |
| 成功解析 | 先探测官方错误 envelope，再解析 private wire success model，再校验成功不变量，最后转换为 public response。未知对象和空对象不得成为成功。 |
| 错误码 | ApiCode 保存原始数字或字符串，不再压缩成 u16。 |
| 重试 | 所有内置 GET、HEAD、OPTIONS、PUT、DELETE 固定 Idempotent；所有内置 POST、PATCH 固定 NonIdempotent。调用方只能逐请求用 with_retry_override(RetryOverride::AssumeIdempotent) 覆盖；该字段不进入 wire body。request_id 本身不构成幂等保证。 |
| 重试状态 | Idempotent 只重试 connect/read timeout、408、425、429、500、502、503、504；401/403 为鉴权错误；其他 4xx、501、505 不重试。 |
| Retry-After | max_attempts 固定 3（含首次）。第 n 次 retry 的 jitter 上限为 min(8s, 200ms * 2^(n-1))，从 [0, 上限] 均匀取值。429/503 的合法 Retry-After 替代 jitter；非法值回退 jitter；delay >= remaining deadline 时直接 Timeout。 |
| HTTP timeout | connect 10s；普通请求每 attempt 60s；整个 retry 120s；SSE/下载没有总 request timeout，使用可重置 idle timeout 60s。 |
| 正文限制 | JSON request 32 MiB；decoded JSON response 32 MiB；错误 body 64 KiB；SSE line/event/buffer 各 1 MiB；multipart 每 request 最多 16 个 file part、file 原始字节合计 128 MiB、非文件字段合计 1 MiB；WS message 8 MiB；WS frame 2 MiB；inbound/outbound realtime audio frame 4 MiB。以上同时是默认值和硬上限；builder 只允许调低。错误 body 的 message 不进入 public error，未知 body 只保留长度与 SHA-256，不保留 preview。 |
| 日志 | 永不记录请求 body、响应 body、API key、Authorization、cookie、工具参数或文件内容。只记录 method、规范化 route template、status、byte count、attempt、elapsed 和服务端 correlation request_id。request_id 只保留最多 128 bytes 的可打印 ASCII，控制字符替换。URL 不记录 userinfo、fragment、query value 和用户提供的 path/resource ID。 |
| 流式协议 | SSE 增量解析，协议要求 [DONE] 的流必须收到 [DONE]；EOF、超限和 decode error 作为 stream item error 后终止。 |
| 文件 IO | multipart 按需打开文件并流式发送。下载写入同目录临时文件，flush、sync_all、rename；任何失败删除 partial file。 |
| Tool effect | ToolEffect 固定为 Pure、Idempotent、SideEffecting，默认 SideEffecting。Pure 才能缓存；Pure 和 Idempotent 才能重试瞬时错误；SideEffecting 不缓存、不重试。 |
| Tool limits | deadline 30s；input 256 KiB；output 1 MiB；并发 8；单 batch 最多 64 calls；只允许同时存在 8 个已创建未完成 task。 |
| Tool cache | canonical JSON bytes 不 trim，SHA-256 key 包含 tool generation；不存明文参数；Instant TTL；严格容量；完整 schema key equality。 |
| Realtime | 使用公平 select；connect/send deadline 10s；close 5s；状态为 Open、Closed、Failed；所有 send/recv/pong/decode/join 错误向消费者传播。 |
| features | default=[]；toolkits 为显式 feature；rmcp-kits 依赖 toolkits；realtime 独立；tool-validation 是一个版本周期的 deprecated alias，等价于 toolkits。 |
| 依赖策略 | 保持 reqwest、Tokio、rustls 和 rmcp 1.8。validator_derive 锁定 0.20.1，清除 proc-macro-error2；本轮不迁移验证框架。 |
| 文档 | mdBook 与 Rustdoc 都执行代码测试；联网示例使用 no_run；伪代码使用 text；ignored doctest 固定为 0。 |
| 测试 | 59 个 operation 中每个 operation 至少验证 method、path、query、auth、content type、request、success 和 error。 |
| 覆盖率 | zai-rs package line >=75%，region >=70%，function >=65%；client、agent、audio、toolkits、realtime 各自 line >=90%。 |
| 发布 | 本计划只生成 release candidate、迁移文档和发布工作流；不会执行任何外部发布动作。 |

## 5. 目标架构

~~~mermaid
flowchart TD
    U["应用代码"] --> C["ZaiClient"]
    C --> S["Service facades<br/>chat / files / agents / knowledge / tools / ..."]
    S --> R["sealed RequestSpec"]
    R --> V["输入与跨字段校验"]
    V --> P["PreparedRequest<br/>validated URL + method + body + retry safety"]
    P --> T["Transport"]
    T --> H["single reqwest::Client"]
    T --> D["error probe + typed decoder + response invariant"]
    T --> X["SSE / download streaming"]
    C --> CFG["ZaiConfig<br/>SecretString + Url + policies"]
    CFG --> T
    C --> RT["Realtime transport"]
    C --> TK["ToolExecutor<br/>feature=toolkits"]
~~~

目标文件边界：

~~~text
src/
  lib.rs
  prelude.rs
  client/
    mod.rs
    config.rs
    secret.rs
    endpoint.rs
    error/
      mod.rs
      api_code.rs
      classification.rs
      redaction.rs
    transport/
      mod.rs
      request.rs
      retry.rs
      decode.rs
      limits.rs
      redirect.rs
    sse/
      mod.rs
      decoder.rs
      stream.rs
  services/
    chat/
    images/
    videos/
    audio/
    embeddings/
    rerank/
    tokenizer/
    moderation/
    files/
    batches/
    knowledge/
    agents/
    tools/
    assistants/
    applications/
    tasks/
    zrag/
    usage/
  toolkits/
  realtime/
~~~

公共调用形态固定为：

~~~rust
let client = ZaiClient::builder(api_key).build()?;

let response = client.chat().complete(request).await?;
let mut stream = client.chat().stream(request).await?;
let task = client.chat().complete_async(request).await?;
let result = client.tasks().get(task.id()).await?;

client.images().generate(request).await?;
client.videos().generate(request).await?;
client.audio().transcribe(request).await?;
client.audio().synthesize(request).await?;
client.files().upload(request).await?;
client.batches().create(request).await?;
client.knowledge().create(request).await?;
client.agents().invoke(request).await?;
~~~

请求构造器不接收 key、base URL、HttpClientConfig 或 reqwest::Client。Request body 字段保持私有，通过 builder 建立有效状态；跨字段关系在 send 前统一校验。

## 6. 执行协议

1. 本计划文件与 docs/README.md 索引必须先由仓库所有者提交。执行者首先运行 test -z "$(git status --porcelain)" 并确认本计划已被 Git 跟踪；非空或未跟踪时立即停止，不 stash、不移动、不提交用户改动。随后确认 e82be86 是当前 HEAD 的 ancestor，并从当前 HEAD 创建 codex/zai-rs-0.5。
2. 严格按 P00 至 P15 执行。一个时刻只实施一个任务；前置任务的验收命令全部通过后才进入后项。
3. 每项任务使用独立提交，提交标题固定为表中给出的标题。固定流程为实施 → 验证 → 更新 ledger → commit → test -z "$(git status --porcelain)"。ledger 记录任务 ID、固定提交标题、父提交、验证命令和结果，不在提交内容中记录当前提交自身的 hash；hash 由 git log --format='%H%x09%s' 推导。
4. P00 创建 scripts/bootstrap-tools.sh。进入后续任务前执行该脚本；脚本只安装第 P14 节固定版本的工具，不读取 latest。
5. 生产代码禁止新增 unsafe、unwrap、expect、panic、明文 secret Debug 和 raw body tracing。
6. 测试只使用 127.0.0.1:0 的本地 HTTP/WS server，不调用真实 Zhipu API。
7. 测试时间使用暂停的 Tokio time 或注入 Clock，禁止真实长 sleep。
8. snapshot、生成文件和 README 索引由 xtask 生成；CI 使用 check 模式确保仓库内容已同步。
9. 每项任务结束时更新 docs/optimization/EXECUTION_LEDGER.md，记录提交、命令、测试数、覆盖率和残留问题。残留问题只能引用后续已有任务 ID，不得新增未排期工作。
10. 任何门禁失败都在当前任务内修复。不得用 allow、ignore、skip、降低阈值或删除断言让门禁变绿。
11. 每次任务有意修改 Cargo.toml、workspace members、dependency、feature 或 package version 后，先且仅一次运行 cargo metadata --format-version 1 >/dev/null 更新根 Cargo.lock，审查 diff 只含该任务声明的 package/feature/root-version 变化；随后该任务的全部 Cargo 验证使用 --locked。P11 创建独立 fuzz workspace 时只运行一次 cargo +nightly-2026-07-10 generate-lockfile --manifest-path fuzz/Cargo.toml，此后 fuzz metadata/audit 均使用已提交 lock。

## 7. 任务依赖图

~~~mermaid
flowchart LR
    P00 --> P01 --> P02 --> P03 --> P04 --> P05 --> P06
    P06 --> P07 --> P08 --> P09 --> P10
    P10 --> P11 --> P12 --> P13 --> P14 --> P15
~~~

## 8. 详细执行任务

### P00 — 冻结上游契约与可复现基线

- 状态：未执行
- 优先级：P0
- 依赖：无
- 提交标题：test(contract): freeze 2026-07-11 upstream specifications

实施：

1. 在任何项目文件变更前，先从仓库外临时脚本安装 Rust 1.88.0（rustfmt、clippy、llvm-tools-preview）、nightly-2026-07-10、cargo-audit 0.22.2 与 cargo-llvm-cov 0.8.7并核对版本；再从 e82be86 建立 detached 临时 worktree。第 2.1 节普通 Cargo 命令全部显式使用 +1.88.0，docs.rs 命令使用 +nightly-2026-07-10；记录两套 rustc -Vv、stdout、stderr、退出码、cargo package --list 和 git status到仓库外临时目录，最后删除 worktree。BASELINE.md 只整理这份原始结果。
2. 新建 spec/upstream/openapi-2026-07-11.json、asyncapi-2026-07-11.json、manual/*.md 和 coding-plan/query-usage.mjs，内容必须与第 3 节全部 byte length、commit 和 SHA-256 完全一致。
3. 新建 spec/upstream/SOURCES.toml，逐文件记录 URL、commit、抓取日期、byte length、SHA-256、path count 和 operation count。
4. 根 Cargo.toml 建立 resolver=3 workspace，P00 的 members 固定为根 package 和 xtask，exclude 固定为 fuzz 及四个嵌套示例工程；P12 把四个示例从 exclude 移入 members，fuzz 始终保持独立 workspace。
5. 新建 xtask workspace crate，并实现以下固定命令：
   - cargo run --locked -p xtask -- contract verify：校验源快照 hash 和操作数。
   - cargo run --locked -p xtask -- contract generate：生成 spec/contracts/operations.json。
   - cargo run --locked -p xtask -- contract check：重新生成到内存并和已提交 manifest 比较。
6. operations.json 为每个 operation 固定保存 source、operation_id、method、path、API family、request content type、Accept、success statuses、auth、request schema、success schema、error schema、response mode、requires_done、retry safety、success invariant、service method、request type、response type、stream item 和允许开放 Map 的字段。OpenAPI 缺 operation_id 时按第 14 节固定表填写，不现场命名。
7. 新建 spec/contracts/manual-constraints.toml，逐字编码第 13 节 Agent、ASR、TTS、Knowledge、Coding Plan、SSE 和 Realtime 固定约束；不得从执行时网页重新提炼。
8. 新建 spec/contracts/coverage.toml，列出 59 个 OpenAPI operation（分布在 53 个 path，其中包含三个 Agent operation）、一个 Coding Plan operation和五类 Realtime path。Rust symbol 与 test name 按第 14 节预先写定，当前实现不存在的条目只把 status 标记 missing；P06 只把 status 改为 covered。
9. 用固定 nightly rustdoc JSON 生成 spec/contracts/public-api-0.4.json，收录基线每个 public path、signature 和 signature hash。新建 public-api.toml，把全部基线 symbol identity 统一标为 removed，并把第 14 节目标 surface 标为 added；相同 path 的新 signature 仍是新的 identity。本计划不保留 symbol alias。xtask public-api check 必须拒绝未归类 symbol。
10. 新建 rust-toolchain.toml，默认 channel 固定 1.88.0，components 固定 rustfmt、clippy、llvm-tools-preview；额外工具链固定 nightly-2026-07-10。
11. 新建 scripts/bootstrap-tools.sh，逐个检查并安装固定版本：cargo-audit 0.22.2、cargo-deny 0.20.2、cargo-llvm-cov 0.8.7、cargo-fuzz 0.13.2、cargo-cyclonedx 0.5.9、cargo-nextest 0.9.114、mdbook 0.5.4、lychee 0.24.2；Rust 工具用 cargo +1.88.0 install --locked --version '=x.y.z'。脚本接受精确工具名参数，无参数时安装全部。gitleaks 8.30.1 只下载 GitHub release 的 gitleaks_8.30.1_linux_x64.tar.gz 或 gitleaks_8.30.1_darwin_arm64.tar.gz：前者 SHA-256 551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb，后者 SHA-256 b40ab0ae55c505963e365f271a8d3846efbc170aa17f2607f13df610a9aeb6a5；其他 OS/arch 直接失败。
12. 新建 spec/package-allowlist.txt，机器可读地只允许 Cargo.toml、Cargo.toml.orig、Cargo.lock、.cargo_vcs_info.json、README.md、LICENSE、CHANGELOG.md、src/**、docs/GETTING_STARTED.md、docs/MIGRATING-0.5.md；spec、其他 docs、tests、fuzz、xtask、examples、release 和 target 固定排除。
13. 新建 spec/forbidden-patterns.toml，把第 11 节及 P01/P03/P04/P05/P10 的负向规则按 phase 固定；xtask forbidden check {phase} 使用 cargo_metadata 与文件遍历区分“无匹配”和工具/IO失败。
14. 新建 docs/optimization/BASELINE.md，记录临时目录中的基线命令、输出摘要、源码行数、公开符号数、依赖节点数和 package 文件清单。
15. 新建 docs/optimization/EXECUTION_LEDGER.md，写入 P00 行。

验收：

- 两个上游文件 hash 与第 3 节一致。
- operations.json 恰好包含 59 个唯一 method/path，path 去重后恰好为 53。
- manifest 按 API family、path、method 稳定排序，连续生成两次没有 diff。
- baseline 文件包含所有质量命令的原始版本信息和退出码。

验证：

~~~bash
cargo run --locked -p xtask -- contract verify
cargo run --locked -p xtask -- contract check
cargo test -p xtask --locked
git diff --check
~~~

### P01 — 关闭已确认的正确性、泄密和供应链缺口

- 状态：未执行
- 优先级：P0
- 依赖：P00
- 提交标题：fix(safety): close confirmed correctness and secret leaks

实施：

1. 先把 tempfile=3.27.0 加入 dev-dependencies，再把 src/model/audio_to_text/model.rs 的模型 ID 改为 glm-asr-2512。
2. 新增全模型 ID 快照测试，覆盖 chat、vision、voice、realtime、image、video、ASR、TTS 和 voice clone；每个 ID 同时断言非空、等于 trim 后结果、存在于冻结契约或 manual constraints。
3. 为当前 ZaiConfig 手写 Debug，api_key 固定显示 [REDACTED]；移除 ZaiConfig 的 Default，统一 from_env 与 builder 缺 key 的错误分类。
4. 删除 examples/gen_video.rs 对 key 的 println；加入捕获根库 tracing 输出的测试，断言 key、Authorization 和 Bearer 均不出现。
5. 删除 examples/chat_vision.rs 的过期签名 URL。示例从第一个 CLI 参数读取媒体 URL，缺参数时输出 usage 并以退出码 2 结束。
6. 删除仓库 data 目录的媒体文件。依赖文件的测试使用 tempfile 生成最小 PNG、WAV、文本和二进制 fixture；示例从 CLI 参数读取路径。
7. 在当前 parser 中先探测 code/message/error envelope，再解析成功类型；知识库 code != 200、chat 空对象和陌生对象都返回 Err。
8. 在当前错误模型内修正 HTTP 分类，使所有 5xx 保留 status，401/403 命中 auth，429 命中 rate limit；字符串 ApiCode 的 breaking redesign 只在 P03 实施。
9. 执行 cargo update -p validator_derive --precise 0.20.1，提交 Cargo.lock；依赖树不得再含 proc-macro-error2。

验收：

- glm-asr-2512 不含尾部空格。
- format!("{config:?}") 和根库 trace 不含测试 key。
- 2xx + code=500、空 chat body、陌生 chat body 均返回 Err。
- 503 plain body 分类为 server，401 为 auth，429 为 rate limit。
- 仓库不再跟踪 data 目录中的真实媒体。
- cargo audit --deny warnings 通过。

验证：

~~~bash
cargo test --locked --all-features --all-targets
cargo clippy --locked --all-features --all-targets -- -D warnings
cargo audit --file Cargo.lock --deny warnings
cargo run --locked -p xtask -- forbidden check P01
~~~

### P02 — 建立共享 ZaiClient、秘密类型与安全 URL 配置

- 状态：未执行
- 优先级：P0
- 依赖：P01
- 提交标题：refactor(client): introduce shared client and validated configuration

实施：

1. 添加 secrecy 0.10.3，并在 src/client/secret.rs 定义内部 ApiSecret。Clone 复制 SecretString；Debug 和 Display 永远输出 [REDACTED]。
2. 把 package version 改为 0.5.0-alpha.0，Cargo.lock 与 docs version status 同步；P15 再改为 0.5.0。
3. 定义 ZaiClient、ZaiClientBuilder 和 ClientInner。ZaiClient 仅包含 Arc&lt;ClientInner&gt;，Clone 不复制配置、连接池或 secret。
4. ZaiClient::builder(api_key) 拒绝空值和只含空白的值；ZaiClient::from_env() 只读取 ZHIPU_API_KEY。
5. EndpointConfig 字段改成私有 Url，API family 与默认 base 固定为：
   - PaasV4 = https://open.bigmodel.cn/api/paas/v4
   - CodingPaasV4 = https://open.bigmodel.cn/api/coding/paas/v4
   - AgentV1 = https://open.bigmodel.cn/api/v1
   - LlmApplication、ApplicationV2、ApplicationV3 = https://open.bigmodel.cn/api/llm-application/open
   - Zrag = https://open.bigmodel.cn/api/zrag
   - Monitor = https://open.bigmodel.cn/api/monitor
   - Realtime = wss://open.bigmodel.cn/api/paas/v4/realtime
6. build 阶段拒绝 relative URL、userinfo、query 和 fragment。Paas/Coding/Agent/Application/Zrag/Monitor family 只接受 HTTP(S)，Realtime 只接受 WS(S)；正式默认值只用 HTTPS/WSS。
7. allow_insecure_transport(true) 只放在 ZaiClientBuilder，并在 Debug 中显示布尔值；HTTP/WS 自定义地址仍必须是 loopback 或 localhost。
8. 动态路径统一通过 push_path_segment，query 统一通过 Url::query_pairs_mut；删除 join_url 和非法 URL 的字符串 fallback。push_path_segment 在调用 url::PathSegmentsMut 前拒绝空值、单独的 . 和单独的 ..。
9. ZaiClientBuilder 只接受 HttpTransportConfig。该配置公开 timeouts、retry policy、resource limits 和 additional_header；limits 只允许调低，retry 只允许完全关闭或把 max_attempts 从 3 降为 1/2，不允许新增 status/error、扩大 backoff/deadline或让 NonIdempotent 重试。逐请求 AssumeIdempotent 是唯一安全覆盖入口。additional_header 的 name 白名单严格限定为 Accept-Language、X-Correlation-ID、X-Test-Client，其他名称全部拒绝；value 最多 1024 bytes且只含可打印字符。
10. SDK 固定设置 Authorization: Bearer、Accept、Content-Type 和 User-Agent: zai-rs/{crate-version}；caller 无法覆盖。HTTP Authorization 与 Realtime auth HeaderValue 立即 set_sensitive(true)，request/header map Debug 测试必须只显示 Sensitive/[REDACTED]。连接池固定 max_idle_per_host=8、idle_timeout=90s、tcp_keepalive=60s。
11. 先建立所有 service facade 的零成本入口：chat、images、videos、audio、embeddings、rerank、tokenizer、moderation、files、batches、knowledge、agents、tools、assistants、applications、tasks、zrag、usage。每个 facade 借用同一个 ZaiClient。
12. 加入一个 crate-private LegacyRequestAdapter，让尚未迁移的请求在 P05 前调用同一个 ClientInner；P05 必须删除该类型。
13. dev-dependency Tokio 显式增加 net；新建 tests/support/http_server.rs。TestServer 绑定 127.0.0.1:0，提供脚本化响应队列、chunked body、连接中断、虚拟延迟、请求捕获和并发 barrier；P02 起的全部传输测试复用它。

验收：

- 创建 1000 个 service facade 不创建额外 reqwest client，不复制 secret。
- Debug 输出中没有 API key。
- 空 ID、.、.. 被拒绝；a/b、a?b、a#b、a%b 分别测试并只形成一个 percent-encoded path segment。
- 非法 base、userinfo、fragment、HTTP 公网地址在 build 时失败。
- additional_header("X-Test-Client", "preserved") 能被本地 mock server 观察到；Authorization、Cookie 和 Proxy-Authorization 覆盖尝试在 build 时失败。

验证：

~~~bash
cargo test --locked client::
cargo test --locked --test client_builder
cargo clippy --locked --all-features --all-targets -- -D warnings
~~~

### P03 — 重建 Transport、错误、重试、timeout、limits 和日志

- 状态：未执行
- 优先级：P0
- 依赖：P02
- 提交标题：refactor(transport): enforce retry safety limits and redaction

实施：

1. 删除全局 HTTP_CLIENTS 和公开 HttpClient trait，建立 crate-private Transport。
2. 定义 sealed RequestSpec 和 PreparedRequest。RequestSpec 的关联项固定包含 OperationId、Response、StreamItem、METHOD、API_FAMILY、PATH_TEMPLATE、REQUEST_CONTENT_TYPE、ACCEPT、SUCCESS_STATUSES、RETRY_SAFETY、RESPONSE_MODE、REQUIRES_DONE 和 SUCCESS_INVARIANT。
3. 定义 RetrySafety::Idempotent 与 RetrySafety::NonIdempotent；所有 request builder 提供 with_retry_override(RetryOverride::AssumeIdempotent) -> Self，override 不进入序列化 body。
4. Transport 固定执行：validate → build URL → encode body → enforce request limit → send/retry → enforce response limit → probe error → decode private wire → validate invariant → public conversion。
5. 实现第 4 节的 method/retry status 矩阵。effective safety 为 NonIdempotent 时只有一次 attempt；Idempotent 最多三次 attempt。注入可种子化 JitterSource；第 n 次 retry 的 full jitter 上限按第 4 节公式计算。
6. 将 connect、attempt、overall、stream idle 四种 timeout 分开。普通请求默认值固定为 10s、60s、120s；stream idle 固定为 60s。
7. Retry-After 只对 429/503 生效，支持整数秒与 HTTP-date；合法值替代 jitter，非法值使用 jitter。delay >= remaining deadline 时直接返回 Timeout。
8. ZaiError 固定为字段私有、#[non_exhaustive] 的 public struct，不暴露 raw reqwest::Error、Url 或 body。公开形状固定为：

| 类型 | 固定内容 |
|---|---|
| ZaiErrorKind | Config、Validation、Transport、Http、Decode、Protocol、PayloadTooLarge、Timeout、RetryExhausted、Io、Realtime、Tool |
| ApiCode | Number(i64)、Text(Box&lt;str&gt;) |
| TransportKind | Connect、Dns、Tls、Read、Write、Closed、Other |
| TimeoutPhase | Connect、Attempt、Overall、Idle、Tool、Close |
| 字段 | kind: ZaiErrorKind；message: Box&lt;str&gt;；status: Option&lt;http::StatusCode&gt;；api_code: Option&lt;ApiCode&gt;；request_id: Option&lt;Box&lt;str&gt;&gt;；retry_after: Option&lt;Duration&gt;；attempts: u8；field: Option&lt;&'static str&gt;；limit: Option&lt;u64&gt;；source: Option&lt;SanitizedSource&gt;，全部 private |
| accessors | kind()、message()、status()、api_code()、request_id()、retry_after()、attempts()、field()、limit()、is_auth_error()、is_rate_limit()、is_retryable() |
| traits | ZaiError: Error + Display + Debug + Send + Sync + 'static，不实现 Clone；kind/code/phase 实现 Clone、Eq，适用时实现 Copy |
| Display | zai-rs {kind-code}: {sanitized-message}，随后只追加存在的 status、numeric code、request_id；Text code 不进入 Display |
| Debug/source | 与 Display 使用同一脱敏字段，Text code 只显示 [TEXT_CODE]；source 只允许 SanitizedSource，不保留 materialized URL、body、query、path ID 或 secret |

ZaiErrorKind 的 kind-code 固定为 config、validation、transport、http、decode、protocol、payload_too_large、timeout、retry_exhausted、io、realtime、tool。ZaiErrorSummary 固定为可 Clone 的 kind、message、request_id 私有字段及同名 accessor。
9. Http 错误保留 status、ApiCode、按 kind/status 固定的通用 message、服务端 correlation request_id 和 retry_after。request_id 只保留 128 bytes 可打印 ASCII。ApiCode::Text 只接受 1..=128 bytes 和字符集 [A-Za-z0-9_.:-]，并拒绝包含当前 API secret 的值；不合规则丢弃为 None。Text code 只可由 api_code() 显式读取，不进入 Display/Debug/source/trace。
10. 错误 body 最多读取 64 KiB。recognized JSON envelope 只提取合法 ApiCode 与 request_id，不保存 server message；未知 body 只保留长度和 SHA-256，public message 固定为 unrecognized error response。恶意 code/message 回显 secret 的测试必须遍历 Display、Debug 与 source chain。
11. 实施第 4 节全部正文限制。gzip 按 decoded bytes 计数，超限后立即停止读取。
12. JSON response 只接受 application/json 或 +json；SSE 只接受 text/event-stream；binary response 只接受 manifest 中的 MIME。mismatch 返回 Protocol::UnexpectedContentType。
13. tracing 只输出固定 metadata 字段。route 使用 PATH_TEMPLATE；不输出 materialized URL、header value、query value、body 或用户 path/resource ID。
14. reqwest client 固定 redirect::Policy::none()。Transport 收到 3xx 后读取最多 8 KiB 的 Location，相对值用当前 Url 解析，再拒绝 userinfo、fragment、非 HTTP(S)、跨源和 TLS downgrade；raw Location 不进入 error/trace。redirect 最多三跳并计入 overall deadline。effective NonIdempotent 的任何 3xx 都不跟随；GET/HEAD 可跟随 301、302、303、307、308；其他 Idempotent 只跟随 307、308。任何 method 都不执行 301/302/303 的 method rewrite，不允许重复发送 NonIdempotent body。
15. 添加 tokio-util =0.7.18、default-features=false、features=["io"]；把 sha2 =0.11.0 改为 core normal dependency并从 realtime feature移除。在 transport 中实现 AtomicDownloadSink 和 MultipartBodyFactory。AtomicDownloadSink 先拒绝已存在目标，以 create_new(true) 在同目录创建随机 .part，Unix 权限 0600；PartialFileGuard 持有 Option&lt;File&gt; 与 path，成功路径 flush+sync_all、关闭句柄、rename 后 commit，Drop 先关闭句柄再删除 partial。MultipartBodyFactory 每个 attempt 重新打开 Path 并用 ReaderStream 输出。
16. dev-dependency Tokio 在本任务增加 test-util；用注入 Clock、Sleeper 和 JitterSource 测试 backoff 与 deadline，测试不发生真实 sleep。

验收：

- 仓库只有一个 reqwest::Client 实例的构建位置。
- effective safety 为 NonIdempotent 且无 override 的 POST/PATCH 在 timeout、连接中断、429、503 下都只发送一次。
- 带 AssumeIdempotent override 的 POST 在两次 503 后 200 时恰好发送三次；override 不出现在 body。
- GET 在两次 503 后 200 时恰好发送三次。
- PUT/DELETE 使用 idempotent 矩阵；501/505 不重试。
- overall timeout 包含 attempt 与 backoff，误差不超过 100ms 虚拟时间。
- 32 MiB JSON、64 KiB error 和压缩炸弹均在固定边界停止。
- 捕获的 Display、Debug、完整 Error::source() chain 和 trace 不含 secret、正文、query value、完整 URL和用户 path/resource ID；允许经过清洗的服务端 correlation request_id。

验证：

~~~bash
cargo test --locked --test transport_retry
cargo test --locked --test transport_limits
cargo test --locked --test transport_redaction
cargo test --locked --test redirect_policy
cargo clippy --locked --all-features --all-targets -- -D warnings
cargo run --locked -p xtask -- forbidden check P03
~~~

### P04 — 修正 Agent、异步任务、ASR、TTS 和 Knowledge 契约

- 状态：未执行
- 优先级：P0
- 依赖：P03
- 提交标题：fix(api): align agent async audio and knowledge contracts

实施：

1. 删除错误的 Agent CRUD、/paas/v4/agents/{id}/chat 和 /history 实现，不保留兼容 alias。
2. 实现三个 AgentV1 RequestSpec：
   - POST /v1/agents，对外方法 client.agents().invoke(request)。
   - POST /v1/agents/async-result，对外方法 client.agents().async_result(request)。
   - POST /v1/agents/conversation，对外方法 client.agents().conversation(request)。
3. Agent 公开签名固定为：
   - agents().invoke(AgentInvokeRequest&lt;NonStreaming&gt;) -> ZaiResult&lt;AgentInvokeResponse&gt;
   - agents().stream(AgentInvokeRequest&lt;Streaming&gt;) -> ZaiResult&lt;AgentEventStream&gt;
   - agents().async_result(AgentAsyncResultRequest) -> ZaiResult&lt;AgentAsyncResult&gt;
   - agents().conversation(AgentConversationRequest) -> ZaiResult&lt;AgentConversationResponse&gt;
4. AgentInvokeRequest 固定包含 agent_id、至少一条 messages、由 type-state 设置的 stream 和 Map&lt;String, Value&gt; custom_variables。Completed response 必须有 id、agent_id 和非空 choices；Pending response 必须有 agent_id、async_id；其他组合返回 Decode。
5. AgentAsyncResult 的 Pending 必须有 agent_id/async_id；Succeeded 必须额外有非空 choices；Failed 保留 agent_id/async_id 并作为正常 task result 返回。Conversation success 必须有 conversation_id、agent_id 和非空 choices；embedded error 返回 Err。
6. 删除 AsyncChatCompletion 的 stream type parameter、with_stream、enable_stream 和 SseStreamable；async chat body 不序列化 stream。
7. AsyncTaskGetRequest 只接收 task_id，不接收 model 或 PhantomData。GET /async-result/{id} 的唯一 public 入口固定为 tasks().get(task_id) -> ZaiResult&lt;AsyncTaskResponse&gt;；AsyncTaskResponse 是 Pending、Chat、Image、Video、Failed 的 tagged public enum，各 variant 按冻结响应字段严格判别。chat/images/videos 不再提供重复 async_result/result adapter。
8. VideoGenerationRequest 增加唯一 send 路径，先执行 prompt/image 跨字段校验，再提交任务。
9. SpeechToTextRequest 使用 type-state 保证 file 与 file_base64 恰好提供一种，字段固定为 model、prompt、hotwords、stream、request_id、user_id；删除 temperature；格式只接受 wav/mp3；hotwords 最多 100；request_id 长度 6..=64；user_id 长度 6..=128；文件最大 25 MiB。30 秒时长与 prompt 小于 8000 字是服务端限制/建议，SDK 文档说明但不在客户端解析音频或拒绝 prompt。
10. ASR 签名固定为 audio().transcribe(SpeechToTextRequest&lt;NonStreaming&gt;) -> SpeechToTextResponse 与 audio().transcribe_stream(SpeechToTextRequest&lt;Streaming&gt;) -> SpeechToTextStream；stream item 固定 SpeechToTextEvent，收到 [DONE] 才正常结束。
11. TextToSpeechRequest 的 model/input/voice 为必需字段；input 最多 1024 Unicode scalar；VoiceId 是非空 string newtype并提供七个系统音色常量，允许复刻音色 ID；speed 0.5..=2；volume 0 < value <=10；response_format 为 wav/pcm，默认 pcm；stream 只允许 pcm；encode_format 只在 stream 中允许 base64/hex。
12. TTS 签名固定为 audio().synthesize(TextToSpeechRequest&lt;NonStreaming&gt;) -> Bytes、audio().synthesize_stream(TextToSpeechRequest&lt;Streaming&gt;) -> AudioByteStream、audio().synthesize_to(TextToSpeechRequest&lt;NonStreaming&gt;, path) -> ()。synthesize_to 使用 P03 AtomicDownloadSink；AudioByteStream 解码 base64/hex 后只产出 Bytes。
13. KnowledgeCreateRequest 增加 embedding_id=12 对应 Embedding-3-pro，并建模 embedding_model 与 contextual。
14. 所有这些响应先经过 error envelope probe，private wire model 的成功字段不使用全结构 default。
15. response-only enum 的未知字符串保存为 Other(String)；request enum 保持闭合。

验收：

- Agent 三个 method/path/body/response 与冻结契约一致。
- 旧 Agent CRUD symbol 和错误路径完全消失。
- async chat JSON 不含 stream。
- ASR 所有字段、长度、文件大小和互斥关系在联网前验证。
- TTS 空 input、1025 字符、volume=0 在联网前失败；wav、pcm 和 stream 成功。
- Knowledge embedding_id=12 序列化正确。
- 2xx 错误 envelope 和空成功对象全部返回 Err。

验证：

~~~bash
cargo test --locked --test agent_contract
cargo test --locked --test async_task_contract
cargo test --locked --test audio_contract
cargo test --locked --test knowledge_contract
cargo run --locked -p xtask -- forbidden check P04
~~~

### P05 — 将现有 API 家族全部迁移到 RequestSpec

- 状态：未执行
- 优先级：P0
- 依赖：P04
- 提交标题：refactor(api): migrate all existing endpoints to request specs

迁移顺序固定为：

1. chat、images、videos、audio、embeddings、rerank、tokenizer、moderation。
2. files、batches、web search、file parser、OCR。
3. knowledge、usage。
4. realtime 的共享配置入口。

实施：

1. 每个 operation 实现一个 RequestSpec；每个 public operation 只有一个 typed send 或 stream 入口。
2. 删除每个请求类型中的 key、materialized URL、EndpointConfig、ApiBase 和 HttpClientConfig 字段。
3. 删除 with_base_url、with_endpoint_config、with_http_config、send_with_query 和 raw get/post/put/delete。
4. body、query 和 path 参数始终在网络连接前验证。list limit、cursor、ID、文件扩展名、文件大小和跨字段约束全部进入 validator。
5. 知识库统一 private ApiEnvelope&lt;T&gt;，code != 200 必须返回 Http error。
6. chat、task、video、file、batch 和 knowledge 使用各自的严格 response；删除包罗所有场景且全 Option 的响应模型。
7. 合并重复的 SearchIntent、SearchResult、WebSearchInfo 和 knowledge envelope。
8. wire types 保持 pub(crate)；public response 的类型名、字段和 accessor 严格使用第 13、14 节规则，并只暴露清洗后的服务端 correlation request_id。
9. 当前 GET /knowledge/{id} 固定命名 KnowledgeGetRequest 与 client.knowledge().get；删除旧 KnowledgeRetrieveRequest。P06 的 POST /knowledge/retrieve 使用不同的 KnowledgeSearchRequest 与 client.knowledge().retrieve。
10. 删除全部 0.4 请求/响应/client 公共路径，不创建 compatibility alias；docs/MIGRATING-0.5.md 逐项记录替代路径。
11. 删除 LegacyRequestAdapter。
12. 每完成一个家族只把 coverage.toml 的状态从 missing 改为 covered；P00 固定的 Rust symbol、test name、method/path 和 public mapping 不得修改。

验收：

- 所有 public 网络调用都经过 Transport。
- 每个 request family 至少有 method、path、auth、body、success、error 和 validation 测试。
- 资源请求类型不再保存凭据或传输配置。
- {} 和 {"unexpected":true} 不能解析为任何成功响应。
- 同一 wire/domain 类型只存在一个定义。

验证：

~~~bash
cargo test --locked --all-features --all-targets
cargo run --locked -p xtask -- contract check
cargo run --locked -p xtask -- forbidden check P05
~~~

### P06 — 补齐冻结 OpenAPI 的缺失 operations

- 状态：未执行
- 优先级：P0
- 依赖：P05
- 提交标题：feat(api): cover the complete frozen operation manifest

P04 已补齐三个 Agent operation。本任务按下面的固定映射实现其余 17 个基线缺口：

| Method/path | 固定 public 方法 |
|---|---|
| POST /paas/v4/async/images/generations | client.images().generate_async |
| POST /paas/v4/files/parser/sync | client.files().parse_sync |
| POST /paas/v4/layout_parsing | client.tools().parse_layout |
| POST /paas/v4/reader | client.tools().read_document |
| POST /paas/v4/assistant | client.assistants().invoke |
| POST /paas/v4/assistant/list | client.assistants().list |
| POST /paas/v4/assistant/conversation/list | client.assistants().conversations |
| POST /llm-application/open/knowledge/retrieve | client.knowledge().retrieve |
| GET /llm-application/open/history_session_record/{app_id}/{conversation_id} | client.applications().history |
| POST /llm-application/open/v2/application/file_stat | client.applications().file_stats |
| POST /llm-application/open/v2/application/file_upload | client.applications().upload_file |
| POST /llm-application/open/v2/application/slice_info | client.applications().slice_info |
| POST /llm-application/open/v2/application/{app_id}/conversation | client.applications().create_conversation |
| GET /llm-application/open/v2/application/{app_id}/variables | client.applications().variables |
| POST /llm-application/open/v3/application/invoke | client.applications().invoke |
| POST /zrag/agent/chat | client.zrag().chat |
| POST /zrag/retrieval/retrieve | client.zrag().retrieve |

实施规则：

1. 每个 operation 使用独立 RequestSpec、private wire request/response 和 public domain request/response。
2. 所有 schema required、enum、min/max、format、content type 和 path/query 约束来自冻结快照。
3. 不使用 public serde_json::Value 代替已知对象；只有第 13.1 节规则命中的 additionalProperties/空 schema、JSON Schema 本体和 custom_variables 使用 Map&lt;String, Value&gt;。
4. 每个 POST 默认 NonIdempotent。冻结文档明确声明幂等的 operation 才在 RequestSpec 标记 Idempotent。
5. 新增 multipart operation 直接使用 P03 的 MultipartBodyFactory；不得建立第二种 multipart factory，不整文件读取、不 clone 文件 bytes。
6. coverage.toml 中 59 个 OpenAPI operation 全部把 status 改为 covered；Rust symbol 与 contract test name 必须继续等于 P00 固定值。
7. xtask 从 operations.json 生成 tests/contract_matrix.rs；每个 operation 都使用 P02 TestServer 运行 request golden、success golden、error golden 和 validation case。
8. Coding Plan 与 Realtime 保持单独 coverage 行，不计入 59 的分母。

验收：

- contract verify 报告 59/59。
- 每个新增 operation 有 request golden、success golden、error golden 和 local mock transport test。
- public API 中不存在未知 schema 的 Value-only response。
- 17 个路径全部经过 percent-encoded dynamic segment 与统一 auth。

验证：

~~~bash
cargo run --locked -p xtask -- contract verify --require-covered
cargo test --locked --test contract_matrix
cargo test --locked --all-features --all-targets
~~~

### P07 — 将上传、下载和轮询改为有界流式实现

- 状态：未执行
- 优先级：P1
- 依赖：P06
- 提交标题：perf(io): stream multipart downloads and polling

实施：

1. 把 file、ASR、OCR、file parser 和 knowledge 的全部上传迁移到 P03 MultipartBodyFactory；ReaderStream chunk 固定 64 KiB，不读取完整文件，不 clone Vec。
2. 单个 multipart request 顺序打开文件，任一时刻最多持有一个文件描述符；ZaiClient 级 upload semaphore 固定最多四个并发上传 request。
3. Path 上传在建连前用 symlink_metadata 拒绝 symlink 与非 regular file，并拒绝不支持扩展名、超过 16 个 file part、overflow 或 file bytes 合计超过 min(endpoint limit,128 MiB)；非文件 multipart 字段序列化后合计不得超过 1 MiB。每个 attempt open 后从 handle metadata 再检查 regular/size，防止 TOCTOU 替换。
4. wire filename 只取 basename，UTF-8 后长度 1..=255 bytes，拒绝控制字符、引号、反斜杠和 /；error/trace 不记录本地完整 path。
5. Bytes 上传在构建 request 时检查限制；重试复用 Arc&lt;Bytes&gt;，不复制底层数据。
6. download_to(path) 复用 P03 AtomicDownloadSink；目标已存在时返回 Io::AlreadyExists 且不覆盖。Linux 与 Windows 测试都验证失败、取消和 Drop 后没有 .part 残留。
7. File content 不再通过 send() 聚合整个响应；send_bytes 只在 128 MiB 限制内显式聚合。
8. SSE、文件下载和音频流每收到有效 chunk 重置 60s idle timer。
9. polling 使用绝对 Instant deadline；sleep 为 min(interval, remaining)，默认 interval 固定 1s，调用方传入小于 1s 时返回 Validation。
10. file parser get_all_results 使用 futures_util::future::try_join 并发请求，不依赖 tokio 宏。
11. 增加取消安全测试：future drop 后关闭 body、删除 partial、没有后台 task 残留。
12. tests/support/tracked_reader.rs 让每个 64 KiB payload 使用 Bytes::from_owner 的 Drop guard，统计已产出但尚未释放的 live payload；带 backpressure 消费 128 MiB 时断言 live payload <=16 MiB、total read=128 MiB、每个 attempt 只 open 一次。RSS 与总 heap 只记录为非阻塞 benchmark。

验收：

- 上传 128 MiB 文件时，TrackedReader 的 live payload 不超过 16 MiB、total read 等于 128 MiB、每个 attempt 只 open 一次。
- multipart 的 idempotent retry 每次重新打开文件且 body 完整；NonIdempotent 不重试。
- 下载失败没有目标文件和 .part 残留。
- polling 不超过 deadline 100ms，测试使用虚拟时间。
- 两个 file parser GET 同时发出。

验证：

~~~bash
cargo test --locked --test multipart_streaming
cargo test --locked --test download_streaming
cargo test --locked --test polling
cargo test --locked --test cancellation
~~~

### P08 — 加固 SSE 与 Realtime 协议终态

- 状态：未执行
- 优先级：P1
- 依赖：P07
- 提交标题：fix(streaming): make sse and realtime bounded and terminal

实施：

1. SSE decoder 接受任意 chunk 边界，正确处理 CRLF 和 UTF-8 跨 chunk；多行 data 用 \n 连接，comment 忽略，event/id/retry 解析为 private metadata但不公开、不改变 retry policy、不触发自动重连。
2. line、event 和未解析 buffer 各执行 1 MiB 限制；超限产生 PayloadTooLarge item 并关闭连接。
3. 每个 streaming operation 使用 operations.json 的 done marker：chat、Agent v1、ASR、TTS 为 data: [DONE]；Zrag Agent 为 type=done 事件。缺少对应 marker 的 EOF 返回 Protocol::UnexpectedEof。
4. JSON decode、HTTP body、idle timeout 和 server error 都作为 stream item Err 发送一次，然后 stream 终止。
5. Realtime 驱动循环使用公平 tokio::select!，每轮同时服务 command、socket receive 和 shutdown。客户端不发送主动 heartbeat；收到 WebSocket Ping 后在 10s send deadline 内立即回复 Pong。
6. connect 与 send deadline 固定 10s，close 固定 5s。配置 tokio-tungstenite 为 message 8 MiB、frame 2 MiB。
7. 新增 watch 状态 Open、Closed、Failed(ZaiErrorSummary)。event/audio stream 在 Closed 结束，在 Failed 先产出 Err 再结束。
8. Pong、base64 decode、socket send/recv、server error、driver join 的错误全部传播。
9. command、event、audio channel capacity 全部固定为 8。command queue 满时 send 受 10s deadline；broadcast Lagged(n) 向该 consumer 产出一次 Realtime::Lagged 后终止。
10. audio delta 同时产生原始 typed event 和 decoded Bytes；单个 inbound decoded audio 与 outbound audio frame 都不得超过 4 MiB。
11. driver 终止时 drop 内部 broadcast sender；close 等待 driver 并返回 join/close handshake 错误。
12. 本地 WS server 覆盖 handshake header、session update、音频、ping/pong、server error、oversize、abrupt EOF 和正常 close；用八个最大 frame 验证 channel 高水位和 Lagged 终态。

验收：

- 高频 command 不会饿死 incoming event。
- 任意错误后消费者都在有限时间内收到终态，不会永久 pending。
- SSE 缺 [DONE]、oversize 和 malformed JSON 均返回明确错误。
- Realtime 所有 size/deadline 限制可由测试稳定复现。

验证：

~~~bash
cargo test --locked --features realtime --test sse_protocol
cargo test --locked --features realtime --test realtime_protocol
cargo test --locked --features realtime --test realtime_fairness
~~~

### P09 — 让 ToolExecutor 的副作用、缓存和并发安全可证明

- 状态：未执行
- 优先级：P1
- 依赖：P08
- 提交标题：fix(toolkits): enforce effect aware execution

实施：

1. 在开始 ToolExecutor 改造前先建立最终 feature 关系和 optional dependencies：

~~~toml
[features]
default = []
toolkits = [
  "dep:async-trait",
  "dep:dashmap",
  "dep:jsonschema",
]
rmcp-kits = ["toolkits", "dep:rmcp"]
realtime = [
  "dep:async-trait",
  "dep:tokio-tungstenite",
  "dep:hmac",
  "dep:tokio-stream",
  "dep:http",
  "dep:uuid",
  "tokio/net",
  "tokio/rt",
  "tokio/macros",
]
tool-validation = ["toolkits"]
~~~

2. async-trait、dashmap、jsonschema 设为 optional；确认 P03 的 sha2 继续作为 core normal dependency且不挂 feature；rmcp 固定 version="=1.8.0"、default-features=false、features=["client"]。tool-validation 是文档声明的 compatibility alias，不产生编译期弃用告警，在 Cargo.toml、feature 表和 Rustdoc 标记将在 0.6 删除。
3. ToolDefinition 必须携带 ToolEffect；未设置时固定为 SideEffecting。
4. 执行矩阵固定为：
   - Pure：允许缓存；瞬时错误最多重试 2 次。
   - Idempotent：不缓存；瞬时错误最多重试 2 次。
   - SideEffecting：不缓存；不重试。
   - Timeout：三类都不自动重试。
5. 瞬时工具错误只包括 Transport connect/read 和远端 408、425、429、500、502、503、504；Validation、Protocol、Decode、Tool handler error 和 Timeout 不重试。effective retry 次数为 min(configured, 2)，默认 configured=0；两次 retry 使用 full jitter、base 200ms、cap 1s。
6. ToolExecutor 默认 cache disabled、retry count 0、deadline 30s、concurrency 8、batch limit 64、input 256 KiB、output 1 MiB。with_cache(capacity: NonZeroUsize, ttl: Duration) 是唯一启用入口，ttl=0 返回 Validation。
7. toolkits feature 开启后始终执行 JSON Schema 参数验证；删除运行时 validate_parameters 开关和无效 logging 开关。
8. tool schema 序列化后 <=256 KiB，JSON nesting <=64、总 node <=4096；$ref 只允许同文档 fragment，拒绝 remote URI。input canonicalization 先迭代检查 depth<=64、nodes<=4096，再做稳定 key 排序；string、key 和 number 语义不 trim。
9. tool generation 使用从 1 开始的进程内 AtomicU64；只有成功 register/re-register 在 schema 验证后分配新 generation。cache key 为 SHA-256(tool name、tool generation、canonical input)，不保存明文 input。
10. TTL 使用 Instant；容量在同一锁内精确维护；过期、unregister 和 re-register 都删除旧 generation 条目。
11. schema cache 使用完整 schema bytes equality 与 SHA-256，不使用单独 u64 hash。
12. batch 调度只创建最多 8 个未完成 future；使用 FuturesUnordered 补槽，不先 spawn 全部调用。
13. RMCP client options 强制 30s deadline、8 concurrency 和 output limit；连接失败与 peer error 转成 Tool error。
14. 注册使用 DashMap entry 原子检查，重复名称返回 AlreadyRegistered；导出 tool 列表按名称排序。
15. 传给模型的 ToolErrorCode/message 固定为：invalid_arguments/Tool arguments are invalid.、not_found/Tool is unavailable.、timeout/Tool execution timed out.、rate_limited/Tool is temporarily rate limited.、output_too_large/Tool output exceeded the limit.、cancelled/Tool execution was cancelled.、failed/Tool execution failed.。AlreadyRegistered 只返回 SDK caller，不传给模型。详细 source 仅进入经过脱敏的 metadata trace。

验收：

- SideEffecting 工具在 timeout、503 和连接中断下只执行一次。
- Pure 缓存命中不执行 handler；字符串 "null" 与 JSON null、含空格 key 与 trim 后 key 使用不同 cache key。
- 同名并发注册只有一个成功。
- 10,000 个 batch 调用过程中已创建未完成 task 不超过 8。
- cache capacity、hit/miss 和 TTL 统计准确。
- 错误返回和日志不含 tool input、secret 或原始 server body。
- depth=65、nodes=4097、remote $ref 和递归 fragment schema 在执行 handler 前失败；property/fuzz 覆盖深层 array/object 与递归 $ref。

验证：

~~~bash
cargo test --locked --features toolkits --test tool_effects
cargo test --locked --features toolkits --test tool_cache
cargo test --locked --features rmcp-kits --test rmcp_executor
cargo test --locked --features toolkits --test tool_concurrency
cargo test --locked --no-default-features --features tool-validation
~~~

### P10 — 收敛公共 API、模块边界和依赖图

- 状态：未执行
- 优先级：P1
- 依赖：P09
- 提交标题：refactor(api): finalize the minimal 0.5 public surface

实施：

1. crate root 只直接导出 ZaiClient、ZaiConfig、ZaiError、ZaiResult；prelude 精确导出这四项以及 ChatRequest、ChatMessage、ChatEventStream、NonStreaming、Streaming、ModelName，不加入其他符号。
2. client 模块只显式导出稳定配置和策略类型；transport、decoder、redaction、RequestSpec 全部为 pub(crate)。
3. 公共请求命名固定为 ChatRequest、ImageGenerationRequest、VideoGenerationRequest、SpeechToTextRequest、TextToSpeechRequest、EmbeddingRequest、RerankRequest、BatchCreateRequest、BatchListRequest、BatchGetRequest 和 BatchCancelRequest。
4. spec/contracts/public-api.toml 把基线全部 symbol identity 标为 removed，并把所有 0.5 symbol identity 标为 added；xtask public-api check 断言没有未归类 symbol。迁移文档列出旧路径的一对一替代。
5. model::tools 移到 chat::tools，旧路径删除。standalone tools service 保持 client.tools()。
6. ModelName 改成 fn id(&self) -> &'static str；内置模型实现 const ID、Copy、Clone、Default；Serialize 直接调用 serializer.serialize_str(self.id())，不构造中间 String。
7. 拆分超过 600 行的生产文件，目标边界按第 5 节目录。xtask module-size check 对手写 src 文件执行 600 行硬门禁。
8. tokio-tungstenite 关闭 default features，只开启 connect 与 rustls-tls-native-roots。
9. reqwest 关闭 default features，开启 rustls、http2、system-proxy、multipart、stream、gzip；删除 json feature。
10. chrono 关闭 default features，开启 clock、std。Tokio 正常依赖最终固定 features=["time","fs","io-util","sync"]；dev-dependency Tokio 最终固定额外 features=["rt-multi-thread","macros","test-util","net"]。
11. tokio-util 固定 version="0.7.18"、default-features=false、features=["io"]。dev-dependencies 保留 P01 的 tempfile=3.27.0并增加 proptest=1.11.0、trybuild=1.0.117。
12. 给 feature 相关 examples 配 required-features；Rustdoc 使用 doc(cfg) 显示 feature。
13. xtask dep-budget 对 target x86_64-unknown-linux-gnu 分别运行 no-default 与 all-features cargo metadata，使用 resolve.nodes 的实际 feature-aware 激活图，从 zai-rs 只沿 normal dep_kind edge 统计唯一 PackageId(name/version/source)，排除 root、dev/build edge 和其他 workspace member；不得遍历未激活的 packages 列表。预算固定 default <=136、all-features <=200。

验收：

- 公共示例只创建一次 ZaiClient，构造器不再接收 API key。
- root 没有 wildcard re-export；prelude 是唯一允许的集中 re-export。
- 内置模型 id() 为零分配；自定义零分配 Serializer 证明 Serialize 不构造中间 String，输出 sink 自身分配不计。
- 所有手写生产文件不超过 600 行。
- default 与每个单 feature 独立编译。
- 依赖图满足 136/200 节点预算。

验证：

~~~bash
cargo check --locked -p zai-rs --no-default-features --all-targets
cargo check --locked -p zai-rs --no-default-features --features toolkits --all-targets
cargo check --locked -p zai-rs --no-default-features --features rmcp-kits --all-targets
cargo check --locked -p zai-rs --no-default-features --features realtime --all-targets
cargo check --locked -p zai-rs --no-default-features --features tool-validation --all-targets
cargo check --locked -p zai-rs --all-features --all-targets
cargo run --locked -p xtask -- module-size check
cargo run --locked -p xtask -- dep-budget check
cargo run --locked -p xtask -- forbidden check P10
~~~

### P11 — 建立真实传输、属性、类型状态、模糊和覆盖率门禁

- 状态：未执行
- 优先级：P1
- 依赖：P10
- 提交标题：test: cover every contract and critical runtime path

实施：

1. 扩展 P02 TestServer，补充 TLS downgrade、WS handshake、半关闭 socket 和 backpressure；不建立第二套 mock server。
2. 删除纯 sleep、手写 retry loop、只检查 serde_json::Value 的伪集成测试。
3. 扩展 P06 contract_matrix，使每个 operation 固定断言 method、path、query、Authorization、content type、request golden、success golden 和 error golden。
4. 每个 public request 增加网络前 validation case；server request count 必须为 0。
5. 使用 proptest 覆盖动态 path/query 编码、ApiCode roundtrip、error envelope probe、canonical JSON、model ID、SSE chunk split 和 retry deadline。
6. 使用 trybuild 覆盖 Agent stream/nonstream、ASR file/file_base64、必填 request builder 和不兼容模型的 compile-pass/compile-fail。
7. 新建 resolver=3 的独立 fuzz workspace 和 targets：fuzz_error_envelope、fuzz_sse_decoder、fuzz_url_segments、fuzz_tool_canonical_json。fuzz/Cargo.toml 只依赖 libfuzzer-sys 和启用 toolkits+realtime 的 path zai-rs，不允许 git dependency；提交 fuzz/Cargo.lock，作为唯一嵌套 lock 例外。xtask fuzz smoke 清空 target/fuzz-run，复制只读 seed corpus 到每个 target 的临时 corpus，把 artifact_prefix 指向 target/fuzz-run/{target}/artifacts，并固定调用 nightly-2026-07-10；不得写 fuzz/corpus。
8. 通过 check-cfg 声明 cfg(fuzzing)，只在该 cfg 下用 src/fuzz_support.rs 薄包装暴露 private decoder/canonicalizer；正常 build 的 public API 不增加 fuzz symbol。每个 target 对任意输入不得 panic、不得超过固定 buffer。
9. 覆盖 Realtime local WS 和 RMCP fake peer 的完整终态，不启动公网连接。
10. cargo llvm-cov 只统计 zai-rs package 并生成 JSON；忽略 tests、examples 和 generated 路径。xtask coverage check 执行 line 75、region 70、function 65 的全包阈值，并对 src/client/**、src/services/agents/**、src/services/audio/**、src/toolkits/**、src/realtime/** 分别执行 line 90 阈值。
11. 增加确定性资源预算测试：
    - ModelName::id() 零分配，Serialize 不构造中间 String。
    - 128 MiB upload 的 TrackedReader live payload <=16 MiB。
    - SSE buffer <=1 MiB。
    - Tool 未完成 task <=8。
12. 所有时间相关测试使用 paused time。.config/nextest.toml 的 ci profile 固定 slow-timeout={period="5s", terminate-after=1}。xtask test-budget 跨平台启动 cargo nextest，90s 到期后终止子进程并失败；doctest 继续使用 cargo test。
13. xtask 增加 syn=2.0.118、default-features=false、features=["full","parsing","visit"]；tests check-no-ignore 用 syn 解析 workspace Rust test attributes，拒绝 #[ignore] 与 cfg_attr(..., ignore)；xtask docs check 独立拒绝 ignored doctest。

验收：

- 59/59 OpenAPI（包含三个 Agent operation）、Coding Plan 和 Realtime coverage 行都有真实 transport test。
- Rust test ignore attribute 为 0；ignored doctest 在 P13 归零。
- 覆盖率达到固定阈值。
- fuzz smoke 各运行 60 秒，无 crash、OOM 或超限。
- 测试不存在真实 ZHIPU_API_KEY 读取和公网域名连接。

验证：

~~~bash
cargo test --locked --workspace --all-features --all-targets
cargo llvm-cov --locked -p zai-rs --all-features --ignore-filename-regex '(^|/)(tests|examples|generated)/' --json --output-path target/coverage.json
cargo run --locked -p xtask -- coverage check target/coverage.json
cargo run --locked -p xtask -- fuzz smoke --seconds 60
cargo nextest run --locked -p zai-rs --no-default-features --no-run
cargo run --locked -p xtask -- test-budget
cargo run --locked -p xtask -- tests check-no-ignore
~~~

### P12 — 纳管所有独立示例并修复 web_chat

- 状态：未执行
- 优先级：P1
- 依赖：P11
- 提交标题：fix(examples): make every workspace example build and run

实施：

1. 扩展 P00 建立的根 workspace：把 examples/web_chat、examples/mcp/client、examples/mcp/servers、examples/mcp/list_remote_mcp_tools 从 exclude 移入 members，resolver 保持 3，exclude 只保留 fuzz。
2. 四个示例工程继承 workspace edition、rust-version、license 和依赖版本，统一 rmcp 1.8；删除四个示例的 Cargo.lock。Git 只跟踪根 Cargo.lock 与 P11 的 fuzz/Cargo.lock。
3. web_chat 的 Config 使用 SecretString 并手写脱敏 Debug，删除不需要的 Serialize/Deserialize；启动日志只输出 port、origins 数量、timeouts 和 feature flags。
4. web_chat 使用 ZaiClient 注入 AppState，不在每次请求 clone API key。
5. web_chat 删除 allow_credentials(true)，保留显式 origin allowlist、Any methods 和 Any headers；应用不使用 cookie。
6. 静态目录固定为 concat!(env!("CARGO_MANIFEST_DIR"), "/static")；删除依赖 current_dir 的重复 static handler。
7. 删除不存在的 service worker 注册、空 sessions router、未注册 handler、重复 index、未使用 server scaffolding。
8. 把下列 immutable jsDelivr 文件保存到 static/vendor，并逐个校验 SHA-256：marked@18.0.6/lib/marked.umd.js = 62ad5de5bea6d79b4c47e5c0b5cbe4be61e25ee8994595c2cc0969b2a144cc5d；dompurify@3.4.11/dist/purify.min.js = dbabb5b205a333ec49c8c09e7fca30ef66df0523bb8bc0fa9ea843841f111dbd；prismjs@1.30.0/prism.min.js = ed5ea2ce218febaea989b0734596bf218694488a4e92b6f107c6536d5675e04b；prismjs@1.30.0/themes/prism-tomorrow.min.css = 1b15fe2971998a048aebb60f26f6eed76122071db9ef3b995abd003224f52a98。删除自制 sanitizer、Prism autoloader 和运行时 CDN；只支持 Prism 默认 bundle 语言，未知 language 回退 plaintext。
9. CSP 固定为 default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: https:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'。把 index.html 的全部 inline script、style 标签和 style 属性迁移到本地 JS/CSS 与 class。
10. 新建 scripts/bootstrap-node.sh，固定 Node 24.13.0：linux-x64 tar.xz SHA-256 e798599612f4bb71333a3397ab0d095fd62214e115aea45aa858a145fc72d67e，darwin-arm64 tar.xz SHA-256 c59a517e9147f25c6167426875a571432f1478c1d7ee7ecc10baa46b0d0e8545，其他平台直接失败。web_chat 提交 private package.json/package-lock.json，唯一 devDependency 为 jsdom=29.1.1，npm 使用 Node 内置 11.6.2并始终 npm ci --ignore-scripts；examples/web_chat/.gitignore 增加 /node_modules。
11. 把 sanitizer 封装为可被 browser 和 Node harness 调用的本地模块。node:test + jsdom 加载 vendored DOMPurify 和 renderer，实际执行 XSS payload。
12. README 删除未实现的 session、Redis、Docker、KaTeX、Mermaid、production-ready 和 WCAG 声明；只描述自动测试覆盖的功能，并记录真实 npm test 命令。
13. 修复 LICENSE 链接；所有媒体示例从 CLI 参数读路径/URL，缺参数返回 usage。
14. README 示例索引由 xtask examples generate 产生，覆盖所有根 examples/*.rs，并记录 required feature、输入、是否需要 API key。
15. 加入 web_chat smoke test：假 key 启动随机端口，/、CSS、JS、/health 返回 200，日志不含 key，进程正常关闭。
16. 向 sanitizer 测试输入 img onerror、javascript: link 和 script 标签，断言 jsdom 输出不含危险属性、scheme 和标签；HTML parser 测试断言不存在无 src script、style 标签与 style 属性，并断言 HTTP CSP header 与第 9 项逐字相等。
17. MCP client 的 SSE parser 增加 success、multiple events、error payload 和 missing data 测试；删除 servers 中未使用的 service/field。

验收：

- cargo test/clippy --workspace --locked 全部通过。
- Git 跟踪的 lockfile 恰好为根 Cargo.lock 和 fuzz/Cargo.lock；examples 下没有 lockfile。
- web_chat 不 panic，静态路径不依赖 cwd，假 key 不出现在日志。
- README 宣称的每个 web_chat route 都存在测试。
- README 示例名集合与 examples/*.rs 完全相等。

验证：

~~~bash
cargo fmt --all -- --check
cargo check --locked --workspace --all-features --all-targets
cargo test --locked --workspace --all-features --all-targets
cargo clippy --locked --workspace --all-features --all-targets -- -D warnings
cargo run --locked -p xtask -- examples check
./scripts/bootstrap-node.sh
npm ci --prefix examples/web_chat --ignore-scripts
npm test --prefix examples/web_chat
test "$(git ls-files '*/Cargo.lock')" = "fuzz/Cargo.lock"
~~~

### P13 — 重写并编译验证全部用户文档

- 状态：未执行
- 优先级：P1
- 依赖：P12
- 提交标题：docs: publish compile checked 0.5 guides

实施：

1. 新建 book.toml 与 docs/SUMMARY.md；book.toml 固定配置 [book] src = "docs" 及 [rust] edition = "2024"，并把 GETTING_STARTED、ARCHITECTURE、ERROR_HANDLING、BEST_PRACTICES、ADVANCED_TOPICS、OCR_GUIDE 和 FAQ 纳入 mdBook。
2. 以 0.5 ZaiClient service API 重写所有示例；全项目环境变量统一为 ZHIPU_API_KEY。
3. 可编译且不应联网运行的 Rustdoc 使用 no_run；故意不能编译的教学片段使用 compile_fail；伪代码、JSON 和 shell 使用正确 fence。删除全部 rust,ignore。
4. 新建 docs/snippets workspace package，package name 固定 docs-snippets，依赖本地 zai-rs 及示例直接使用的 tokio、futures-util；把它加入根 workspace members。mdBook 代码通过 include 引用该 package 中的完整 source；同一段代码不在 README、docs 和 Rustdoc 复制。每个可编译 fence 以隐藏行引入 extern crate zai_rs。
5. 模块首页至少包含一个最小 no_run 示例，覆盖 chat、images、videos、audio、files、batches、knowledge、agents、toolkits、realtime 和 usage。
6. docs/MIGRATING-0.5.md 固定列出：
   - 一次性 ZaiClient 构造。
   - 每请求 key/config 构造器删除。
   - 错误 Agent CRUD 删除及三个 AgentV1 替代入口。
   - async chat stream 删除。
   - typed send/stream 与错误 variant 迁移。
   - 被删除类型/模块到新 service API 的对照表。
   - feature 变更。
7. README 明确区分 crates.io 已发布的 0.2 与 main 分支 0.5 release candidate；main 安装示例使用 Git branch=main，不声称 crates.io 已有 0.5。
8. README、CHANGELOG、Cargo.toml、docs 版本由 xtask version check 验证；0.5 内容统一标为 unreleased release candidate。
9. 修复全部 intra-doc link；使用 docs.rs 等价 nightly 命令验证 doc(cfg)。
10. release hard gate 使用 lychee --offline 只检查本地链接，保证可复现。P14 的每周 job 检查外部链接，timeout 20s、重试 3 次；该 job 生成报告但不阻塞 release candidate。
11. xtask docs check 校验 ignored fence=0、环境变量拼写、禁用旧 API symbol、include 同步和 README 示例索引。

验收：

- Rustdoc 0 ignored，mdbook test 通过。
- docs.rs 等价构建零 warning。
- 文档不存在旧 Agent CRUD、AsyncChat stream、每请求 key 构造器、ZAI_API_KEY 和 raw post/get 示例。
- 本地链接全部有效；官方契约来源已由 P00 hash 校验。
- 用户复制任一完整示例都能编译。

验证：

~~~bash
cargo test --locked -p docs-snippets
cargo build --locked -p docs-snippets --all-targets
cargo build --locked -p zai-rs --all-features
mdbook test -L target/debug/deps .
cargo test --locked -p zai-rs --all-features --doc
RUSTDOCFLAGS='--cfg docsrs -D warnings' cargo +nightly-2026-07-10 doc --locked -p zai-rs --all-features --no-deps
cargo run --locked -p xtask -- docs check
cargo run --locked -p xtask -- version check
lychee --offline --config lychee.toml README.md docs src
~~~

### P14 — 固化 CI、供应链和自动发布准备

- 状态：未执行
- 优先级：P1
- 依赖：P13
- 提交标题：ci: enforce complete quality and supply chain gates

实施：

1. 所有 GitHub Actions 设置 permissions: contents: read。PR/CI 每 job timeout-minutes=45，weekly fuzz=15，release publish=15；concurrency group 固定拼接 github.workflow、连字符和 github.ref，CI cancel-in-progress=true，release=false。只使用以下两个 action：
   - actions/checkout 固定 34e114876b0b11c390a56381ad16ebd13914f8d5。
   - actions/upload-artifact 固定 ea165f8d65b6e75b540449e92b4886f43607fa02。
   Rust toolchain、审计和发布步骤全部使用 shell 命令，不再引入其他 action；本轮 CI 不使用跨 job dependency cache。
2. ci.yml 固定 jobs：
   - fmt。
   - clippy workspace all-features all-targets。
   - root package check+test：no features、toolkits、rmcp-kits、realtime、tool-validation、all features。
   - workspace all-features check+test。
   - MSRV 1.88.0 check。
   - nightly docs.rs rustdoc。
   - mdBook test 与 lychee。
   - workspace examples 与 web_chat Node sanitizer。
   - contract verify。
   - llvm-cov 与阈值。
   - audit、deny、gitleaks。
   - package dry-run。
3. 所有 Cargo 命令使用 --locked。Linux runner 固定 ubuntu-24.04；另加 macOS 和 Windows 的 workspace check。fmt/clippy/MSRV 不安装外部工具；root default test job单独 bootstrap cargo-nextest 并运行 test-budget；coverage 只 bootstrap cargo-llvm-cov；docs 只 bootstrap mdbook 与 lychee；examples job 运行 bootstrap-node、npm ci --ignore-scripts、npm test；supply-chain 只 bootstrap audit、deny、cyclonedx、gitleaks；fuzz 只 bootstrap cargo-fuzz。macOS 运行 workspace check；Windows 还运行 download_streaming 与 cancellation 两个 integration test。
4. 新建 deny.toml，执行 advisories、bans、licenses、sources；allow list 固定为 MIT、MIT-0、Apache-2.0、BSD-2-Clause、BSD-3-Clause、0BSD、ISC、Unicode-3.0、Zlib、CC0-1.0、CDLA-Permissive-2.0 和 Unlicense；registry 只允许 crates.io，git source 列表为空。
5. 所有 checkout 设置 fetch-depth: 0 与 persist-credentials: false。.gitleaks.toml 继承 default rules，global path allowlist 只排除 (?:^|/)(?:target|node_modules)(?:/|$)，不得排除 src/docs/tests。gitleaks job 用校验过 SHA-256 的官方 binary 分别运行 git --redact --log-opts='--all' 和 dir --redact .；测试假 key 使用 test-id.test-secret。
6. cargo audit --deny warnings 同时检查 Cargo.lock 与 fuzz/Cargo.lock。xtask future-incompat check 设置 CARGO_TARGET_DIR=target/future-incompat，固定运行 cargo check --locked --workspace --all-features --all-targets --future-incompat-report，只解析本次报告，任何条目都失败。
7. Cargo.toml 的 include 与 spec/package-allowlist.txt 固定只覆盖 P00 第 12 项列出的路径。xtask package check 调用 cargo package --list --allow-dirty，把排序结果与该 allowlist 精确比较，任何额外路径都失败；P15 另用不带 allow-dirty 的 publish dry-run证明 clean package。
8. ci.yml 增加 workflow_call。release.yml 只监听 refs/tags/v0.5.0，checkout fetch-depth=0/persist-credentials=false，用 git cat-file -t 'v0.5.0^{tag}' 验证 annotated tag，quality job 复用 ci.yml，publish job needs quality 且 environment=crates-io。publish job 在没有 token 的 step 先生成 package并核对 tarball hash/tag commit；CARGO_REGISTRY_TOKEN 只放在最终 cargo publish --locked -p zai-rs --all-features --no-verify step 的 env，值来自 secrets.CRATES_IO_TOKEN，其他 step/job 不可读取。
9. xtask sbom generate 校验 Cargo.lock hash 与 cargo metadata --locked，在解压后的 root package临时目录再次对 Cargo.lock 运行 metadata --locked；SOURCE_DATE_EPOCH 固定为 git show -s --format=%ct HEAD 的十进制值并写入 evidence。随后运行 cargo cyclonedx --format json --spec-version 1.5 --all-features --target x86_64-unknown-linux-gnu，只把结果移到 target/sbom/zai-rs.cdx.json，并断言临时/主 lock hash 与工作树都未变。P14 预提交验证使用显式 --allow-dirty，P15 clean gate 不使用该 flag。上传 SBOM、metadata、coverage、rustdoc 和 package tarball，retention-days=30。
10. 新建每周一 fuzz workflow：运行 xtask fuzz smoke --seconds 60；该命令先执行 metadata --locked 并校验 fuzz/Cargo.lock hash，再在 target/fuzz-run 下运行四个 target。发现 crash 时上传临时 corpus/artifact，不改写 fuzz/corpus。另对 fuzz workspace 执行 fmt 和 cargo audit。
11. 新建每周外链 job，以 timeout 20s、重试 3 次运行 online lychee并上传报告；continue-on-error=true，不进入 release needs。
12. Dependabot 每周一更新 Cargo 和 GitHub Actions；同一生态更新归组，CI 门禁不变。

验收：

- PR workflow 覆盖所有固定 jobs，最小权限生效。
- audit、deny、gitleaks、future incompat 和 package gate 全绿。
- feature 矩阵能发现任一 feature 漏依赖。
- release workflow 在无 tag、本地执行和普通 PR 中不会发布。

验证：

~~~bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-features --all-targets -- -D warnings
cargo check --locked -p zai-rs --no-default-features --all-targets
cargo test --locked -p zai-rs --no-default-features --all-targets
cargo test --locked -p zai-rs --no-default-features --features toolkits --all-targets
cargo test --locked -p zai-rs --no-default-features --features rmcp-kits --all-targets
cargo test --locked -p zai-rs --no-default-features --features realtime --all-targets
cargo test --locked -p zai-rs --no-default-features --features tool-validation --all-targets
cargo test --locked -p zai-rs --all-features --all-targets
cargo check --locked --workspace --all-features --all-targets
cargo test --locked --workspace --all-features --all-targets
./scripts/bootstrap-node.sh
npm ci --prefix examples/web_chat --ignore-scripts
npm test --prefix examples/web_chat
cargo nextest run --locked -p zai-rs --no-default-features --no-run
cargo run --locked -p xtask -- test-budget
cargo +1.88.0 check --locked --workspace --all-features --all-targets
cargo audit --file Cargo.lock --deny warnings
cargo audit --file fuzz/Cargo.lock --deny warnings
cargo deny --locked check advisories bans licenses sources
cargo run --locked -p xtask -- future-incompat check
cargo run --locked -p xtask -- package check
gitleaks git --redact --log-opts='--all'
gitleaks dir --redact .
cargo publish --dry-run --locked -p zai-rs --all-features --allow-dirty
cargo run --locked -p xtask -- sbom generate --allow-dirty
~~~

### P15 — 生成 0.5.0 release candidate 并执行总验收

- 状态：未执行
- 优先级：P0
- 依赖：P14
- 提交标题：chore(release): prepare 0.5.0 release candidate

实施：

1. Cargo.toml version 固定为 0.5.0；Cargo.lock、CHANGELOG、README、docs 和示例依赖同步。按执行协议更新根 lock 后，再且仅一次运行 cargo +nightly-2026-07-10 metadata --manifest-path fuzz/Cargo.toml >/dev/null，确认 fuzz/Cargo.lock 只把 path package zai-rs 从 0.5.0-alpha.0 改成 0.5.0；随后恢复全部 --locked 门禁。
2. CHANGELOG 增加 0.5.0 日期、Breaking、Security、Correctness、Added、Changed、Removed、Migration 七节，并链接 docs/MIGRATING-0.5.md。
3. 新建 release/0.5.0.md，记录冻结契约 hash、MSRV、features、依赖节点、测试数、覆盖率、package 文件数和全部门禁结果。release/** 被 package 排除；tarball hash 只写入 target/release-evidence，避免 commit/VCS metadata 自引用。
4. xtask version check 不允许出现宣称 0.3/0.4 已在 crates.io 发布的文字；README 继续明确 0.5 是 release candidate。
5. 先用 --allow-dirty 的 package dry-run 和其余本地门禁生成测试数、覆盖率和 package 文件数，写入 release 文档；更新 EXECUTION_LEDGER 的 P00..P15 为 complete，然后创建 P15 提交。
6. 从 P15 commit 建立 detached 临时 worktree。执行前取得主 workspace 绝对路径，把 evidence 目录固定为主 workspace/target/release-evidence/{P15-commit}/；第 9 节的日志、SBOM、package tarball 直接写入该目录并记录 SHA-256，不写临时 worktree 的 target/release-evidence。
7. 门禁失败时回主 worktree 修复并 amend P15 commit，删除旧临时 worktree，再从新 commit 重跑全部门禁；直到一次完整运行全部退出 0。
8. cargo package 后解压到临时目录，在不访问仓库源码的条件下执行 cargo check、cargo test --doc 和 consumer smoke crate。
9. consumer smoke 固定覆盖 default、toolkits、rmcp-kits、realtime、tool-validation 五种依赖方式。
10. 执行 git diff --check、git status、cargo metadata，确认没有 generated drift、ignored test、未跟踪 fixture，且 Git 跟踪的 lockfile 只有 Cargo.lock 与 fuzz/Cargo.lock。
11. 删除临时 worktree，停在本地 release candidate。不得 tag、push、publish 或创建远端 release。

验收：

- 第 10 节 Definition of Done 全部满足。
- clean package 能被五个 consumer mode 编译。
- release evidence 含所有命令退出码 0。
- 工作树 clean，当前分支未产生远端副作用。

验证：

~~~bash
cargo run --locked -p xtask -- release verify 0.5.0
cargo publish --dry-run --locked -p zai-rs --all-features
git diff --check
test -z "$(git status --porcelain)"
~~~

## 9. 全量质量门

P15 必须在 clean checkout 中依次执行以下命令；任何命令失败都回到对应任务修复。

~~~bash
cargo fmt --all -- --check
cargo +1.88.0 fmt --manifest-path fuzz/Cargo.toml -- --check
cargo clippy --locked --workspace --all-features --all-targets -- -D warnings
cargo check --locked -p zai-rs --no-default-features --all-targets
cargo check --locked -p zai-rs --no-default-features --features toolkits --all-targets
cargo check --locked -p zai-rs --no-default-features --features rmcp-kits --all-targets
cargo check --locked -p zai-rs --no-default-features --features realtime --all-targets
cargo check --locked -p zai-rs --no-default-features --features tool-validation --all-targets
cargo check --locked --workspace --all-features --all-targets
cargo +1.88.0 check --locked --workspace --all-features --all-targets
cargo test --locked -p zai-rs --no-default-features --all-targets
cargo test --locked -p zai-rs --no-default-features --features toolkits --all-targets
cargo test --locked -p zai-rs --no-default-features --features rmcp-kits --all-targets
cargo test --locked -p zai-rs --no-default-features --features realtime --all-targets
cargo test --locked -p zai-rs --no-default-features --features tool-validation --all-targets
cargo test --locked -p zai-rs --all-features --all-targets
cargo test --locked --workspace --all-features --all-targets
./scripts/bootstrap-node.sh
npm ci --prefix examples/web_chat --ignore-scripts
npm test --prefix examples/web_chat
cargo nextest run --locked -p zai-rs --no-default-features --no-run
cargo run --locked -p xtask -- test-budget
cargo run --locked -p xtask -- tests check-no-ignore
cargo test --locked -p docs-snippets
cargo build --locked -p docs-snippets --all-targets
cargo test --locked -p zai-rs --all-features --doc
RUSTDOCFLAGS='--cfg docsrs -D warnings' cargo +nightly-2026-07-10 doc --locked -p zai-rs --all-features --no-deps
cargo build --locked -p zai-rs --all-features
mdbook test -L target/debug/deps .
cargo run --locked -p xtask -- contract verify --require-covered
cargo run --locked -p xtask -- public-api check
cargo run --locked -p xtask -- docs check
cargo run --locked -p xtask -- examples check
cargo run --locked -p xtask -- module-size check
cargo run --locked -p xtask -- dep-budget check
cargo llvm-cov --locked -p zai-rs --all-features --ignore-filename-regex '(^|/)(tests|examples|generated)/' --json --output-path target/coverage.json
cargo run --locked -p xtask -- coverage check target/coverage.json
cargo run --locked -p xtask -- fuzz smoke --seconds 60
cargo audit --file Cargo.lock --deny warnings
cargo audit --file fuzz/Cargo.lock --deny warnings
cargo deny --locked check advisories bans licenses sources
cargo run --locked -p xtask -- future-incompat check
gitleaks git --redact --log-opts='--all'
gitleaks dir --redact .
lychee --offline --config lychee.toml README.md docs src
cargo run --locked -p xtask -- package check
cargo run --locked -p xtask -- sbom generate
cargo publish --dry-run --locked -p zai-rs --all-features
cargo run --locked -p xtask -- release verify 0.5.0
test "$(git ls-files '*/Cargo.lock')" = "fuzz/Cargo.lock"
test -z "$(git status --porcelain)"
~~~

## 10. Definition of Done

以下条件必须同时成立：

1. 冻结 OpenAPI operation 覆盖 59/59；其中三个 Agent operation 同时通过手工参考约束，Realtime 与 Coding Plan 独立契约全部通过。
2. 所有 public 请求只经 ZaiClient → service → RequestSpec → Transport。
3. HTTP_CLIENTS、公开 raw HttpClient、每请求 key/config、字符串 URL 拼接全部消失。
4. 错误 Agent CRUD 和 async chat stream 全部消失；Agent v1、ASR、TTS、Knowledge 与冻结契约一致。
5. NonIdempotent 请求从不自动重试；Idempotent retry、Retry-After 和总 deadline 有真实传输测试。
6. secret、body、query value、用户提供的 path/resource ID、工具参数和文件内容不会出现在 Debug、Display、error 或 tracing；只允许第 4 节定义的服务端 correlation request_id。
7. JSON、error、SSE、file、WS、tool input/output 全部执行固定上限。
8. multipart 与下载为有界流式实现，取消和失败不留 partial。
9. Tool effect、cache、retry、TTL、并发和 RMCP deadline 满足固定矩阵。
10. Realtime 不饿死接收、错误不丢失、所有消费者能观察终态。
11. default feature 为空，单 feature 与 all-features 独立编译，依赖图满足预算。
12. line >=75%、region >=70%、function >=65%，五个关键模块 line >=90%。
13. test、doctest、mdBook、docs.rs、examples、MSRV、Clippy、audit、deny、gitleaks 和 package 全绿。
14. ignored doctest=0，伪集成测试=0，公网测试=0，真实媒体 fixture=0；嵌套 Cargo.lock 只允许被提交的 fuzz/Cargo.lock。
15. 0.5.0 release candidate 文档、迁移指南、SBOM、package 和证据完整；没有 tag、push 或 publish。

## 11. 禁止残留模式

xtask release verify 必须执行下面的语义检查，并把任何匹配视为失败：

~~~text
src 中出现 HTTP_CLIENTS
src 中出现公开 HttpClient trait
资源请求类型出现 api_key: String、pub key: String、EndpointConfig 或 HttpClientConfig 字段
生产代码记录 request/response body 或 Authorization header value
src/agent 中出现 create_agent、update_agent、delete_agent 或 /paas/v4/agents
AsyncChat 类型实现 SseStreamable 或序列化 stream
模型 ID 首尾存在空白
文档存在 rust,ignore、ZAI_API_KEY 或 raw post/get 示例
examples 中打印 key/token/secret
仓库存在 data 目录媒体、过期签名 URL或 examples 下的 Cargo.lock；fuzz/Cargo.lock 是唯一允许的嵌套 lock
手写生产 Rust 文件超过 600 行
contract coverage 出现 missing
测试使用真实 open.bigmodel.cn 网络
~~~

## 12. 任务清单

| ID | 交付物 | 依赖 | 完成 |
|---|---|---|---|
| P00 | 冻结契约、xtask、baseline、ledger | 无 | [ ] |
| P01 | 已知正确性、secret、媒体、audit hotfix | P00 | [ ] |
| P02 | ZaiClient、SecretString、validated URL | P01 | [ ] |
| P03 | Transport、error、retry、timeout、limits、logging | P02 | [ ] |
| P04 | Agent、async、ASR、TTS、Knowledge 契约 | P03 | [ ] |
| P05 | 现有 API 全量 RequestSpec 迁移 | P04 | [ ] |
| P06 | 59/59 operation coverage | P05 | [ ] |
| P07 | streaming multipart/download/polling | P06 | [ ] |
| P08 | SSE 与 Realtime 终态 | P07 | [ ] |
| P09 | Tool effect/cache/concurrency/RMCP | P08 | [ ] |
| P10 | public API、module、feature、dependency 收敛 | P09 | [ ] |
| P11 | contract/property/trybuild/fuzz/coverage | P10 | [ ] |
| P12 | workspace examples 与 web_chat | P11 | [ ] |
| P13 | mdBook、Rustdoc、README、migration | P12 | [ ] |
| P14 | CI、supply chain、release workflow | P13 | [ ] |
| P15 | 0.5.0 release candidate 与总验收 | P14 | [ ] |

## 13. manual-constraints.toml 固定内容

P00 把本节逐项编码为 TOML。来源冲突时，本节对列出的字段优先；未列字段继续使用冻结 OpenAPI/AsyncAPI。执行者不得从更新后的网页改变这些值。

### 13.1 通用 wire 与 public type 规则

1. OpenAPI 的 required、enum、minimum、maximum、minLength、maxLength、minItems、maxItems、format、success status 和 MIME 原样进入 private wire model。
2. Public request/response 名由第 14 节固定。字段名使用 OpenAPI property 的 snake_case；request 字段私有，Type::builder() 的 setter 同名，build() -> ZaiResult&lt;Type&gt; 校验所有 required 与跨字段规则。
3. 只有 stream/nonstream 与互斥输入使用 public type-state。其余 required 字段在 build 时校验，不为每个字段创建泛型状态。
4. Public response 字段私有；required 字段 accessor 返回值或引用，optional 字段 accessor 返回 Option。response enum 未知字符串保存 Other(String)，request enum 不接受未知字符串。
5. serde_json::Value/Map 只允许对应 OpenAPI additionalProperties=true、空 schema、JSON Schema 本体或 custom_variables 的字段；其他已知对象必须 typed。
6. Success invariant 为：全部 OpenAPI required 字段存在，并且至少一个文档化响应字段非 null。第 13.2 至 13.6 的更严格 invariant 覆盖本规则。
7. 所有内置 POST/PATCH 的 retry_safety=NonIdempotent；GET/HEAD/OPTIONS/PUT/DELETE 为 Idempotent。不存在内置 Idempotent POST。
8. JSON 的 success MIME 为 application/json 或 +json；SSE 为 text/event-stream；binary 按 operation manifest 的明确 MIME。
9. 59 个 operation 的 operation_id 固定使用第 14 节 public call，例如 knowledge.list_documents；不使用空的上游 operationId 现场造名。
10. Public nested type 命名固定为：$ref 直接使用 component schema name；inline object 使用 ParentType + property PascalCase；array item 再追加 Item；跨 service 同名时前置 service PascalCase。xtask generator 对相同 Rust name 但不同 schema hash 直接失败，不自动加数字后缀。

### 13.2 Agent v1

| 项目 | 固定约束 |
|---|---|
| invoke | POST /v1/agents；AgentInvokeRequest&lt;S&gt;；agent_id 非空；messages 至少 1；role 仅 system/user/assistant；content 依官方 string/object/array union；custom_variables 为开放 Map |
| nonstream success | AgentInvokeResponse::Completed 需要 id、agent_id、choices 非空；Pending 需要 agent_id、async_id |
| stream | AgentInvokeRequest&lt;Streaming&gt; 序列化 stream=true；MIME text/event-stream；data: [DONE] 为唯一正常 EOF marker |
| async result request | async_id、agent_id 均非空 |
| async result | status pending/success/failed；Pending 需要 agent_id/async_id；Succeeded 还需要 choices 非空；Failed 是 task result，不转成 transport error |
| conversation | agent_id、conversation_id 必需；success 需要两个 ID 与非空 choices；embedded error 转 ZaiError |

### 13.3 ASR

| 字段 | 固定约束 |
|---|---|
| model | 仅 glm-asr-2512 |
| input | file 与 file_base64 恰好一个；两者同时存在直接 Validation，不采用服务端的 file 优先行为 |
| file | wav/mp3；<=25 MiB；音频 30 秒由服务端检查 |
| prompt | optional；文档的 8000 字为建议，客户端不增加硬拒绝 |
| hotwords | <=100 |
| request_id | 6..=64 chars |
| user_id | 6..=128 chars |
| nonstream | stream=false；application/json；SpeechToTextResponse 需要 id、model、text |
| stream | stream=true；text/event-stream；SpeechToTextEvent；必须 data: [DONE] |

### 13.4 TTS

| 字段 | 固定约束 |
|---|---|
| model | 仅 glm-tts |
| input | required；1..=1024 Unicode scalar |
| voice | required VoiceId；内置常量 tongtong、chuichui、xiaochen、jam、kazi、douji、luodo；其他非空字符串作为复刻音色 ID |
| speed | 0.5..=2，默认 1 |
| volume | 0 < value <=10，默认 1 |
| response_format | wav/pcm，默认 pcm；stream=true 时只允许 pcm |
| encode_format | base64/hex；只允许 stream=true |
| watermark_enabled | optional bool |
| nonstream | audio/wav、audio/x-wav、audio/pcm 或 application/octet-stream；返回 Bytes |
| stream | text/event-stream；base64/hex 解码为 Bytes；必须 data: [DONE] |

### 13.5 Knowledge 与业务 envelope

| 项目 | 固定约束 |
|---|---|
| create required | embedding_id、name |
| embedding | 3=Embedding-2、11=Embedding-3、12=Embedding-3-pro；embedding_id 与 embedding_model 同时给出时必须匹配 |
| contextual | 0/1 |
| envelope | code 必须等于 200；code 缺失、非 200、data 缺失均为 Err |
| detail name | GET /knowledge/{id} = KnowledgeGetRequest/client.knowledge().get |
| search name | POST /knowledge/retrieve = KnowledgeSearchRequest/client.knowledge().retrieve |

### 13.6 Streaming、Realtime 与 Coding Plan

| 项目 | 固定约束 |
|---|---|
| SSE data | 多行用 \n 连接；comment 忽略；event/id/retry 只存 private metadata，不自动重连 |
| done marker | chat、Agent v1、ASR、TTS 使用 data: [DONE]；Zrag Agent 使用 event payload type=done |
| stream error | 恰好产出一个 Err item，随后终止 |
| Zrag | POST /zrag/agent/chat 固定 stream-only；AgentStreamEvent type 为 session_created/reasoning/thought/tool_call/tool_result/answer/done/error |
| Realtime | 不主动 heartbeat；收到 Ping 在 10s 内 Pong；command/event/audio capacity=8；Lagged 后该 consumer 收一个 Err 并终止 |
| Coding Plan scope | 本 release 只公开 GET /monitor/usage/quota/limit；官方插件中的 model-usage 与 tool-usage 不属于本 release surface |

### 13.7 API error 分类

| 分类 | 固定 code/status |
|---|---|
| auth | HTTP 401/403；业务 code 1000、1001、1003、1005、1220 |
| rate-limit helper | HTTP 429；业务 code 1113、1302、1305、1308、1309、1310、1311、1313、1314、1315、1316、1317、1318、1319、1320、1321 |
| retryable rate | 只有无业务 code 的 HTTP 429，以及业务 code 1302、1305 |
| non-retryable quota/billing | 1113、1308、1309、1310、1311、1313、1314、1315、1316、1317、1318、1319、1320、1321 |
| server | HTTP 500..=599；业务 code 1200、1230、1234；仍排除 501/505 |
| validation/content | 1210、1211、1212、1213、1214、1215、1221、1222、1261、1301；不重试 |

HTTP status 与业务 code 冲突时，non-retryable quota/billing 和 validation/content 优先于 status retry 集合；随后才应用 effective RetrySafety 与 max_attempts。
分类器把 ApiCode::Number(1001) 与 ApiCode::Text("1001") 视为同一业务 code；保留 variant 只用于无损 roundtrip。

## 14. 59 个 OpenAPI operation 的固定 Rust 映射

表中的 public call、request 和 response 是 P00 manifest 的最终值。所有 call 返回 ZaiResult&lt;Response&gt;；标为 stream 的 call 返回 ZaiResult&lt;StreamType&gt;。同一 HTTP operation 出现两个 call 时，共享一个 RequestSpec，并由 type-state 固定 wire 的 stream 字段。

| # | Method/path | Public call | Request → Response |
|---:|---|---|---|
| 1 | GET /llm-application/open/document | knowledge.list_documents | DocumentListRequest → DocumentListResponse |
| 2 | POST /llm-application/open/document/embedding/{id} | knowledge.reembed_document | DocumentReembedRequest → DocumentReembedResponse |
| 3 | POST /llm-application/open/document/slice/image_list/{id} | knowledge.list_document_images | DocumentImageListRequest → DocumentImageListResponse |
| 4 | POST /llm-application/open/document/upload_document/{id} | knowledge.upload_document | DocumentUploadRequest → DocumentUploadResponse |
| 5 | POST /llm-application/open/document/upload_url | knowledge.upload_document_url | DocumentUrlUploadRequest → DocumentUrlUploadResponse |
| 6 | DELETE /llm-application/open/document/{id} | knowledge.delete_document | DocumentDeleteRequest → DocumentDeleteResponse |
| 7 | GET /llm-application/open/document/{id} | knowledge.get_document | DocumentGetRequest → DocumentGetResponse |
| 8 | GET /llm-application/open/history_session_record/{app_id}/{conversation_id} | applications.history | ApplicationHistoryRequest → ApplicationHistoryResponse |
| 9 | GET /llm-application/open/knowledge | knowledge.list | KnowledgeListRequest → KnowledgeListResponse |
| 10 | POST /llm-application/open/knowledge | knowledge.create | KnowledgeCreateRequest → KnowledgeCreateResponse |
| 11 | GET /llm-application/open/knowledge/capacity | knowledge.capacity | KnowledgeCapacityRequest → KnowledgeCapacityResponse |
| 12 | POST /llm-application/open/knowledge/retrieve | knowledge.retrieve | KnowledgeSearchRequest → KnowledgeSearchResponse |
| 13 | DELETE /llm-application/open/knowledge/{id} | knowledge.delete | KnowledgeDeleteRequest → KnowledgeDeleteResponse |
| 14 | GET /llm-application/open/knowledge/{id} | knowledge.get | KnowledgeGetRequest → KnowledgeGetResponse |
| 15 | PUT /llm-application/open/knowledge/{id} | knowledge.update | KnowledgeUpdateRequest → KnowledgeUpdateResponse |
| 16 | POST /llm-application/open/v2/application/file_stat | applications.file_stats | ApplicationFileStatsRequest → ApplicationFileStatsResponse |
| 17 | POST /llm-application/open/v2/application/file_upload | applications.upload_file | ApplicationFileUploadRequest → ApplicationFileUploadResponse |
| 18 | POST /llm-application/open/v2/application/slice_info | applications.slice_info | ApplicationSliceInfoRequest → ApplicationSliceInfoResponse |
| 19 | POST /llm-application/open/v2/application/{app_id}/conversation | applications.create_conversation | ApplicationConversationCreateRequest → ApplicationConversationCreateResponse |
| 20 | GET /llm-application/open/v2/application/{app_id}/variables | applications.variables | ApplicationVariablesRequest → ApplicationVariablesResponse |
| 21 | POST /llm-application/open/v3/application/invoke | applications.invoke | ApplicationInvokeRequest → ApplicationInvokeResponse |
| 22 | POST /paas/v4/assistant | assistants.invoke | AssistantInvokeRequest → AssistantInvokeResponse |
| 23 | POST /paas/v4/assistant/conversation/list | assistants.conversations | AssistantConversationListRequest → AssistantConversationListResponse |
| 24 | POST /paas/v4/assistant/list | assistants.list | AssistantListRequest → AssistantListResponse |
| 25 | GET /paas/v4/async-result/{id} | tasks.get | AsyncTaskGetRequest → AsyncTaskResponse |
| 26 | POST /paas/v4/async/chat/completions | chat.complete_async | AsyncChatRequest → AsyncTaskResponse |
| 27 | POST /paas/v4/async/images/generations | images.generate_async | AsyncImageGenerationRequest → AsyncTaskResponse |
| 28 | POST /paas/v4/audio/speech | audio.synthesize / audio.synthesize_stream | TextToSpeechRequest&lt;NonStreaming&gt; → Bytes / TextToSpeechRequest&lt;Streaming&gt; → AudioByteStream |
| 29 | POST /paas/v4/audio/transcriptions | audio.transcribe / audio.transcribe_stream | SpeechToTextRequest&lt;NonStreaming&gt; → SpeechToTextResponse / SpeechToTextRequest&lt;Streaming&gt; → SpeechToTextStream |
| 30 | GET /paas/v4/batches | batches.list | BatchListRequest → BatchListResponse |
| 31 | POST /paas/v4/batches | batches.create | BatchCreateRequest → BatchCreateResponse |
| 32 | GET /paas/v4/batches/{batch_id} | batches.get | BatchGetRequest → BatchGetResponse |
| 33 | POST /paas/v4/batches/{batch_id}/cancel | batches.cancel | BatchCancelRequest → BatchCancelResponse |
| 34 | POST /paas/v4/chat/completions | chat.complete / chat.stream | ChatRequest&lt;NonStreaming&gt; → ChatResponse / ChatRequest&lt;Streaming&gt; → ChatEventStream |
| 35 | POST /paas/v4/embeddings | embeddings.create | EmbeddingRequest → EmbeddingResponse |
| 36 | GET /paas/v4/files | files.list | FileListRequest → FileListResponse |
| 37 | POST /paas/v4/files | files.upload | FileUploadRequest → FileUploadResponse |
| 38 | POST /paas/v4/files/ocr | files.ocr | OcrRequest → OcrResponse |
| 39 | POST /paas/v4/files/parser/create | files.parse | FileParseRequest → AsyncTaskResponse |
| 40 | GET /paas/v4/files/parser/result/{taskId}/{format_type} | files.parse_result | FileParseResultRequest → FileParseResultResponse |
| 41 | POST /paas/v4/files/parser/sync | files.parse_sync | FileParseSyncRequest → FileParseResponse |
| 42 | DELETE /paas/v4/files/{file_id} | files.delete | FileDeleteRequest → FileDeleteResponse |
| 43 | GET /paas/v4/files/{file_id}/content | files.content | FileContentRequest → ByteStream |
| 44 | POST /paas/v4/images/generations | images.generate | ImageGenerationRequest → ImageGenerationResponse |
| 45 | POST /paas/v4/layout_parsing | tools.parse_layout | LayoutParsingRequest → LayoutParsingResponse |
| 46 | POST /paas/v4/moderations | moderation.check | ModerationRequest → ModerationResponse |
| 47 | POST /paas/v4/reader | tools.read_document | ReaderRequest → ReaderResponse |
| 48 | POST /paas/v4/rerank | rerank.create | RerankRequest → RerankResponse |
| 49 | POST /paas/v4/tokenizer | tokenizer.count | TokenizerRequest → TokenizerResponse |
| 50 | POST /paas/v4/videos/generations | videos.generate | VideoGenerationRequest → AsyncTaskResponse |
| 51 | POST /paas/v4/voice/clone | audio.clone_voice | VoiceCloneRequest → VoiceCloneResponse |
| 52 | POST /paas/v4/voice/delete | audio.delete_voice | VoiceDeleteRequest → VoiceDeleteResponse |
| 53 | GET /paas/v4/voice/list | audio.list_voices | VoiceListRequest → VoiceListResponse |
| 54 | POST /paas/v4/web_search | tools.web_search | WebSearchRequest → WebSearchResponse |
| 55 | POST /v1/agents | agents.invoke / agents.stream | AgentInvokeRequest&lt;NonStreaming&gt; → AgentInvokeResponse / AgentInvokeRequest&lt;Streaming&gt; → AgentEventStream |
| 56 | POST /v1/agents/async-result | agents.async_result | AgentAsyncResultRequest → AgentAsyncResult |
| 57 | POST /v1/agents/conversation | agents.conversation | AgentConversationRequest → AgentConversationResponse |
| 58 | POST /zrag/agent/chat | zrag.chat | ZragChatRequest → ZragEventStream |
| 59 | POST /zrag/retrieval/retrieve | zrag.retrieve | ZragRetrieveRequest → ZragRetrieveResponse |

额外非 OpenAPI 映射固定为 client.usage().coding_plan()，请求 GET /monitor/usage/quota/limit，类型 CodingPlanUsageRequest → CodingPlanUsageResponse。Realtime 入口固定为 client.realtime().session(RealtimeSessionConfig) → RealtimeSession。
