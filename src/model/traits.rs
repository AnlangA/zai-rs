//! # Core Traits for AI Model Abstractions
//!
//! Defines the fundamental traits enabling type-safe interactions with
//! different AI models and capabilities in the Zhipu AI ecosystem.
//!
//! # Trait Categories
//!
//! ## Model Identification
//!
//! - `ModelName` — Converts model types to string identifiers used in API
//!   requests
//!
//! ## Capability Markers
//!
//! These marker traits carry no runtime data but encode model capabilities at
//! compile time:
//!
//! - `Chat` — Synchronous chat completion
//! - `AsyncChat` — Asynchronous (queued) chat completion
//! - `ChatRequestModel` — Frozen request limits shared by a chat model family
//! - `ChatToolSupport` — Tool input accepted by a text or vision chat model
//! - `ResponseFormatEnable` — Text-only structured response formatting
//! - `WatermarkEnable` — Audio-chat watermark control
//! - `ThinkEnable` — Thinking / reasoning mode
//! - `ReasoningEffortEnable` — `reasoning_effort` depth control (GLM-5.2+)
//! - `ToolStreamEnable` — Streaming tool-call output
//! - `VideoGen` — Video generation
//! - `ImageGen` — Image generation
//! - `AudioToText` — Speech recognition
//! - `TextToAudio` — Text-to-speech synthesis
//! - `VoiceClone` — Voice cloning
//!
//! ## Type-Safety Traits
//!
//! - `Bounded` — Compile-time model ↔ message compatibility check
//! - `StreamState` — Type-state pattern for streaming control
//!
//! # Type-State Pattern
//!
//! `StreamState` and its implementations (`StreamOn`, `StreamOff`)
//! enforce streaming vs. non-streaming semantics at the type level, preventing
//! invalid API usage without any runtime cost.
//!
/// Trait implemented by built-in, wire-serializable model identifiers.
///
/// [`NAME`](Self::NAME) avoids serializing or cloning a zero-sized model just
/// to inspect the provider identifier during validation.
pub trait ModelName: Into<String> + serde::Serialize {
    /// Exact provider model identifier used on the wire.
    const NAME: &'static str;
}

/// Marker trait for compile-time model-message compatibility checking.
///
/// This trait is used in conjunction with the type system to ensure that
/// specific model types are only used with compatible message types,
/// preventing invalid API calls at compile time.
pub trait Bounded {}

pub(crate) mod sealed {
    pub trait Chat {}
    pub trait AsyncChat {}
    pub trait ChatRequestModel {}
    pub trait AudioToText {}
    pub trait TextToAudio {}
    pub trait VideoGen {}
    pub trait ImageGen {}
    pub trait VoiceClone {}
}

/// Indicates that a model supports synchronous chat completion.
///
/// Models implementing this trait can be used with the chat-completion API for
/// real-time conversational interactions. This trait is sealed to the model
/// ids in the frozen synchronous Chat request enum.
pub trait Chat: sealed::Chat {}

/// Indicates that a model supports asynchronous chat completion.
///
/// Models implementing this trait can be used with the asynchronous
/// chat-completion API for queued conversational requests. This trait is
/// sealed to frozen asynchronous model ids.
pub trait AsyncChat: sealed::AsyncChat {}

/// Frozen request constraints shared by every chat model schema.
///
/// The associated constants let validation enforce family-specific limits
/// without exposing a catch-all request body that accepts invalid fields. The
/// trait is sealed; downstream model identifiers cannot opt into frozen Chat
/// operations by implementing capability markers themselves.
///
/// ```compile_fail
/// use zai_rs::model::traits::{ChatRequestModel, ModelName};
///
/// #[derive(serde::Serialize)]
/// struct UnofficialChat;
///
/// impl From<UnofficialChat> for String {
///     fn from(_: UnofficialChat) -> Self {
///         "unofficial-chat".to_owned()
///     }
/// }
///
/// impl ModelName for UnofficialChat {
///     const NAME: &'static str = "unofficial-chat";
/// }
/// impl ChatRequestModel for UnofficialChat {
///     const MAX_TOKENS: u32 = 1_024;
/// }
/// ```
pub trait ChatRequestModel: ModelName + sealed::ChatRequestModel {
    /// Largest `max_tokens` value accepted by this model's request schema.
    const MAX_TOKENS: u32;

