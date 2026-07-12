//! # LLM-Application service (plan P06).
//!
//! Seven application endpoints spanning three API families:
//!
//! | Endpoint | Method | Family | Path |
//! |----------|--------|--------|------|
//! | `file_stats` | POST | `ApplicationV2` | `v2/application/file_stat` |
//! | `file_upload` | POST multipart | `ApplicationV2` | `v2/application/file_upload` |
//! | `slice_info` | POST | `ApplicationV2` | `v2/application/slice_info` |
//! | `conversation_create` | POST | `ApplicationV2` | `v2/application/{app_id}/conversation` |
//! | `variables` | GET | `ApplicationV2` | `v2/application/{app_id}/variables` |
//! | `history` | GET | `LlmApplication` | `history_session_record/{app_id}/{conversation_id}` |
//! | `invoke` | POST | `ApplicationV3` | `v3/application/invoke` |
//!
//! Each request type follows the established P05 `send_via(&ZaiClient)` pattern.

mod request;
mod response;

pub use request::*;
pub use response::*;
