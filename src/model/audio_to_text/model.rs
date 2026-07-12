use super::super::traits::{AudioToText, define_model_type, impl_model_markers};

// GLM ASR model identifier
define_model_type!(GlmAsr, "glm-asr-2512");
impl_model_markers!(GlmAsr: AudioToText);
