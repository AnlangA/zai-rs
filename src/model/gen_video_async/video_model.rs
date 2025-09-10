use super::super::traits::*;
use crate::{define_model_type, impl_model_markers};

define_model_type!(CogVideoX3, "cogvideox-3");
impl_model_markers!(CogVideoX3: VideoGen, AsyncChat);
