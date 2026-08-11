//! Strict scalar query serialization for internal HTTP operations.
//!
//! Request structs retain their Serde field order while this boundary rejects
//! nested shapes and converts every failure into one redacted SDK error.

use crate::{ZaiError, ZaiResult};
use serde::Serialize;
use serde::ser::{Impossible, SerializeMap, SerializeStruct};

const INVALID_QUERY_MESSAGE: &str =
    "query parameters must be a flat object with unique string keys and scalar values";

#[derive(Debug, Clone, Copy)]
struct QueryEncodeError;

impl std::fmt::Display for QueryEncodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(INVALID_QUERY_MESSAGE)
    }
}

impl std::error::Error for QueryEncodeError {}

impl serde::ser::Error for QueryEncodeError {
    fn custom<T>(_message: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self
    }
}

fn invalid_query_error() -> ZaiError {
    ZaiError::ApiError {
        code: crate::client::error::codes::SDK_CONFIG,
        message: INVALID_QUERY_MESSAGE.to_owned(),
    }
}

#[derive(Debug)]
enum ScalarToken {
    Omit,
    Value(String),
}

#[derive(Clone, Copy)]
enum ScalarPosition {
    Key,
    Value,
}

#[derive(Clone, Copy)]
struct ScalarSerializer {
    position: ScalarPosition,
}

impl ScalarSerializer {
    const fn key() -> Self {
        Self {
            position: ScalarPosition::Key,
        }
    }

    const fn value() -> Self {
        Self {
            position: ScalarPosition::Value,
        }
    }

    fn scalar(self, value: String) -> Result<ScalarToken, QueryEncodeError> {
        match self.position {
            ScalarPosition::Key => Err(QueryEncodeError),
            ScalarPosition::Value => Ok(ScalarToken::Value(value)),
        }
    }

    const fn accepts_wrappers(self) -> bool {
        matches!(self.position, ScalarPosition::Value)
    }
}

type RejectedScalar = Impossible<ScalarToken, QueryEncodeError>;

impl serde::Serializer for ScalarSerializer {
    type Ok = ScalarToken;
    type Error = QueryEncodeError;
    type SerializeSeq = RejectedScalar;
    type SerializeTuple = RejectedScalar;
    type SerializeTupleStruct = RejectedScalar;
    type SerializeTupleVariant = RejectedScalar;
    type SerializeMap = RejectedScalar;
    type SerializeStruct = RejectedScalar;
    type SerializeStructVariant = RejectedScalar;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        if value.is_finite() {
            self.scalar(value.to_string())
        } else {
            Err(QueryEncodeError)
        }
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        if value.is_finite() {
            self.scalar(value.to_string())
        } else {
            Err(QueryEncodeError)
        }
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        self.scalar(value.to_string())
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        Ok(ScalarToken::Value(value.to_owned()))
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        if self.accepts_wrappers() {
            Ok(ScalarToken::Omit)
        } else {
            Err(QueryEncodeError)
        }
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        if self.accepts_wrappers() {
            value.serialize(self)
        } else {
            Err(QueryEncodeError)
        }
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.scalar(variant.to_owned())
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        if self.accepts_wrappers() {
            value.serialize(self)
        } else {
            Err(QueryEncodeError)
        }
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(QueryEncodeError)
    }

    fn serialize_seq(self, _length: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_tuple(self, _length: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_map(self, _length: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(QueryEncodeError)
    }

    fn collect_str<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: std::fmt::Display + ?Sized,
    {
        Err(QueryEncodeError)
    }
}

#[derive(Default)]
struct QueryFields {
    pairs: Vec<(String, String)>,
    seen: Vec<String>,
}

impl QueryFields {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            pairs: Vec::with_capacity(capacity),
            seen: Vec::with_capacity(capacity),
        }
    }

    fn register(&mut self, key: &str) -> Result<(), QueryEncodeError> {
        if self.seen.iter().any(|existing| existing == key) {
            return Err(QueryEncodeError);
        }
        self.seen.push(key.to_owned());
        Ok(())
    }

    fn push(&mut self, key: String, value: ScalarToken) {
        if let ScalarToken::Value(value) = value {
            self.pairs.push((key, value));
        }
    }
}

