# 架构说明

`zai-rs` 是面向 Zhipu AI / BigModel HTTP API 的类型安全 Rust SDK。项目的核心目标是把「请求构造、端点选择、鉴权、传输、响应解析、错误分类」这些重复细节集中起来，让每个业务 crate 模块只表达自己的 API 语义。

## 设计目标

- 类型安全：通过模型 marker trait、`Bounded` 约束和 streaming type-state，尽量在编译期阻止不兼容的模型/消息组合。
- 统一传输：所有 REST 请求都走 `client::http` 的共享 retry、连接池、敏感信息 masking 和 typed response 解析。
- 可配置端点：`EndpointConfig` 集中管理 PAAS v4、Coding PAAS v4、知识库、Realtime、Monitor 五类 API base，`ZaiConfig` 作为面向用户的中心配置入口。
- 小模块边界：chat、file、knowledge、batches、tool、realtime、usage 等模块独立维护请求/响应类型，再通过 crate root 和模块 root 做选择性 re-export。
- 离线可测：集成测试使用本地 mock server 捕获真实 SDK HTTP 请求，不依赖外部 API key 或网络。

## API 家族

官方文档把通用 HTTP API 端点定义为 `https://open.bigmodel.cn/api/paas/v4`，Coding Plan 需要使用专属 `https://open.bigmodel.cn/api/coding/paas/v4`，请求使用 `Authorization: Bearer YOUR_API_KEY`。SDK 中对应为：

| API 家族 | `ApiBase` | 默认 base | 代表模块 |
|----------|-----------|-----------|----------|
| 通用 PAAS v4 | `PaasV4` | `https://open.bigmodel.cn/api/paas/v4` | `model`, `file`, `batches`, `tool` |
| Coding PAAS v4 | `CodingPaasV4` | `https://open.bigmodel.cn/api/coding/paas/v4` | `model::chat::ChatCompletion::with_coding_plan` |
| 知识库 / LLM application | `LlmApplication` | `https://open.bigmodel.cn/api/llm-application/open` | `knowledge`, `agent` |
| Realtime WebSocket | `Realtime` | `wss://open.bigmodel.cn/api/paas/v4/realtime` | `realtime` |
| Monitor / usage | `Monitor` | `https://open.bigmodel.cn/api/monitor` | `usage` |

需要代理或国际站时，优先覆盖 `EndpointConfig` 或 `ZaiConfig::builder()` 中对应的 base，而不是在业务模块里手写完整 URL。

## 分层

```text
src/lib.rs
  client/        config, endpoint registry, transport, errors
  model/         chat / async chat / multimodal / embeddings / moderation / SSE
  file/          upload, list, content, delete
  knowledge/     knowledge-base CRUD, document upload/list/retrieve/re-embed
  batches/       batch job create/list/retrieve/cancel
  tool/          web search and file parser API wrappers
  toolkits/      local dynamic tool execution and optional RMCP bridge
  realtime/      WebSocket protocol, auth, transport, session
  usage/         Coding Plan quota / remaining usage query
```

请求模块通常遵循同一形态：

1. `new(...)` 构造默认 endpoint、body、HTTP config。
2. `with_*` builder 方法覆盖参数、base URL、endpoint config 或 HTTP config。
3. 实现 `HttpClient`，由 `post/get/put/delete` 统一发送。
4. `send()` 只负责校验、调用传输层、解析 typed response。

## 错误与重试

官方错误码由 HTTP 状态码和响应体内业务错误码两层组成，错误 envelope 形如 `{"error":{"code":"1002","message":"..."}}`。SDK 在 `ZaiError::from_api_response` 中做业务分类：

- `1000-1004, 1100` -> `AuthError`
- `1110-1121` -> `AccountError`
- `1200-1234` -> `ApiError`
- `1300-1301` -> `ContentPolicyError`
- `1302-1305, 1308-1313` -> `RateLimitError`
- `1400-1499` -> `FileError`

自动 retry 只处理可恢复错误：HTTP 429/5xx、`RateLimitError` 和网络错误。认证、账户、参数、内容策略错误不会自动重试。

## 测试策略

- 单元测试覆盖 URL join/query、错误分类、retry delay、模型/工具序列化和响应转换。
- 集成测试用 `hyper` 本地 mock server 捕获 method/path/query/header/body，验证 SDK 构造出的真实 HTTP 请求。
- 新增 API family 或 endpoint 时，至少补一个 URL 构造测试；涉及传输行为时，优先补 mock-server 集成测试。
- 不需要真实 `ZHIPU_API_KEY` 的测试应保持默认测试路径，真实 API 示例放在 `examples/`。

## 文档策略

- crate-level Rustdoc 解释整体能力和快速开始。
- 模块级 Rustdoc 解释模块边界、常用类型和设计约束。
- `docs/` 放面向使用者的长文档：入门、错误处理、最佳实践、架构说明和 FAQ。
- 示例代码若需要 API key，应使用 `ZHIPU_API_KEY` 环境变量，避免把密钥写入源码或文档。

## 参考资料

- 智谱 AI 使用概述：https://docs.bigmodel.cn/cn/api/introduction
- 智谱 AI HTTP API 调用指南：https://docs.bigmodel.cn/cn/guide/develop/http/introduction
- 智谱 AI 错误码：https://docs.bigmodel.cn/cn/api/api-code
- Z.AI Errors 文档：https://docs.z.ai/api-reference/api-code
- Rust API client 设计经验：https://nullderef.com/blog/web-api-client/
