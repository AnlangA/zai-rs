use super::super::traits::*;
use crate::{define_model_type, impl_model_markers};

// GLM ASR model identifier
define_model_type!(GlmAsr, "glm-asr");
impl_model_markers!(GlmAsr: AudioToText);
