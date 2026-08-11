use super::super::traits::endpoint_model_registry;

endpoint_model_registry! {
    snapshot: VOICE_CLONE_MODEL_REGISTRY_SNAPSHOT,
    family: "voice_clone",
    capability: VoiceClone;
    GlmTtsClone => "glm-tts-clone";
}
