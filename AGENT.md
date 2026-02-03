# ZAI-RS 代码优化计划

> **创建时间**: 2026-02-03
> **状态**: 待执行
> **目标**: 统一文件读取接口，消除代码重复，提升性能

---

## 🎯 核心问题

### 问题1: 阻塞IO在异步上下文中 (Critical)
**影响**: 性能下降，可能阻塞异步运行时线程池

| 文件 | 行号 | 问题 | 优先级 |
|------|------|------|--------|
| `src/file/upload.rs` | 89 | `std::fs::read()` 阻塞读取 | P0 |
| `src/knowledge/document_upload_file.rs` | 201 | `std::fs::read()` 阻塞读取 | P0 |
| `src/file/content.rs` | 53,56 | `std::fs::create_dir_all()`, `File::create()` | P1 |

### 问题2: MIME类型推断重复 (High)
**影响**: 代码重复，维护困难

| 文件 | 行号 | 重复代码 |
|------|------|----------|
| `src/model/ocr/data.rs` | 162-171 | 手动匹配4种图片格式 |
| `src/model/audio_to_text/data.rs` | 144-148 | if-else判断2种音频格式 |
| `src/file/upload.rs` | 81-87 | 文件名提取逻辑 |

### 问题3: HTTP客户端未复用 (Medium)
**影响**: 无法利用连接池，每次请求创建新连接

| 文件 | 行号 | 问题 |
|------|------|------|
| `src/file/upload.rs` | 100 | `reqwest::Client::new()` |
| `src/knowledge/document_upload_file.rs` | 205 | `reqwest::Client::new()` |
| `src/model/audio_to_text/data.rs` | 173 | `reqwest::Client::new()` |

---

## 📋 优化方案

### 方案概览

创建统一的 `src/io` 模块，提供：
1. **异步文件读取** - `read_file()` 返回 `FileContent`
2. **MIME类型推断** - `infer_mime_type()` 覆盖所有常见格式
3. **文件验证** - `FileValidation` 结构体

### 架构设计

```
src/io/mod.rs (新建)
├── pub struct FileContent { bytes, file_name, mime_type, size }
├── pub async fn read_file() -> ZaiResult<FileContent>
├── pub fn infer_mime_type() -> String
└── pub struct FileValidation { ... }
```

---

## 🚀 实施计划

### Phase 1: 基础设施 (Day 1)

#### Task 1.1: 创建 io 模块骨架
**文件**: `src/io/mod.rs` (新建)

```rust
//! 统一的文件I/O操作
//!
//! 提供异步文件读取、MIME类型推断等功能

use std::path::Path;

/// 文件内容封装
pub struct FileContent {
    pub bytes: Vec<u8>,
    pub file_name: String,
    pub mime_type: String,
    pub size: u64,
}

/// MIME类型推断（完整实现见Task 1.2）
pub fn infer_mime_type(path: &Path) -> String {
    "application/octet-stream".to_string()
}
```

**验证**: `cargo check --lib` 通过

#### Task 1.2: 实现 MIME 类型推断
**文件**: `src/io/mod.rs` (修改)

**实现内容**:
- 图片: png, jpg, jpeg, bmp, gif, webp
- 音频: mp3, wav, ogg, m4a, aac
- 文档: pdf, doc, docx, xls, xlsx, ppt, pptx, txt, md, csv, json, xml
- 视频: mp4, avi, mkv, mov, webm
- 压缩: zip, rar, 7z, tar, gz
- 默认: application/octet-stream

**验证**: 添加单元测试
```bash
cargo test io::tests::test_infer_mime_type
```

#### Task 1.3: 实现异步文件读取
**文件**: `src/io/mod.rs` (修改)

```rust
use tokio::fs;

pub async fn read_file(path: impl AsRef<Path>) -> crate::ZaiResult<FileContent> {
    let path = path.as_ref();

    // 1. 检查文件存在
    if !path.exists() {
        return Err(crate::client::error::ZaiError::FileError {
            code: 0,
            message: format!("File not found: {}", path.display()),
        });
    }

    // 2. 异步读取
    let bytes = fs::read(path).await?;

    // 3. 提取元数据
    let file_name = path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mime_type = infer_mime_type(path);
    let size = bytes.len() as u64;

    Ok(FileContent { bytes, file_name, mime_type, size })
}
```

