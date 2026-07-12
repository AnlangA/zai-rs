use serde::{Deserialize, Serialize};
use validator::Validate;

/// Web search engine options supported by the API
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchEngine {
    /// Zhipu basic search engine
    SearchStd,
    /// Zhipu advanced search engine
    SearchPro,
    /// Sogou search engine
    SearchProSogou,
    /// Quark search engine
    SearchProQuark,
}

/// Search result recency filter options
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SearchRecencyFilter {
    /// Search within one day
    OneDay,
    /// Search within one week
    OneWeek,
    /// Search within one month
    OneMonth,
    /// Search within one year
    OneYear,
    /// No time limit (default)
    NoLimit,
}

/// Content size options for search results
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentSize {
    /// Medium content size with summary information for basic reasoning needs
    Medium,
    /// High content size with maximized context and detailed information
    High,
}

/// Web search request body
#[derive(Clone, Serialize, Validate)]
pub struct WebSearchBody {
    /// Search query content (max 70 characters)
    #[validate(length(
        min = 1,
        max = 70,
        message = "search_query must be between 1 and 70 characters"
    ))]
    pub search_query: String,

    /// Search engine to use
    pub search_engine: SearchEngine,

    /// Whether to perform search intent recognition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_intent: Option<bool>,

    /// Number of results to return (1-50)
    #[validate(range(min = 1, max = 50, message = "count must be between 1 and 50"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i32>,

    /// Domain filter for search results (whitelist)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_domain_filter: Option<String>,

    /// Time range filter for search results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_recency_filter: Option<SearchRecencyFilter>,

    /// Content size control
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_size: Option<ContentSize>,

    /// Unique request identifier
    #[validate(length(
        min = 6,
        max = 64,
        message = "request_id must be between 6 and 64 characters"
    ))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    /// End user unique ID (6-128 characters)
    #[validate(length(
        min = 6,
        max = 128,
        message = "user_id must be between 6 and 128 characters"
    ))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl std::fmt::Debug for WebSearchBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("WebSearchBody")
            .field("search_query", &"[REDACTED]")
            .field("search_engine", &self.search_engine)
            .field("search_intent", &self.search_intent)
            .field("count", &self.count)
            .field(
                "search_domain_filter",
                &self.search_domain_filter.as_ref().map(|_| "[REDACTED]"),
            )
            .field("search_recency_filter", &self.search_recency_filter)
            .field("content_size", &self.content_size)
            .field(
                "request_id",
                &self.request_id.as_ref().map(|_| "[REDACTED]"),
            )
            .field("user_id", &self.user_id.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

impl WebSearchBody {
    /// Create a new web search request body with required parameters
    pub fn new(search_query: impl Into<String>, search_engine: SearchEngine) -> Self {
        Self {
            search_query: search_query.into(),
            search_engine,
            search_intent: Some(false),
            count: None,
            search_domain_filter: None,
            search_recency_filter: None,
            content_size: None,
            request_id: None,
            user_id: None,
        }
    }

    /// Enable search intent recognition
    pub fn with_search_intent(mut self, enabled: bool) -> Self {
        self.search_intent = Some(enabled);
        self
    }

    /// Set the number of results to return
    pub fn with_count(mut self, count: i32) -> Self {
        self.count = Some(count);
        self
    }

    /// Set domain filter for search results
    pub fn with_domain_filter(mut self, domain: impl Into<String>) -> Self {
        self.search_domain_filter = Some(domain.into());
        self
    }

    /// Set time range filter for search results
    pub fn with_recency_filter(mut self, filter: SearchRecencyFilter) -> Self {
        self.search_recency_filter = Some(filter);
        self
    }

    /// Set content size preference
    pub fn with_content_size(mut self, size: ContentSize) -> Self {
        self.content_size = Some(size);
        self
    }

    /// Set custom request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Validate the request body constraints
    pub fn validate_constraints(&self) -> crate::ZaiResult<()> {
        self.validate().map_err(crate::ZaiError::from)?;

        if self.search_query.trim().is_empty() {
            return Err(crate::client::validation::invalid(
                "search_query cannot be blank",
            ));
        }
        if self.search_intent.is_none() {
            return Err(crate::client::validation::invalid(
                "search_intent is required",
            ));
        }
        if self
            .search_domain_filter
            .as_deref()
            .is_some_and(|domain| domain.trim().is_empty())
        {
            return Err(crate::client::validation::invalid(
                "search_domain_filter cannot be blank",
            ));
        }

        // Additional validation for count based on search engine
        if let Some(count) = self.count
            && matches!(self.search_engine, SearchEngine::SearchProSogou)
        {
            match count {
                10 | 20 | 30 | 40 | 50 => {},
                _ => {
                    return Err(crate::client::validation::invalid(
                        "search_pro_sogou only supports count values: 10, 20, 30, 40, 50",
                    ));
                },
            }
        }

        if matches!(self.search_engine, SearchEngine::SearchProQuark)
            && (self.count.is_some() || self.search_domain_filter.is_some())
        {
            return Err(crate::client::validation::invalid(
                "search_pro_quark does not support count or search_domain_filter",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_recency_use_official_wire_values() {
        let body = WebSearchBody::new("query".to_string(), SearchEngine::SearchStd)
            .with_recency_filter(SearchRecencyFilter::OneWeek);
        let value = serde_json::to_value(body).unwrap();

        assert_eq!(value["search_intent"], false);
        assert_eq!(value["search_recency_filter"], "oneWeek");
    }

    #[test]
    fn debug_redacts_query_domain_and_identifiers() {
        let body = WebSearchBody::new("private query".to_owned(), SearchEngine::SearchStd)
            .with_domain_filter("private.example")
            .with_request_id("private-request")
            .with_user_id("private-user");
        let debug = format!("{body:?}");
        for secret in [
            "private query",
            "private.example",
            "private-request",
            "private-user",
        ] {
            assert!(!debug.contains(secret));
        }
    }
}
