//! # Macros for AI Model Trait Implementations
//!
//! This module provides declarative macros for defining AI model types and
//! their associated trait implementations. These macros help reduce boilerplate
//! code when creating new model definitions and binding them to message types
//! and capability markers.
//!
/// Macro for defining AI model types with standard implementations.
///
/// This macro generates a model type with the following implementations:
/// - `Debug` and `Clone` traits
/// - `Into<String>` for API identifier conversion
/// - `Serialize` for JSON serialization
/// - `ModelName` trait marker
///
/// ## Usage Examples
///
/// ```rust,ignore
/// // Basic model definition
/// define_model_type!(GLM4_5, "glm-4.5");
///
/// // Model with attributes
/// define_model_type!(
///     #[allow(non_camel_case_types)]
///     GLM4_5_flash,
///     "glm-4.5-flash"
/// );
/// ```
#[macro_export]
macro_rules! define_model_type {
    ($(#[$meta:meta])* $name:ident, $s:expr) => {
        #[derive(Debug, Clone)]
        $(#[$meta])*
        pub struct $name {}

        impl ::core::convert::From<String> for $name {
            fn from(_: String) -> Self { $name {} }
        }

        impl ::core::convert::From<$name> for String {
            fn from(_: $name) -> String { $s.to_string() }
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

/// Macro for binding message types to AI models.
///
/// This macro creates compile-time associations between model types and
/// message types, ensuring type safety in chat completion requests.
///
/// ## Usage Examples
///
/// ```rust,ignore
/// // Single message type binding
/// impl_message_binding!(GLM4_5, TextMessage);
///
/// // Multiple message type bindings
/// impl_message_binding!(GLM4_5, TextMessage, VisionMessage);
/// ```
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

/// Macro for implementing multiple capability traits on model types.
///
/// This macro provides a convenient way to mark models with multiple
/// capabilities in a single declaration.
///
/// ## Usage Examples
///
/// ```rust,ignore
/// // Single model, multiple traits
/// impl_model_markers!(GLM4_5_flash: AsyncChat, Chat);
///
/// // Multiple models, same traits
/// impl_model_markers!([GLM4_5, GLM4_5_air]: Chat);
/// ```
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
