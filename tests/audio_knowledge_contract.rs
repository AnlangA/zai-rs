//! Contract-alignment tests for audio and knowledge APIs.

use validator::Validate;

use zai_rs::knowledge::{CreateKnowledgeBody, EmbeddingId};
use zai_rs::model::audio_to_text::AudioToTextBody;
use zai_rs::model::audio_to_text::GlmAsr;
use zai_rs::model::text_to_audio::GlmTts;
use zai_rs::model::text_to_audio::TextToAudioBody;

// ---------------------------------------------------------------------------
// ASR (§13.3)
// ---------------------------------------------------------------------------

#[test]
fn asr_body_has_no_temperature() {
    let body = AudioToTextBody::new(GlmAsr {});
    // temperature field must not exist (compile-time: this would not compile if
    // re-added without going through the builder).
    let json = serde_json::to_value(&body).unwrap();
    assert!(
        json.get("temperature").is_none(),
        "temperature must be removed"
    );
}

#[test]
fn asr_body_supports_prompt_and_hotwords() {
    let body = AudioToTextBody::new(GlmAsr {})
        .with_prompt("some prompt")
        .with_hotwords(vec!["word1".into(), "word2".into()])
        .unwrap();
    let json = serde_json::to_value(&body).unwrap();
    assert_eq!(json["prompt"], "some prompt");
    assert_eq!(json["hotwords"][0], "word1");
}

#[test]
fn asr_hotwords_rejects_over_100() {
    let too_many: Vec<String> = (0..101).map(|i| format!("w{i}")).collect();
    assert!(
        AudioToTextBody::new(GlmAsr {})
            .with_hotwords(too_many)
            .is_err()
    );
}

// ---------------------------------------------------------------------------
// TTS (§13.4)
// ---------------------------------------------------------------------------

#[test]
fn tts_input_max_is_1024_not_4096() {
    // The validator attribute is now `length(max = 1024)`; 1025 chars fails.
    let body = TextToAudioBody::new(GlmTts {}).with_input("x".repeat(1025));
    assert!(
        body.validate().is_err(),
        "1025-char input must fail (max 1024)"
    );
    // 1024 chars is accepted.
    let body = TextToAudioBody::new(GlmTts {}).with_input("x".repeat(1024));
    assert!(body.validate().is_ok(), "1024-char input must pass");
}

// ---------------------------------------------------------------------------
// Knowledge (§13.5)
// ---------------------------------------------------------------------------

#[test]
fn knowledge_embedding_id_12_serializes_for_embedding_3_pro() {
    let body = CreateKnowledgeBody {
        embedding_id: EmbeddingId::Embedding3Pro,
        name: "kb".into(),
        description: None,
        background: None,
        icon: None,
        embedding_model: None,
        contextual: None,
    };
    let json = serde_json::to_value(&body).unwrap();
    assert_eq!(json["embedding_id"], 12);
}

#[test]
fn knowledge_embedding_id_roundtrips_3_11_12() {
    for (id, n) in [
        (EmbeddingId::Embedding2, 3),
        (EmbeddingId::Embedding3New, 11),
        (EmbeddingId::Embedding3Pro, 12),
    ] {
        assert_eq!(id.as_i64(), n);
        let json = serde_json::to_value(&CreateKnowledgeBody {
            embedding_id: id,
            name: "kb".into(),
            description: None,
            background: None,
            icon: None,
            embedding_model: None,
            contextual: None,
        })
        .unwrap();
        let back: CreateKnowledgeBody = serde_json::from_value(json).unwrap();
        assert_eq!(back.embedding_id, id);
    }
}

#[test]
fn knowledge_body_supports_embedding_model_and_contextual() {
    let body = CreateKnowledgeBody {
        embedding_id: EmbeddingId::Embedding3Pro,
        name: "kb".into(),
        description: None,
        background: None,
        icon: None,
        embedding_model: Some("embedding-3-pro".into()),
        contextual: Some(1),
    };
    let json = serde_json::to_value(&body).unwrap();
    assert_eq!(json["embedding_model"], "embedding-3-pro");
    assert_eq!(json["contextual"], 1);
}
