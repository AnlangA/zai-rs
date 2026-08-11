//! Crate-private Serde adapters shared by independent API modules.

use std::fmt;

#[cfg(any(feature = "realtime", test))]
use std::{borrow::Cow, collections::HashSet};

use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};

/// Stable diagnostic used when an object repeats a key at any nesting depth.
///
/// Keep this message independent of the rejected key and value: provider JSON
/// can contain application secrets, and Serde errors can reach diagnostics.
pub(crate) const DUPLICATE_JSON_KEY_ERROR: &str = "duplicate JSON object key";

/// A JSON value deserializer that rejects duplicate object keys recursively.
///
/// `serde_json::Value` normally retains only the final value for a repeated
/// object key. Response decoders that inspect a value before selecting a typed
/// union variant use this wrapper so ambiguous discriminators and nested data
/// fail closed.
pub(crate) struct UniqueJsonValue(Value);

impl UniqueJsonValue {
    /// Consume the wrapper and return the validated semantic JSON value.
    pub(crate) fn into_inner(self) -> Value {
        self.0
    }
}

impl<'de> Deserialize<'de> for UniqueJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer
            .deserialize_any(UniqueJsonValueVisitor)
            .map(Self)
    }
}

struct UniqueJsonValueVisitor;

impl<'de> Visitor<'de> for UniqueJsonValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        UniqueJsonValue::deserialize(deserializer).map(UniqueJsonValue::into_inner)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
        while let Some(value) = sequence.next_element::<UniqueJsonValue>()? {
            values.push(value.into_inner());
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(<A::Error as de::Error>::custom(DUPLICATE_JSON_KEY_ERROR));
            }
            values.insert(key, object.next_value::<UniqueJsonValue>()?.into_inner());
        }
        Ok(Value::Object(values))
    }
}

/// Validate that every JSON object key occurs exactly once without retaining
/// the semantic value tree.
///
/// This is the allocation-light preflight for hot paths that can deserialize
/// directly into a typed response. Object keys are retained only for the
/// lifetime of their containing object so duplicates can be detected; string
/// values and other payload data are visited and discarded.
#[cfg(any(feature = "realtime", test))]
pub(crate) fn validate_unique_json(input: &str) -> serde_json::Result<()> {
    let mut deserializer = serde_json::Deserializer::from_str(input);
    UniqueJsonValidator::deserialize(&mut deserializer)?;
    deserializer.end()
}

#[cfg(any(feature = "realtime", test))]
struct UniqueJsonValidator;

#[cfg(any(feature = "realtime", test))]
impl<'de> Deserialize<'de> for UniqueJsonValidator {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueJsonValidatorVisitor)?;
        Ok(Self)
    }
}

#[cfg(any(feature = "realtime", test))]
struct UniqueJsonValidatorVisitor;

#[cfg(any(feature = "realtime", test))]
const INLINE_UNIQUE_JSON_KEYS: usize = 16;

#[cfg(any(feature = "realtime", test))]
struct JsonKey<'a>(Cow<'a, str>);

#[cfg(any(feature = "realtime", test))]
impl<'de> Deserialize<'de> for JsonKey<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(JsonKeyVisitor)
    }
}

#[cfg(any(feature = "realtime", test))]
struct JsonKeyVisitor;

#[cfg(any(feature = "realtime", test))]
impl<'de> Visitor<'de> for JsonKeyVisitor {
    type Value = JsonKey<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object key")
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(JsonKey(Cow::Borrowed(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(JsonKey(Cow::Owned(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(JsonKey(Cow::Owned(value)))
    }
}

#[cfg(any(feature = "realtime", test))]
struct UniqueJsonKeys<'a> {
    inline: [Option<Cow<'a, str>>; INLINE_UNIQUE_JSON_KEYS],
    inline_len: usize,
    overflow: Option<HashSet<Cow<'a, str>>>,
}

#[cfg(any(feature = "realtime", test))]
impl<'a> UniqueJsonKeys<'a> {
    fn new() -> Self {
        Self {
            inline: std::array::from_fn(|_| None),
            inline_len: 0,
            overflow: None,
        }
    }

    fn insert(&mut self, key: Cow<'a, str>) -> bool {
        if let Some(keys) = &mut self.overflow {
            return keys.insert(key);
        }
        if self.inline[..self.inline_len]
            .iter()
            .flatten()
            .any(|existing| existing.as_ref() == key.as_ref())
        {
            return false;
        }
        if self.inline_len < INLINE_UNIQUE_JSON_KEYS {
            self.inline[self.inline_len] = Some(key);
            self.inline_len += 1;
            return true;
        }

        let mut keys = HashSet::with_capacity(INLINE_UNIQUE_JSON_KEYS * 2);
        for existing in &mut self.inline {
            if let Some(existing) = existing.take() {
                keys.insert(existing);
            }
        }
        let inserted = keys.insert(key);
        self.inline_len = 0;
        self.overflow = Some(keys);
        inserted
    }
}

#[cfg(any(feature = "realtime", test))]
impl<'de> Visitor<'de> for UniqueJsonValidatorVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object keys")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_borrowed_str<E>(self, _value: &'de str) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        UniqueJsonValidator::deserialize(deserializer).map(drop)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<UniqueJsonValidator>()?.is_some() {}
        Ok(())
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = UniqueJsonKeys::new();
        while let Some(JsonKey(key)) = object.next_key::<JsonKey<'de>>()? {
            if !keys.insert(key) {
                return Err(<A::Error as de::Error>::custom(DUPLICATE_JSON_KEY_ERROR));
            }
            object.next_value::<UniqueJsonValidator>()?;
        }
        Ok(())
    }
}

/// Deserialize an optional identifier that upstream may encode as either a
/// JSON string or number.
pub(crate) fn optional_string_from_number_or_string<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(value) => Ok(Some(value)),
        serde_json::Value::Number(value) => Ok(Some(value.to_string())),
        serde_json::Value::Bool(_) | serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            Err(serde::de::Error::custom("expected string, number, or null"))
        },
    }
}

