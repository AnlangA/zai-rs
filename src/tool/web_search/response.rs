use serde::{Deserialize, Serialize};
use validator::Validate;

/// Web search item returned by the service.
/// Notes:
/// - `link` and media URLs may be temporary; consider downloading or caching if
///   needed.
/// - Fields are optional and may vary by search provider/source.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct WebSearchInfo {
    /// Source website icon
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Search result title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Search result page link
    #[serde(skip_serializing_if = "Option::is_none")]
    #[validate(url)]
    pub link: Option<String>,
    /// Media source name of the page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Publish date on the website
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_date: Option<String>,
    /// Quoted text content from the search result page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Corner mark sequence number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refer: Option<String>,
}

/// Web search API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResponse {
    /// Task ID
    pub id: String,
    /// Request creation time as Unix timestamp
    pub created: i64,
    /// Request identifier
    pub request_id: String,
    /// Search intent results
    pub search_intent: Vec<SearchIntent>,
    /// Search results
    pub search_result: Vec<SearchResult>,
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

impl WebSearchResponse {
    /// Get the total number of search results
    pub fn result_count(&self) -> usize {
        self.search_result.len()
    }

    /// Get the search intent results
    pub fn intents(&self) -> &Vec<SearchIntent> {
        &self.search_intent
    }

    /// Get the search results
    pub fn results(&self) -> &Vec<SearchResult> {
        &self.search_result
    }

    /// Get the task ID
    pub fn task_id(&self) -> &str {
        &self.id
    }

    /// Get the request creation time as Unix timestamp
    pub fn created_at(&self) -> i64 {
        self.created
    }

    /// Get the request ID
    pub fn request_id(&self) -> &str {
        &self.request_id
    }
}
