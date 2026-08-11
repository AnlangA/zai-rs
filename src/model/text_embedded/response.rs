use std::fmt;

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{Error as _, Visitor},
};

/// Top-level response from the embeddings endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct EmbeddingResponse {
    /// Model that produced the embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Object kind of the response envelope (currently `list`).
    ///
    /// A future unknown string is exposed as `None` while the model, data, and
    /// usage payload remains available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<ResponseObjectKind>,
    /// Per-input embedding vectors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<EmbeddingData>>,
    /// Token-usage statistics for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<EmbeddingUsage>,
}

#[derive(Deserialize)]
struct EmbeddingResponseWire {
    model: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_response_object_kind"
    )]
    object: Option<ResponseObjectKind>,
    data: Option<Vec<EmbeddingData>>,
    usage: Option<EmbeddingUsage>,
}

enum ResponseObjectKindWire {
    List,
    Unknown,
}

impl<'de> Deserialize<'de> for ResponseObjectKindWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MarkerVisitor;

        impl Visitor<'_> for MarkerVisitor {
            type Value = ResponseObjectKindWire;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an embedding response object string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "list" => ResponseObjectKindWire::List,
                    _ => ResponseObjectKindWire::Unknown,
                })
            }
        }

        deserializer.deserialize_str(MarkerVisitor)
    }
}

fn deserialize_optional_response_object_kind<'de, D>(
    deserializer: D,
) -> Result<Option<ResponseObjectKind>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        match Option::<ResponseObjectKindWire>::deserialize(deserializer)? {
            Some(ResponseObjectKindWire::List) => Some(ResponseObjectKind::List),
            Some(ResponseObjectKindWire::Unknown) | None => None,
        },
    )
}

impl<'de> Deserialize<'de> for EmbeddingResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EmbeddingResponseWire::deserialize(deserializer)?;
        if wire.model.is_none()
            && wire.object.is_none()
            && wire.data.is_none()
            && wire.usage.is_none()
        {
            return Err(D::Error::custom(
                "embedding response contained no documented fields",
            ));
        }
        Ok(Self {
            model: wire.model,
            object: wire.object,
            data: wire.data,
            usage: wire.usage,
        })
    }
}

/// Top-level object kind returned by the embeddings endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseObjectKind {
    /// A list of embedding items.
    List,
}

/// A single embedding vector with its index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingData {
    /// Position of this input within the request batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
    /// Object kind (currently `embedding`).
    ///
    /// A future unknown string is exposed as `None` while the index and vector
    /// payload remains available.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_embedding_object_kind",
        skip_serializing_if = "Option::is_none"
    )]
    pub object: Option<EmbeddingObjectKind>,
    /// The embedding vector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

/// Object kind for an embedding item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingObjectKind {
    /// Embedding object.
    Embedding,
}

enum EmbeddingObjectKindWire {
    Embedding,
    Unknown,
}

impl<'de> Deserialize<'de> for EmbeddingObjectKindWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MarkerVisitor;

        impl Visitor<'_> for MarkerVisitor {
            type Value = EmbeddingObjectKindWire;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an embedding item object string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "embedding" => EmbeddingObjectKindWire::Embedding,
                    _ => EmbeddingObjectKindWire::Unknown,
                })
            }
        }

        deserializer.deserialize_str(MarkerVisitor)
    }
}

fn deserialize_optional_embedding_object_kind<'de, D>(
    deserializer: D,
) -> Result<Option<EmbeddingObjectKind>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        match Option::<EmbeddingObjectKindWire>::deserialize(deserializer)? {
            Some(EmbeddingObjectKindWire::Embedding) => Some(EmbeddingObjectKind::Embedding),
            Some(EmbeddingObjectKindWire::Unknown) | None => None,
        },
    )
}

