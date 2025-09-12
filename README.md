# zai-rs

一个简洁、类型安全的 Zhipu AI Rust SDK。专注提升 Rust 开发者的接入效率：更少样板代码、更一致的错误处理、可读的请求/响应类型，以及开箱即用的示例。

## 项目作用（What is it for）
- 以 Rust 的方式封装 Zhipu AI 开放平台能力（模型、工具、文件、批处理、知识库、实时等）
- 提供统一的 HTTP 客户端、鉴权与错误解析
- 使用 serde 进行序列化/反序列化，并结合 validator 做入参校验
- 保持模块扁平、注释规范、示例可直接运行

## 功能概览
- 模型 API：对话补全（含异步）、图像生成、ASR/TTS、文本嵌入/重排/分词等
- 工具 API：网络搜索、内容安全、文件解析与结果查询
- 文件 API：列表/上传/删除/获取内容
- 批处理 API：创建、查询、取消、列出
- 知识库 API：
  - 知识库：列表/创建/详情/编辑/删除/使用量
  - 文档：列表、上传文件、上传 URL、详情、删除、解析图片列表、重新向量化
- 实时 API：音视频通话（进行中）

## 快速开始
1. 准备环境
   - Rust 1.74+（或更高）
   - 设置环境变量：`ZHIPU_API_KEY="<your_api_key>"`
2. 构建
   - `cargo build`
3. 运行示例（examples/ 目录内）
   - `cargo run --example chat_loop`

## 示例（examples/）
仓库中包含可直接运行的示例，覆盖常用场景：
- 对话示例
  - `chat_loop`：可持续对话（输入 exit 或 quit 退出）

- 知识库管理
  - `knowledge_update`：编辑知识库（示例已简化为仅修改描述）
  - `knowledge_delete`：删除知识库
  - `knowledge_capacity`：查询使用量
- 文档管理
  - `knowledge_document_list`：文档列表
  - `knowledge_document_detail`：文档详情
  - `knowledge_document_delete`：删除文档
  - `knowledge_document_upload_file`：上传文件文档（支持切片参数、回调）
  - `knowledge_document_upload_url`：上传 URL 文档
  - `knowledge_document_image_list`：解析出的图片序号与链接
  - `knowledge_document_reembedding`：重新向量化（支持回调）

运行方式（示例）：
```bash
# Windows PowerShell
$Env:ZHIPU_API_KEY = "<your_api_key>"
cargo run --example chat_loop

# macOS/Linux
export ZHIPU_API_KEY="<your_api_key>"
cargo run --example chat_loop
```

## TODO

- 模型API
    - [x] POST 对话补全
    - [x] POST 对话补全(异步)
    - [x] POST 生成视频(异步)
    - [x] 查询异步结果 GET
    - [x] POST 图像生成
    - [x] POST 语音转文本
    - [x] POST 文本转语音
    - [x] POST 音色复刻
    - [x] GET 音色列表
    - [x] POST 删除音色
    - [x] POST 文本嵌入
    - [x] POST 文本重排序
    - [x] POST 文本分词器
- 工具 API
    - [x] POST 网络搜索
    - [x] POST 内容安全
    - [x] POST 文件解析
    - [x] GET 解析结果
- Agent API
    - [ ] POST 智能体对话
    - [ ] POST 异步结果
    - [ ] POST 对话历史
- 文件 API
    - [x] GET 文件列表
    - [x] POST 上传文件
    - [x] DEL 删除文件
    - [x] GET 文件内容

- 批处理API
    - [x] GET 列出批处理任务
    - [x] POST 创建批处理任务
    - [x] GET 检索批处理任务
    - [x] POST 取消批处理任务
- 知识库API
    - [x] GET 知识库列表
    - [x] POST 创建知识库
    - [x] GET 知识库详情
    - [x] PUT 编辑知识库
    - [x] DEL 删除知识库
    - [x] GET 知识库使用量
    - [x] GET 文档列表
    - [x] POST 上传文件文档
    - [x] POST 上传URL文档
    - [x] POST 解析文档图片
    - [x] GET 文档详情
    - [x] DEL 删除文档
    - [x] POST 重新向量化
- 实时API
    - [ ] 音视频通话