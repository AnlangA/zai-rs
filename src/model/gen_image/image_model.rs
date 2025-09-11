use super::super::traits::*;
use crate::{define_model_type, impl_model_markers};

define_model_type!(CogView4, "cogview-4");
impl_model_markers!(CogView4: ImageGen);