struct QueryMapSerializer {
    fields: QueryFields,
    pending_key: Option<String>,
}

impl SerializeMap for QueryMapSerializer {
    type Ok = Vec<(String, String)>;
    type Error = QueryEncodeError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        if self.pending_key.is_some() {
            return Err(QueryEncodeError);
        }
        let ScalarToken::Value(key) = key.serialize(ScalarSerializer::key())? else {
            return Err(QueryEncodeError);
        };
        self.fields.register(&key)?;
        self.pending_key = Some(key);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        let key = self.pending_key.take().ok_or(QueryEncodeError)?;
        let value = value.serialize(ScalarSerializer::value())?;
        self.fields.push(key, value);
        Ok(())
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: Serialize + ?Sized,
        V: Serialize + ?Sized,
    {
        self.serialize_key(key)?;
        self.serialize_value(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        if self.pending_key.is_some() {
            Err(QueryEncodeError)
        } else {
            Ok(self.fields.pairs)
        }
    }
}

struct QueryStructSerializer {
    fields: QueryFields,
}

impl SerializeStruct for QueryStructSerializer {
    type Ok = Vec<(String, String)>;
    type Error = QueryEncodeError;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.fields.register(key)?;
        let value = value.serialize(ScalarSerializer::value())?;
        self.fields.push(key.to_owned(), value);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.fields.pairs)
    }
}

struct QuerySerializer;

type QueryPairs = Vec<(String, String)>;
type RejectedQuery = Impossible<QueryPairs, QueryEncodeError>;

impl serde::Serializer for QuerySerializer {
    type Ok = QueryPairs;
    type Error = QueryEncodeError;
    type SerializeSeq = RejectedQuery;
    type SerializeTuple = RejectedQuery;
    type SerializeTupleStruct = RejectedQuery;
    type SerializeTupleVariant = RejectedQuery;
    type SerializeMap = QueryMapSerializer;
    type SerializeStruct = QueryStructSerializer;
    type SerializeStructVariant = RejectedQuery;

