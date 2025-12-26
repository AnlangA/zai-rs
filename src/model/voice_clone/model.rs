use super::super::traits::*;
use crate::{define_model_type, impl_model_markers};

// GLM-TTS voice clone model identifier
define_model_type!(GlmTtsClone, "glm-tts-clone");
impl_model_markers!(GlmTtsClone: VoiceClone);
