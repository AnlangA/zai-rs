use std::path::Path;

use serde::Serialize;
use validator::Validate;

use super::{super::traits::*, request::AudioToTextBody};
use crate::client::ZaiClient;
use crate::client::error::codes;

/// Audio transcription request encoded as `multipart/form-data`.
///
/// Builder for the audio-transcription (ASR) endpoint. Set the audio file via
/// [`AudioToTextRequest::with_file_path`], then call
/// [`AudioToTextRequest::send_via`].
pub struct AudioToTextRequest<N>
where
    N: ModelName + AudioToText + Serialize,
{
    /// Multipart form fields other than the audio file itself.
    pub body: AudioToTextBody<N>,
    file_path: Option<String>,
}

impl<N> AudioToTextRequest<N>
where
    N: ModelName + AudioToText + Serialize + Clone,
{
    /// Create a new transcription request for the given ASR model.
    pub fn new(model: N) -> Self {
        Self {
            body: AudioToTextBody::new(model),
            file_path: None,
        }
    }

    /// Set the local audio file path to transcribe (required).
    pub fn with_file_path(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Set an optional prompt (advisory 8000-char limit; not hard-rejected).
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.body = self.body.with_prompt(prompt);
        self
    }

    /// Set the hot-word list (at most 100 entries).
    pub fn with_hotwords(mut self, hotwords: Vec<String>) -> crate::ZaiResult<Self> {
        self.body = self.body.with_hotwords(hotwords)?;
        Ok(self)
    }

    /// Enable/disable streaming responses.
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.body = self.body.with_stream(stream);
        self
    }

    /// Set the client-side request id.
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }

    /// Set the end-user id.
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }

    /// Validate body constraints and that the supplied file path exists.
    pub fn validate(&self) -> crate::ZaiResult<()> {
        // Check body constraints
        self.body
            .validate()
            .map_err(crate::client::error::ZaiError::from)?;
        // Ensure file path exists
        let p =
            self.file_path
                .as_ref()
                .ok_or_else(|| crate::client::error::ZaiError::ApiError {
                    code: crate::client::error::codes::SDK_VALIDATION,
                    message: "file_path is required".to_string(),
                })?;

        if !Path::new(p).exists() {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_NOT_FOUND,
                message: format!("file_path not found: {p}"),
            });
        }

        Ok(())
    }

    /// Submit the multipart request via a [`ZaiClient`] and parse the typed
    /// transcription response.
    pub async fn send_via(
        &self,
        client: &ZaiClient,
    ) -> crate::ZaiResult<super::response::AudioToTextResponse>
    where
        N: Clone + Send + Sync + 'static,
    {
        self.validate()?;

        let route = crate::client::routes::AUDIO_TRANSCRIBE;
        let url = client.endpoints().resolve_route(route, &[])?;
        let file_path =
            self.file_path
                .clone()
                .ok_or_else(|| crate::client::error::ZaiError::ApiError {
                    code: crate::client::error::codes::SDK_VALIDATION,
                    message: "file_path is required".to_string(),
                })?;

        let file_name = Path::new(&file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("audio.wav")
            .to_string();
        let file_bytes = tokio::fs::read(&file_path).await?;

        // Basic MIME guess by extension
        let mime = if file_name.to_ascii_lowercase().ends_with(".mp3") {
            "audio/mpeg"
        } else {
            "audio/wav"
        };

        let prompt = self.body.prompt.clone();
        let hotwords = self.body.hotwords.clone();
        let stream = self.body.stream;
        let request_id = self.body.request_id.clone();
        let user_id = self.body.user_id.clone();
        let model_name: String = self.body.model.clone().into();

        let mut factory = crate::client::transport::multipart::MultipartBodyFactory::new()
            .field("model", model_name)?
            .bytes_named("file", file_name, mime, file_bytes)?;
        if let Some(p) = prompt {
            factory = factory.field("prompt", p)?;
        }
        if !hotwords.is_empty() {
            factory = factory.field("hotwords", hotwords.join(","))?;
        }
        if let Some(s) = stream {
            factory = factory.field("stream", s.to_string())?;
        }
        if let Some(rid) = request_id {
            factory = factory.field("request_id", rid)?;
        }
        if let Some(uid) = user_id {
            factory = factory.field("user_id", uid)?;
        }
        client
            .send_multipart::<super::response::AudioToTextResponse>(route.method(), url, &factory)
            .await
    }
}
