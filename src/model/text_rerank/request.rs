use serde::{Deserialize, Serialize};

/// Rerank model enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum RerankModel {
    /// The default rerank model.
    #[default]
    Rerank,
}

/// Request body for rerank API
#[derive(Debug, Clone, Serialize)]
pub struct RerankBody {
    /// Model identifier; currently always `rerank`.
    pub model: RerankModel,

    /// Query text (at most 4,096 characters).
    pub query: String,

    /// Candidate documents (1–128 items, at most 4,096 characters each).
    pub documents: Vec<String>,

    /// Number of highest-ranked documents to return; omission returns all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_n: Option<usize>,

    /// Whether to include the original document text; defaults to `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_documents: Option<bool>,

    /// Whether to include raw relevance scores; defaults to `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_raw_scores: Option<bool>,

    /// Client-provided request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// End-user identifier used for abuse monitoring.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl RerankBody {
    /// Create a new rerank body from a model, query, and candidate documents.
    pub fn new(model: RerankModel, query: impl Into<String>, documents: Vec<String>) -> Self {
        Self {
            model,
            query: query.into(),
            documents,
            top_n: None,
            return_documents: None,
            return_raw_scores: None,
            request_id: None,
            user_id: None,
        }
    }

    /// Set how many top-ranked documents to return.
    pub fn with_top_n(mut self, n: usize) -> Self {
        self.top_n = Some(n);
        self
    }
    /// Whether to include the document text in the response.
    pub fn with_return_documents(mut self, v: bool) -> Self {
        self.return_documents = Some(v);
        self
    }
    /// Whether to include raw relevance scores in the response.
    pub fn with_return_raw_scores(mut self, v: bool) -> Self {
        self.return_raw_scores = Some(v);
        self
    }
    /// Set the client-side request id.
    pub fn with_request_id(mut self, v: impl Into<String>) -> Self {
        self.request_id = Some(v.into());
        self
    }
    /// Set the end-user id.
    pub fn with_user_id(mut self, v: impl Into<String>) -> Self {
        self.user_id = Some(v.into());
        self
    }

    /// Optional runtime validation for constraints expressed in the docs
    pub fn validate_constraints(&self) -> crate::ZaiResult<()> {
        if self.query.chars().count() > 4096 {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "query length exceeds 4096 characters".to_string(),
            });
        }
        if self.documents.is_empty() {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "documents must not be empty".to_string(),
            });
        }
        if self.documents.len() > 128 {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "documents length exceeds 128".to_string(),
            });
        }
        for (i, d) in self.documents.iter().enumerate() {
            if d.chars().count() > 4096 {
                return Err(crate::client::error::ZaiError::ApiError {
                    code: crate::client::error::codes::SDK_VALIDATION,
                    message: format!("document at index {i} exceeds 4096 characters"),
                });
            }
        }
        if let Some(n) = self.top_n
            && n > self.documents.len()
        {
            return Err(crate::client::error::ZaiError::ApiError {
                code: crate::client::error::codes::SDK_VALIDATION,
                message: "top_n cannot exceed documents length".to_string(),
            });
        }
        Ok(())
    }
}
