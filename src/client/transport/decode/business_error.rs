//! Allocation-bounded probing for provider business-error envelopes.
//!
//! The response body can contain large numeric embedding/rerank arrays.  An
//! untagged enum would first buffer that complete JSON tree before trying its
//! variants, even though this probe only needs four top-level fields.  The
//! visitors below consume unknown values with `IgnoredAny`, retain only the
//! documented envelope fields, and preserve the nested-first fallback rules.

use std::{borrow::Cow, fmt};

use serde::{
    Deserialize, Deserializer,
    de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Number, Value, value::RawValue};

const MAX_CAPTURED_CODE_BYTES: usize = 128;
const MAX_CAPTURED_NUMERIC_LITERAL_BYTES: usize = 128;
const MAX_JSON_NESTING_DEPTH: usize = 128;
// One decoded byte can occupy at most six bytes in a JSON string escape
// (`\u00XX`). Include the quotes so values whose decoded form could still fit
// are parsed exactly, while larger raw literals are elided before serde_json
// needs to allocate an escape-decoding scratch buffer.
const MAX_CAPTURED_CODE_LITERAL_BYTES: usize = MAX_CAPTURED_CODE_BYTES * 6 + 2;
const ELIDED_CODE_STRING: &str = "<text>";
const ELIDED_CODE_NUMBER: &str = "<number>";

/// Error code, message, and request id extracted from a recognized business
/// error envelope.
#[derive(Debug, Clone)]
pub struct BusinessError {
    /// Numeric, textual, or otherwise open-format business code.
    pub code: Option<Value>,
    /// Human-readable service error message.
    pub message: String,
    /// Provider request identifier, when returned as a JSON string.
    pub request_id: Option<String>,
}

/// Allocation-bounded transport result for one complete JSON body.
///
/// `Ambiguous` is reserved for duplicate fields that can change whether a
/// provider business error is recognized. It is deliberately distinct from a
/// clean non-envelope so a typed success decoder cannot ignore the duplicated
/// fields and turn an ambiguous error response into success.
#[derive(Default)]
pub(crate) enum ProbeOutcome {
    #[default]
    Clean,
    Error(BusinessError),
    Ambiguous,
    Malformed,
}

struct WireProbe<'a> {
    envelope: WireEnvelope<'a>,
    ambiguous_reserved_duplicate: bool,
}

enum WireEnvelope<'a> {
    Nested {
        error: WireError<'a>,
        request_id: Option<Cow<'a, str>>,
    },
    Flat {
        code: CapturedCode,
        message: Option<Cow<'a, str>>,
        request_id: Option<Cow<'a, str>>,
    },
    NotEnvelope,
}

struct WireError<'a> {
    code: CapturedCode,
    message: Option<Cow<'a, str>>,
}

impl WireEnvelope<'_> {
    fn into_business_error(self) -> Option<BusinessError> {
        match self {
            Self::Nested { error, request_id } => Some(BusinessError {
                code: Some(error.code.into_value()),
                message: error
                    .message
                    .map_or_else(default_error_message, Cow::into_owned),
                request_id: request_id.map(Cow::into_owned),
            }),
            Self::Flat {
                code,
                message,
                request_id,
            } if !code.is_success() => Some(BusinessError {
                code: Some(code.into_value()),
                message: message.map_or_else(default_error_message, Cow::into_owned),
                request_id: request_id.map(Cow::into_owned),
            }),
            Self::Flat { .. } | Self::NotEnvelope => None,
        }
    }
}

fn default_error_message() -> String {
    "API request failed".to_owned()
}

#[cfg(test)]
pub(crate) fn is_success_code(code: &Value) -> bool {
    match code {
        Value::Number(number) => matches!(number.as_u64(), Some(0 | 200)),
        Value::String(value) => matches!(value.as_str(), "0" | "200"),
        _ => false,
    }
}

/// Parse and extract a genuine business error envelope's code/message, or
/// `None` if the body is a success or is not an envelope.
#[cfg(test)]
pub fn extract_error_envelope(body: &str) -> Option<BusinessError> {
    serde_json::from_str::<WireProbe<'_>>(body)
        .ok()?
        .envelope
        .into_business_error()
}

