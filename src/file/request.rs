use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Validate)]
pub struct FileListQuery {
    /// 分页游标
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// 按用途过滤文件（可选，为兼容 cURL 示例中未提供）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<FilePurpose>,

    /// 排序方式（目前仅 created_at）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<FileOrder>,

    /// 每页数量 1..=100（默认 20）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(range(min = 1, max = 100))]
    pub limit: Option<u32>,
}

impl FileListQuery {
    pub fn new() -> Self {
        Self { after: None, purpose: None, order: None, limit: None }
    }
    pub fn with_after(mut self, after: impl Into<String>) -> Self { self.after = Some(after.into()); self }
    pub fn with_purpose(mut self, p: FilePurpose) -> Self { self.purpose = Some(p); self }
    pub fn with_order(mut self, o: FileOrder) -> Self { self.order = Some(o); self }
    pub fn with_limit(mut self, limit: u32) -> Self { self.limit = Some(limit); self }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilePurpose {
    #[serde(rename = "batch")] Batch,
    #[serde(rename = "file-extract")] FileExtract,
    #[serde(rename = "code-interpreter")] CodeInterpreter,
    #[serde(rename = "agent")] Agent,
    #[serde(rename = "voice-clone-input")] VoiceCloneInput,
}

impl FilePurpose {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilePurpose::Batch => "batch",
            FilePurpose::FileExtract => "file-extract",
            FilePurpose::CodeInterpreter => "code-interpreter",
            FilePurpose::Agent => "agent",
            FilePurpose::VoiceCloneInput => "voice-clone-input",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileOrder {
    #[serde(rename = "created_at")] CreatedAt,
}

impl FileOrder {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileOrder::CreatedAt => "created_at",
        }
    }
}

