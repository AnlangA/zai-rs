use crate::{define_model_type, impl_model_markers};
use super::super::traits::*;


define_model_type!(CogVideoX3, "cogvideox-3");
impl_model_markers!(CogVideoX3: VideoGen, AsyncChat);