/// Token-usage statistics for an embeddings request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    /// Number of prompt tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    /// Number of completion tokens (typically 0 for embeddings).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u64>,
    /// Total tokens for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_requires_one_documented_non_null_field() {
        assert!(serde_json::from_str::<EmbeddingResponse>("{}").is_err());
        assert!(serde_json::from_str::<EmbeddingResponse>(r#"{"data":null}"#).is_err());
        assert!(serde_json::from_str::<EmbeddingResponse>(r#"{"data":[]}"#).is_ok());
    }

    #[test]
    fn nested_properties_follow_their_optional_schema() {
        let item: EmbeddingData = serde_json::from_str("{}").unwrap();
        assert!(item.index.is_none());
        assert!(item.object.is_none());
        assert!(item.embedding.is_none());
        let item: EmbeddingData = serde_json::from_str(r#"{"object":null}"#).unwrap();
        assert!(item.object.is_none());
        let usage: EmbeddingUsage = serde_json::from_str("{}").unwrap();
        assert!(usage.prompt_tokens.is_none());
    }

    #[test]
    fn known_embedding_markers_and_public_serde_stay_unchanged() {
        let response: EmbeddingResponse = serde_json::from_str(
            r#"{
                "model":"embedding-3",
                "object":"list",
                "data":[{"index":2,"object":"embedding","embedding":[0.25,-0.5]}]
            }"#,
        )
        .unwrap();

        assert!(matches!(response.object, Some(ResponseObjectKind::List)));
        let item = &response.data.as_ref().unwrap()[0];
        assert!(matches!(item.object, Some(EmbeddingObjectKind::Embedding)));
        assert_eq!(
            serde_json::to_string(&ResponseObjectKind::List).unwrap(),
            r#""list""#
        );
        assert_eq!(
            serde_json::to_string(&EmbeddingObjectKind::Embedding).unwrap(),
            r#""embedding""#
        );
        assert!(serde_json::from_str::<ResponseObjectKind>(r#""future-list""#).is_err());
        assert!(serde_json::from_str::<EmbeddingObjectKind>(r#""future-embedding""#).is_err());
    }

    #[test]
    fn unknown_embedding_markers_are_ignored_without_losing_useful_payload() {
        let response: EmbeddingResponse = serde_json::from_str(
            r#"{
                "model":"embedding-future",
                "object":"future-list",
                "data":[{
                    "index":7,
                    "object":"future-embedding",
                    "embedding":[0.25,-0.5]
                }]
            }"#,
        )
        .unwrap();

        assert_eq!(response.model.as_deref(), Some("embedding-future"));
        assert!(response.object.is_none());
        let data = response.data.as_ref().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].index, Some(7));
        assert!(data[0].object.is_none());
        assert_eq!(data[0].embedding.as_deref(), Some(&[0.25, -0.5][..]));
    }

    #[test]
    fn embedding_marker_leniency_is_string_only_and_keeps_empty_invariant() {
        assert!(serde_json::from_str::<EmbeddingResponse>(r#"{"object":"future-list"}"#).is_err());
        assert!(serde_json::from_str::<EmbeddingResponse>(r#"{"model":"m","object":1}"#).is_err());
        assert!(
            serde_json::from_str::<EmbeddingResponse>(r#"{"model":"m","object":["list"]}"#)
                .is_err()
        );
        assert!(
            serde_json::from_str::<EmbeddingResponse>(r#"{"model":"m","object":{"kind":"list"}}"#)
                .is_err()
        );
        assert!(
            serde_json::from_str::<EmbeddingResponse>(
                r#"{"model":"m","object":{"future-list":null}}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<EmbeddingResponse>(
                r#"{"data":[{"index":0,"object":1,"embedding":[1.0]}]}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<EmbeddingResponse>(
                r#"{"data":[{"index":0,"object":["embedding"],"embedding":[1.0]}]}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<EmbeddingResponse>(
                r#"{"data":[{"index":0,"object":{"kind":"embedding"},"embedding":[1.0]}]}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<EmbeddingResponse>(
                r#"{"data":[{"index":0,"object":{"future-embedding":null},"embedding":[1.0]}]}"#
            )
            .is_err()
        );

        let missing: EmbeddingResponse = serde_json::from_str(r#"{"model":"m"}"#).unwrap();
        let null: EmbeddingResponse =
            serde_json::from_str(r#"{"model":"m","object":null}"#).unwrap();
        assert!(missing.object.is_none());
        assert!(null.object.is_none());
        assert!(matches!(
            serde_json::from_str::<EmbeddingResponse>(r#"{"object":"list"}"#)
                .unwrap()
                .object,
            Some(ResponseObjectKind::List)
        ));

        // A non-null `data` field remains a documented top-level field even
        // when its optional nested marker is the only unknown value.
        let nested_unknown_only: EmbeddingResponse =
            serde_json::from_str(r#"{"data":[{"object":"future-embedding"}]}"#).unwrap();
        assert!(nested_unknown_only.data.unwrap()[0].object.is_none());
    }
}