**验证**:
```bash
cargo test io::tests::test_read_file
```

#### Task 1.4: 在 lib.rs 中导出模块
**文件**: `src/lib.rs:49` (修改)

```rust
pub mod io;  // 添加这一行
pub mod file;
```

**验证**: `cargo build` 成功

---

### Phase 2: 重构核心模块 (Day 2-3)

#### Task 2.1: 重构 file/upload.rs
**文件**: `src/file/upload.rs:89-98`

**Before**:
```rust
let part = reqwest::multipart::Part::bytes(std::fs::read(&path)?)
    .file_name(fname);
```

**After**:
```rust
use crate::io;

let content = io::read_file(&path).await?;
let mut part = reqwest::multipart::Part::bytes(content.bytes)
    .file_name(content.file_name);
if let Some(ct) = content_type {
    part = part.mime_str(&ct)?;
} else {
    part = part.mime_str(&content.mime_type)?;
}
```

**验证**:
```bash
cargo run --example files_upload
```

#### Task 2.2: 重构 knowledge/document_upload_file.rs
**文件**: `src/knowledge/document_upload_file.rs:195-203`

**Before**:
```rust
for path in files {
    let fname = path.file_name()...
    let part = reqwest::multipart::Part::bytes(std::fs::read(&path)?)
        .file_name(fname);
    form = form.part("files", part);
}
```

**After**:
```rust
use crate::io;

for path in files {
    let content = io::read_file(&path).await?;
    let part = reqwest::multipart::Part::bytes(content.bytes)
        .file_name(content.file_name)
        .mime_str(&content.mime_type)?;
    form = form.part("files", part);
}
```

**验证**:
```bash
cargo run --example knowledge_document_upload_file
```

#### Task 2.3: 重构 model/ocr/data.rs
**文件**: `src/model/ocr/data.rs:155-176`

**删除代码** (行155-176):
- 文件名提取逻辑
- MIME类型匹配逻辑

**替换为**:
```rust
use crate::io;

let content = io::read_file(&file_path).await
    .map_err(|e| ZaiError::FileError {
        code: 0,
        message: format!("Failed to read OCR file: {}", e),
    })?;

let part = reqwest::multipart::Part::bytes(content.bytes)
    .file_name(content.file_name)
    .mime_str(&content.mime_type)?;
```

**验证**:
```bash
cargo run --example ocr
```

#### Task 2.4: 重构 model/audio_to_text/data.rs
**文件**: `src/model/audio_to_text/data.rs:137-153`

**删除代码** (行137-153):
- 文件名提取
- MIME类型if-else判断

**替换为**:
```rust
use crate::io;

let content = io::read_file(&file_path).await?;
let part = reqwest::multipart::Part::bytes(content.bytes)
    .file_name(content.file_name)
    .mime_str(&content.mime_type)?;
```

**验证**:
```bash
cargo run --example audio_to_text
```

#### Task 2.5: 重构 file/content.rs (可选)
**文件**: `src/file/content.rs:45-59`

**修改异步写入**:
```rust
pub async fn send_to<P: AsRef<std::path::Path>>(&self, path: P) -> crate::ZaiResult<usize> {
    let bytes = self.send().await?;
    let p = path.as_ref();

    if let Some(parent) = p.parent()
        && !parent.as_os_str().is_empty()
    {
        tokio::fs::create_dir_all(parent).await?;
    }

    tokio::fs::write(p, &bytes).await?;
    Ok(bytes.len())
}
```

---

### Phase 3: 统一HTTP客户端 (Day 4)

#### Task 3.1: 利用现有共享客户端
**文件**: `src/file/upload.rs:100-105`

**观察**: 项目已有 `http_client_with_config()` 函数

**修改**: 使用共享客户端而非每次创建新的

```rust
// Before
let resp = reqwest::Client::new()
    .post(url)
    .bearer_auth(key)
    .multipart(form)
    .send()
    .await?;

// After
use crate::client::http::http_client_with_config;
use crate::client::http::HttpClientConfig;

let client = http_client_with_config(&HttpClientConfig::default());
let resp = client
    .post(url)
    .bearer_auth(key)
    .multipart(form)
    .send()
    .await?;
```