/// Strict transport probe. This reports a duplicate reserved envelope field as
/// ambiguous while retaining the legacy nested-first/flat-fallback projection
/// covered by the decode contract tests.
pub(crate) fn probe_error_envelope(body: &str) -> ProbeOutcome {
    // RawValue's allocation-free scalar path uses serde_json's iterative
    // `ignore_value`, whose scratch buffer otherwise grows with pathological
    // array/object depth. Reject that shape before deserialization. This scan
    // understands JSON string escaping, so brackets inside strings do not
    // affect the depth.
    if exceeds_json_nesting_limit(body) {
        return ProbeOutcome::Malformed;
    }
    let Ok(probe) = serde_json::from_str::<WireProbe<'_>>(body) else {
        return ProbeOutcome::Malformed;
    };
    if probe.ambiguous_reserved_duplicate {
        return ProbeOutcome::Ambiguous;
    }
    match probe.envelope.into_business_error() {
        Some(error) => ProbeOutcome::Error(error),
        None => ProbeOutcome::Clean,
    }
}

fn exceeds_json_nesting_limit(body: &str) -> bool {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for byte in body.bytes() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'[' | b'{' => {
                depth += 1;
                if depth > MAX_JSON_NESTING_DEPTH {
                    return true;
                }
            },
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {},
        }
    }
    false
}

impl<'de> Deserialize<'de> for WireProbe<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(WireEnvelopeVisitor)
    }
}

struct WireEnvelopeVisitor;

impl WireEnvelopeVisitor {
    fn clean<'de>() -> WireProbe<'de> {
        WireProbe {
            envelope: WireEnvelope::NotEnvelope,
            ambiguous_reserved_duplicate: false,
        }
    }
}

impl<'de> Visitor<'de> for WireEnvelopeVisitor {
    type Value = WireProbe<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON response value")
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut error_seen = false;
        let mut nested_valid = true;
        let mut nested_error = None;

        let mut code_seen = false;
        let mut message_seen = false;
        let mut flat_valid = true;
        let mut code = None;
        let mut message = None;

        let mut request_id_seen = false;
        let mut request_id_valid = true;
        let mut request_id = None;
        let mut ambiguous_reserved_duplicate = false;

        while let Some(field) = object.next_key::<EnvelopeField>()? {
            match field {
                EnvelopeField::Error if error_seen => {
                    nested_valid = false;
                    ambiguous_reserved_duplicate = true;
                    object.next_value::<IgnoredAny>()?;
                },
                EnvelopeField::Error => {
                    error_seen = true;
                    match object.next_value::<ErrorCandidate<'de>>()? {
                        ErrorCandidate::Valid {
                            error,
                            ambiguous_reserved_duplicate: nested_ambiguous,
                        } => {
                            nested_error = Some(error);
                            ambiguous_reserved_duplicate |= nested_ambiguous;
                        },
                        ErrorCandidate::Invalid {
                            ambiguous_reserved_duplicate: nested_ambiguous,
                        } => {
                            nested_valid = false;
                            ambiguous_reserved_duplicate |= nested_ambiguous;
                        },
                    }
                },
                EnvelopeField::Code if code_seen => {
                    flat_valid = false;
                    ambiguous_reserved_duplicate = true;
                    object.next_value::<IgnoredAny>()?;
                },
                EnvelopeField::Code => {
                    code_seen = true;
                    code = Some(object.next_value::<CapturedCode>()?);
                },
                EnvelopeField::Message if message_seen => {
                    flat_valid = false;
                    ambiguous_reserved_duplicate = true;
                    object.next_value::<IgnoredAny>()?;
                },
                EnvelopeField::Message => {
                    message_seen = true;
                    match object.next_value::<CapturedString<'de>>()? {
                        CapturedString::Text(value) => message = Some(value),
                        CapturedString::Null => {},
                        CapturedString::Invalid => flat_valid = false,
                    }
                },
                EnvelopeField::RequestId if request_id_seen => {
                    request_id_valid = false;
                    ambiguous_reserved_duplicate = true;
                    object.next_value::<IgnoredAny>()?;
                },
                EnvelopeField::RequestId => {
                    request_id_seen = true;
                    if let CapturedString::Text(value) =
                        object.next_value::<CapturedString<'de>>()?
                    {
                        request_id = Some(value);
                    }
                },
                EnvelopeField::Other => {
                    object.next_value::<IgnoredAny>()?;
                },
            }
        }

        let envelope = if !request_id_valid {
            WireEnvelope::NotEnvelope
        } else if error_seen
            && nested_valid
            && let Some(error) = nested_error
        {
            WireEnvelope::Nested { error, request_id }
        } else if flat_valid
            && code_seen
            && let Some(code) = code
        {
            WireEnvelope::Flat {
                code,
                message,
                request_id,
            }
        } else {
            WireEnvelope::NotEnvelope
        };
        Ok(WireProbe {
            envelope,
            ambiguous_reserved_duplicate,
        })
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(Self::clean())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(Self::clean())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(Self::clean())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(Self::clean())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(Self::clean())
    }

    fn visit_borrowed_str<E>(self, _value: &'de str) -> Result<Self::Value, E> {
        Ok(Self::clean())
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(Self::clean())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Self::clean())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Self::clean())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        WireProbe::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        Ok(Self::clean())
    }
}

