use std::fmt;

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{Error as _, Visitor},
};

/// Image generation response payload
///
/// Every property is optional in OpenAPI, but a successful response must carry
/// at least one documented, non-null property.
#[derive(Debug, Clone, Serialize)]
pub struct ImageResponse {
    /// Request created time, Unix timestamp (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<u64>,

    /// Array containing generated image URLs. Currently only one image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<ImageDataItem>>,

    /// Content safety related information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_filter: Option<Vec<ImageContentFilterInfo>>,
}

#[derive(Deserialize)]
struct ImageResponseWire {
    created: Option<u64>,
    data: Option<Vec<ImageDataItem>>,
    content_filter: Option<Vec<ImageContentFilterInfo>>,
}

impl<'de> Deserialize<'de> for ImageResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ImageResponseWire::deserialize(deserializer)?;
        if wire.created.is_none() && wire.data.is_none() && wire.content_filter.is_none() {
            return Err(D::Error::custom(
                "image response contained no documented fields",
            ));
        }
        Ok(Self {
            created: wire.created,
            data: wire.data,
            content_filter: wire.content_filter,
        })
    }
}

/// Single generated image item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDataItem {
    /// Image link. Temporary URL valid for ~30 days; persist if needed.
    pub url: String,
}

/// Content-safety metadata returned by image generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContentFilterInfo {
    /// Stage at which the safety filter applied.
    ///
    /// A future unknown string is exposed as `None` while the generated image
    /// and the remaining filter metadata stay available.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_filter_role",
        skip_serializing_if = "Option::is_none"
    )]
    pub role: Option<ImageContentFilterRole>,
    /// Severity from 0 (most severe) through 3 (least severe).
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_filter_level"
    )]
    pub level: Option<i32>,
}

/// Safety-filter stage defined by the image response schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageContentFilterRole {
    /// Model output.
    Assistant,
    /// Current user input.
    User,
    /// Conversation history.
    History,
}

enum ImageContentFilterRoleWire {
    Assistant,
    User,
    History,
    Unknown,
}

impl<'de> Deserialize<'de> for ImageContentFilterRoleWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FilterRoleVisitor;

        impl Visitor<'_> for FilterRoleVisitor {
            type Value = ImageContentFilterRoleWire;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an image content-filter role string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "assistant" => ImageContentFilterRoleWire::Assistant,
                    "user" => ImageContentFilterRoleWire::User,
                    "history" => ImageContentFilterRoleWire::History,
                    _ => ImageContentFilterRoleWire::Unknown,
                })
            }
        }

        deserializer.deserialize_str(FilterRoleVisitor)
    }
}

fn deserialize_optional_filter_role<'de, D>(
    deserializer: D,
) -> Result<Option<ImageContentFilterRole>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        match Option::<ImageContentFilterRoleWire>::deserialize(deserializer)? {
            Some(ImageContentFilterRoleWire::Assistant) => Some(ImageContentFilterRole::Assistant),
            Some(ImageContentFilterRoleWire::User) => Some(ImageContentFilterRole::User),
            Some(ImageContentFilterRoleWire::History) => Some(ImageContentFilterRole::History),
            Some(ImageContentFilterRoleWire::Unknown) | None => None,
        },
    )
}

fn deserialize_optional_filter_level<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    let level = Option::<i32>::deserialize(deserializer)?;
    if level.is_some_and(|level| !(0..=3).contains(&level)) {
        return Err(D::Error::custom(
            "image content-filter level must be between 0 and 3",
        ));
    }
    Ok(level)
}

// --- Getters ---
impl ImageResponse {
    /// Request created time, Unix timestamp (seconds).
    pub fn created(&self) -> Option<u64> {
        self.created
    }
    /// Generated image items (currently a single item).
    pub fn data(&self) -> Option<&[ImageDataItem]> {
        self.data.as_deref()
    }
    /// Content-safety filter results, if any.
    pub fn content_filter(&self) -> Option<&[ImageContentFilterInfo]> {
        self.content_filter.as_deref()
    }
}

impl ImageDataItem {
    /// Temporary image URL (~30 days validity).
    pub fn url(&self) -> &str {
        &self.url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_requires_one_documented_non_null_field() {
        assert!(serde_json::from_str::<ImageResponse>("{}").is_err());
        assert!(serde_json::from_str::<ImageResponse>(r#"{"data":null}"#).is_err());
        assert!(serde_json::from_str::<ImageResponse>(r#"{"data":[]}"#).is_ok());
    }

    #[test]
    fn image_url_is_required_and_filter_constraints_are_enforced() {
        assert!(serde_json::from_str::<ImageDataItem>("{}").is_err());
        assert!(serde_json::from_str::<ImageDataItem>(r#"{"url":"relative"}"#).is_ok());
        assert!(
            serde_json::from_str::<ImageContentFilterInfo>(r#"{"role":"assistant","level":3}"#)
                .is_ok()
        );
        assert!(serde_json::from_str::<ImageContentFilterInfo>(r#"{"level":4}"#).is_err());
    }

    #[test]
    fn filter_role_is_forward_compatible_only_at_the_response_field() {
        let response: ImageResponse = serde_json::from_value(serde_json::json!({
            "created": 1,
            "data": [{"url": "https://example.com/generated.png"}],
            "content_filter": [{
                "role": "future_provider_stage",
                "level": 2,
                "provider_category": "future-category"
            }]
        }))
        .expect("an additive role must not hide a generated image");

        assert_eq!(
            response
                .data()
                .and_then(|items| items.first())
                .map(ImageDataItem::url),
            Some("https://example.com/generated.png")
        );
        let filter = &response.content_filter().unwrap()[0];
        assert_eq!(filter.role, None);
        assert_eq!(filter.level, Some(2));

        for (wire, expected) in [
            ("assistant", ImageContentFilterRole::Assistant),
            ("user", ImageContentFilterRole::User),
            ("history", ImageContentFilterRole::History),
        ] {
            let filter: ImageContentFilterInfo =
                serde_json::from_value(serde_json::json!({"role": wire})).unwrap();
            assert_eq!(filter.role, Some(expected));
        }

        let missing: ImageContentFilterInfo = serde_json::from_str("{}").unwrap();
        let null: ImageContentFilterInfo = serde_json::from_str(r#"{"role":null}"#).unwrap();
        assert_eq!(missing.role, None);
        assert_eq!(null.role, None);

        for malformed in [
            serde_json::json!({"role": 1}),
            serde_json::json!({"role": true}),
            serde_json::json!({"role": []}),
            serde_json::json!({"role": {}}),
        ] {
            assert!(
                serde_json::from_value::<ImageContentFilterInfo>(malformed).is_err(),
                "non-string roles must remain malformed"
            );
        }

        assert!(
            serde_json::from_value::<ImageContentFilterRole>(serde_json::json!(
                "future_provider_stage"
            ))
            .is_err(),
            "the public enum's direct Serde contract must remain strict"
        );
        assert_eq!(
            serde_json::to_value(ImageContentFilterRole::Assistant).unwrap(),
            serde_json::json!("assistant"),
            "known public-enum serialization must remain unchanged"
        );
    }
}
