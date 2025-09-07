//! Model type definitions for the API.
//!
//! This module defines the various model types that can be used with the API,
//! along with their associated traits and implementations.

use crate::model::chat_message_types::{TextMessage, VisionMessage};
use crate::{define_model_type, impl_message_binding, impl_think_enable};

// Define basic model types
define_model_type!(GLM4_5, "glm-4.5");
impl_think_enable!(GLM4_5);
impl_message_binding!(GLM4_5, TextMessage);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5_flash,
    "glm-4.5-flash"
);
impl_think_enable!(GLM4_5_flash);
impl_message_binding!(GLM4_5_flash, TextMessage);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5_air,
    "glm-4.5-air"
);
impl_think_enable!(GLM4_5_air);
impl_message_binding!(GLM4_5_air, TextMessage);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5_x,
    "glm-4.5-X"
);
impl_think_enable!(GLM4_5_x);
impl_message_binding!(GLM4_5_x, TextMessage);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5_airx,
    "glm-4.5-airx"
);
impl_think_enable!(GLM4_5_airx);
impl_message_binding!(GLM4_5_airx, TextMessage);

define_model_type!(
    #[allow(non_camel_case_types)]
    GLM4_5v,
    "glm-4.5v"
);
impl_message_binding!(GLM4_5v, VisionMessage);