enum EnvelopeField {
    Error,
    Code,
    Message,
    RequestId,
    Other,
}

impl<'de> Deserialize<'de> for EnvelopeField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(EnvelopeFieldVisitor)
    }
}

struct EnvelopeFieldVisitor;

impl Visitor<'_> for EnvelopeFieldVisitor {
    type Value = EnvelopeField;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object key")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(match value {
            "error" => EnvelopeField::Error,
            "code" => EnvelopeField::Code,
            "message" => EnvelopeField::Message,
            "request_id" => EnvelopeField::RequestId,
            _ => EnvelopeField::Other,
        })
    }

    fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(value)
    }
}

enum ErrorCandidate<'a> {
    Valid {
        error: WireError<'a>,
        ambiguous_reserved_duplicate: bool,
    },
    Invalid {
        ambiguous_reserved_duplicate: bool,
    },
}

impl<'de> Deserialize<'de> for ErrorCandidate<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ErrorCandidateVisitor)
    }
}

struct ErrorCandidateVisitor;

impl<'de> Visitor<'de> for ErrorCandidateVisitor {
    type Value = ErrorCandidate<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a nested JSON error object")
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut code_seen = false;
        let mut message_seen = false;
        let mut valid = true;
        let mut code = None;
        let mut message = None;
        let mut ambiguous_reserved_duplicate = false;

        while let Some(field) = object.next_key::<NestedErrorField>()? {
            match field {
                NestedErrorField::Code if code_seen => {
                    valid = false;
                    ambiguous_reserved_duplicate = true;
                    object.next_value::<IgnoredAny>()?;
                },
                NestedErrorField::Code => {
                    code_seen = true;
                    code = Some(object.next_value::<CapturedCode>()?);
                },
                NestedErrorField::Message if message_seen => {
                    valid = false;
                    ambiguous_reserved_duplicate = true;
                    object.next_value::<IgnoredAny>()?;
                },
                NestedErrorField::Message => {
                    message_seen = true;
                    match object.next_value::<CapturedString<'de>>()? {
                        CapturedString::Text(value) => message = Some(value),
                        CapturedString::Null => {},
                        CapturedString::Invalid => valid = false,
                    }
                },
                NestedErrorField::Other => {
                    object.next_value::<IgnoredAny>()?;
                },
            }
        }

        if valid
            && code_seen
            && let Some(code) = code
        {
            Ok(ErrorCandidate::Valid {
                error: WireError { code, message },
                ambiguous_reserved_duplicate,
            })
        } else {
            Ok(ErrorCandidate::Invalid {
                ambiguous_reserved_duplicate,
            })
        }
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(invalid_error_candidate())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(invalid_error_candidate())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(invalid_error_candidate())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(invalid_error_candidate())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(invalid_error_candidate())
    }

    fn visit_borrowed_str<E>(self, _value: &'de str) -> Result<Self::Value, E> {
        Ok(invalid_error_candidate())
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(invalid_error_candidate())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(invalid_error_candidate())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(invalid_error_candidate())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        ErrorCandidate::deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        Ok(invalid_error_candidate())
    }
}

fn invalid_error_candidate<'a>() -> ErrorCandidate<'a> {
    ErrorCandidate::Invalid {
        ambiguous_reserved_duplicate: false,
    }
}

enum NestedErrorField {
    Code,
    Message,
    Other,
}

impl<'de> Deserialize<'de> for NestedErrorField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_identifier(NestedErrorFieldVisitor)
    }
}

struct NestedErrorFieldVisitor;

impl Visitor<'_> for NestedErrorFieldVisitor {
    type Value = NestedErrorField;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a nested error object key")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(match value {
            "code" => NestedErrorField::Code,
            "message" => NestedErrorField::Message,
            _ => NestedErrorField::Other,
        })
    }

    fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(value)
    }
}

/// Allocation-bounded projection of an open-format provider business code.
///
/// Scalar values used by the provider's documented code mapping remain exact.
/// Composite values retain only their JSON shape because transport diagnostics
/// expose them as `<array>` or `<object>` regardless of their contents. Long
/// numeric/string literals similarly become fixed, non-numeric sentinels
/// instead of copying attacker-controlled response data into the retained
/// error.
enum CapturedCode {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    ElidedString,
    ElidedNumber,
    Array,
    Object,
}

