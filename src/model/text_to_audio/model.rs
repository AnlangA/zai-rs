use super::super::traits::*;
use crate::{define_model_type, impl_model_markers};

// CogTTS model identifier
define_model_type!(GlmTts, "glm-tts");
impl_model_markers!(GlmTts: TextToAudio);
