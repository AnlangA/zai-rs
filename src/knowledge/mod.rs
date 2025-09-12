//! Knowledge module (flat structure)
//!
//! Provides APIs for knowledge base operations.

pub mod types;
pub mod list;
pub mod create;
pub mod retrieve;

pub use types::{KnowledgeItem, KnowledgeListData, KnowledgeListResponse, KnowledgeDetailResponse};
pub use list::{KnowledgeListQuery, KnowledgeListRequest};
pub use create::{EmbeddingId, BackgroundColor, KnowledgeIcon, CreateKnowledgeBody, CreateKnowledgeRequest, CreateKnowledgeResponse};
pub use retrieve::{KnowledgeRetrieveRequest, KnowledgeRetrieveResponse};
