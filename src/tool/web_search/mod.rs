//! Web search API module for the zai-rs crate.
//!
//! This module provides functionality to perform web searches using the Zhipu
//! AI web search API. It supports multiple search engines, intent recognition,
//! and various filtering options.
//!
//! # Features
//!
//! - Multiple search engines (Zhipu basic/advanced, Sougou, Quark)
//! - Search intent recognition
//! - Configurable result count (1-50)
//! - Domain filtering
//! - Time-based filtering
//! - Content size control
//! - Comprehensive validation
//!
//! # Example
//!
//! ```text
//! use zai_rs::tool::web_search::{SearchEngine, WebSearchRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let api_key = std::env::var("ZHIPU_API_KEY")?;
//!
//!     let request = WebSearchRequest::new(
//!         api_key,
//!         "rust programming language".to_string(),
//!         SearchEngine::SearchStd,
//!     )
//!     .with_count(10)
//!     .with_search_intent(true);
//!
//!     let response = request.send_via(&client).await?;
//!     println!("Found {} results", response.result_count());
//!
//!     Ok(())
//! }
//! ```

/// Request builder and client for the web-search tool.
pub mod data;
/// Request body types for web search.
pub mod request;
/// Response body types for web search.
pub mod response;

// Re-export main types for convenience
pub use data::WebSearchRequest;
pub use request::{
    ContentSize, SearchEngine, SearchIntent, SearchRecencyFilter, SearchResult, WebSearchBody,
};
pub use response::{
    SearchIntent as ResponseSearchIntent, SearchResult as ResponseSearchResult, WebSearchInfo,
    WebSearchResponse,
};
