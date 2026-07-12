use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

/// Top-level response from the embeddings endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingResponse {
    /// Model that produced the embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Object kind of the response envelope (always `list`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<ResponseObjectKind>,
    /// Per-input embedding vectors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<EmbeddingData>>,
    /// Token-usage statistics for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<EmbeddingUsage>,
}

#[derive(Deserialize)]
struct EmbeddingResponseWire {
    model: Option<String>,
    object: Option<ResponseObjectKind>,
    data: Option<Vec<EmbeddingData>>,
    usage: Option<EmbeddingUsage>,
}

impl<'de> Deserialize<'de> for EmbeddingResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EmbeddingResponseWire::deserialize(deserializer)?;
        if wire.model.is_none()
            && wire.object.is_none()
            && wire.data.is_none()
            && wire.usage.is_none()
        {
            return Err(D::Error::custom(
                "embedding response contained no documented fields",
            ));
        }
        Ok(Self {
            model: wire.model,
            object: wire.object,
            data: wire.data,
            usage: wire.usage,
        })
    }
}

/// Top-level object kind returned by the embeddings endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseObjectKind {
    /// A list of embedding items.
    List,
}

/// A single embedding vector with its index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    /// Position of this input within the request batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
    /// Object kind (always `embedding`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<EmbeddingObjectKind>,
    /// The embedding vector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

/// Object kind for an embedding item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingObjectKind {
    /// Embedding object.
    Embedding,
}

/// Token-usage statistics for an embeddings request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    /// Number of prompt tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    /// Number of completion tokens (typically 0 for embeddings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u64>,
    /// Total tokens for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_requires_one_documented_non_null_field() {
        assert!(serde_json::from_str::<EmbeddingResponse>("{}").is_err());
        assert!(serde_json::from_str::<EmbeddingResponse>(r#"{"data":null}"#).is_err());
        assert!(serde_json::from_str::<EmbeddingResponse>(r#"{"data":[]}"#).is_ok());
    }

    #[test]
    fn nested_properties_follow_their_optional_schema() {
        let item: EmbeddingData = serde_json::from_str("{}").unwrap();
        assert!(item.index.is_none());
        assert!(item.object.is_none());
        assert!(item.embedding.is_none());
        let usage: EmbeddingUsage = serde_json::from_str("{}").unwrap();
        assert!(usage.prompt_tokens.is_none());
    }
}