    fn serialize_bool(self, _value: bool) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_i8(self, _value: i8) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_i16(self, _value: i16) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_i32(self, _value: i32) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_i64(self, _value: i64) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_i128(self, _value: i128) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_u8(self, _value: u8) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_u16(self, _value: u16) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_u32(self, _value: u32) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_u64(self, _value: u64) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_u128(self, _value: u128) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_f32(self, _value: f32) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_f64(self, _value: f64) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_char(self, _value: char) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_str(self, _value: &str) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(QueryEncodeError)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(QueryEncodeError)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        Err(QueryEncodeError)
    }

    fn serialize_seq(self, _length: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_tuple(self, _length: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(QueryEncodeError)
    }

    fn serialize_map(self, length: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(QueryMapSerializer {
            fields: QueryFields::with_capacity(length.unwrap_or(0)),
            pending_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        length: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(QueryStructSerializer {
            fields: QueryFields::with_capacity(length),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _length: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(QueryEncodeError)
    }

    fn collect_str<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: std::fmt::Display + ?Sized,
    {
        Err(QueryEncodeError)
    }
}

pub(crate) fn encode<T>(query: &T) -> ZaiResult<QueryPairs>
where
    T: Serialize + ?Sized,
{
    query
        .serialize(QuerySerializer)
        .map_err(|_| invalid_query_error())
}

pub(crate) fn extend<T>(pairs: &mut QueryPairs, query: &T) -> ZaiResult<()>
where
    T: Serialize + ?Sized,
{
    let encoded = encode(query)?;
    if encoded
        .iter()
        .any(|(key, _)| pairs.iter().any(|(existing, _)| existing == key))
    {
        return Err(invalid_query_error());
    }
    pairs.extend(encoded);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Serialize)]
    enum QueryMode {
        #[serde(rename = "renamed-mode")]
        Renamed,
    }

    #[derive(Serialize)]
    struct Cursor<'a>(&'a str);

    #[derive(Serialize)]
    struct EveryScalar<'a> {
        borrowed: &'a str,
        owned: String,
        character: char,
        enabled: bool,
        i8_value: i8,
        i16_value: i16,
        i32_value: i32,
        i64_value: i64,
        i128_value: i128,
        isize_value: isize,
        u8_value: u8,
        u16_value: u16,
        u32_value: u32,
        u64_value: u64,
        u128_value: u128,
        usize_value: usize,
        f32_value: f32,
        f64_value: f64,
        mode: QueryMode,
        cursor: Cursor<'a>,
        optional: Option<u16>,
        omitted: Option<u16>,
    }

    fn invalid_query<T>(query: &T) -> ZaiError
    where
        T: Serialize + ?Sized,
    {
        match encode(query) {
            Ok(_) => panic!("query unexpectedly encoded"),
            Err(error) => error,
        }
    }

    fn assert_invalid_query<T>(query: &T)
    where
        T: Serialize + ?Sized,
    {
        let error = invalid_query(query);
        match &error {
            ZaiError::ApiError { code, message } => {
                assert_eq!(*code, crate::client::error::codes::SDK_CONFIG);
                assert_eq!(message, INVALID_QUERY_MESSAGE);
            },
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn scalar_query_preserves_struct_order_and_omits_none() {
        let query = EveryScalar {
            borrowed: "text",
            owned: "owned".to_owned(),
            character: '中',
            enabled: true,
            i8_value: -8,
            i16_value: -16,
            i32_value: -32,
            i64_value: -64,
            i128_value: -128,
            isize_value: -9,
            u8_value: 8,
            u16_value: 16,
            u32_value: 32,
            u64_value: 64,
            u128_value: 128,
            usize_value: 9,
            f32_value: 1.25,
            f64_value: -2.5,
            mode: QueryMode::Renamed,
            cursor: Cursor("opaque-cursor"),
            optional: Some(7),
            omitted: None,
        };

        let encoded = encode(&query).unwrap();

        assert_eq!(
            encoded,
            vec![
                ("borrowed".to_owned(), "text".to_owned()),
                ("owned".to_owned(), "owned".to_owned()),
                ("character".to_owned(), "中".to_owned()),
                ("enabled".to_owned(), "true".to_owned()),
                ("i8_value".to_owned(), "-8".to_owned()),
                ("i16_value".to_owned(), "-16".to_owned()),
                ("i32_value".to_owned(), "-32".to_owned()),
                ("i64_value".to_owned(), "-64".to_owned()),
                ("i128_value".to_owned(), "-128".to_owned()),
                ("isize_value".to_owned(), "-9".to_owned()),
                ("u8_value".to_owned(), "8".to_owned()),
                ("u16_value".to_owned(), "16".to_owned()),
                ("u32_value".to_owned(), "32".to_owned()),
                ("u64_value".to_owned(), "64".to_owned()),
                ("u128_value".to_owned(), "128".to_owned()),
                ("usize_value".to_owned(), "9".to_owned()),
                ("f32_value".to_owned(), "1.25".to_owned()),
                ("f64_value".to_owned(), "-2.5".to_owned()),
                ("mode".to_owned(), "renamed-mode".to_owned()),
                ("cursor".to_owned(), "opaque-cursor".to_owned()),
                ("optional".to_owned(), "7".to_owned()),
            ]
        );
    }

    #[test]
    fn string_keyed_map_is_supported_and_non_string_keys_are_rejected() {
        let query = BTreeMap::from([
            ("alpha".to_owned(), Some(1_u8)),
            ("beta".to_owned(), None),
            ("gamma".to_owned(), Some(3_u8)),
        ]);
        let encoded = encode(&query).unwrap();
        assert_eq!(
            encoded,
            vec![
                ("alpha".to_owned(), "1".to_owned()),
                ("gamma".to_owned(), "3".to_owned()),
            ]
        );

        assert_invalid_query(&BTreeMap::from([(7_u8, "private-value")]));
        assert_invalid_query(&BTreeMap::from([('x', "private-value")]));
    }

    #[derive(Serialize)]
    struct FloatQuery {
        value: f64,
    }

    #[derive(Serialize)]
    struct Float32Query {
        value: f32,
    }

    #[test]
    fn non_finite_floats_are_rejected() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_invalid_query(&FloatQuery { value });
        }
        for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_invalid_query(&Float32Query { value });
        }
    }

    #[derive(Serialize)]
    struct Inner {
        value: u8,
    }

    #[derive(Serialize)]
    struct Nested<T> {
        private_field_name: T,
    }

    struct BytesValue;

    impl Serialize for BytesValue {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_bytes(b"private-bytes")
        }
    }

    struct CollectedValue;

    impl Serialize for CollectedValue {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.collect_str("private-collected-value")
        }
    }

    #[derive(Serialize)]
    enum PayloadEnum {
        Newtype(u8),
    }

    #[test]
    fn nested_and_non_scalar_values_are_rejected() {
        assert_invalid_query(&Nested {
            private_field_name: Inner { value: 1 },
        });
        assert_invalid_query(&Nested {
            private_field_name: BTreeMap::from([("key", 1_u8)]),
        });
        assert_invalid_query(&Nested {
            private_field_name: vec![1_u8, 2],
        });
        assert_invalid_query(&Nested {
            private_field_name: (1_u8, 2_u8),
        });
        assert_invalid_query(&Nested {
            private_field_name: BytesValue,
        });
        assert_invalid_query(&Nested {
            private_field_name: (),
        });
        assert_invalid_query(&Nested {
            private_field_name: PayloadEnum::Newtype(1),
        });
        assert_invalid_query(&Nested {
            private_field_name: CollectedValue,
        });
    }

    #[test]
    fn top_level_must_be_a_struct_or_map() {
        assert_invalid_query("private-scalar");
        assert_invalid_query(&["private-sequence"]);
        assert_invalid_query(&Some("private-option"));
    }

    struct DuplicateMap;

    impl Serialize for DuplicateMap {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("duplicate", &1_u8)?;
            map.serialize_entry("duplicate", &2_u8)?;
            map.end()
        }
    }

    struct DuplicateStruct;

    impl Serialize for DuplicateStruct {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut fields = serializer.serialize_struct("DuplicateStruct", 2)?;
            fields.serialize_field("duplicate", &1_u8)?;
            fields.serialize_field("duplicate", &2_u8)?;
            fields.end()
        }
    }

    #[test]
    fn duplicate_keys_are_rejected_within_and_across_calls() {
        assert_invalid_query(&DuplicateMap);
        assert_invalid_query(&DuplicateStruct);

        #[derive(Serialize)]
        struct First {
            duplicate: u8,
        }
        #[derive(Serialize)]
        struct Second {
            duplicate: u16,
        }

        let mut pairs = encode(&First { duplicate: 1 }).unwrap();
        let error = extend(&mut pairs, &Second { duplicate: 2 }).unwrap_err();
        assert!(matches!(
            error,
            ZaiError::ApiError {
                code: crate::client::error::codes::SDK_CONFIG,
                ..
            }
        ));
    }

    struct LeakySerializeError;

    impl Serialize for LeakySerializeError {
        fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(<S::Error as serde::ser::Error>::custom(
                "private-serializer-message",
            ))
        }
    }

    #[test]
    fn serialization_errors_are_fixed_and_redacted() {
        let error = invalid_query(&Nested {
            private_field_name: LeakySerializeError,
        });
        let rendered = format!("{error:?}\n{error}");
        assert!(!rendered.contains("private-serializer-message"));
        assert!(!rendered.contains("private_field_name"));
        assert_eq!(
            rendered.matches(INVALID_QUERY_MESSAGE).count(),
            2,
            "Debug and Display should both use the fixed message"
        );
    }
}
