//! # Tool Module
//!
//! Clients and wire types for Zhipu's standalone tool APIs.
//!
//! # Available Tools
//!
//! - [`web_search`] — Live web search for retrieving current information
//! - [`file_parser_create`] — Create file-parsing tasks for document analysis
//! - [`file_parser_result`] — Retrieve results from file-parsing operations
//!
//! These request builders call their corresponding HTTP endpoints directly;
//! they are distinct from user-defined
//! [dynamic tools](crate::toolkits::core::DynTool) registered with a
//! [tool executor](crate::toolkits::ToolExecutor).

pub mod file_parser_create;
pub mod file_parser_result;
pub mod web_search;

// File Parser Create
pub use file_parser_create::{FileParseRequest, FileParserCreateResponse, FileType, ToolType};
// File Parser Result
pub use file_parser_result::{
    FileParseResultRequest, FileParseResultResponse, FormatType, ParserStatus,
};
// Web Search
pub use web_search::{
    ContentSize, SearchEngine, SearchIntent as WebSearchIntent, SearchIntentKind,
    SearchRecencyFilter, SearchResult as WebSearchResult, WebSearchBody, WebSearchRequest,
    WebSearchResponse,
};
