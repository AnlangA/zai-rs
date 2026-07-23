//! Cross-family model-ID normalization regression tests.
//!
//! Every built-in model id across chat / vision / voice / realtime / image /
//! video / ASR / TTS / voice-clone is asserted to be:
//!   1. non-empty,
//!   2. equal to its own trimmed value (no leading/trailing whitespace — the
//!      `glm-asr-2512 ` regression this guards against),
//!   3. equal to the manually pinned ASR/TTS values where applicable.
//!
//! These are pure compile/runtime identity checks — no network is touched.

use zai_rs::model::{
    audio_to_text::GlmAsr,
    gen_image::{CogView3Flash, CogView4, CogView4_250304, GlmImage},
    text_to_audio::GlmTts,
    voice_clone::GlmTtsClone,
    *,
};

/// Collect (category, Rust type name, actual id, expected id) for every
/// supported model.
fn all_model_ids() -> Vec<(&'static str, &'static str, String, &'static str)> {
    vec![
        // chat (text)
        ("chat", "GLM5_2", GLM5_2 {}.into(), "glm-5.2"),
        ("chat", "GLM5_1", GLM5_1 {}.into(), "glm-5.1"),
        (
            "chat",
            "GLM5_1_highspeed",
            GLM5_1_highspeed {}.into(),
            "glm-5.1-highspeed",
        ),
        ("chat", "GLM5_turbo", GLM5_turbo {}.into(), "glm-5-turbo"),
        ("chat", "GLM5", GLM5 {}.into(), "glm-5"),
        ("chat", "GLM4_7", GLM4_7 {}.into(), "glm-4.7"),
        (
            "chat",
            "GLM4_7_flash",
            GLM4_7_flash {}.into(),
            "glm-4.7-flash",
        ),
        (
            "chat",
            "GLM4_7_flashx",
            GLM4_7_flashx {}.into(),
            "glm-4.7-flashx",
        ),
        ("chat", "GLM4_6", GLM4_6 {}.into(), "glm-4.6"),
        (
            "chat",
            "GLM4_5_flash",
            GLM4_5_flash {}.into(),
            "glm-4.5-flash",
        ),
        ("chat", "GLM4_5_air", GLM4_5_air {}.into(), "glm-4.5-air"),
        ("chat", "GLM4_5_airx", GLM4_5_airx {}.into(), "glm-4.5-airx"),
        (
            "chat",
            "GLM4_flash_250414",
            GLM4_flash_250414 {}.into(),
            "glm-4-flash-250414",
        ),
        (
            "chat",
            "GLM4_flashx_250414",
            GLM4_flashx_250414 {}.into(),
            "glm-4-flashx-250414",
        ),
        // vision
        (
            "vision",
            "GLM5V_turbo",
            GLM5V_turbo {}.into(),
            "glm-5v-turbo",
        ),
        (
            "vision",
            "autoglm_phone",
            autoglm_phone {}.into(),
            "autoglm-phone",
        ),
        ("vision", "GLM4_6v", GLM4_6v {}.into(), "glm-4.6v"),
        (
            "vision",
            "GLM4_6v_flash",
            GLM4_6v_flash {}.into(),
            "glm-4.6v-flash",
        ),
        (
            "vision",
            "GLM4_6v_flashx",
            GLM4_6v_flashx {}.into(),
            "glm-4.6v-flashx",
        ),
        (
            "vision",
            "GLM4v_flash",
            GLM4v_flash {}.into(),
            "glm-4v-flash",
        ),
        (
            "vision",
            "GLM4_1v_thinking_flash",
            GLM4_1v_thinking_flash {}.into(),
            "glm-4.1v-thinking-flash",
        ),
        (
            "vision",
            "GLM4_1v_thinking_flashx",
            GLM4_1v_thinking_flashx {}.into(),
            "glm-4.1v-thinking-flashx",
        ),
        // voice
        ("voice", "GLM4_voice", GLM4_voice {}.into(), "glm-4-voice"),
        // realtime
        (
            "realtime",
            "GLM_realtime_flash",
            GLM_realtime_flash {}.into(),
            "glm-realtime-flash",
        ),
        (
            "realtime",
            "GLM_realtime_air",
            GLM_realtime_air {}.into(),
            "glm-realtime-air",
        ),
        // image
        ("image", "GlmImage", GlmImage {}.into(), "glm-image"),
        (
            "image",
            "CogView4_250304",
            CogView4_250304 {}.into(),
            "cogview-4-250304",
        ),
        ("image", "CogView4", CogView4 {}.into(), "cogview-4"),
        (
            "image",
            "CogView3Flash",
            CogView3Flash {}.into(),
            "cogview-3-flash",
        ),
        // video
        ("video", "CogVideoX3", CogVideoX3 {}.into(), "cogvideox-3"),
        // ASR
        ("asr", "GlmAsr", GlmAsr {}.into(), "glm-asr-2512"),
        // TTS
        ("tts", "GlmTts", GlmTts {}.into(), "glm-tts"),
        // voice clone
        (
            "voice_clone",
            "GlmTtsClone",
            GlmTtsClone {}.into(),
            "glm-tts-clone",
        ),
    ]
}

/// Model ids that the frozen manual constraints pin to a single value
/// (ASR = glm-asr-2512, TTS = glm-tts). Keys are the manual-constraint model
/// enums from spec/contracts/manual-constraints.toml.
const MANUAL_PINNED: &[(&str, &str)] = &[("asr", "glm-asr-2512"), ("tts", "glm-tts")];

#[test]
fn all_model_ids_match_the_frozen_contract() {
    for (category, type_name, id, expected) in all_model_ids() {
        assert!(!id.is_empty(), "{category}/{type_name}: model id is empty");
        assert_eq!(
            id,
            id.trim(),
            "{category}/{type_name}: model id `{id:?}` has leading/trailing whitespace"
        );
        assert_eq!(
            id, expected,
            "{category}/{type_name}: model id drifted from the frozen contract"
        );
    }
}

#[test]
fn asr_and_tts_match_manual_constraints() {
    let models = all_model_ids();
    for (category, expected) in MANUAL_PINNED {
        let actual = models
            .iter()
            .find(|(c, _, _, _)| *c == *category)
            .map(|(_, _, id, _)| id.as_str())
            .unwrap_or_else(|| panic!("no model registered for category {category}"));
        assert_eq!(
            actual, *expected,
            "{category}: id `{actual}` does not match manual-constraint pin `{expected}`"
        );
    }
}