    /// Whether the upstream accepts `thinking.type = "disabled"` for this
    /// model.
    ///
    /// GLM-5.3 always thinks: disabling thinking is rejected upstream, so
    /// request validation fails fast with `thinking_cannot_be_disabled`
    /// instead of surfacing a server error. Legacy requests that used
    /// `disabled` should migrate to `enabled` plus a light
    /// [`ReasoningEffort::Low`](super::tools::ReasoningEffort::Low).
    const THINKING_DISABLE_SUPPORTED: bool;

    /// `reasoning_effort` levels accepted by this model's request schema.
    ///
    /// Empty for models without [`ReasoningEffortEnable`]. GLM-5.3 accepts
    /// only `low`, `high`, and `max`, while GLM-5.2 accepts every
    /// [`ReasoningEffort`](super::tools::ReasoningEffort) variant. Setting an
    /// unsupported level fails validation with `reasoning_effort_not_supported`.
    const REASONING_EFFORTS: &'static [super::tools::ReasoningEffort];
}

/// Indicates that a chat model accepts tools and defines their public input
/// type.
///
/// Text models use [`Tools`](super::tools::Tools), while vision models use
/// [`Function`](super::tools::Function). This prevents retrieval, web-search,
/// and MCP tools from being attached to a vision request at compile time.
pub trait ChatToolSupport: ChatRequestModel {
    /// Public tool type accepted by `add_tool` and `add_tools`.
    type Tool: Into<super::tools::Tools>;
}

/// Indicates that a text chat model accepts `response_format`.
pub trait ResponseFormatEnable: ChatRequestModel {}

/// Indicates that an audio chat model accepts `watermark_enabled`.
pub trait WatermarkEnable: ChatRequestModel {}

/// Indicates that a model supports thinking/reasoning capabilities.
///
/// Models implementing this trait can enable the provider's extended reasoning
/// mode. Whether reasoning text is returned is model- and endpoint-dependent.
/// GLM-5.3 keeps thinking always on: `ThinkingType::disabled()` is rejected
/// for `GLM5_3` requests (see
/// [`ChatRequestModel::THINKING_DISABLE_SUPPORTED`]).
pub trait ThinkEnable: ChatRequestModel {}

/// Indicates that a model supports the `reasoning_effort` parameter, which
/// controls the depth of reasoning when thinking mode is enabled.
///
/// Supported by GLM-5.2 and above. Each model accepts its own subset of
/// [`ReasoningEffort`](super::tools::ReasoningEffort) levels, frozen in
/// [`ChatRequestModel::REASONING_EFFORTS`]: GLM-5.2 accepts every level,
/// while GLM-5.3 accepts only `low`, `high`, and `max` (its default).
pub trait ReasoningEffortEnable: ChatRequestModel {}

/// Indicates that a model supports streaming tool calls (tool_stream
/// parameter). Only models implementing this marker can enable tool_stream in
/// requests.
pub trait ToolStreamEnable: ChatToolSupport {}

/// Indicates that an official model supports video generation.
///
/// This trait is sealed to the model ids documented by the frozen API schema.
pub trait VideoGen: ModelName + sealed::VideoGen {}

/// Indicates that an official model supports image generation.
///
/// This trait is sealed to the model ids documented by the frozen API schema.
pub trait ImageGen: ModelName + sealed::ImageGen {}

/// Indicates that an official model supports speech recognition.
///
/// This trait is sealed because the ASR request schema is frozen to the
/// documented model ids. Downstream crates cannot opt arbitrary model names
/// into that schema.
///
/// ```compile_fail
/// use zai_rs::model::traits::{AudioToText, ModelName};
///
/// #[derive(serde::Serialize)]
/// struct UnofficialAsr;
///
/// impl From<UnofficialAsr> for String {
///     fn from(_: UnofficialAsr) -> Self {
///         "unofficial-asr".to_owned()
///     }
/// }
///
/// impl ModelName for UnofficialAsr {
///     const NAME: &'static str = "unofficial-asr";
/// }
/// impl AudioToText for UnofficialAsr {}
/// ```
pub trait AudioToText: ModelName + sealed::AudioToText {}

