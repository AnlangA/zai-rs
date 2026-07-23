//! Typed responses shared by asynchronous task-submission and polling APIs.

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::{
    ZaiResult,
    client::error::{ZaiError, codes},
    model::chat_base_response::{
        Choice, ContentFilterInfo, TaskStatus, Usage, VideoResultItem, WebSearchInfo,
    },
};

fn invalid_response(message: impl Into<String>) -> ZaiError {
    ZaiError::ApiError {
        code: codes::SDK_VALIDATION,
        message: message.into(),
    }
}

/// Acknowledgement returned after an asynchronous task is accepted.
///
/// Every field is optional in the upstream schema. A successful operation must
/// nevertheless return at least one documented non-null field; call
/// [`validate`](Self::validate) after deserializing an untrusted value.
#[derive(Debug, Clone, Serialize)]
pub struct AsyncResponse {
    /// Model that accepted the task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Task identifier used by the async-result endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Caller-supplied or generated request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Current task status, when returned at submission time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_status: Option<TaskStatus>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AsyncResponseWire {
    model: Option<String>,
    id: Option<String>,
    request_id: Option<String>,
    task_status: Option<TaskStatus>,
}

impl<'de> Deserialize<'de> for AsyncResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AsyncResponseWire::deserialize(deserializer)?;
        let response = Self {
            model: wire.model,
            id: wire.id,
            request_id: wire.request_id,
            task_status: wire.task_status,
        };
        response
            .validate()
            .map_err(|error| D::Error::custom(error.to_string()))?;
        Ok(response)
    }
}

impl AsyncResponse {
    /// Enforce the operation contract's non-empty response invariant.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.model.is_some()
            || self.id.is_some()
            || self.request_id.is_some()
            || self.task_status.is_some()
        {
            return Ok(());
        }
        Err(invalid_response(
            "async task response contained no documented fields",
        ))
    }

    /// Borrow the task identifier.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Borrow the request identifier.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    /// Borrow the model identifier.
    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    /// Borrow the current task status.
    pub const fn status(&self) -> Option<&TaskStatus> {
        self.task_status.as_ref()
    }
}

/// Descriptive alias for the acknowledgement returned by task submissions.
pub type AsyncTaskResponse = AsyncResponse;

/// A completed asynchronous chat-completion result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsyncChatTaskResult {
    /// Task identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Creation time as a Unix timestamp in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
    /// Current task status, when returned with the completion payload.
    /// One of `PROCESSING` / `SUCCESS` / `FAIL`; a successful completion
    /// typically carries `SUCCESS` alongside `choices` and `usage`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_status: Option<TaskStatus>,
    /// Model that generated the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Generated choices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<Choice>>,
    /// Token usage statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    /// Web-search sources used by the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search: Option<Vec<WebSearchInfo>>,
    /// Content-safety classifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_filter: Option<Vec<ContentFilterInfo>>,
}

impl AsyncChatTaskResult {
    /// Borrow the generated choices.
    pub fn choices(&self) -> Option<&[Choice]> {
        self.choices.as_deref()
    }

    /// Borrow the current task status.
    pub const fn status(&self) -> Option<&TaskStatus> {
        self.task_status.as_ref()
    }

    fn has_documented_value(&self) -> bool {
        self.id.is_some()
            || self.request_id.is_some()
            || self.created.is_some()
            || self.task_status.is_some()
            || self.model.is_some()
            || self.choices.is_some()
            || self.usage.is_some()
            || self.web_search.is_some()
            || self.content_filter.is_some()
    }
}

/// A completed asynchronous video-generation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsyncVideoTaskResult {
    /// Model that generated the video.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Current task status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_status: Option<TaskStatus>,
    /// Generated videos.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_result: Option<Vec<VideoResultItem>>,
    /// Request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl AsyncVideoTaskResult {
    /// Borrow the current task status.
    pub const fn status(&self) -> Option<&TaskStatus> {
        self.task_status.as_ref()
    }

    /// Borrow the generated video items.
    pub fn videos(&self) -> Option<&[VideoResultItem]> {
        self.video_result.as_deref()
    }

    fn has_documented_value(&self) -> bool {
        self.model.is_some()
            || self.task_status.is_some()
            || self.video_result.is_some()
            || self.request_id.is_some()
    }
}

/// One generated image returned by an asynchronous image task.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsyncImageResultItem {
    /// URL of the generated image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl AsyncImageResultItem {
    /// Borrow the generated image URL.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

