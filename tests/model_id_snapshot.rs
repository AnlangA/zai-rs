//! Model-ID snapshot tests (plan P01.2).
//!
//! Every built-in model id across chat / vision / voice / realtime / image /
//! video / ASR / TTS / voice-clone is asserted to be:
//!   1. non-empty,
//!   2. equal to its own trimmed value (no leading/trailing whitespace — the
//!      `glm-asr-2512 ` regression this guards against),
//!   3. present in the frozen contract (OpenAPI) or manual constraints
//!      (ASR/TTS), where applicable.
//!
//! These are pure compile/runtime identity checks — no network is touched.

use zai_rs::model::{
    audio_to_text::GlmAsr, gen_image::CogView4, text_to_audio::GlmTts, voice_clone::GlmTtsClone, *,
};

/// Collect (category, rust type name, id) for every built-in model.
fn all_model_ids() -> Vec<(&'static str, &'static str, String)> {
    vec![
        // chat (text)
        ("chat", "GLM5_2", GLM5_2 {}.into()),
        ("chat", "GLM5_1", GLM5_1 {}.into()),
        ("chat", "GLM5_turbo", GLM5_turbo {}.into()),
        ("chat", "GLM5", GLM5 {}.into()),
        ("chat", "GLM4_7", GLM4_7 {}.into()),
        ("chat", "GLM4_7_flash", GLM4_7_flash {}.into()),
        ("chat", "GLM4_7_flashx", GLM4_7_flashx {}.into()),
        ("chat", "GLM4_6", GLM4_6 {}.into()),
        ("chat", "GLM4_5", GLM4_5 {}.into()),
        ("chat", "GLM4_5_x", GLM4_5_x {}.into()),
        ("chat", "GLM4_5_flash", GLM4_5_flash {}.into()),
        ("chat", "GLM4_5_air", GLM4_5_air {}.into()),
        ("chat", "GLM4_5_airx", GLM4_5_airx {}.into()),
        // vision
        ("vision", "GLM5V_turbo", GLM5V_turbo {}.into()),
        ("vision", "autoglm_phone", autoglm_phone {}.into()),
        ("vision", "GLM4_6v", GLM4_6v {}.into()),
        ("vision", "GLM4_6v_flash", GLM4_6v_flash {}.into()),
        ("vision", "GLM4_6v_flashx", GLM4_6v_flashx {}.into()),
        ("vision", "GLM4_5v", GLM4_5v {}.into()),
        // voice
        ("voice", "GLM4_voice", GLM4_voice {}.into()),
        // realtime
        ("realtime", "GLM_realtime", GLM_realtime {}.into()),
        ("realtime", "GLM4_5_voice", GLM4_5_voice {}.into()),
        // image
        ("image", "CogView4", CogView4 {}.into()),
        // video
        ("video", "CogVideoX3", CogVideoX3 {}.into()),
        // ASR
        ("asr", "GlmAsr", GlmAsr {}.into()),
        // TTS
        ("tts", "GlmTts", GlmTts {}.into()),
        // voice clone
        ("voice_clone", "GlmTtsClone", GlmTtsClone {}.into()),
    ]
}

/// Model ids that the frozen manual constraints pin to a single value
/// (ASR = glm-asr-2512, TTS = glm-tts). Keys are the manual-constraint model
/// enums from spec/contracts/manual-constraints.toml.
const MANUAL_PINNED: &[(&str, &str)] = &[("asr", "glm-asr-2512"), ("tts", "glm-tts")];

#[test]
fn all_model_ids_are_non_empty_and_untrimmed() {
    for (category, type_name, id) in all_model_ids() {
        assert!(!id.is_empty(), "{category}/{type_name}: model id is empty");
        assert_eq!(
            id,
            id.trim(),
            "{category}/{type_name}: model id `{id:?}` has leading/trailing whitespace"
        );
    }
}

#[test]
fn asr_and_tts_match_manual_constraints() {
    let models = all_model_ids();
    for (category, expected) in MANUAL_PINNED {
        let actual = models
            .iter()
            .find(|(c, _, _)| *c == *category)
            .map(|(_, _, id)| id.as_str())
            .unwrap_or_else(|| panic!("no model registered for category {category}"));
        assert_eq!(
            actual, *expected,
            "{category}: id `{actual}` does not match manual-constraint pin `{expected}`"
        );
    }
}

#[test]
fn asr_id_has_no_trailing_space_regression() {
    // Direct guard for the P01.1 regression: "glm-asr-2512 " (trailing space).
    let id: String = GlmAsr {}.into();
    assert_eq!(id, "glm-asr-2512");
    assert!(
        !id.ends_with(' '),
        "ASR id gained trailing whitespace again"
    );
}

#[test]
fn snapshot_chat_model_ids() {
    let chat: Vec<String> = all_model_ids()
        .into_iter()
        .filter(|(c, _, _)| *c == "chat")
        .map(|(_, _, id)| id)
        .collect();
    insta_like_eq(
        &chat,
        &[
            "glm-5.2",
            "glm-5.1",
            "glm-5-turbo",
            "glm-5",
            "glm-4.7",
            "glm-4.7-flash",
            "glm-4.7-flashx",
            "glm-4.6",
            "glm-4.5",
            "glm-4.5-X",
            "glm-4.5-flash",
            "glm-4.5-air",
            "glm-4.5-airx",
        ],
    );
}

/// Lightweight snapshot assertion without an external crate: compare against an
/// expected slice so a drift produces a precise diff in the failure message.
fn insta_like_eq(actual: &[String], expected: &[&str]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "model-id count mismatch: got {} expected {}",
        actual.len(),
        expected.len()
    );
    for (a, e) in actual.iter().zip(expected.iter()) {
        assert_eq!(a, e, "model-id mismatch: got `{a}` expected `{e}`");
    }
}
