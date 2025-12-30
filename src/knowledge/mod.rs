//! # Knowledge Base Module
//!
//! Provides comprehensive knowledge base management capabilities for the Zhipu
//! AI API. Knowledge bases allow you to upload documents and use them as
//! context for AI conversations.
//!
//! ## Module Components
//!
//! ### Knowledge Management
//! - [`create`] - Create new knowledge bases
//! - [`list`] - List existing knowledge bases
//! - [`retrieve`] - Retrieve knowledge base details
//! - [`update`] - Update knowledge base metadata
//! - [`delete`] - Delete knowledge bases
//! - [`capacity`] - Check knowledge base capacity and usage
//!
//! ### Document Management
//! - [`document_upload_file`] - Upload documents from local files
//! - [`document_upload_url`] - Upload documents from URLs
//! - [`document_list`] - List documents in a knowledge base
//! - [`document_retrieve`] - Retrieve document details
//! - [`document_delete`] - Delete documents
//! - [`document_image_list`] - List images in documents
//! - [`document_reembedding`] - Re-embed documents
//!
//! ### Knowledge Retrieval
//! - [`retrieve`] - Retrieve relevant content from knowledge base
//!
//! - [`types`] - Shared data types
//!
//! ## Supported Operations
//!
//! ### Create Knowledge Base
//! ```rust,ignore
//! use zai_rs::knowledge::{CreateKnowledgeRequest, CreateKnowledgeBody, KnowledgeIcon};
//!
//! let body = CreateKnowledgeBody {
//!     name: "My Knowledge Base".to_string(),
//!     description: Some("Documentation for my project".to_string()),
//!     icon: KnowledgeIcon::Text,
//!     background_color: None,
//!     permission: None,
//!     embedding_id: None,
//! };
//!
//! let request = CreateKnowledgeRequest::new(body);
//! let response = client.create_knowledge(&request).await?;
//! ```
//!
//! ### Upload Document
//! ```rust,ignore
//! use zai_rs::knowledge::{DocumentUploadFileRequest, UploadFileOptions};
//! use tokio::fs::File;
//!
//! let file = File::open("document.pdf").await?;
//! let options = UploadFileOptions {
//!     chunk_size: None,
//!     slice_type: None,
//! };
//!
//! let request = DocumentUploadFileRequest::new(knowledge_id, file, options);
//! let response = client.upload_document(&request).await?;
//! ```
//!
//! ### List Documents
//! ```rust,ignore
//! use zai_rs::knowledge::{DocumentListRequest, DocumentListQuery};
//!
//! let query = DocumentListQuery {
//!     limit: Some(20),
//!     page: Some(1),
//! };
//!
//! let request = DocumentListRequest::new(knowledge_id, query);
//! let response = client.list_documents(&request).await?;
//! ```
//!
//! ### Delete Document
//! ```rust,ignore
//! use zai_rs::knowledge::{DocumentDeleteRequest, DocumentDeleteBody};
//!
//! let body = DocumentDeleteBody {
//!     document_ids: vec!["doc_123".to_string(), "doc_456".to_string()],
//! };
//!
//! let request = DocumentDeleteRequest::new(knowledge_id, body);
//! let response = client.delete_documents(&request).await?;
//! ```
//!
//! ### Retrieve Knowledge
//! ```rust,ignore
//! use zai_rs::knowledge::{KnowledgeRetrieveRequest, KnowledgeRetrieveBody};
//!
//! let body = KnowledgeRetrieveBody {
//!     question: "How do I use the API?".to_string(),
//!     top_k: Some(3),
//! };
//!
//! let request = KnowledgeRetrieveRequest::new(knowledge_id, body);
//! let response = client.retrieve_knowledge(&request).await?;
//! ```
//!
//! ## Use Cases
//!
//! - **Document Q&A**: Upload documentation and ask questions about it
//! - **Knowledge Search**: Search across multiple documents efficiently
//! - **Context Enhancement**: Provide context to AI conversations
//! - **Document Management**: Organize and manage document collections
//!
//! ## Supported Document Types
//!
//! - PDF documents
//! - Plain text files
//! - Markdown files
//! - Word documents
//! - HTML pages
//!
//! ## Knowledge Base Features
//!
//! - Automatic text extraction and segmentation
//! - Vector embedding for semantic search
//! - Re-embedding capability for updated documents
//! - Image extraction from documents
//! - Usage tracking and capacity management

pub mod capacity;
pub mod create;
pub mod delete;
pub mod document_delete;
pub mod document_image_list;
pub mod document_list;
pub mod document_reembedding;
pub mod document_retrieve;
pub mod document_upload_file;
pub mod document_upload_url;
pub mod list;
pub mod retrieve;
pub mod types;
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