/// A completed asynchronous image-generation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsyncImageTaskResult {
    /// Model that generated the image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Current task status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_status: Option<TaskStatus>,
    /// Generated images.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_result: Option<Vec<AsyncImageResultItem>>,
    /// Request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl AsyncImageTaskResult {
    /// Borrow the current task status.
    pub const fn status(&self) -> Option<&TaskStatus> {
        self.task_status.as_ref()
    }

    /// Borrow the generated image items.
    pub fn images(&self) -> Option<&[AsyncImageResultItem]> {
        self.image_result.as_deref()
    }

    fn has_documented_value(&self) -> bool {
        self.model.is_some()
            || self.task_status.is_some()
            || self.image_result.is_some()
            || self.request_id.is_some()
    }
}

/// Status-only task result used while a task is pending or after it fails.
///
/// Chat, image, and video tasks share this wire shape, so it cannot be assigned
/// to one media-specific variant until a result field is present.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AsyncTaskState {
    /// Task identifier, when supplied by the service.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Model handling the task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Creation time as a Unix timestamp in seconds, when supplied by the
    /// service while a task is still processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,
    /// Current task status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_status: Option<TaskStatus>,
}

impl AsyncTaskState {
    /// Borrow the current task status.
    pub const fn status(&self) -> Option<&TaskStatus> {
        self.task_status.as_ref()
    }

    /// Return whether the task is still processing.
    pub fn is_processing(&self) -> bool {
        matches!(self.task_status, Some(TaskStatus::Processing))
    }

    /// Return whether the task completed successfully without a typed payload.
    pub fn is_success(&self) -> bool {
        matches!(self.task_status, Some(TaskStatus::Success))
    }

    /// Return whether the task failed.
    pub fn is_failed(&self) -> bool {
        matches!(self.task_status, Some(TaskStatus::Fail))
    }

    fn has_documented_value(&self) -> bool {
        self.id.is_some()
            || self.model.is_some()
            || self.request_id.is_some()
            || self.created.is_some()
            || self.task_status.is_some()
    }
}

/// Closed set of payloads returned by the asynchronous result endpoint.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum AsyncTaskResult {
    /// Completed chat-completion payload.
    Chat(AsyncChatTaskResult),
    /// Completed video-generation payload.
    Video(AsyncVideoTaskResult),
    /// Completed image-generation payload.
    Image(AsyncImageTaskResult),
    /// Shared status-only payload for pending, failed, or result-less tasks.
    State(AsyncTaskState),
}

impl AsyncTaskResult {
    fn has_documented_value(&self) -> bool {
        match self {
            Self::Chat(result) => result.has_documented_value(),
            Self::Video(result) => result.has_documented_value(),
            Self::Image(result) => result.has_documented_value(),
            Self::State(result) => result.has_documented_value(),
        }
    }

    /// Borrow the status supplied by any task-result payload.
    pub const fn status(&self) -> Option<&TaskStatus> {
        match self {
            Self::Chat(result) => result.status(),
            Self::Video(result) => result.task_status.as_ref(),
            Self::Image(result) => result.task_status.as_ref(),
            Self::State(result) => result.task_status.as_ref(),
        }
    }

    /// Borrow the chat payload when this is a completed chat task.
    pub const fn as_chat(&self) -> Option<&AsyncChatTaskResult> {
        match self {
            Self::Chat(result) => Some(result),
            _ => None,
        }
    }

    /// Borrow the video payload when this is a completed video task.
    pub const fn as_video(&self) -> Option<&AsyncVideoTaskResult> {
        match self {
            Self::Video(result) => Some(result),
            _ => None,
        }
    }

    /// Borrow the image payload when this is a completed image task.
    pub const fn as_image(&self) -> Option<&AsyncImageTaskResult> {
        match self {
            Self::Image(result) => Some(result),
            _ => None,
        }
    }

