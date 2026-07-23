# 能力选择指引

同一能力在 SDK 中有时存在两个入口。这不是冗余实现，而是上游 BigModel
平台不同时期 API 的分别建模：SDK 对每个上游端点做类型安全封装，保留各自
的语义差异。本文对照三组常见重叠能力，给出选型建议。

## 图像生成：同步 vs 异步

| | `model::gen_image` | `services::images` |
|---|---|---|
| 端点 | `POST /paas/v4/images/generations` | `POST /paas/v4/async/images/generations` |
| 语义 | 同步返回生成结果 | 提交异步任务，凭任务 ID 轮询结果 |
| 模型 | `GlmImage`、`CogView4_250304`、`CogView4`、`CogView3Flash` | GLM-Image |
| 示例 | `examples/gen_image.rs` | — |

建议：

- 默认使用 `model::gen_image`：一次请求拿到图片，代码最短。
- 生成耗时长、需要批量提交后统一收结果，或对请求超时敏感时，使用
  `services::images` 的异步任务接口。

## OCR：手写识别 vs 文档版式解析

| | `model::ocr` | `services::tools`（layout_parsing） |
|---|---|---|
| 端点 | `POST /paas/v4/files/ocr` | `POST /paas/v4/layout_parsing` |
| 定位 | 图片中文字 / 手写体识别 | 文档版式结构化解析（版面元素、阅读顺序等） |
| 模型 | 专用 OCR 模型 | `glm-ocr` |
| 示例 | `examples/ocr.rs` | — |
| 文档 | [OCR 指南](OCR_GUIDE.md) | — |

建议：

- 识别图片中的文字（尤其是手写内容）使用 `model::ocr`。
- 需要把整页文档解析成带版面结构的元素序列时使用
  `services::tools::LayoutParsingRequest`。

## 文件解析：异步任务 vs 同步解析

| | `tool::file_parser_create` + `tool::file_parser_result` | `file::parse_sync` |
|---|---|---|
| 端点 | `POST /paas/v4/files/parser/create` + `GET /paas/v4/files/parser/result/...` | `POST /paas/v4/files/parser/sync` |
| 语义 | 创建异步解析任务，之后轮询结果 | multipart 上传后同步返回解析结果 |
| 适用 | 大文件、批量任务、长耗时解析 | 小文件、简单集成 |
| 示例 | `examples/file_parser_demo.rs` | — |

建议：

- 文件较小、希望一次请求拿到结果：使用 `file::parse_sync`。
- 文件较大或需要批量提交：使用 `tool::file_parser_create` 创建任务，再用
  `tool::file_parser_result` 轮询（`examples/file_parser_demo.rs` 演示了完整
  提交 + 轮询流程）。

## 通用原则

- 两组入口都走同一个 `ZaiClient` 与统一传输管线（鉴权、重试、错误分类完全
  一致），切换入口不改变工程质量特性。
- 不确定时默认选同步入口；遇到超时或批量需求再迁移到异步任务入口。
