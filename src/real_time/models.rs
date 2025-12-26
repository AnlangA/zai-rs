use super::traits::*;
use crate::{define_model_type, impl_model_markers};

define_model_type!(GLMRealtime, "glm-realtime");
impl_model_markers!(GLMRealtime: RealTimeModel);
