use super::super::traits::endpoint_model_registry;

endpoint_model_registry! {
    snapshot: VIDEO_MODEL_REGISTRY_SNAPSHOT,
    family: "video",
    capability: VideoGen;
    CogVideoX3 => "cogvideox-3";
}
