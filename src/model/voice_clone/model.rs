use super::super::traits::*;
use crate::{define_model_type, impl_model_markers};

// CogTTS voice clone model identifier
define_model_type!(CogTtsClone, "cogtts-clone");
impl_model_markers!(CogTtsClone: VoiceClone);
