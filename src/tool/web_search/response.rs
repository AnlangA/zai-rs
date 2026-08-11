use std::fmt;

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{Error as _, Visitor},
};

/// Web search API response
#[derive(Debug, Clone, Serialize)]
pub struct WebSearchResponse {
    /// Task ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Request creation time as Unix timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<i64>,
    /// Request identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Search intent results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_intent: Option<Vec<SearchIntent>>,
    /// Search results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_result: Option<Vec<SearchResult>>,
}

#[derive(Deserialize)]
struct WebSearchResponseWire {
    id: Option<String>,
    created: Option<i64>,
    request_id: Option<String>,
    search_intent: Option<Vec<SearchIntent>>,
    search_result: Option<Vec<SearchResult>>,
}

impl<'de> Deserialize<'de> for WebSearchResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = WebSearchResponseWire::deserialize(deserializer)?;
        if wire.id.is_none()
            && wire.created.is_none()
            && wire.request_id.is_none()
            && wire.search_intent.is_none()
            && wire.search_result.is_none()
        {
            return Err(D::Error::custom(
                "web-search response contained no documented non-null fields",
            ));
        }
        Ok(Self {
            id: wire.id,
            created: wire.created,
            request_id: wire.request_id,
            search_intent: wire.search_intent,
            search_result: wire.search_result,
        })
    }
}

/// Search intent result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIntent {
    /// The search query
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// The detected intent type.
    ///
    /// A future unknown string is exposed as `None` while the query, keywords,
    /// and surrounding search results remain available.
    #[serde(
        default,
        deserialize_with = "deserialize_optional_search_intent_kind",
        skip_serializing_if = "Option::is_none"
    )]
    pub intent: Option<SearchIntentKind>,
    /// Extracted keywords
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
}

/// Search-intent classifications returned by the standalone web-search API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchIntentKind {
    /// Search the public web.
    #[serde(rename = "SEARCH_ALL")]
    SearchAll,
    /// No search is needed.
    #[serde(rename = "SEARCH_NONE")]
    SearchNone,
    /// Search unconditionally because intent detection was disabled.
    #[serde(rename = "SEARCH_ALWAYS")]
    SearchAlways,
}

enum SearchIntentKindWire {
    SearchAll,
    SearchNone,
    SearchAlways,
    Unknown,
}

impl<'de> Deserialize<'de> for SearchIntentKindWire {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct MarkerVisitor;

        impl Visitor<'_> for MarkerVisitor {
            type Value = SearchIntentKindWire;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a search intent string")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(match value {
                    "SEARCH_ALL" => SearchIntentKindWire::SearchAll,
                    "SEARCH_NONE" => SearchIntentKindWire::SearchNone,
                    "SEARCH_ALWAYS" => SearchIntentKindWire::SearchAlways,
                    _ => SearchIntentKindWire::Unknown,
                })
            }
        }

        deserializer.deserialize_str(MarkerVisitor)
    }
}

fn deserialize_optional_search_intent_kind<'de, D>(
    deserializer: D,
) -> Result<Option<SearchIntentKind>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(
        match Option::<SearchIntentKindWire>::deserialize(deserializer)? {
            Some(SearchIntentKindWire::SearchAll) => Some(SearchIntentKind::SearchAll),
            Some(SearchIntentKindWire::SearchNone) => Some(SearchIntentKind::SearchNone),
            Some(SearchIntentKindWire::SearchAlways) => Some(SearchIntentKind::SearchAlways),
            Some(SearchIntentKindWire::Unknown) | None => None,
        },
    )
}

/// Individual search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Title of the search result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Content summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// URL link to the result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// Website/media name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// Website icon URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Reference index number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refer: Option<String>,
    /// Publication date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_date: Option<String>,
}

impl WebSearchResponse {
    /// Get the total number of search results
    pub fn result_count(&self) -> usize {
        self.search_result.as_ref().map_or(0, Vec::len)
    }

    /// Borrow the search-intent results when the service returned the field.
    pub fn intents(&self) -> Option<&[SearchIntent]> {
        self.search_intent.as_deref()
    }

    /// Borrow the search results when the service returned the field.
    pub fn results(&self) -> Option<&[SearchResult]> {
        self.search_result.as_deref()
    }

    /// Borrow the task ID when returned.
    pub fn task_id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Return the request creation time when returned.
    pub fn created_at(&self) -> Option<i64> {
        self.created
    }

