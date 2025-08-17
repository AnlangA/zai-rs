pub trait ModelName: Into<String> {}

pub trait Bounded {}

pub trait ChatText {}

pub trait ChatView {}

pub trait ChatVoice {}

pub trait ChatRole {}

pub trait ThinkEnable {}

/// Define a model type with common impls (Into<String>, Serialize, ModelName, ThinkEnable, Bounded).
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
        impl $crate::model::traits::ThinkEnable for $name {}
        impl $crate::model::traits::Bounded for ($name, $crate::model::base_requst::TextMessage) {}
    };
}

