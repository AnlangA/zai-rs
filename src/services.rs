//! Request and response types for service-oriented Z.AI endpoints.

/// LLM-application file, conversation, variable, history, and invocation APIs.
pub mod applications;
/// Assistant invocation, listing, and conversation-list APIs.
pub mod assistants;
/// Asynchronous image-generation APIs.
pub mod images;
/// Document layout-parsing and reader APIs.
pub mod tools;
