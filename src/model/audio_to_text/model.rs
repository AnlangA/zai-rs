use super::super::traits::endpoint_model_registry;

endpoint_model_registry! {
    snapshot: ASR_MODEL_REGISTRY_SNAPSHOT,
    family: "asr",
    capability: AudioToText;
    GlmAsr => "glm-asr-2512";
}
