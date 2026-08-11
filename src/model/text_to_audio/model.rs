use super::super::traits::endpoint_model_registry;

endpoint_model_registry! {
    snapshot: TTS_MODEL_REGISTRY_SNAPSHOT,
    family: "tts",
    capability: TextToAudio;
    GlmTts => "glm-tts";
}
