use std::{path::Path, sync::Arc};

use serde::Serialize;
use validator::Validate;

use super::{super::traits::*, request::AudioToTextBody};
use crate::client::{
    endpoints::{ApiBase, EndpointConfig, paths},
    error::codes,
    http::{HttpClient, HttpClientConfig, parse_typed_response, send_multipart_request},
};

/// Audio transcription request (multipart/form-data)
///
/// Builder for the audio-transcription (ASR) endpoint. Set the audio file via
/// [`AudioToTextRequest::with_file_path`], then call
/// [`AudioToTextRequest::send`].
pub struct AudioToTextRequest<N>
where
    N: ModelName + AudioToText + Serialize,
{
    /// Zhipu AI API key used for `Authorization: Bearer …`.
    pub key: String,
    url: String,
    endpoint_config: EndpointConfig,
    api_base: ApiBase,
    http_config: Arc<HttpClientConfig>,
    /// Multipart form fields (model, temperature, …).
    pub body: AudioToTextBody<N>,
    file_path: Option<String>,
}

impl<N> AudioToTextRequest<N>
where
    N: ModelName + AudioToText + Serialize + Clone,
{
    /// Create a new transcription request for the given ASR model.
    pub fn new(model: N, key: String) -> Self {
        let endpoint_config = EndpointConfig::default();
        let api_base = ApiBase::PaasV4;
        let url = endpoint_config.url(&api_base, paths::AUDIO_TRANSCRIPTIONS);
        Self {
            key,
            url,
            endpoint_config,
            api_base,
            http_config: Arc::new(HttpClientConfig::default()),
            body: AudioToTextBody::new(model),
            file_path: None,
        }
    }

    /// Set the local audio file path to transcribe (required).
    pub fn with_file_path(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Set the sampling temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.body = self.body.with_temperature(temperature);
        self
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

    /// Override the base URL (uses [`ApiBase::Custom`]).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.api_base = ApiBase::Custom(base_url.into());
        self.url = self
            .endpoint_config
            .url(&self.api_base, paths::AUDIO_TRANSCRIPTIONS);
        self
    }

    /// Replace the full [`EndpointConfig`] used to resolve URLs.
    pub fn with_endpoint_config(mut self, endpoint_config: EndpointConfig) -> Self {
        self.endpoint_config = endpoint_config;
        self.url = self
            .endpoint_config
            .url(&self.api_base, paths::AUDIO_TRANSCRIPTIONS);
        self
    }

    /// Replace the HTTP client configuration (timeouts, retries, …).
    pub fn with_http_config(mut self, config: HttpClientConfig) -> Self {
        self.http_config = Arc::new(config);
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
                    code: 1200,
                    message: "file_path is required".to_string(),
                })?;

        if !Path::new(p).exists() {
            return Err(crate::client::error::ZaiError::FileError {
                code: codes::SDK_FILE_NOT_FOUND,
                message: format!("file_path not found: {}", p),
            });
        }

        Ok(())
    }

    /// Submit the multipart request and parse the typed transcription response.
    pub async fn send(&self) -> crate::ZaiResult<super::response::AudioToTextResponse>
    where
        N: Clone + Send + Sync + 'static,
    {
        self.validate()?;

        let resp = self.post().await?;

        parse_typed_response::<super::response::AudioToTextResponse>(resp).await
    }
}

impl<N> HttpClient for AudioToTextRequest<N>
where
    N: ModelName + AudioToText + Serialize + Clone + Send + Sync + 'static,
{
    type Body = AudioToTextBody<N>;
    type ApiUrl = String;
    type ApiKey = String;

    /// Resolved target URL for the request.
    fn api_url(&self) -> &Self::ApiUrl {
        &self.url
    }

    /// API key used for `Authorization: Bearer …`.
    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }

    /// Form fields for the multipart request.
    fn body(&self) -> &Self::Body {
        &self.body
    }

    /// POST the multipart form (file + fields) to the transcription endpoint.
    fn post(
        &self,
    ) -> impl std::future::Future<Output = crate::ZaiResult<reqwest::Response>> + Send {
        let key = self.key.clone();

        let url = self.url.clone();
        let config = self.http_config.clone();

        let body = self.body.clone();

        let file_path_opt = self.file_path.clone();

        async move {
            let file_path =
                file_path_opt.ok_or_else(|| crate::client::error::ZaiError::ApiError {
                    code: 1200,
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

            let temperature = body.temperature;
            let stream = body.stream;
            let request_id = body.request_id.clone();
            let user_id = body.user_id.clone();
            let model_name: String = body.model.into();
            send_multipart_request(reqwest::Method::POST, url, key, config, move || {
                let part = reqwest::multipart::Part::bytes(file_bytes.clone())
                    .file_name(file_name.clone())
                    .mime_str(mime)?;
                let mut form = reqwest::multipart::Form::new()
                    .part("file", part)
                    .text("model", model_name.clone());
                if let Some(t) = temperature {
                    form = form.text("temperature", t.to_string());
                }
                if let Some(s) = stream {
                    form = form.text("stream", s.to_string());
                }
                if let Some(rid) = request_id.as_ref() {
                    form = form.text("request_id", rid.clone());
                }
                if let Some(uid) = user_id.as_ref() {
                    form = form.text("user_id", uid.clone());
                }
                Ok(form)
            })
            .await
        }
    }

    /// HTTP client configuration (timeouts, retries, …).
    fn http_config(&self) -> Arc<HttpClientConfig> {
        self.http_config.clone()
    }
}
