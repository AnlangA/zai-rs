# zai-rs

一个简洁、类型安全的 Zhipu AI Rust SDK。专注提升 Rust 开发者的接入效率：更少样板代码、更一致的错误处理、可读的请求/响应类型，以及开箱即用的示例。

## 项目状态

### 🚀 最新进展（2025-12-29）
- ✅ 完成第一阶段：基础设施（CI/CD、代码质量工具配置）
- ✅ 新增 35 个单元测试，测试覆盖率达到 50%
- ✅ 修复错误处理系统，提高 API 错误解析准确性
- 🔄 正在进行：测试覆盖率提升、文档完善
- 📋 详见：[PROGRESS.md](PROGRESS.md)

## 快速开始
1. 准备环境
   - Rust 1.74+（或更高）
   - 设置环境变量：`ZHIPU_API_KEY="<your_api_key>"`
2. 构建
   - `cargo build`
3. 运行示例（examples/ 目录内）
   - `cargo run --example chat_loop`

## 示例（examples/）

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