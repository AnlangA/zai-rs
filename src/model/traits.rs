pub trait ModelName: Into<String> {}

pub trait Bounded {}

pub trait ChatText {}

pub trait ChatView {}

pub trait ChatVoice {}

pub trait ChatRole {}

pub trait ThinkEnable {}

// Macro to implement Serialize for model types that convert to String via Clone + Into<String>
#[macro_export]
macro_rules! impl_model_serialize {
    ($($t:ty),+ $(,)?) => { $( impl serde::Serialize for $t {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: serde::Serializer {
            let model_name: String = self.clone().into();
            serializer.serialize_str(&model_name)
        }
    } )+ };
}
