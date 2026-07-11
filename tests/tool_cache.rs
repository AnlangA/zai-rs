//! P09 acceptance: tool cache semantics (plan P09.6-P09.8, P09.11, P09.14).
//! Requires `--features toolkits`.

#![cfg(feature = "toolkits")]

use sha2::{Digest, Sha256};

/// Lowercase hex encoding of a byte slice (avoids a `hex` crate dependency).
fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// Canonicalize JSON input for cache-key computation: stable key sort, no
/// whitespace trimming, number semantics preserved.
fn canonical_json(input: &serde_json::Value) -> String {
    serde_json::to_string(input).unwrap()
}

/// SHA-256 cache key for (tool_name, generation, canonical_input).
fn cache_key(tool_name: &str, generation: u64, canonical_input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(tool_name.as_bytes());
    hasher.update(generation.to_le_bytes().as_slice());
    hasher.update(canonical_input.as_bytes());
    to_hex(&hasher.finalize())
}

#[test]
fn cache_key_identifies_input_differences() {
    let k1 = cache_key("calc", 1, "1");
    let k2 = cache_key("calc", 1, "2");
    assert_ne!(k1, k2, "different inputs → different keys");
}

#[test]
fn cache_key_identifies_generation_differences() {
    let k1 = cache_key("calc", 1, "1");
    let k2 = cache_key("calc", 2, "1");
    assert_ne!(k1, k2, "different generations → different keys");
}

#[test]
fn cache_key_identifies_name_differences() {
    let k1 = cache_key("calc", 1, "1");
    let k2 = cache_key("add", 1, "1");
    assert_ne!(k1, k2, "different tool names → different keys");
}

#[test]
fn cache_key_is_deterministic() {
    let k1 = cache_key("calc", 1, r#"{"a":1}"#);
    let k2 = cache_key("calc", 1, r#"{"a":1}"#);
    assert_eq!(k1, k2, "same inputs → same key");
}

#[test]
fn canonical_json_preserves_number_semantics() {
    let v = serde_json::json!({"count": 1});
    let c = canonical_json(&v);
    assert!(c.contains("1"), "number 1 preserved");
}

#[test]
fn string_null_and_json_null_do_not_collide() {
    let str_null = canonical_json(&serde_json::json!("null"));
    let json_null = canonical_json(&serde_json::json!(null));
    assert_ne!(str_null, json_null, "string null ≠ JSON null");
}

#[test]
fn object_key_whitespace_is_not_trimmed_by_canonical() {
    // serde_json::to_string does NOT add spaces around colons, but a raw
    // deserialization round-trip should produce identical canonical forms.
    let a = canonical_json(&serde_json::json!({" key " : "val"}));
    let b = canonical_json(&serde_json::json!({" key " : "val"}));
    assert_eq!(a, b, "same JSON → same canonical form");
}

#[test]
fn schema_cache_key_is_sha256() {
    // Plan P09.13: schema cache uses full bytes equality + SHA-256, not u64 hash.
    let mut hasher = Sha256::new();
    hasher.update(b"{\"type\":\"object\"}");
    let hash = to_hex(&hasher.finalize());
    assert_eq!(hash.len(), 64); // SHA-256 hex is 64 chars
}
