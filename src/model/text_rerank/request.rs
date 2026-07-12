use serde::{Deserialize, Serialize};

/// Rerank model enum
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RerankModel {
    /// The default rerank model.
    #[default]
    Rerank,
}

/// Request body for rerank API
#[derive(Clone, Serialize)]
pub struct RerankBody {
    /// Model identifier; currently always `rerank`.
    pub model: RerankModel,

    /// Query text (at most 4,096 characters).
    pub query: String,

    /// Candidate documents (1–128 items, at most 4,096 characters each).
    pub documents: Vec<String>,

    /// Number of highest-ranked documents to return; `0` or omission returns
    /// all documents.
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

impl std::fmt::Debug for RerankBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RerankBody")
            .field("model", &self.model)
            .field("query", &"[REDACTED]")
            .field("documents_len", &self.documents.len())
            .field("top_n", &self.top_n)
            .field("return_documents", &self.return_documents)
            .field("return_raw_scores", &self.return_raw_scores)
            .field(
                "request_id",
                &self.request_id.as_ref().map(|_| "[REDACTED]"),
            )
            .field("user_id", &self.user_id.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
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
        if self.query.trim().is_empty() {
            return Err(crate::client::validation::invalid(
                "query must not be empty",
            ));
        }
        if self.query.chars().count() > 4096 {
            return Err(crate::client::validation::invalid(
                "query length exceeds 4096 characters",
            ));
        }
        if self.documents.is_empty() {
            return Err(crate::client::validation::invalid(
                "documents must not be empty",
            ));
        }
        if self.documents.len() > 128 {
            return Err(crate::client::validation::invalid(
                "documents length exceeds 128",
            ));
        }
        for (i, d) in self.documents.iter().enumerate() {
            if d.trim().is_empty() {
                return Err(crate::client::validation::invalid(format!(
                    "document at index {i} must not be empty"
                )));
            }
            if d.chars().count() > 4096 {
                return Err(crate::client::validation::invalid(format!(
                    "document at index {i} exceeds 4096 characters"
                )));
            }
        }
        if let Some(n) = self.top_n
            && n > self.documents.len()
        {
            return Err(crate::client::validation::invalid(
                "top_n cannot exceed documents length",
            ));
        }
        if let Some(request_id) = self.request_id.as_deref()
            && !(6..=64).contains(&request_id.chars().count())
        {
            return Err(crate::client::validation::invalid(
                "request_id must contain between 6 and 64 characters",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_rejects_blank_values_and_accepts_zero_top_n() {
        assert!(
            RerankBody::new(RerankModel::Rerank, " ", vec!["doc".into()])
                .validate_constraints()
                .is_err()
        );
        assert!(
            RerankBody::new(RerankModel::Rerank, "query", vec![" ".into()])
                .validate_constraints()
                .is_err()
        );
        assert!(
            RerankBody::new(RerankModel::Rerank, "query", vec!["doc".into()])
                .with_top_n(0)
                .validate_constraints()
                .is_ok()
        );
    }

    #[test]
    fn debug_redacts_query_documents_and_identifiers() {
        let body = RerankBody::new(
            RerankModel::Rerank,
            "private query",
            vec!["private document".to_owned()],
        )
        .with_request_id("private-request")
        .with_user_id("private-user");
        let debug = format!("{body:?}");
        for secret in [
            "private query",
            "private document",
            "private-request",
            "private-user",
        ] {
            assert!(!debug.contains(secret));
        }
    }
}
