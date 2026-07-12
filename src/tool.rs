//! # Tool Module
//!
//! Provides tool implementations that can be used with AI models through
//! function calling.
//!
//! # Available Tools
//!
//! - [`web_search`] — Live web search for retrieving current information
//! - [`file_parser_create`] — Create file-parsing tasks for document analysis
//! - [`file_parser_result`] — Retrieve results from file-parsing operations
//!
//! # Tool Registration
//!
//! Tools implement the [`DynTool`](crate::toolkits::core::DynTool) trait and
//! can be registered with the [`ToolExecutor`](crate::toolkits::ToolExecutor):
//!
//! ```text
//! use zai_rs::toolkits::ToolExecutor;
//! use zai_rs::tool::web_search::WebSearchTool;
//!
//! let mut executor = ToolExecutor::new();
//! executor.register_tool(Box::new(WebSearchTool::new()))?;
//! ```

pub mod file_parser_create;
pub mod file_parser_result;
pub mod web_search;

// File Parser Create
pub use file_parser_create::{
    FileParserCreateRequest, FileParserCreateResponse, FileType, ToolType,
};
// File Parser Result
pub use file_parser_result::{
    FileParserResultRequest, FileParserResultResponse, FormatType, ParserStatus,
};
// Web Search
pub use web_search::{
    ContentSize, ResponseSearchIntent as WebSearchIntent, ResponseSearchResult as WebSearchResult,
    SearchEngine, SearchIntent as WebSearchRequestBodyIntent, SearchRecencyFilter, WebSearchBody,
    WebSearchInfo, WebSearchRequest, WebSearchResponse,
};
