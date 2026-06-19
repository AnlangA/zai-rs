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
//! | Create | [`create`] | Create a new knowledge base |
//! | List | [`list`] | List knowledge bases |
//! | Retrieve | [`retrieve`] | Get knowledge-base details / search |
//! | Update | [`update`] | Update metadata |
//! | Delete | [`delete`] | Delete a knowledge base |
//! | Capacity | [`capacity`] | Check usage / quota |
//!
//! # Document Operations
//!
//! | Operation | Module | Description |
//! |-----------|--------|-------------|
//! | Upload (file) | [`document_upload_file`] | Upload a local file |
//! | Upload (URL) | [`document_upload_url`] | Upload from a URL |
//! | List | [`document_list`] | List documents in a KB |
//! | Retrieve | [`document_retrieve`] | Get document details |
//! | Delete | [`document_delete`] | Delete documents |
//! | Re-embed | [`document_reembedding`] | Re-run vectorisation |
//! | Images | [`document_image_list`] | List extracted images |
//!
//! # Supported Document Types
//!
//! PDF, plain text, Markdown, Word, HTML, and more.
//!
//! # Usage
//!
//! ```rust,ignore
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
pub mod capacity;
/// Create a new knowledge base.
pub mod create;
/// Delete a knowledge base.
pub mod delete;
/// Delete documents from a knowledge base.
pub mod document_delete;
/// List images extracted from a document.
pub mod document_image_list;
/// List documents in a knowledge base.
pub mod document_list;
/// Re-run vectorisation (embedding) for documents.
pub mod document_reembedding;
/// Retrieve document details.
pub mod document_retrieve;
/// Upload a local file as a document.
pub mod document_upload_file;
/// Upload a document from a URL.
pub mod document_upload_url;
/// List knowledge bases.
pub mod list;
/// Retrieve knowledge-base details / semantic search.
pub mod retrieve;
/// Shared knowledge-base data types.
pub mod types;
/// Update knowledge-base metadata.
pub mod update;

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
pub use types::{
    DocumentDetailResponse, DocumentFailInfo, DocumentImageItem, DocumentImageListData,
    DocumentImageListResponse, DocumentItem, DocumentListData, DocumentListResponse,
    KnowledgeCapacityData, KnowledgeCapacityResponse, KnowledgeDetailResponse, KnowledgeItem,
    KnowledgeListData, KnowledgeListResponse, KnowledgeUsageCounts, UploadFileData,
    UploadFileFailedInfo, UploadFileResponse, UploadFileSuccessInfo, UploadUrlData,
    UploadUrlFailedInfo, UploadUrlResponse, UploadUrlSuccessInfo,
};
pub use update::{KnowledgeUpdateRequest, KnowledgeUpdateResponse, UpdateKnowledgeBody};
