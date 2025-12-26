use serde::{Deserialize, Serialize};

/// Embedding model enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmbeddingModel {
    #[serde(rename = "embedding-3")]
    Embedding3,
    #[serde(rename = "embedding-2")]
    Embedding2,
}

/// Input can be a single string or an array of strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Single(String),
    Batch(Vec<String>),
}

/// Output vector dimensions for embeddings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingDimensions {
    D2048,
    D1024,
    D512,
    D256,
}

impl Serialize for EmbeddingDimensions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let v: u16 = match self {
            EmbeddingDimensions::D2048 => 2048,
            EmbeddingDimensions::D1024 => 1024,
            EmbeddingDimensions::D512 => 512,
            EmbeddingDimensions::D256 => 256,
        };
        serializer.serialize_u16(v)
    }
}

/// Request body for embeddings
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingBody {
    /// 嵌入模型：embedding-3 或 embedding-2
    pub model: EmbeddingModel,

    /// 输入文本，支持字符串或字符串数组
    pub input: EmbeddingInput,

    /// 输出维度，Embedding-3 支持 256/512/1024/2048；Embedding-2 固定 1024（可不填）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<EmbeddingDimensions>,
}

impl EmbeddingBody {
    pub fn new(model: EmbeddingModel, input: EmbeddingInput) -> Self {
        Self {
            model,
            input,
            dimensions: None,
        }
    }

    pub fn with_dimensions(mut self, dims: EmbeddingDimensions) -> Self {
        self.dimensions = Some(dims);
        self
    }

    /// Optional helper to enforce cross-field constraints at runtime.
    /// Call this before sending if you want strict validation.
    pub fn validate_model_constraints(&self) -> Result<(), validator::ValidationError> {
        use validator::ValidationError;
        // If input is Batch for embedding-3, enforce max 64 items (per API doc)
        if let EmbeddingModel::Embedding3 = self.model
            && let EmbeddingInput::Batch(ref v) = self.input
                && v.len() > 64 {
                    return Err(ValidationError::new("batch_too_long"));
                }
        // If model = embedding-2 and dimensions is Some, it must be 1024
        if let EmbeddingModel::Embedding2 = self.model
            && let Some(d) = self.dimensions
                && d != EmbeddingDimensions::D1024 {
                    return Err(ValidationError::new("embedding2_dims_must_be_1024"));
                }
        Ok(())
    }
}
