//! Web search API module for the zai-rs crate.
//!
//! This module provides functionality to perform web searches using the Zhipu AI web search API.
//! It supports multiple search engines, intent recognition, and various filtering options.
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
//! ```rust
//! use zai_rs::tool::web_search::{WebSearchRequest, SearchEngine};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
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
//!     let response = request.send().await?;
//!     println!("Found {} results", response.result_count());
//!
//!     Ok(())
//! }
//! ```

pub mod data;
pub mod request;
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
