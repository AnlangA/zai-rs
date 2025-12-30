//! # Tool Module
//!
//! Provides tool implementations for various external services and APIs
//! that can be used with AI models through function calling.
//!
//! ## Available Tools
//!
//! ### File Parsing Tools
//! - [`file_parser_create`] - Create file parsing tasks for document analysis
//! - [`file_parser_result`] - Retrieve results from file parsing operations
//!
//! ### Web Search Tools
//! - [`web_search`] - Web search capabilities for retrieving current
//!   information
//!
//! ## Tool Registration
//!
//! Tools can be registered with the
//! [`ToolExecutor`](crate::toolkits::ToolExecutor) for use in AI conversations:
//!
//! ```rust,ignore
//! use zai_rs::toolkits::{ToolExecutor, ToolMetadata};
//! use zai_rs::tool::web_search::{WebSearchRequest, WebSearchTool};
//!
//! let mut executor = ToolExecutor::new();
//! executor.register_tool(WebSearchTool::new())?;
//! ```
//!
//! ## Tool Implementation Pattern
//!
//! Each tool implements the [`DynTool`](crate::toolkits::core::DynTool) trait,
//! providing:
//! - Metadata (name, description, parameters)
//! - Schema generation for LLM integration
//! - Async execution handler
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use zai_rs::tool::web_search::WebSearchRequest;
//! use zai_rs::client::ZaiClient;
//!
//! let client = ZaiClient::new(api_key);
//! let request = WebSearchRequest::new("Rust programming language")
//!     .num_results(5)
//!     .build()?;
//!
//! let results = client.web_search(&request).await?;
//! ```

pub mod file_parser_create;
pub mod file_parser_result;
pub mod web_search;

// File Parser Create
pub use file_parser_create::{
    data::FileParserCreateRequest,
    request::{FileType, ToolType},
    response::FileParserCreateResponse,
};
// File Parser Result
pub use file_parser_result::data::FileParserResultRequest;
pub use file_parser_result::{
    request::FormatType,
    response::{FileParserResultResponse, ParserStatus},
};
// Web Search
pub use web_search::data::WebSearchRequest;
pub use web_search::{
    request::{
        ContentSize, SearchEngine, SearchIntent as WebSearchRequestBodyIntent, SearchRecencyFilter,
        WebSearchBody,
    },
    response::{
        SearchIntent as WebSearchIntent, SearchResult as WebSearchResult, WebSearchInfo,
        WebSearchResponse,
    },
};