    /// Borrow the request ID when returned.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_optional_frozen_schema_but_rejects_empty_success() {
        assert!(serde_json::from_str::<WebSearchResponse>("{}").is_err());
        assert!(serde_json::from_str::<WebSearchResponse>(r#"{"search_result":null}"#).is_err());
        assert!(serde_json::from_str::<WebSearchResponse>(r#"{"search_intent":null}"#).is_err());

        let response: WebSearchResponse = serde_json::from_str(
            r#"{"search_intent":[{"intent":"SEARCH_ALWAYS"}],"search_result":[]}"#,
        )
        .unwrap();
        assert_eq!(response.result_count(), 0);
        assert_eq!(
            response.intents().unwrap()[0].intent,
            Some(SearchIntentKind::SearchAlways)
        );
        assert!(response.task_id().is_none());
    }

    #[test]
    fn known_search_intents_and_public_serde_stay_unchanged() {
        for (wire, expected) in [
            ("SEARCH_ALL", SearchIntentKind::SearchAll),
            ("SEARCH_NONE", SearchIntentKind::SearchNone),
            ("SEARCH_ALWAYS", SearchIntentKind::SearchAlways),
        ] {
            let intent: SearchIntent =
                serde_json::from_str(&format!(r#"{{"intent":"{wire}"}}"#)).unwrap();
            assert_eq!(intent.intent, Some(expected));
            assert_eq!(serde_json::to_value(expected).unwrap(), wire);
        }
        assert!(serde_json::from_str::<SearchIntentKind>(r#""SEARCH_FUTURE""#).is_err());
    }

    #[test]
    fn unknown_search_intent_keeps_query_keywords_and_results() {
        let response: WebSearchResponse = serde_json::from_str(
            r#"{
                "search_intent":[{
                    "query":"bounded Rust channels",
                    "intent":"SEARCH_FUTURE",
                    "keywords":"rust async backpressure"
                }],
                "search_result":[{
                    "title":"Result title",
                    "content":"Result summary",
                    "link":"https://example.test/result"
                }]
            }"#,
        )
        .unwrap();

        let intent = &response.intents().unwrap()[0];
        assert_eq!(intent.query.as_deref(), Some("bounded Rust channels"));
        assert!(intent.intent.is_none());
        assert_eq!(intent.keywords.as_deref(), Some("rust async backpressure"));
        assert_eq!(response.result_count(), 1);
        let result = &response.results().unwrap()[0];
        assert_eq!(result.title.as_deref(), Some("Result title"));
        assert_eq!(result.content.as_deref(), Some("Result summary"));
        assert_eq!(result.link.as_deref(), Some("https://example.test/result"));
    }

    #[test]
    fn search_intent_leniency_is_string_only_and_preserves_optional_shapes() {
        let missing: SearchIntent = serde_json::from_str(r#"{"query":"q"}"#).unwrap();
        let null: SearchIntent = serde_json::from_str(r#"{"query":"q","intent":null}"#).unwrap();
        let unknown: SearchIntent =
            serde_json::from_str(r#"{"query":"q","intent":"SEARCH_FUTURE"}"#).unwrap();
        assert!(missing.intent.is_none());
        assert!(null.intent.is_none());
        assert!(unknown.intent.is_none());

        for invalid in [
            r#"{"query":"q","intent":1}"#,
            r#"{"query":"q","intent":{"kind":"SEARCH_ALL"}}"#,
            r#"{"query":"q","intent":{"SEARCH_FUTURE":null}}"#,
            r#"{"query":"q","intent":["SEARCH_ALL"]}"#,
        ] {
            assert!(
                serde_json::from_str::<SearchIntent>(invalid).is_err(),
                "non-string intent unexpectedly accepted: {invalid}"
            );
        }

        let empty_intent_vector: WebSearchResponse =
            serde_json::from_str(r#"{"search_intent":[]}"#).unwrap();
        assert!(empty_intent_vector.intents().unwrap().is_empty());

        // The existing response invariant treats a non-null intent vector as
        // a documented field; an unknown optional nested marker must not turn
        // that vector into a missing top-level field.
        let unknown_only: WebSearchResponse =
            serde_json::from_str(r#"{"search_intent":[{"intent":"SEARCH_FUTURE"}]}"#).unwrap();
        assert!(unknown_only.intents().unwrap()[0].intent.is_none());
    }
}
