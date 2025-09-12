//! Knowledge module (flat structure)
//!
//! Provides APIs for knowledge base operations.

pub mod types;
pub mod list;
pub mod create;
pub mod retrieve;
pub mod update;
pub mod delete;
pub mod capacity;
pub mod document_list;
pub mod document_upload_url;
pub mod document_upload_file;
pub mod document_retrieve;
pub mod document_delete;
pub mod document_image_list;
pub mod document_reembedding;

pub use types::{
    KnowledgeItem,
    KnowledgeListData,
    KnowledgeListResponse,
    KnowledgeDetailResponse,
    KnowledgeUsageCounts,
    KnowledgeCapacityData,
    KnowledgeCapacityResponse,
    DocumentFailInfo,
    DocumentItem,
    DocumentDetailResponse,
    DocumentListData,
    DocumentListResponse,
    DocumentImageItem,
    DocumentImageListData,
    DocumentImageListResponse,
    UploadUrlSuccessInfo,
    UploadUrlFailedInfo,
    UploadUrlData,
    UploadUrlResponse,
    UploadFileSuccessInfo,
    UploadFileFailedInfo,
    UploadFileData,
    UploadFileResponse,
};
pub use list::{KnowledgeListQuery, KnowledgeListRequest};
pub use create::{EmbeddingId, BackgroundColor, KnowledgeIcon, CreateKnowledgeBody, CreateKnowledgeRequest, CreateKnowledgeResponse};
pub use retrieve::{KnowledgeRetrieveRequest, KnowledgeRetrieveResponse};
pub use update::{UpdateKnowledgeBody, KnowledgeUpdateRequest, KnowledgeUpdateResponse};
pub use delete::{KnowledgeDeleteRequest, KnowledgeDeleteResponse};
pub use capacity::KnowledgeCapacityRequest;
pub use document_list::{DocumentListQuery, DocumentListRequest};
pub use document_upload_url::{UploadUrlDetail, UploadUrlBody, DocumentUploadUrlRequest};
pub use document_upload_file::{DocumentSliceType, UploadFileOptions, DocumentUploadFileRequest};
pub use document_retrieve::DocumentRetrieveRequest;
pub use document_delete::{DocumentDeleteRequest, DocumentDeleteResponse};
pub use document_image_list::DocumentImageListRequest;
pub use document_reembedding::{DocumentReembeddingRequest, DocumentReembeddingBody, DocumentReembeddingResponse};
