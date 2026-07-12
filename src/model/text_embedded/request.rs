use serde::{Deserialize, Serialize};

/// Embedding model enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmbeddingModel {
    /// embedding-3 (supports configurable dimensions).
    #[serde(rename = "embedding-3")]
    Embedding3,
    /// embedding-2 (fixed 1024 dimensions).
    #[serde(rename = "embedding-2")]
    Embedding2,
}

/// Input can be a single string or an array of strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    /// A single input string.
    Single(String),
    /// A batch of input strings.
    Batch(Vec<String>),
}

/// Output vector dimensions for embeddings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingDimensions {
    /// 2048-dimensional embedding.
    D2048,
    /// 1024-dimensional embedding.
    D1024,
    /// 512-dimensional embedding.
    D512,
    /// 256-dimensional embedding.
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
    /// Embedding model (`embedding-3` or `embedding-2`).
    pub model: EmbeddingModel,

    /// A single input string or a batch of strings.
    pub input: EmbeddingInput,

    /// Output dimensions. `embedding-3` supports all variants;
    /// `embedding-2` accepts only 1,024 dimensions or omission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<EmbeddingDimensions>,
}

impl EmbeddingBody {
    /// Create a new embedding request body from a model and input.
    pub fn new(model: EmbeddingModel, input: EmbeddingInput) -> Self {
        Self {
            model,
            input,
            dimensions: None,
        }
    }

    /// Set the output vector dimensionality (embedding-3 only).
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
            && v.len() > 64
        {
            return Err(ValidationError::new("batch_too_long"));
        }
        // If model = embedding-2 and dimensions is Some, it must be 1024
        if let EmbeddingModel::Embedding2 = self.model
            && let Some(d) = self.dimensions
            && d != EmbeddingDimensions::D1024
        {
            return Err(ValidationError::new("embedding2_dims_must_be_1024"));
        }
        Ok(())
    }
}
