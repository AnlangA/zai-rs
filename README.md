# zai-rs

一个简洁、类型安全的 Zhipu AI Rust SDK。专注提升 Rust 开发者的接入效率：更少样板代码、更一致的错误处理、可读的请求/响应类型，以及开箱即用的示例。

## 快速开始
1. 准备环境
   - Rust 1.74+（或更高）
   - 设置环境变量：`ZHIPU_API_KEY="<your_api_key>"`
2. 构建
   - `cargo build`
3. 运行示例（examples/ 目录内）
   - `cargo run --example chat_loop`

## 示例（examples/）

### 可用示例

| 示例 | 描述 |
|------|------|
| `chat_text` | 基础文本对话 |
| `chat_stream` | 流式响应 |
| `chat_loop` | 多轮对话循环 |
| `glm45_thinking_mode` | GLM-4.5 深度思考模式 |
| `ocr` | OCR 手写文字识别 |
| `gen_image` | 图像生成 |
| `gen_video` | 视频生成 |
| `text_to_audio` | 文本转语音 |
| `audio_to_text` | 语音转文字 |
| `voice_clone` | 音色复刻 |
| `function_call` | 函数调用 |
| `embedding` | 文本嵌入 |
| `files_upload` | 文件上传 |
| `knowledge_create` | 知识库创建 |
| `web_search` | 网络搜索 |
| `translation_bot` | 翻译机器人 |

### 运行方式

```bash
# Windows PowerShell
$Env:ZHIPU_API_KEY = "<your_api_key>"
cargo run --example chat_loop

# macOS/Linux
export ZHIPU_API_KEY="<your_api_key>"
cargo run --example chat_loop
```

## API 覆盖度

### 模型 API
- [x] POST 对话补全（同步/异步/流式）
- [x] GLM-4.5/GLM-4.6/GLM-4.7 支持
- [x] 思考模式（Thinking Mode）
- [x] 图像生成
- [x] 视频生成（异步）
- [x] 语音转文本
- [x] 文本转语音
- [x] 音色复刻/列表/删除
- [x] 文本嵌入/重排序/分词
- [x] OCR 手写识别

### 工具 API
- [x] POST 网络搜索
- [x] POST 内容安全
- [x] POST 文件解析
- [x] GET 解析结果

### Agent API ✨ 新增
- [x] POST 创建智能体
- [x] GET 查询智能体
- [x] PUT 更新智能体
- [x] DELETE 删除智能体
- [x] POST 智能体对话
- [x] GET 对话历史

### 文件 API
- [x] GET 文件列表
- [x] POST 上传文件
- [x] DELETE 删除文件
- [x] GET 文件内容

### 批处理 API
- [x] GET 列出批处理任务
- [x] POST 创建批处理任务
- [x] GET 检索批处理任务
- [x] POST 取消批处理任务

### 知识库 API
- [x] GET 知识库列表
- [x] POST 创建知识库
- [x] GET 知识库详情
- [x] PUT 编辑知识库
- [x] DELETE 删除知识库
- [x] GET 知识库使用量
- [x] GET 文档列表
- [x] POST 上传文件文档
- [x] POST 上传 URL 文档
- [x] GET 文档详情
- [x] DELETE 删除文档
- [x] POST 重新向量化

### 实时 API 🚧 框架就绪
- [x] WebSocket 类型定义
- [x] 会话管理框架
- [ ] 音视频通话实现（待完善）