impl CapturedCode {
    fn is_success(&self) -> bool {
        match self {
            Self::Number(number) => matches!(number.as_u64(), Some(0 | 200)),
            Self::String(value) => matches!(value.as_str(), "0" | "200"),
            Self::Null
            | Self::Bool(_)
            | Self::ElidedString
            | Self::ElidedNumber
            | Self::Array
            | Self::Object => false,
        }
    }

    fn into_value(self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Bool(value) => Value::Bool(value),
            Self::Number(value) => Value::Number(value),
            Self::String(value) => Value::String(value),
            Self::ElidedString => Value::String(ELIDED_CODE_STRING.to_owned()),
            Self::ElidedNumber => Value::String(ELIDED_CODE_NUMBER.to_owned()),
            Self::Array => Value::Array(Vec::new()),
            Self::Object => Value::Object(serde_json::Map::new()),
        }
    }
}

impl<'de> Deserialize<'de> for CapturedCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = <&'de RawValue>::deserialize(deserializer)?;
        capture_raw_code(raw.get()).map_err(de::Error::custom)
    }
}

fn capture_raw_code(raw: &str) -> serde_json::Result<CapturedCode> {
    let raw = raw.trim();
    match raw.as_bytes().first().copied() {
        Some(b'n') => Ok(CapturedCode::Null),
        Some(b't') => Ok(CapturedCode::Bool(true)),
        Some(b'f') => Ok(CapturedCode::Bool(false)),
        Some(b'[') => Ok(CapturedCode::Array),
        Some(b'{') => Ok(CapturedCode::Object),
        Some(b'"') if raw.len() > MAX_CAPTURED_CODE_LITERAL_BYTES => Ok(CapturedCode::ElidedString),
        Some(b'"') => {
            let value = serde_json::from_str::<String>(raw)?;
            Ok(if value.len() <= MAX_CAPTURED_CODE_BYTES {
                CapturedCode::String(value)
            } else {
                CapturedCode::ElidedString
            })
        },
        Some(b'-' | b'0'..=b'9') if raw.len() > MAX_CAPTURED_NUMERIC_LITERAL_BYTES => {
            Ok(CapturedCode::ElidedNumber)
        },
        Some(b'-' | b'0'..=b'9') => serde_json::from_str::<Number>(raw).map(CapturedCode::Number),
        _ => serde_json::from_str::<Value>(raw).map(|_| CapturedCode::Null),
    }
}

#[cfg(test)]
#[allow(dead_code)] // This file is also included by a harness-free allocation bench.
pub(super) fn project_legacy_code_for_transport_observation(code: Value) -> Value {
    match code {
        Value::String(value) if value.len() > MAX_CAPTURED_CODE_BYTES => {
            Value::String(ELIDED_CODE_STRING.to_owned())
        },
        Value::Array(_) => Value::Array(Vec::new()),
        Value::Object(_) => Value::Object(serde_json::Map::new()),
        scalar => scalar,
    }
}

enum CapturedString<'a> {
    Null,
    Text(Cow<'a, str>),
    Invalid,
}

impl<'de> Deserialize<'de> for CapturedString<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(CapturedStringVisitor)
    }
}

struct CapturedStringVisitor;

impl<'de> Visitor<'de> for CapturedStringVisitor {
    type Value = CapturedString<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON string, null, or ignored value")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(CapturedString::Text(Cow::Owned(value.to_owned())))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(CapturedString::Text(Cow::Borrowed(value)))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(CapturedString::Text(Cow::Owned(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(CapturedString::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(CapturedString::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        CapturedString::deserialize(deserializer)
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(CapturedString::Invalid)
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(CapturedString::Invalid)
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(CapturedString::Invalid)
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(CapturedString::Invalid)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        Ok(CapturedString::Invalid)
    }

    fn visit_map<A>(self, mut object: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while object.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(CapturedString::Invalid)
    }
}

#[cfg(test)]
#[allow(dead_code)] // The harness-free allocation bench compiles this module with `cfg(test)`.
mod tests {
    use super::*;

    const LARGE_NUMERIC_VALUES: usize = 1024 * 1024;

    fn error_code(body: &str) -> Value {
        match probe_error_envelope(body) {
            ProbeOutcome::Error(BusinessError {
                code: Some(code), ..
            }) => code,
            _ => panic!("expected a business-error envelope"),
        }
    }

