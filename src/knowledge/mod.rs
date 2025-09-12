//! Knowledge module (flat structure)
//!
//! Provides APIs for knowledge base operations.

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
