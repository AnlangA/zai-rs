# zai-rs 文档

这里是 zai-rs（Zhipu AI Rust SDK）的使用与维护文档。

## 入门指南

- **[快速入门指南](GETTING_STARTED.md)** - 新用户的完整入门教程
  - 安装和配置
  - 基础用法示例
  - 错误处理入门
  - 高级配置

## 使用指南

- **[架构说明](ARCHITECTURE.md)** - 项目分层、端点配置、错误/重试策略和测试边界
  - API 家族和默认 endpoint
  - 请求 builder / 传输层分工
  - 错误码映射和 retry 策略
  - 测试与文档维护规则

- **[全面优化路线图](OPTIMIZATION_PLAN.md)** - 按风险和依赖排序的持续改进计划
  - 当前构建、测试与安全基线
  - HTTP、流式 I/O、Realtime 和公共 API 里程碑
  - 每项工作的验收标准与预估工作量

- **[6.0.1 安全加固迁移说明](HARDENING_MIGRATION.md)** - 从 0.6.0 升级时的可观察行为变化

- **[发布清单](RELEASING.md)** - OIDC、SBOM、provenance 与发布前外部配置

- **[错误处理指南](ERROR_HANDLING.md)** - 详细的错误处理机制和最佳实践
  - 错误类型说明
  - 基础和高级错误处理
  - 重试机制详解
  - 日志安全实践

- **[最佳实践](BEST_PRACTICES.md)** - 使用 zai-rs 的推荐做法
  - API 密钥管理
  - 错误处理
  - 请求优化
  - 日志和监控
  - 并发和性能
  - 安全性建议

- **[高级主题](ADVANCED_TOPICS.md)** - 进阶功能和高级用法
  - 重试机制详解
  - 流式处理
  - 工具调用
  - 异步聊天
  - 文件和知识库管理
  - 批量处理
  - 性能优化技巧

- **[能力选择指引](CHOOSING_AN_API.md)** - 重叠能力入口的对照与选型建议
  - 图像生成：同步 vs 异步任务
  - OCR：手写识别 vs 文档版式解析
  - 文件解析：同步 vs 异步任务

- **[常见问题 (FAQ)](FAQ.md)** - 常见问题和解决方案
  - 安装和配置问题
  - 使用问题
  - 错误处理
  - 性能优化
  - 故障排除

## API 参考

- **Rustdoc** - 在仓库根目录运行 `cargo doc --all-features --open`

## 示例代码

- **[示例目录](../examples/)** - 实际使用示例
  - 聊天补全
  - typed SSE 流式响应
  - 图像和视频生成
  - 语音处理
  - 工具调用
  - 文件管理

书中的 Rust 代码块依赖本 crate 及 Tokio 等 Cargo 依赖。`mdbook test` 无法为
独立章节注入这些依赖，因此标记为 `rust,ignore`；同一 API 的可执行版本位于
`examples/`，并由工作区的 examples 构建检查验证。crate 内的 rustdoc 示例则
使用可编译的 `rust` 或 `rust,no_run`。

## 外部资源

- [智谱AI API 文档](https://docs.bigmodel.cn/) - 官方 API 文档
- [智谱AI开放平台](https://open.bigmodel.cn/) - API 密钥申请和管理
- [GitHub Issues](https://github.com/AnlangA/zai-rs/issues) - 问题反馈和讨论

## 版本说明

当前 Cargo 版本为 `6.0.1`。从较早版本升级前请阅读
[0.6 迁移指南](MIGRATING-0.6.md)；从 `0.6.0` 升级时还必须阅读
[6.0.1 安全加固迁移说明](HARDENING_MIGRATION.md)。

## 维护注记

仓库根目录的 `README.md`（中文）与 `README.en.md`（英文）内容保持一致；
修改其中一版时，请同步更新另一版。
