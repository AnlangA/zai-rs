//! Cross-field validation for realtime session configuration.

use std::collections::HashSet;

use crate::ZaiResult;
use crate::realtime::protocol::{ChatMode, SessionConfig, TurnDetectionType};

use super::protocol_error;

/// Validate combinations that cannot be represented by independent field
/// validators, before the session performs network I/O.
pub(super) fn validate_session_config(config: &SessionConfig) -> ZaiResult<()> {
    if let Some(temperature) = config.temperature
        && (!temperature.is_finite() || !(0.0..=1.0).contains(&temperature))
    {
        return Err(protocol_error(
            "realtime temperature must be a finite value between 0 and 1",
        ));
    }
    if let Some(tokens) = config.max_response_output_tokens
        && !(1..=1024).contains(&tokens)
    {
        return Err(protocol_error(
            "realtime max_response_output_tokens must be between 1 and 1024",
        ));
    }
    if config.modalities.is_empty() {
        return Err(protocol_error(
            "realtime modalities must contain text, audio, or both",
        ));
    }
    if config.modalities.len() > 2
        || (config.modalities.len() == 2 && config.modalities[0] == config.modalities[1])
    {
        return Err(protocol_error(
            "realtime modalities must not contain duplicate values",
        ));
    }

    let turn_detection = &config.turn_detection;
    let has_server_vad_options = turn_detection.create_response.is_some()
        || turn_detection.interrupt_response.is_some()
        || turn_detection.prefix_padding_ms.is_some()
        || turn_detection.silence_duration_ms.is_some()
        || turn_detection.threshold.is_some();
    if turn_detection.type_ == TurnDetectionType::ClientVad && has_server_vad_options {
        return Err(protocol_error(
            "realtime server-VAD options require turn_detection type server_vad",
        ));
    }
    if let Some(threshold) = turn_detection.threshold
        && (!threshold.is_finite() || !(0.0..=1.0).contains(&threshold))
    {
        return Err(protocol_error(
            "realtime VAD threshold must be a finite value between 0 and 1",
        ));
    }

    if config.beta_fields.chat_mode.is_none() {
        return Err(protocol_error(
            "realtime beta_fields.chat_mode is required when beta_fields is present",
        ));
    }
    if config
        .beta_fields
        .tts_source
        .as_deref()
        .is_some_and(|source| source != "e2e")
    {
        return Err(protocol_error(
            "unsupported realtime beta_fields.tts_source; the current protocol supports only \"e2e\"",
        ));
    }

    if config.tools.iter().any(|tool| tool.type_ != "function") {
        return Err(protocol_error("realtime tools must use type \"function\""));
    }
    if config
        .tools
        .iter()
        .any(|tool| tool.name.trim().is_empty() || tool.description.trim().is_empty())
    {
        return Err(protocol_error(
            "realtime tools require non-blank names and descriptions",
        ));
    }
    if config.tools.iter().any(|tool| !tool.parameters.is_object()) {
        return Err(protocol_error(
            "realtime tool parameters must be a JSON Schema object",
        ));
    }
    let mut tool_names = HashSet::with_capacity(config.tools.len());
    if config
        .tools
        .iter()
        .any(|tool| !tool_names.insert(tool.name.as_str()))
    {
        return Err(protocol_error("realtime tool names must be unique"));
    }
    if !config.tools.is_empty() && config.beta_fields.chat_mode != Some(ChatMode::Audio) {
        return Err(protocol_error(
            "realtime function tools are supported only in audio chat mode",
        ));
    }

    if let Some(content) = config
        .greeting_config
        .as_ref()
        .and_then(|greeting| greeting.content.as_deref())
        && content.chars().count() > 1024
    {
        return Err(protocol_error(
            "realtime greeting content must not exceed 1024 characters",
        ));
    }
    Ok(())
}
