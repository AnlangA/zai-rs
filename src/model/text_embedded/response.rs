use serde::{Deserialize, Serialize};

/// Top-level response from the embeddings endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// Model that produced the embeddings.
    pub model: String,
    /// Object kind of the response envelope (always `list`).
    pub object: ResponseObjectKind,
    /// Per-input embedding vectors.
    pub data: Vec<EmbeddingData>,
    /// Token-usage statistics for the request.
    pub usage: EmbeddingUsage,
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
    pub index: usize,
    /// Object kind (always `embedding`).
    pub object: EmbeddingObjectKind,
    /// The embedding vector.
    pub embedding: Vec<f32>,
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
    pub prompt_tokens: u64,
    /// Number of completion tokens (typically 0 for embeddings).
    pub completion_tokens: u64,
    /// Total tokens for this request.
    pub total_tokens: u64,
}
