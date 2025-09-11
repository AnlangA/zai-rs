use super::request::AudioTranscriptionBody;
use super::super::traits::*;
use serde::Serialize;
use std::path::Path;

use crate::client::http::HttpClient;

/// Audio transcription request (multipart/form-data)
pub struct AudioTranscriptionRequest<N>
where
    N: ModelName + AudioToText + Serialize,
{
    pub key: String,
    pub body: AudioTranscriptionBody<N>,
    file_path: Option<String>,
}

impl<N> AudioTranscriptionRequest<N>
where
    N: ModelName + AudioToText + Serialize + Clone,
{
    pub fn new(model: N, key: String) -> Self {
        Self { key, body: AudioTranscriptionBody::new(model), file_path: None }
    }

    pub fn with_file_path(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.body = self.body.with_temperature(temperature);
        self
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.body = self.body.with_stream(stream);
        self
    }

    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.body = self.body.with_request_id(request_id);
        self
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.body = self.body.with_user_id(user_id);
        self
    }
}

impl<N> HttpClient for AudioTranscriptionRequest<N>
where
    N: ModelName + AudioToText + Serialize + Clone + Send + Sync + 'static,
{
    type Body = AudioTranscriptionBody<N>;
    type ApiUrl = &'static str;
    type ApiKey = String;

    fn api_url(&self) -> &Self::ApiUrl {
        &"https://open.bigmodel.cn/api/paas/v4/audio/transcriptions"
    }

    fn api_key(&self) -> &Self::ApiKey {
        &self.key
    }

    fn body(&self) -> &Self::Body {
        &self.body
    }

    // Override default JSON post with multipart/form-data post
    fn post(&self) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
        let key = self.key.clone();
        let url = (*self.api_url()).to_string();
        let body = self.body.clone();
        let file_path_opt = self.file_path.clone();

        async move {
            let file_path = file_path_opt.ok_or_else(|| anyhow::anyhow!("file_path is required"))?;

            let mut form = reqwest::multipart::Form::new();

            // file
            let file_name = Path::new(&file_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("audio.wav");
            let file_bytes = tokio::fs::read(&file_path).await?;

            // Basic MIME guess by extension
            let mime = if file_name.to_ascii_lowercase().ends_with(".mp3") {
                "audio/mpeg"
            } else {
                "audio/wav"
            };

            let part = reqwest::multipart::Part::bytes(file_bytes)
                .file_name(file_name.to_string())
                .mime_str(mime)?;
            form = form.part("file", part);

            // model
            let model_name: String = body.model.into();
            form = form.text("model", model_name);

            // Optional fields
            if let Some(t) = body.temperature {
                form = form.text("temperature", t.to_string());
            }
            if let Some(s) = body.stream {
                form = form.text("stream", s.to_string());
            }
            if let Some(rid) = body.request_id {
                form = form.text("request_id", rid);
            }
            if let Some(uid) = body.user_id {
                form = form.text("user_id", uid);
            }

            let resp = reqwest::Client::new()
                .post(url)
                .bearer_auth(key)
                .multipart(form)
                .send()
                .await?;

            Ok(resp)
        }
    }
}

