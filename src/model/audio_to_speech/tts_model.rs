use super::super::traits::*;
use crate::{define_model_type, impl_model_markers};

// CogTTS model identifier
define_model_type!(CogTts, "cogtts");
impl_model_markers!(CogTts: TextToSpeech);