**同样的修改应用于**:
- `src/knowledge/document_upload_file.rs:205`
- `src/model/audio_to_text/data.rs:173`

---

### Phase 4: 测试验证 (Day 5)

#### Task 4.1: 单元测试
```bash
cargo test --lib
```

**预期**: 所有测试通过，无新增警告

#### Task 4.2: 运行受影响的示例
```bash
cargo run --example files_upload
cargo run --example files_content
cargo run --example knowledge_document_upload_file
cargo run --example ocr
cargo run --example audio_to_text
```

**预期**: 所有示例正常运行

#### Task 4.3: Clippy 检查
```bash
cargo clippy --all-targets --all-features
```

**预期**: 无新警告

#### Task 4.4: Fmt 检查
```bash
cargo fmt --check
```

**预期**: 格式正确

---

## 📊 验收标准

### 功能验收
- [ ] 所有单元测试通过
- [ ] 所有受影响的示例正常运行
- [ ] 无 clippy 警告
- [ ] 代码格式符合规范

### 性能验收
- [ ] 异步文件读取不阻塞运行时
- [ ] HTTP连接复用生效（可通过日志验证）

### 代码质量
- [ ] 删除了所有重复的MIME类型推断代码
- [ ] 删除了所有重复的文件名提取代码
- [ ] 统一的错误处理

---

## 🔄 回滚计划

如果出现问题：

1. **Git revert**
```bash
git revert HEAD~N  # N为提交次数
```

2. **分支切换**
```bash
git checkout main
```

3. **保留原代码**
每个重构任务都在同一文件中修改，保留注释标记：
```rust
// OLD: std::fs::read(&path)
// NEW: io::read_file(&path).await
```

---

## 📝 提交规范

### Commit Message 格式
```
refactor(io): unify async file reading interface

- Create src/io module with read_file() and infer_mime_type()
- Replace std::fs::read with io::read_file in upload.rs
- Replace std::fs::read with io::read_file in document_upload_file.rs
- Remove duplicate MIME type detection code from ocr/data.rs
- Remove duplicate MIME type detection code from audio_to_text/data.rs

Benefits:
- Consistent async I/O across all modules
- Eliminates code duplication
- Enables HTTP connection pooling

Refs: AGENT.md Task 2.1-2.4
```

### 每个Task单独提交
- Task 1.1-1.4: "feat(io): add unified file I/O module"
- Task 2.1: "refactor(file): use io::read_file in upload.rs"
- Task 2.2: "refactor(knowledge): use io::read_file in document_upload_file"
- Task 2.3: "refactor(model): use io::read_file in ocr module"
- Task 2.4: "refactor(model): use io::read_file in audio_to_text"
- Task 3.1: "perf(http): use shared HTTP client for file uploads"

---

## ⏱️ 时间估算

| Phase | 任务 | 预计时间 |
|-------|------|---------|
| Phase 1 | 基础设施 (4个任务) | 4-6小时 |
| Phase 2 | 重构模块 (5个任务) | 6-8小时 |
| Phase 3 | HTTP客户端统一 | 2-3小时 |
| Phase 4 | 测试验证 | 2-3小时 |
| **总计** | | **14-20小时** |

---

## 🎓 后续优化 (Out of Scope)

本计划专注于**文件读取接口统一**，以下优化留待后续：

1. **完整的 FileValidation 实现** - 当前MVP仅实现基础功能
2. **错误类型扩展** - 新增 `FileNotFound`, `FileTooLarge` 等
3. **性能基准测试** - 对比优化前后的具体数据
4. **文档更新** - 更新 README 和 API 文档

---

## 📚 参考资料

- [Tokio FileSystem](https://tokio.rs/tokio/tutorial/fs)
- [Reqwest Multipart](https://docs.rs/reqwest/latest/reqwest/multipart/index.html)
- [MIME Types](https://www.iana.org/assignments/media-types/media-types.xhtml)

---

**维护者**: 开发团队
**最后更新**: 2026-02-03
