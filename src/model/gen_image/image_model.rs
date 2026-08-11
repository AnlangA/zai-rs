use super::super::traits::endpoint_model_registry;

endpoint_model_registry! {
    snapshot: IMAGE_MODEL_REGISTRY_SNAPSHOT,
    family: "image",
    capability: ImageGen;
    GlmImage => "glm-image";
    CogView4_250304 => "cogview-4-250304";
    CogView4 => "cogview-4";
    CogView3Flash => "cogview-3-flash";
}