/// Deserialize an optional JSON-encoded string while tolerating providers
/// that send the decoded JSON value directly.
pub(crate) fn optional_json_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(value) => Ok(Some(value)),
        other => serde_json::to_string(&other)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_json_rejects_same_value_duplicates_without_echoing_data() {
        for payload in [
            r#"{"private-top-level-key":7,"private-top-level-key":7}"#,
            r#"{"outer":{"private-nested-key":"secret","private-nested-key":"secret"}}"#,
        ] {
            let error = match serde_json::from_str::<UniqueJsonValue>(payload) {
                Ok(_) => panic!("duplicate-key JSON unexpectedly decoded"),
                Err(error) => error,
            };
            let diagnostic = error.to_string();
            assert!(diagnostic.contains(DUPLICATE_JSON_KEY_ERROR));
            for secret in ["private-top-level-key", "private-nested-key", "secret"] {
                assert!(!diagnostic.contains(secret));
            }
        }
    }

    #[test]
    fn unique_json_preserves_a_large_nested_payload() {
        let expected = serde_json::json!({
            "items": (0..4_096)
                .map(|index| serde_json::json!({
                    "index": index,
                    "nested": {"even": index % 2 == 0, "label": format!("item-{index}")}
                }))
                .collect::<Vec<_>>()
        });
        let encoded = serde_json::to_vec(&expected).unwrap();
        let actual = serde_json::from_slice::<UniqueJsonValue>(&encoded)
            .unwrap()
            .into_inner();
        assert_eq!(actual, expected);
    }

    #[test]
    fn unique_json_validator_rejects_nested_duplicates_without_echoing_data() {
        validate_unique_json(r#"{"safe":[true,null,{"value":"private"}],"count":3,"ratio":0.5}"#)
            .unwrap();

        for payload in [
            r#"{"private-top-level-key":1,"private-top-level-key":2}"#,
            r#"{"outer":{"private-nested-key":"first","private-nested-key":"secret"}}"#,
        ] {
            let error = validate_unique_json(payload).unwrap_err();
            let diagnostic = error.to_string();
            assert!(diagnostic.contains(DUPLICATE_JSON_KEY_ERROR));
            for secret in ["private-top-level-key", "private-nested-key", "secret"] {
                assert!(!diagnostic.contains(secret));
            }
        }
    }

    #[test]
    fn unique_json_validator_handles_escaped_keys_and_inline_set_overflow() {
        let escaped = validate_unique_json(r#"{"key":1,"\u006bey":2}"#).unwrap_err();
        assert!(escaped.to_string().contains(DUPLICATE_JSON_KEY_ERROR));

        let mut object = String::from("{");
        for index in 0..17 {
            if index != 0 {
                object.push(',');
            }
            object.push_str(&format!(r#""key-{index}":{index}"#));
        }
        object.push_str(r#", "\u006bey-0":99}"#);
        let overflow = validate_unique_json(&object).unwrap_err();
        assert!(overflow.to_string().contains(DUPLICATE_JSON_KEY_ERROR));

        let unique = object.replace(r#""\u006bey-0":99"#, r#""key-17":17"#);
        validate_unique_json(&unique).unwrap();
    }

    #[test]
    fn unique_json_validator_rejects_trailing_values() {
        assert!(validate_unique_json(r#"{"ok":true} {"also":true}"#).is_err());
    }

    #[test]
    fn optional_identifier_type_errors_never_echo_the_value() {
        #[derive(Debug, serde::Deserialize)]
        struct Wire {
            #[serde(deserialize_with = "optional_string_from_number_or_string")]
            id: Option<String>,
        }

        for secret_value in [
            r#"{"echo":"abc.0123456789abcdef"}"#,
            r#"["private-prompt"]"#,
            "true",
        ] {
            let payload = format!(r#"{{"id":{secret_value}}}"#);
            let error = serde_json::from_str::<Wire>(&payload).unwrap_err();
            let diagnostic = error.to_string();
            assert!(diagnostic.contains("expected string, number, or null"));
            assert!(!diagnostic.contains("0123456789abcdef"));
            assert!(!diagnostic.contains("private-prompt"));
        }

        let wire = serde_json::from_str::<Wire>(r#"{"id":42}"#).unwrap();
        assert_eq!(wire.id.as_deref(), Some("42"));
    }
}