    /// Borrow the status-only payload.
    pub const fn as_state(&self) -> Option<&AsyncTaskState> {
        match self {
            Self::State(result) => Some(result),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for AsyncTaskResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("async task result must be a JSON object"))?;

        let result = if object.contains_key("image_result") {
            Self::Image(
                serde_json::from_value(value)
                    .map_err(|error| D::Error::custom(error.to_string()))?,
            )
        } else if object.contains_key("video_result") {
            Self::Video(
                serde_json::from_value(value)
                    .map_err(|error| D::Error::custom(error.to_string()))?,
            )
        } else if ["choices", "usage", "web_search", "content_filter"]
            .iter()
            .any(|field| object.contains_key(*field))
            || (object.contains_key("id")
                && !object.contains_key("task_status")
                && !object.contains_key("model")
                && !object.contains_key("request_id"))
        {
            Self::Chat(
                serde_json::from_value(value)
                    .map_err(|error| D::Error::custom(error.to_string()))?,
            )
        } else if ["id", "model", "request_id", "created", "task_status"]
            .iter()
            .any(|field| object.get(*field).is_some_and(|value| !value.is_null()))
        {
            Self::State(
                serde_json::from_value(value)
                    .map_err(|error| D::Error::custom(error.to_string()))?,
            )
        } else {
            return Err(D::Error::custom(
                "async task result contained no documented non-null fields",
            ));
        };

        if !result.has_documented_value() {
            return Err(D::Error::custom(
                "async task result contained no documented non-null fields",
            ));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_acknowledgement_requires_a_documented_value() {
        assert!(serde_json::from_str::<AsyncResponse>("{}").is_err());
        assert!(serde_json::from_str::<AsyncResponse>(r#"{"id":null}"#).is_err());

        let response: AsyncResponse = serde_json::from_value(serde_json::json!({
            "id": "task-1",
            "task_status": "PROCESSING"
        }))
        .unwrap();
        assert_eq!(response.id(), Some("task-1"));
        assert!(matches!(response.status(), Some(TaskStatus::Processing)));
        assert!(response.validate().is_ok());
    }

    #[test]
    fn task_result_rejects_empty_and_unknown_payloads() {
        assert!(serde_json::from_str::<AsyncTaskResult>("{}").is_err());
        assert!(serde_json::from_str::<AsyncTaskResult>(r#"{"task_id":"legacy"}"#).is_err());
        assert!(serde_json::from_str::<AsyncTaskResult>(r#"{"choices":null}"#).is_err());
        assert!(serde_json::from_str::<AsyncTaskResult>(r#"{"video_result":null}"#).is_err());
        assert!(serde_json::from_str::<AsyncTaskResult>(r#"{"image_result":null}"#).is_err());
    }

    #[test]
    fn task_result_selects_each_closed_variant() {
        let chat: AsyncTaskResult = serde_json::from_value(serde_json::json!({
            "id": "chat-1",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "done"}}]
        }))
        .unwrap();
        assert_eq!(
            chat.as_chat()
                .and_then(|value| value.choices())
                .map(<[_]>::len),
            Some(1)
        );

        let video: AsyncTaskResult = serde_json::from_value(serde_json::json!({
            "model": "cogvideox-3",
            "task_status": "SUCCESS",
            "video_result": [{"url": "https://example.com/video.mp4"}]
        }))
        .unwrap();
        assert_eq!(
            video
                .as_video()
                .and_then(|value| value.videos())
                .map(<[_]>::len),
            Some(1)
        );

        let image: AsyncTaskResult = serde_json::from_value(serde_json::json!({
            "model": "glm-image",
            "task_status": "SUCCESS",
            "image_result": [{"url": "https://example.com/image.png"}]
        }))
        .unwrap();
        assert_eq!(
            image
                .as_image()
                .and_then(|value| value.images())
                .map(<[_]>::len),
            Some(1)
        );

        let state: AsyncTaskResult = serde_json::from_value(serde_json::json!({
            "request_id": "request-1",
            "task_status": "PROCESSING"
        }))
        .unwrap();
        assert!(state.as_state().is_some_and(AsyncTaskState::is_processing));
    }

    /// The async-result endpoint returns `task_status` while a chat task is
    /// still processing, and alongside `choices`/`usage` once it succeeds.
    /// Both shapes must deserialize; the processing payload routes to the
    /// shared `State` variant and the success payload to `Chat`. Regression
    /// test for the "unknown field `task_status`" failure seen while polling.
    #[test]
    fn task_result_handles_chat_task_status_at_every_stage() {
        // Processing chat task: the polling endpoint echoes the creation
        // timestamp and task_status but no completion payload.
        let processing: AsyncTaskResult = serde_json::from_value(serde_json::json!({
            "id": "task-1",
            "model": "glm-5.2",
            "created": 1_700_000_000_u64,
            "request_id": "request-1",
            "task_status": "PROCESSING"
        }))
        .unwrap();
        let processing_state = processing
            .as_state()
            .expect("processing chat task must route to the State variant");
        assert!(processing_state.is_processing());

        // Completed chat task: the success payload now carries task_status.
        let done: AsyncTaskResult = serde_json::from_value(serde_json::json!({
            "id": "task-1",
            "model": "glm-5.2",
            "created": 1_700_000_000_u64,
            "request_id": "request-1",
            "task_status": "SUCCESS",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "done"}}]
        }))
        .unwrap();
        let chat = done
            .as_chat()
            .expect("completed chat task must route to the Chat variant");
        assert_eq!(
            chat.status(),
            Some(&TaskStatus::Success),
            "completed chat result should expose its task_status"
        );
        assert_eq!(
            done.status(),
            Some(&TaskStatus::Success),
            "the unified result should preserve completed chat status"
        );
        assert_eq!(
            chat.choices().map(<[_]>::len),
            Some(1),
            "completed chat result should expose its choices"
        );
    }
}
