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
//! | Create | [`CreateKnowledgeRequest`] | Create a new knowledge base |
//! | List | [`KnowledgeListRequest`] | List knowledge bases |
//! | Retrieve | [`KnowledgeRetrieveRequest`] | Get knowledge-base details / search |
//! | Update | [`KnowledgeUpdateRequest`] | Update metadata |
//! | Delete | [`KnowledgeDeleteRequest`] | Delete a knowledge base |
//! | Capacity | [`KnowledgeCapacityRequest`] | Check usage / quota |
//!
//! # Document Operations
//!
//! | Operation | Module | Description |
//! |-----------|--------|-------------|
//! | Upload (file) | [`DocumentUploadFileRequest`] | Upload a local file |
//! | Upload (URL) | [`DocumentUploadUrlRequest`] | Upload from a URL |
//! | List | [`DocumentListRequest`] | List documents in a KB |
//! | Retrieve | [`DocumentRetrieveRequest`] | Get document details |
//! | Delete | [`DocumentDeleteRequest`] | Delete documents |
//! | Re-embed | [`DocumentReembeddingRequest`] | Re-run vectorisation |
//! | Images | [`DocumentImageListRequest`] | List extracted images |
//!
//! # Supported Document Types
//!
//! PDF, plain text, Markdown, Word, HTML, and more.
//!
//! # Usage
//!
//! ```text
//! use zai_rs::knowledge::*;
//!
//! // Create a knowledge base
//! let kb = client.create_knowledge(&CreateKnowledgeRequest::new(body)).await?;
//!
//! // Upload a document
//! let doc = client.upload_document(&DocumentUploadFileRequest::new(kb_id, file, opts)).await?;
//!
//! // Semantic search
//! let results = client.retrieve_knowledge(&KnowledgeRetrieveRequest::new(kb_id, query)).await?;
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
/// Search knowledge-base (POST /knowledge/retrieve, P06).
mod search;
/// Shared knowledge-base data types.
mod types;
/// Update knowledge-base metadata.
mod update;

pub use capacity::KnowledgeCapacityRequest;
pub use create::{
    BackgroundColor, CreateKnowledgeBody, CreateKnowledgeRequest, CreateKnowledgeResponse,
    EmbeddingId, KnowledgeIcon,
};
pub use delete::{KnowledgeDeleteRequest, KnowledgeDeleteResponse};
pub use document_delete::{DocumentDeleteRequest, DocumentDeleteResponse};
pub use document_image_list::DocumentImageListRequest;
pub use document_list::{DocumentListQuery, DocumentListRequest};
pub use document_reembedding::{
    DocumentReembeddingBody, DocumentReembeddingRequest, DocumentReembeddingResponse,
};
pub use document_retrieve::DocumentRetrieveRequest;
pub use document_upload_file::{DocumentSliceType, DocumentUploadFileRequest, UploadFileOptions};
pub use document_upload_url::{DocumentUploadUrlRequest, UploadUrlBody, UploadUrlDetail};
pub use list::{KnowledgeListQuery, KnowledgeListRequest};
pub use retrieve::{KnowledgeRetrieveRequest, KnowledgeRetrieveResponse};
pub use search::{KnowledgeSearchBody, KnowledgeSearchRequest, KnowledgeSearchResponse};
pub use types::{
    DocumentDetailResponse, DocumentFailInfo, DocumentImageItem, DocumentImageListData,
    DocumentImageListResponse, DocumentItem, DocumentListData, DocumentListResponse,
    KnowledgeCapacityData, KnowledgeCapacityResponse, KnowledgeDetailResponse, KnowledgeItem,
    KnowledgeListData, KnowledgeListResponse, KnowledgeUsageCounts, UploadFileData,
    UploadFileFailedInfo, UploadFileResponse, UploadFileSuccessInfo, UploadUrlData,
    UploadUrlFailedInfo, UploadUrlResponse, UploadUrlSuccessInfo,
};
pub use update::{KnowledgeUpdateRequest, KnowledgeUpdateResponse, UpdateKnowledgeBody};
