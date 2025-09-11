pub trait ModelName: Into<String> {}

pub trait Bounded {}

pub trait Chat {}

pub trait AsyncChat {}

pub trait ThinkEnable {}

pub trait VideoGen {}

pub trait ImageGen {}

pub trait AudioToText {}

pub trait TextToSpeech {}

pub trait VoiceClone {}

/// Define a basic model type with core implementations (Into<String>, Serialize, ModelName).

// Shared type-state for compile-time streaming capability gating
pub trait StreamState {}
pub struct StreamOn;
pub struct StreamOff;
impl StreamState for StreamOn {}
impl StreamState for StreamOff {}

use crate::client::http::HttpClient;
use futures::StreamExt;
use log::info;

/// Optional capability trait for types that support Server-Sent Events (SSE) streaming.
/// Default implementation relies on `HttpClient::post()` returning an SSE response.
/// It parses `data: ` lines, logs each payload, invokes the provided sync callback,
/// and stops on `[DONE]`.
pub trait SseStreamable: HttpClient {
    fn stream_sse_for_each<'a, F>(
        &'a mut self,
        mut on_data: F,
    ) -> impl core::future::Future<Output = anyhow::Result<()>> + 'a
    where
        F: FnMut(&[u8]) + 'a,
    {
        async move {
            let resp = self.post().await?;
            let mut stream = resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();

            while let Some(next) = stream.next().await {
                match next {
                    Ok(bytes) => {
                        buf.extend_from_slice(&bytes);
                        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                            let line_vec: Vec<u8> = buf.drain(..=pos).collect();
                            let mut line = &line_vec[..];
                            if line.ends_with(b"\n") {
                                line = &line[..line.len() - 1];
                            }
                            if line.ends_with(b"\r") {
                                line = &line[..line.len() - 1];
                            }
                            if line.is_empty() {
                                continue;
                            }
                            const PREFIX: &[u8] = b"data: ";
                            if line.starts_with(PREFIX) {
                                let rest = &line[PREFIX.len()..];
                                info!("SSE data: {}", String::from_utf8_lossy(rest));
                                if rest == b"[DONE]" {
                                    return Ok(());
                                }
                                on_data(rest);
                            }
                        }
                    }
                    Err(e) => return Err(anyhow::anyhow!("Stream error: {}", e)),
                }
            }
            Ok(())
        }
    }
}

/// Usage examples:
///   define_model_type!(GLM4_5, "glm-4.5");
///   define_model_type!(#[allow(non_camel_case_types)] GLM4_5_flash, "glm-4.5-flash");
#[macro_export]
macro_rules! define_model_type {
    ($(#[$meta:meta])* $name:ident, $s:expr) => {
        #[derive(Debug, Clone)]
        $(#[$meta])*
        pub struct $name {}

        impl ::core::convert::Into<String> for $name {
            fn into(self) -> String { $s.to_string() }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: ::serde::Serializer {
                let model_name: String = self.clone().into();
                serializer.serialize_str(&model_name)
            }
        }

        impl $crate::model::traits::ModelName for $name {}
    };
}

/// Implement thinking capability for a model type.
/// Usage: impl_think_enable!(GLM4_5);
#[macro_export]
macro_rules! impl_think_enable {
    ($name:ident) => {
        impl $crate::model::traits::ThinkEnable for $name {}
    };
}

/// Implement message type binding for a model type.
///
/// Supports single or multiple message types:
/// - Single: impl_message_binding!(GLM4_5, TextMessage);
/// - Multiple: impl_message_binding!(GLM4_5, TextMessage, VisionMessage);
#[macro_export]
macro_rules! impl_message_binding {
    // Single message type
    ($name:ident, $message_type:ty) => {
        impl $crate::model::traits::Bounded for ($name, $message_type) {}
    };
    // Multiple message types
    ($name:ident, $message_type:ty, $($message_types:ty),+) => {
        impl $crate::model::traits::Bounded for ($name, $message_type) {}
        $(
            impl $crate::model::traits::Bounded for ($name, $message_types) {}
        )+
    };
}

/// Implement one or more marker traits for a model type in a single call.
/// Examples:
///   impl_model_markers!(GLM4_5_flash: AsyncChat, Chat);
///   impl_model_markers!([GLM4_5, GLM4_5_air]: Chat);
#[macro_export]
macro_rules! impl_model_markers {
    // Single model, multiple markers
    ($model:ident : $($marker:path),+ $(,)?) => {
        $( impl $marker for $model {} )+
    };
    // Multiple models, multiple markers
    ([$($model:ident),+ ] : $($marker:path),+ $(,)?) => {
        $( $( impl $marker for $model {} )+ )+
    };
}