/// Indicates that an official model supports text-to-speech synthesis.
///
/// This trait is sealed because the TTS request schema is frozen to the
/// documented model ids. Downstream crates cannot opt arbitrary model names
/// into that schema.
///
/// ```compile_fail
/// use zai_rs::model::traits::{ModelName, TextToAudio};
///
/// #[derive(serde::Serialize)]
/// struct UnofficialTts;
///
/// impl From<UnofficialTts> for String {
///     fn from(_: UnofficialTts) -> Self {
///         "unofficial-tts".to_owned()
///     }
/// }
///
/// impl ModelName for UnofficialTts {
///     const NAME: &'static str = "unofficial-tts";
/// }
/// impl TextToAudio for UnofficialTts {}
/// ```
pub trait TextToAudio: ModelName + sealed::TextToAudio {}

/// Indicates that an official model supports voice cloning.
///
/// This trait is sealed to the model ids documented by the frozen API schema.
pub trait VoiceClone: ModelName + sealed::VoiceClone {}

/// Type-state trait for compile-time streaming capability control.
///
/// This trait enables the type system to enforce whether a request
/// supports streaming (`StreamOn`) or non-streaming (`StreamOff`) responses,
/// preventing invalid API usage patterns.
pub trait StreamState {}

/// Type-state indicating that streaming is enabled.
///
/// Types parameterized with this marker support Server-Sent Events (SSE)
/// streaming for real-time response processing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct StreamOn;

/// Type-state indicating that streaming is disabled.
///
/// Types parameterized with this marker receive complete responses
/// rather than streaming chunks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct StreamOff;

impl StreamState for StreamOn {}
impl StreamState for StreamOff {}

// Generates a zero-sized built-in model id and its standard wire traits.
macro_rules! define_model_type {
    ($(#[$meta:meta])* $name:ident, $s:expr) => {
        #[doc = concat!(" AI model type backed by the upstream id `", $s, "`.")]
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
        $(#[$meta])*
        pub struct $name {}

        impl ::core::convert::From<$name> for String {
            fn from(_val: $name) -> Self { $s.to_string() }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: ::serde::Serializer {
                serializer.serialize_str(&$s)
            }
        }

        impl $crate::model::traits::ModelName for $name {
            const NAME: &'static str = $s;
        }
    };
}
pub(crate) use define_model_type;

// Binds the message schemas accepted by one built-in model.
macro_rules! impl_message_binding {
    ($name:ident, $message_type:ty) => {
        impl $crate::model::traits::Bounded for ($name, $message_type) {}
    };
}
pub(crate) use impl_message_binding;

// Declares the frozen model ids accepted by one non-chat HTTP request schema.
//
// Keeping the wire id and both halves of the sealed capability implementation
// in one invocation prevents a model from accidentally being serializable but
// unusable (or request-compatible under the wrong endpoint). The generated
// metadata and contract test only exist in unit-test builds; this is not a
// public runtime registry.
macro_rules! endpoint_model_registry {
    (
        snapshot: $snapshot:ident,
        family: $family:literal,
        capability: $capability:ident;
        $(
            $(#[$meta:meta])*
            $model:ident => $model_id:literal;
        )+
    ) => {
        $(
            $crate::model::traits::define_model_type!(
                $(#[$meta])*
                $model,
                $model_id
            );
            impl $crate::model::traits::sealed::$capability for $model {}
            impl $crate::model::traits::$capability for $model {}
        )+

        #[cfg(test)]
        pub(crate) const $snapshot: &[(&str, &str, &str, &str)] = &[
            $(
                (
                    $family,
                    stringify!($model),
                    $model_id,
                    stringify!($capability),
                ),
            )+
        ];

        #[cfg(test)]
        mod registry_contract_tests {
            use super::*;

            fn assert_registration<N>(expected_id: &str)
            where
                N: $crate::model::traits::ModelName
                    + $crate::model::traits::$capability
                    + Default,
            {
                assert_eq!(N::NAME, expected_id);
                assert_eq!(Into::<String>::into(N::default()), expected_id);
                assert_eq!(
                    serde_json::to_value(N::default()).expect("model id must serialize"),
                    serde_json::Value::String(expected_id.to_owned()),
                );
            }

            #[test]
            fn generated_wire_and_request_capability_contracts_hold() {
                $(assert_registration::<$model>($model_id);)+
            }
        }
    };
}
pub(crate) use endpoint_model_registry;
