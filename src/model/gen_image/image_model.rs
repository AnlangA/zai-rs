use super::super::traits::{ImageGen, define_model_type, impl_model_markers};

define_model_type!(GlmImage, "glm-image");
define_model_type!(CogView4_250304, "cogview-4-250304");
define_model_type!(CogView4, "cogview-4");
define_model_type!(CogView3Flash, "cogview-3-flash");
impl_model_markers!(GlmImage: ImageGen);
impl_model_markers!(CogView4_250304: ImageGen);
impl_model_markers!(CogView4: ImageGen);
impl_model_markers!(CogView3Flash: ImageGen);
