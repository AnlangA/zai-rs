use serde::{Deserialize, Serialize};
use validator::Validate;

/// Web search engine options supported by the API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchEngine {
    /// Zhipu basic search engine
    SearchStd,
    /// Zhipu advanced search engine
    SearchPro,
    /// Sougou search engine
    SearchProSogou,
    /// Quark search engine
    SearchProQuark,
}

/// Search result recency filter options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentSize {
    /// Medium content size with summary information for basic reasoning needs
    Medium,
    /// High content size with maximized context and detailed information
    High,
}

/// Search intent result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIntent {
    /// The search query
    pub query: String,
    /// The detected intent type
    pub intent: String,
    /// Extracted keywords
    pub keywords: String,
}

/// Individual search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Title of the search result
    pub title: String,
    /// Content summary
    pub content: String,
    /// URL link to the result
    pub link: String,
    /// Website/media name
    pub media: String,
    /// Website icon URL
    pub icon: String,
    /// Reference index number
    pub refer: String,
    /// Publication date
    pub publish_date: String,
}

/// Web search request body
#[derive(Debug, Clone, Serialize, Validate)]
pub struct WebSearchBody {
    /// Search query content (max 70 characters)
    #[validate(length(max = 70, message = "search_query cannot exceed 70 characters"))]
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

impl WebSearchBody {
    /// Create a new web search request body with required parameters
    pub fn new(search_query: String, search_engine: SearchEngine) -> Self {
        Self {
            search_query,
            search_engine,
            search_intent: None,
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
    pub fn with_domain_filter(mut self, domain: String) -> Self {
        self.search_domain_filter = Some(domain);
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
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Set user ID
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Validate the request body constraints
    pub fn validate_constraints(&self) -> crate::ZaiResult<()> {
        self.validate()
            .map_err(|e| crate::client::error::ZaiError::ApiError {
                code: 1200,
                message: format!("Validation error: {}", e),
            })?;

        // Additional validation for count based on search engine
        if let Some(count) = self.count {
            if matches!(self.search_engine, SearchEngine::SearchProSogou) {
                match count {
                    10 | 20 | 30 | 40 | 50 => {}
                    _ => {
                        return Err(crate::client::error::ZaiError::ApiError {
                            code: 1200,
                            message:
                                "search_pro_sogou only supports count values: 10, 20, 30, 40, 50"
                                    .to_string(),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}