    fn numeric_array_envelope(prefix: &str, suffix: &str) -> String {
        let mut body =
            String::with_capacity(prefix.len() + LARGE_NUMERIC_VALUES * 2 + suffix.len());
        body.push_str(prefix);
        for index in 0..LARGE_NUMERIC_VALUES {
            if index != 0 {
                body.push(',');
            }
            body.push('0');
        }
        body.push_str(suffix);
        assert!(body.len() >= 2 * 1024 * 1024);
        body
    }

    #[test]
    fn captured_code_preserves_scalar_business_semantics() {
        for (wire_code, expected) in [
            ("1302", serde_json::json!(1302)),
            ("-7", serde_json::json!(-7)),
            ("1.25", serde_json::json!(1.25)),
            ("true", serde_json::json!(true)),
            ("null", Value::Null),
            (r#""UPSTREAM_BUSY""#, serde_json::json!("UPSTREAM_BUSY")),
        ] {
            let body = format!(r#"{{"code":{wire_code}}}"#);
            assert_eq!(error_code(&body), expected, "{wire_code}");
        }

        for wire_code in ["0", "200", r#""0""#, r#""200""#] {
            let body = format!(r#"{{"code":{wire_code}}}"#);
            assert!(matches!(probe_error_envelope(&body), ProbeOutcome::Clean));
        }

        assert!(matches!(
            probe_error_envelope(r#"{"code":"\u0032\u0030\u0030"}"#),
            ProbeOutcome::Clean
        ));
    }

    #[test]
    fn captured_code_retains_only_composite_shape() {
        assert_eq!(
            error_code(r#"{"code":[1,{"secret":"value"}]}"#),
            serde_json::json!([])
        );
        assert_eq!(
            error_code(r#"{"code":{"secret":"value","nested":[1,2]}}"#),
            serde_json::json!({})
        );
        assert_eq!(
            error_code(r#"{"error":{"code":[1,{"secret":"value"}]}}"#),
            serde_json::json!([])
        );
        assert_eq!(
            error_code(r#"{"error":{"code":{"secret":"value"}}}"#),
            serde_json::json!({})
        );
    }

    #[test]
    fn top_level_and_nested_large_numeric_code_arrays_use_shape_sentinels() {
        let flat = numeric_array_envelope(r#"{"code":["#, "]}");
        assert_eq!(error_code(&flat), serde_json::json!([]));

        let nested = numeric_array_envelope(r#"{"error":{"code":["#, "]}}");
        assert_eq!(error_code(&nested), serde_json::json!([]));
    }

    #[test]
    fn overlong_code_strings_use_a_fixed_non_numeric_sentinel() {
        let original = "7".repeat(MAX_CAPTURED_CODE_BYTES + 1);
        let body = format!(r#"{{"code":"{original}"}}"#);
        assert_eq!(
            error_code(&body),
            Value::String(ELIDED_CODE_STRING.to_owned())
        );
        assert!(ELIDED_CODE_STRING.parse::<u16>().is_err());
        assert!(!ELIDED_CODE_STRING.contains(&original));
    }

    #[test]
    fn overlong_numeric_codes_use_a_fixed_non_numeric_sentinel() {
        let original = "7".repeat(MAX_CAPTURED_NUMERIC_LITERAL_BYTES + 1);
        let body = format!(r#"{{"code":{original}}}"#);
        assert_eq!(
            error_code(&body),
            Value::String(ELIDED_CODE_NUMBER.to_owned())
        );
        assert!(ELIDED_CODE_NUMBER.parse::<u16>().is_err());
        assert!(!ELIDED_CODE_NUMBER.contains(&original));
    }

    #[test]
    fn malformed_is_distinct_from_valid_non_envelopes() {
        for body in ["null", "true", "17", r#""text""#, "[]", r#"[0,{"code":1}]"#] {
            assert!(
                matches!(probe_error_envelope(body), ProbeOutcome::Clean),
                "{body}"
            );
        }

        for body in [r#"{"code":1302"#, r#"{"code":1302} trailing"#, "["] {
            assert!(
                matches!(probe_error_envelope(body), ProbeOutcome::Malformed),
                "{body}"
            );
        }

        let too_deep = format!(
            "{{\"code\":{}0{}}}",
            "[".repeat(MAX_JSON_NESTING_DEPTH),
            "]".repeat(MAX_JSON_NESTING_DEPTH)
        );
        assert!(matches!(
            probe_error_envelope(&too_deep),
            ProbeOutcome::Malformed
        ));

        let brackets_in_string = format!(
            r#"{{"message":"{}"}}"#,
            "[".repeat(MAX_JSON_NESTING_DEPTH + 1)
        );
        assert!(matches!(
            probe_error_envelope(&brackets_in_string),
            ProbeOutcome::Clean
        ));
    }
}
