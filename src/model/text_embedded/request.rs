use serde::{Deserialize, Serialize};

/// Embedding model enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EmbeddingInput {
    /// A single input string.
    Single(String),
    /// A batch of input strings.
    Batch(Vec<String>),
}

impl std::fmt::Debug for EmbeddingInput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Single(_) => formatter
                .debug_tuple("Single")
                .field(&"[REDACTED]")
                .finish(),
            Self::Batch(values) => formatter
                .debug_struct("Batch")
                .field("len", &values.len())
                .finish(),
        }
    }
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
#[derive(Clone, Serialize)]
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

impl std::fmt::Debug for EmbeddingBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EmbeddingBody")
            .field("model", &self.model)
            .field("input", &self.input)
            .field("dimensions", &self.dimensions)
            .finish()
    }
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

    /// Enforce input and model/dimension constraints before sending.
    pub fn validate_model_constraints(&self) -> Result<(), validator::ValidationError> {
        use validator::ValidationError;
        let has_empty_input = match &self.input {
            EmbeddingInput::Single(value) => value.trim().is_empty(),
            EmbeddingInput::Batch(values) => {
                values.is_empty() || values.iter().any(|value| value.trim().is_empty())
            },
        };
        if has_empty_input {
            return Err(ValidationError::new("input_must_not_be_empty"));
        }
        if let EmbeddingModel::Embedding3 = self.model
            && let EmbeddingInput::Batch(ref v) = self.input
            && v.len() > 64
        {
            return Err(ValidationError::new("batch_too_long"));
        }
        if let EmbeddingModel::Embedding2 = self.model
            && let Some(d) = self.dimensions
            && d != EmbeddingDimensions::D1024
        {
            return Err(ValidationError::new("embedding2_dims_must_be_1024"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_rejects_empty_inputs() {
        for input in [
            EmbeddingInput::Single(" ".to_owned()),
            EmbeddingInput::Batch(Vec::new()),
            EmbeddingInput::Batch(vec!["valid".to_owned(), String::new()]),
        ] {
            assert!(
                EmbeddingBody::new(EmbeddingModel::Embedding3, input)
                    .validate_model_constraints()
                    .is_err()
            );
        }
    }

    #[test]
    fn debug_redacts_embedding_inputs() {
        let body = EmbeddingBody::new(
            EmbeddingModel::Embedding3,
            EmbeddingInput::Batch(vec!["private one".to_owned(), "private two".to_owned()]),
        );
        let debug = format!("{body:?}");
        assert!(!debug.contains("private one"));
        assert!(!debug.contains("private two"));
        assert!(debug.contains("len: 2"));
    }
}
