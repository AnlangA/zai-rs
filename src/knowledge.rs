//! # Knowledge Base Module
//!
//! Provides knowledge-base management for the Zhipu AI API: create, update,
//! delete knowledge bases; upload, list, and retrieve documents; perform
//! semantic search.
//!
//! # Knowledge-Base Operations
//!
//! | Operation | Module | Description |
//! |-----------|--------|-------------|
//! | Create | [`KnowledgeCreateRequest`] | Create a new knowledge base |
//! | List | [`KnowledgeListRequest`] | List knowledge bases |
//! | Retrieve | [`KnowledgeGetRequest`] | Get knowledge-base details |
//! | Search | [`KnowledgeSearchRequest`] | Run semantic retrieval against a knowledge base |
//! | Update | [`KnowledgeUpdateRequest`] | Update metadata |
//! | Delete | [`KnowledgeDeleteRequest`] | Delete a knowledge base |
//! | Capacity | [`KnowledgeCapacityRequest`] | Check usage / quota |
//!
//! # Document Operations
//!
//! | Operation | Module | Description |
//! |-----------|--------|-------------|
//! | Upload (file) | [`DocumentUploadRequest`] | Upload a local file |
//! | Upload (URL) | [`DocumentUrlUploadRequest`] | Upload from a URL |
//! | List | [`DocumentListRequest`] | List documents in a KB |
//! | Retrieve | [`DocumentGetRequest`] | Get document details |
//! | Delete | [`DocumentDeleteRequest`] | Delete documents |
//! | Re-embed | [`DocumentReembedRequest`] | Re-run vectorisation |
//! | Images | [`DocumentImageListRequest`] | List extracted images |
//!
//! # Supported Document Types
//!
//! PDF, plain text, Markdown, Word, HTML, and more.
//!
//! # Usage
//!
//! ```rust,no_run
//! use zai_rs::{ZaiResult, client::ZaiClient, knowledge::*};
//!
//! # async fn example(client: &ZaiClient) -> ZaiResult<()> {
//! let created = KnowledgeCreateRequest::new(EmbeddingId::Embedding3New, "Product docs")
//!     .send_via(client)
//!     .await?;
//! let uploaded = DocumentUploadRequest::new("knowledge-base-id")
//!     .add_file_path("guide.pdf")
//!     .send_via(client)
//!     .await?;
//! let matches = KnowledgeSearchRequest::new("knowledge-base-id", "How do I authenticate?")
//!     .with_top_k(5)
//!     .send_via(client)
//!     .await?;
//! # let _ = (created, uploaded, matches);
//! # Ok(())
//! # }
//! ```

/// Knowledge-base capacity / quota query.
mod capacity;
/// Create a new knowledge base.
mod create;
/// Delete a knowledge base.
mod delete;
/// Delete documents from a knowledge base.
mod document_delete;
/// List images extracted from a document.
mod document_image_list;
/// List documents in a knowledge base.
mod document_list;
/// Re-run vectorisation (embedding) for documents.
mod document_reembedding;
/// Retrieve document details.
mod document_retrieve;
/// Upload a local file as a document.
mod document_upload_file;
/// Upload a document from a URL.
mod document_upload_url;
/// List knowledge bases.
mod list;
/// Retrieve knowledge-base details / semantic search.
mod retrieve;
/// Search a knowledge base (`POST …/knowledge/retrieve`).
mod search;
/// Shared knowledge-base data types.
mod types;
/// Update knowledge-base metadata.
mod update;

pub use capacity::KnowledgeCapacityRequest;
pub use create::{
    BackgroundColor, EmbeddingId, KnowledgeCreateBody, KnowledgeCreateData, KnowledgeCreateRequest,
    KnowledgeCreateResponse, KnowledgeIcon,
};
pub use delete::{KnowledgeDeleteRequest, KnowledgeDeleteResponse};
pub use document_delete::{DocumentDeleteRequest, DocumentDeleteResponse};
pub use document_image_list::DocumentImageListRequest;
pub use document_list::{DocumentListQuery, DocumentListRequest};
pub use document_reembedding::{
    DocumentReembedBody, DocumentReembedRequest, DocumentReembedResponse,
};
pub use document_retrieve::DocumentGetRequest;
pub use document_upload_file::{DocumentSliceType, DocumentUploadOptions, DocumentUploadRequest};
pub use document_upload_url::{
    DocumentUrlUploadBody, DocumentUrlUploadDetail, DocumentUrlUploadRequest,
};
pub use list::{KnowledgeListQuery, KnowledgeListRequest};
pub use retrieve::KnowledgeGetRequest;
pub use search::{
    KnowledgeRecallMethod, KnowledgeRerankModel, KnowledgeSearchBody, KnowledgeSearchMetadata,
    KnowledgeSearchRequest, KnowledgeSearchResponse, KnowledgeSearchResult,
};
pub use types::{
    DocumentFailInfo, DocumentGetResponse, DocumentImageItem, DocumentImageListData,
    DocumentImageListResponse, DocumentItem, DocumentListData, DocumentListResponse,
    DocumentUploadData, DocumentUploadFailedInfo, DocumentUploadResponse,
    DocumentUploadSuccessInfo, DocumentUrlUploadData, DocumentUrlUploadFailedInfo,
    DocumentUrlUploadResponse, DocumentUrlUploadSuccessInfo, KnowledgeCapacityData,
    KnowledgeCapacityResponse, KnowledgeGetResponse, KnowledgeItem, KnowledgeListData,
    KnowledgeListResponse, KnowledgeOperationResponse, KnowledgeResponse, KnowledgeUsageCounts,
};
pub use update::{KnowledgeUpdateBody, KnowledgeUpdateRequest, KnowledgeUpdateResponse};
