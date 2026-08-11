use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::{ZaiResult, client::ZaiClient, serde_helpers::UniqueJsonValue};

/// One knowledge base, optionally restricted to selected documents.
#[derive(Clone, Serialize)]
pub struct ZragKnowledge {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    doc_ids: Option<Vec<String>>,
}

impl ZragKnowledge {
    /// Select a knowledge base by identifier.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            doc_ids: None,
        }
    }

    /// Restrict retrieval to these document identifiers.
    pub fn with_document_ids(mut self, document_ids: Vec<String>) -> Self {
        self.doc_ids = Some(document_ids);
        self
    }

    /// Borrow the knowledge-base identifier.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Borrow the optional document restriction.
    pub fn document_ids(&self) -> Option<&[String]> {
        self.doc_ids.as_deref()
    }

    fn validate(&self) -> ZaiResult<()> {
        require_non_blank(&self.id, "knows[].id")?;
        if let Some(document_ids) = &self.doc_ids {
            require_non_empty_strings(document_ids, "knows[].doc_ids")?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for ZragKnowledge {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragKnowledge")
            .field("id", &"[REDACTED]")
            .field("document_id_count", &self.doc_ids.as_ref().map(Vec::len))
            .finish()
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum ZragMultimodalPartType {
    ImageUrl,
}

/// Image input used by a multimodal ZRAG query.
#[derive(Clone, Serialize)]
pub struct ZragImagePart {
    #[serde(rename = "type")]
    type_: ZragMultimodalPartType,
    url: String,
}

impl ZragImagePart {
    /// Create an `image_url` multimodal part.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            type_: ZragMultimodalPartType::ImageUrl,
            url: url.into(),
        }
    }

    /// Borrow the image URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    fn validate(&self) -> ZaiResult<()> {
        require_non_blank(&self.url, "multimodal_parts[].url")
    }
}

impl std::fmt::Debug for ZragImagePart {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragImagePart")
            .field("type", &"image_url")
            .field("url", &"[REDACTED]")
            .finish()
    }
}

/// Retrieval strategy for text queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ZragRecallMethod {
    /// Vector-similarity retrieval.
    Embedding,
    /// Keyword retrieval.
    Keyword,
    /// Hybrid vector and keyword retrieval.
    Mixed,
}

/// Role accepted by the optional query-rewrite conversation history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ZragRetrieveMessageRole {
    /// End-user message.
    User,
    /// Assistant message.
    Assistant,
}

/// One text-only message used to rewrite a retrieval query.
#[derive(Clone, Serialize)]
pub struct ZragRetrieveMessage {
    role: ZragRetrieveMessageRole,
    content: String,
}

impl ZragRetrieveMessage {
    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ZragRetrieveMessageRole::User,
            content: content.into(),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ZragRetrieveMessageRole::Assistant,
            content: content.into(),
        }
    }

    /// Return the message role.
    pub const fn role(&self) -> ZragRetrieveMessageRole {
        self.role
    }

    /// Borrow the text content.
    pub fn content(&self) -> &str {
        &self.content
    }

    fn validate(&self) -> ZaiResult<()> {
        require_non_blank(&self.content, "messages[].content")
    }
}

impl std::fmt::Debug for ZragRetrieveMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragRetrieveMessage")
            .field("role", &self.role)
            .field("content", &"[REDACTED]")
            .finish()
    }
}

/// One index-type filter scoped to a knowledge base.
#[derive(Clone, Serialize)]
pub struct ZragIndexTypeFilter {
    know_id: String,
    index_type_id: i64,
}

impl ZragIndexTypeFilter {
    /// Create an index filter with the exact provider index identifier.
    pub fn new(knowledge_id: impl Into<String>, index_type_id: i64) -> Self {
        Self {
            know_id: knowledge_id.into(),
            index_type_id,
        }
    }

    /// Borrow the knowledge-base identifier.
    pub fn knowledge_id(&self) -> &str {
        &self.know_id
    }

    /// Return the provider index-type identifier.
    pub const fn index_type_id(&self) -> i64 {
        self.index_type_id
    }

    fn validate(&self) -> ZaiResult<()> {
        require_non_blank(&self.know_id, "search_filters.index_types[].know_id")
    }
}

impl std::fmt::Debug for ZragIndexTypeFilter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragIndexTypeFilter")
            .field("knowledge_id", &"[REDACTED]")
            .field("index_type_id", &"[REDACTED]")
            .finish()
    }
}

/// How a tag filter obtains its comparison value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ZragFilterValueType {
    /// Use the literal value in the request.
    Fixed,
    /// Resolve the value through a provider-side reference.
    Ref,
}

/// Comparison operation encoded by the ZRAG tag-filter wire schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum ZragTagFilterOperator {
    /// `>=` comparison (`1`).
    GreaterThanOrEqual = 1,
    /// `<=` comparison (`2`).
    LessThanOrEqual = 2,
    /// Contains comparison (`3`).
    Contains = 3,
    /// Does-not-contain comparison (`4`).
    NotContains = 4,
}

impl ZragTagFilterOperator {
    /// Return the exact integer value sent to the provider.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl Serialize for ZragTagFilterOperator {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(self.as_u8())
    }
}

/// One typed tag filter.
#[derive(Clone, Serialize)]
pub struct ZragTagFilter {
    tag_id: String,
    value_type: ZragFilterValueType,
    filter_type: ZragTagFilterOperator,
    filter_value: String,
    multiple_value: Vec<String>,
}

impl ZragTagFilter {
    /// Construct a complete tag filter.
    pub fn new(
        tag_id: impl Into<String>,
        value_type: ZragFilterValueType,
        operator: ZragTagFilterOperator,
        filter_value: impl Into<String>,
        multiple_values: Vec<String>,
    ) -> Self {
        Self {
            tag_id: tag_id.into(),
            value_type,
            filter_type: operator,
            filter_value: filter_value.into(),
            multiple_value: multiple_values,
        }
    }

    /// Borrow the tag identifier.
    pub fn tag_id(&self) -> &str {
        &self.tag_id
    }

    /// Return the value-source mode.
    pub const fn value_type(&self) -> ZragFilterValueType {
        self.value_type
    }

    /// Return the comparison operator.
    pub const fn operator(&self) -> ZragTagFilterOperator {
        self.filter_type
    }

    /// Borrow the scalar comparison value.
    pub fn filter_value(&self) -> &str {
        &self.filter_value
    }

    /// Borrow the multi-value comparison list.
    pub fn multiple_values(&self) -> &[String] {
        &self.multiple_value
    }

    fn validate(&self) -> ZaiResult<()> {
        require_non_blank(&self.tag_id, "search_filters.tags[].tag_id")?;
        if self
            .multiple_value
            .iter()
            .any(|value| value.trim().is_empty())
        {
            return Err(invalid(
                "search_filters.tags[].multiple_value cannot contain blank values",
            ));
        }
        Ok(())
    }
}

impl std::fmt::Debug for ZragTagFilter {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragTagFilter")
            .field("tag_id", &"[REDACTED]")
            .field("value_type", &self.value_type)
            .field("operator", &self.filter_type)
            .field("filter_value", &"[REDACTED]")
            .field("multiple_value_count", &self.multiple_value.len())
            .finish()
    }
}

/// Optional QA-intervention filter configuration.
#[derive(Clone, Serialize)]
pub struct ZragQaIntervention {
    qa_similarity_threshold: f64,
    qa_intervention_ids: Vec<String>,
}

impl ZragQaIntervention {
    /// Construct QA intervention with a score threshold and knowledge IDs.
    pub fn new(similarity_threshold: f64, knowledge_ids: Vec<String>) -> Self {
        Self {
            qa_similarity_threshold: similarity_threshold,
            qa_intervention_ids: knowledge_ids,
        }
    }

    /// Return the configured similarity threshold.
    pub const fn similarity_threshold(&self) -> f64 {
        self.qa_similarity_threshold
    }

    /// Borrow the QA-intervention knowledge IDs.
    pub fn knowledge_ids(&self) -> &[String] {
        &self.qa_intervention_ids
    }

    fn validate(&self) -> ZaiResult<()> {
        require_finite(
            self.qa_similarity_threshold,
            "search_filters.qa_intervention.qa_similarity_threshold",
        )?;
        require_non_empty_strings(
            &self.qa_intervention_ids,
            "search_filters.qa_intervention.qa_intervention_ids",
        )
    }
}

impl std::fmt::Debug for ZragQaIntervention {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragQaIntervention")
            .field("similarity_threshold", &self.qa_similarity_threshold)
            .field("knowledge_id_count", &self.qa_intervention_ids.len())
            .finish()
    }
}

/// Optional structured filters for a ZRAG retrieval request.
#[derive(Clone, Default, Serialize)]
pub struct ZragSearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    index_types: Option<Vec<ZragIndexTypeFilter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<ZragTagFilter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    qa_intervention: Option<ZragQaIntervention>,
}

impl ZragSearchFilters {
    /// Create an empty filter builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the index-type filters.
    pub fn with_index_types(mut self, filters: Vec<ZragIndexTypeFilter>) -> Self {
        self.index_types = Some(filters);
        self
    }

    /// Replace the tag filters.
    pub fn with_tags(mut self, filters: Vec<ZragTagFilter>) -> Self {
        self.tags = Some(filters);
        self
    }

    /// Set QA-intervention filtering.
    pub fn with_qa_intervention(mut self, intervention: ZragQaIntervention) -> Self {
        self.qa_intervention = Some(intervention);
        self
    }

    /// Borrow the optional index-type filters.
    pub fn index_types(&self) -> Option<&[ZragIndexTypeFilter]> {
        self.index_types.as_deref()
    }

    /// Borrow the optional tag filters.
    pub fn tags(&self) -> Option<&[ZragTagFilter]> {
        self.tags.as_deref()
    }

    /// Borrow the optional QA-intervention configuration.
    pub fn qa_intervention(&self) -> Option<&ZragQaIntervention> {
        self.qa_intervention.as_ref()
    }

    fn validate(&self) -> ZaiResult<()> {
        if self.index_types.is_none() && self.tags.is_none() && self.qa_intervention.is_none() {
            return Err(invalid("search_filters must contain at least one filter"));
        }
        if let Some(filters) = &self.index_types {
            if filters.is_empty() {
                return Err(invalid(
                    "search_filters.index_types must not be empty when provided",
                ));
            }
            for filter in filters {
                filter.validate()?;
            }
        }
        if let Some(filters) = &self.tags {
            if filters.is_empty() {
                return Err(invalid(
                    "search_filters.tags must not be empty when provided",
                ));
            }
            for filter in filters {
                filter.validate()?;
            }
        }
        if let Some(intervention) = &self.qa_intervention {
            intervention.validate()?;
        }
        Ok(())
    }
}

impl std::fmt::Debug for ZragSearchFilters {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragSearchFilters")
            .field("index_type_count", &self.index_types.as_ref().map(Vec::len))
            .field("tag_count", &self.tags.as_ref().map(Vec::len))
            .field("qa_intervention", &self.qa_intervention)
            .finish()
    }
}

/// Typed request for `POST /api/zrag/retrieval/retrieve`.
#[derive(Clone, Serialize)]
pub struct ZragRetrieveRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    multimodal: Option<bool>,
    knows: Vec<ZragKnowledge>,
    #[serde(skip_serializing_if = "Option::is_none")]
    query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    multimodal_parts: Option<Vec<ZragImagePart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recall_method: Option<ZragRecallMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recall_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_rerank: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_rewrite: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enable_expansion: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    similarity_threshold: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    messages: Option<Vec<ZragRetrieveMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_filters: Option<ZragSearchFilters>,
}

impl ZragRetrieveRequest {
    /// Create a request for one or more knowledge bases.
    ///
    /// Add a text query, image parts, or both before validation or dispatch.
    pub fn new(knows: Vec<ZragKnowledge>) -> Self {
        Self {
            multimodal: None,
            knows,
            query: None,
            multimodal_parts: None,
            top_k: None,
            top_n: None,
            recall_method: None,
            recall_ratio: None,
            enable_rerank: None,
            enable_rewrite: None,
            enable_expansion: None,
            similarity_threshold: None,
            messages: None,
            search_filters: None,
        }
    }

    /// Select whether the provider uses its multimodal retrieval path.
    pub fn with_multimodal(mut self, enabled: bool) -> Self {
        self.multimodal = Some(enabled);
        self
    }

    /// Replace the knowledge-base selection.
    pub fn with_knows(mut self, knows: Vec<ZragKnowledge>) -> Self {
        self.knows = knows;
        self
    }

    /// Set the optional text query.
    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Set image inputs for a multimodal query.
    pub fn with_image_parts(mut self, parts: Vec<ZragImagePart>) -> Self {
        self.multimodal_parts = Some(parts);
        self
    }

    /// Set the final result count.
    pub fn with_top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Set the initial recall count.
    pub fn with_top_n(mut self, top_n: u32) -> Self {
        self.top_n = Some(top_n);
        self
    }

    /// Set the text-retrieval strategy.
    pub fn with_recall_method(mut self, method: ZragRecallMethod) -> Self {
        self.recall_method = Some(method);
        self
    }

    /// Set the vector-retrieval weight in `0.0..=1.0`.
    pub fn with_recall_ratio(mut self, ratio: f64) -> Self {
        self.recall_ratio = Some(ratio);
        self
    }

    /// Enable or disable reranking.
    pub fn with_reranking(mut self, enabled: bool) -> Self {
        self.enable_rerank = Some(enabled);
        self
    }

    /// Enable or disable query rewriting.
    pub fn with_rewrite(mut self, enabled: bool) -> Self {
        self.enable_rewrite = Some(enabled);
        self
    }

    /// Enable or disable expanded recall.
    pub fn with_expansion(mut self, enabled: bool) -> Self {
        self.enable_expansion = Some(enabled);
        self
    }

    /// Set the provider similarity threshold.
    pub fn with_similarity_threshold(mut self, threshold: f64) -> Self {
        self.similarity_threshold = Some(threshold);
        self
    }

    /// Set the optional conversation history used for query rewriting.
    pub fn with_messages(mut self, messages: Vec<ZragRetrieveMessage>) -> Self {
        self.messages = Some(messages);
        self
    }

    /// Set typed retrieval filters.
    pub fn with_search_filters(mut self, filters: ZragSearchFilters) -> Self {
        self.search_filters = Some(filters);
        self
    }

    /// Borrow the selected knowledge bases.
    pub fn knows(&self) -> &[ZragKnowledge] {
        &self.knows
    }

    /// Borrow the optional text query.
    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    /// Borrow the optional image-query parts.
    pub fn image_parts(&self) -> Option<&[ZragImagePart]> {
        self.multimodal_parts.as_deref()
    }

    /// Validate all local request and cross-field constraints without network I/O.
    pub fn validate(&self) -> ZaiResult<()> {
        if self.knows.is_empty() {
            return Err(invalid("knows must contain at least one knowledge base"));
        }
        for knowledge in &self.knows {
            knowledge.validate()?;
        }

        let has_query = self
            .query
            .as_deref()
            .is_some_and(|query| !query.trim().is_empty());
        if self.query.is_some() && !has_query {
            return Err(invalid("query must not be blank when provided"));
        }
        let has_image_parts = self
            .multimodal_parts
            .as_ref()
            .is_some_and(|parts| !parts.is_empty());
        if self.multimodal_parts.is_some() && !has_image_parts {
            return Err(invalid("multimodal_parts must not be empty when provided"));
        }
        if let Some(parts) = &self.multimodal_parts {
            for part in parts {
                part.validate()?;
            }
        }
        if !has_query && !has_image_parts {
            return Err(invalid(
                "a text query, at least one multimodal part, or both is required",
            ));
        }

        if self.top_k == Some(0) {
            return Err(invalid("top_k must be at least 1"));
        }
        if self.top_n == Some(0) {
            return Err(invalid("top_n must be at least 1"));
        }
        if let Some(ratio) = self.recall_ratio
            && (!ratio.is_finite() || !(0.0..=1.0).contains(&ratio))
        {
            return Err(invalid("recall_ratio must be finite and in 0.0..=1.0"));
        }
        if let Some(threshold) = self.similarity_threshold {
            require_finite(threshold, "similarity_threshold")?;
        }
        if let Some(messages) = &self.messages {
            if messages.is_empty() {
                return Err(invalid("messages must not be empty when provided"));
            }
            for message in messages {
                message.validate()?;
            }
        }
        if let Some(filters) = &self.search_filters {
            filters.validate()?;
        }
        Ok(())
    }

    /// Validate, send through `client`, and decode the typed response.
    pub async fn send_via(&self, client: &ZaiClient) -> ZaiResult<ZragRetrieveResponse> {
        self.validate()?;
        client
            .operation(crate::client::routes::ZRAG_RETRIEVE)
            .send_json::<_, ZragRetrieveResponse>(self)
            .await
    }
}

impl std::fmt::Debug for ZragRetrieveRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragRetrieveRequest")
            .field("multimodal", &self.multimodal)
            .field("knowledge_count", &self.knows.len())
            .field("query_configured", &self.query.is_some())
            .field(
                "multimodal_part_count",
                &self.multimodal_parts.as_ref().map(Vec::len),
            )
            .field("top_k", &self.top_k)
            .field("top_n", &self.top_n)
            .field("recall_method", &self.recall_method)
            .field("recall_ratio", &self.recall_ratio)
            .field("enable_rerank", &self.enable_rerank)
            .field("enable_rewrite", &self.enable_rewrite)
            .field("enable_expansion", &self.enable_expansion)
            .field("similarity_threshold", &self.similarity_threshold)
            .field("message_count", &self.messages.as_ref().map(Vec::len))
            .field("search_filters", &self.search_filters)
            .finish()
    }
}

/// One media object attached to retrieved text.
#[derive(Clone, Serialize, Deserialize)]
pub struct ZragMedia {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl ZragMedia {
    /// Borrow the optional media identifier.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Borrow the optional media URL.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Borrow the optional media description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl std::fmt::Debug for ZragMedia {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragMedia")
            .field("id_configured", &self.id.is_some())
            .field("url_configured", &self.url.is_some())
            .field("description_configured", &self.description.is_some())
            .finish()
    }
}

/// URL wrapper used by image and video response fields.
#[derive(Clone, Serialize, Deserialize)]
pub struct ZragUrl {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

impl ZragUrl {
    /// Borrow the optional URL.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

impl std::fmt::Debug for ZragUrl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragUrl")
            .field("url_configured", &self.url.is_some())
            .finish()
    }
}

/// Source metadata associated with one retrieved content item.
#[derive(Clone, Serialize, Deserialize)]
pub struct ZragRetrieveMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    doc_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    doc_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    doc_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    index: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    page_index: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    clip_index: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    start_time: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end_time: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    duration: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    frames: Option<Vec<String>>,
}

impl ZragRetrieveMetadata {
    /// Borrow the optional document type.
    pub fn document_type(&self) -> Option<&str> {
        self.doc_type.as_deref()
    }

    /// Borrow the optional document name.
    pub fn document_name(&self) -> Option<&str> {
        self.doc_name.as_deref()
    }

    /// Borrow the optional document URL.
    pub fn document_url(&self) -> Option<&str> {
        self.doc_url.as_deref()
    }

    /// Return the optional source slice index.
    pub const fn index(&self) -> Option<i64> {
        self.index
    }

    /// Return the optional source page index.
    pub const fn page_index(&self) -> Option<i64> {
        self.page_index
    }

    /// Return the optional video-clip index.
    pub const fn clip_index(&self) -> Option<i64> {
        self.clip_index
    }

    /// Return the optional clip start timestamp.
    pub const fn start_time(&self) -> Option<i64> {
        self.start_time
    }

    /// Return the optional clip end timestamp.
    pub const fn end_time(&self) -> Option<i64> {
        self.end_time
    }

    /// Return the optional clip duration.
    pub const fn duration(&self) -> Option<i64> {
        self.duration
    }

    /// Borrow optional key-frame values.
    pub fn frames(&self) -> Option<&[String]> {
        self.frames.as_deref()
    }
}

impl std::fmt::Debug for ZragRetrieveMetadata {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragRetrieveMetadata")
            .field("document_type_configured", &self.doc_type.is_some())
            .field("document_name_configured", &self.doc_name.is_some())
            .field("document_url_configured", &self.doc_url.is_some())
            .field("index", &self.index)
            .field("page_index", &self.page_index)
            .field("clip_index", &self.clip_index)
            .field("start_time", &self.start_time)
            .field("end_time", &self.end_time)
            .field("duration", &self.duration)
            .field("frame_count", &self.frames.as_ref().map(Vec::len))
            .finish()
    }
}

/// One multimodal content item returned by ZRAG retrieval.
#[derive(Clone, Serialize, Deserialize)]
pub struct ZragRetrieveContent {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    know_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    doc_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    medias: Option<Vec<ZragMedia>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    image_url: Option<ZragUrl>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    video_url: Option<ZragUrl>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    index: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rerank_index: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rerank_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    metadata: Option<ZragRetrieveMetadata>,
}

impl ZragRetrieveContent {
    /// Borrow the optional content identifier.
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    /// Borrow the optional knowledge-base identifier.
    pub fn knowledge_id(&self) -> Option<&str> {
        self.know_id.as_deref()
    }

    /// Borrow the optional document identifier.
    pub fn document_id(&self) -> Option<&str> {
        self.doc_id.as_deref()
    }

    /// Borrow the optional retrieved text.
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    /// Borrow attached media objects.
    pub fn media(&self) -> Option<&[ZragMedia]> {
        self.medias.as_deref()
    }

    /// Borrow the optional image URL object.
    pub fn image_url(&self) -> Option<&ZragUrl> {
        self.image_url.as_ref()
    }

    /// Borrow the optional video URL object.
    pub fn video_url(&self) -> Option<&ZragUrl> {
        self.video_url.as_ref()
    }

    /// Return the optional recall position.
    pub const fn index(&self) -> Option<i64> {
        self.index
    }

    /// Return the optional recall score.
    pub const fn score(&self) -> Option<f64> {
        self.score
    }

    /// Return the optional reranked position.
    pub const fn rerank_index(&self) -> Option<i64> {
        self.rerank_index
    }

    /// Return the optional reranking score.
    pub const fn rerank_score(&self) -> Option<f64> {
        self.rerank_score
    }

    /// Borrow optional source metadata.
    pub fn metadata(&self) -> Option<&ZragRetrieveMetadata> {
        self.metadata.as_ref()
    }
}

impl std::fmt::Debug for ZragRetrieveContent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragRetrieveContent")
            .field("id_configured", &self.id.is_some())
            .field("knowledge_id_configured", &self.know_id.is_some())
            .field("document_id_configured", &self.doc_id.is_some())
            .field("text_configured", &self.text.is_some())
            .field("media_count", &self.medias.as_ref().map(Vec::len))
            .field("image_url_configured", &self.image_url.is_some())
            .field("video_url_configured", &self.video_url.is_some())
            .field("index", &self.index)
            .field("score", &self.score)
            .field("rerank_index", &self.rerank_index)
            .field("rerank_score", &self.rerank_score)
            .field("metadata", &self.metadata)
            .finish()
    }
}

/// Query-rewrite details returned by the provider.
#[derive(Clone, Serialize, Deserialize)]
pub struct ZragRewrittenQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    original_query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    multi_queries: Option<Vec<String>>,
}

impl ZragRewrittenQuery {
    /// Borrow the optional original query.
    pub fn original_query(&self) -> Option<&str> {
        self.original_query.as_deref()
    }

    /// Borrow provider-generated query variants.
    pub fn queries(&self) -> Option<&[String]> {
        self.multi_queries.as_deref()
    }
}

impl std::fmt::Debug for ZragRewrittenQuery {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragRewrittenQuery")
            .field("original_query_configured", &self.original_query.is_some())
            .field("query_count", &self.multi_queries.as_ref().map(Vec::len))
            .finish()
    }
}

/// Data payload returned by ZRAG retrieval.
#[derive(Clone, Serialize, Deserialize)]
pub struct ZragRetrieveData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    contents: Option<Vec<ZragRetrieveContent>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rewritten_query: Option<ZragRewrittenQuery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    elapsed_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    total_tokens: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
}

impl ZragRetrieveData {
    /// Borrow retrieved content items.
    pub fn contents(&self) -> Option<&[ZragRetrieveContent]> {
        self.contents.as_deref()
    }

    /// Borrow optional query-rewrite details.
    pub fn rewritten_query(&self) -> Option<&ZragRewrittenQuery> {
        self.rewritten_query.as_ref()
    }

    /// Return the optional provider elapsed time in milliseconds.
    pub const fn elapsed_ms(&self) -> Option<i64> {
        self.elapsed_ms
    }

    /// Return the optional token count.
    pub const fn total_tokens(&self) -> Option<i64> {
        self.total_tokens
    }

    /// Borrow the optional provider request identifier.
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }
}

impl std::fmt::Debug for ZragRetrieveData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragRetrieveData")
            .field("content_count", &self.contents.as_ref().map(Vec::len))
            .field("rewritten_query", &self.rewritten_query)
            .field("elapsed_ms", &self.elapsed_ms)
            .field("total_tokens", &self.total_tokens)
            .field("request_id_configured", &self.request_id.is_some())
            .finish()
    }
}

/// Forward-compatible response from ZRAG retrieval.
///
/// Every documented field is optional in the frozen upstream schema. Unknown
/// additive fields are ignored, but a payload containing no documented
/// non-null field is rejected instead of being accepted as a false success.
#[derive(Clone, Serialize)]
pub struct ZragRetrieveResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<ZragRetrieveData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

#[derive(Deserialize)]
struct ZragRetrieveResponseWire {
    data: Option<ZragRetrieveData>,
    code: Option<i64>,
    message: Option<String>,
}

impl<'de> Deserialize<'de> for ZragRetrieveResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = UniqueJsonValue::deserialize(deserializer)?.into_inner();
        let serde_json::Value::Object(object) = value else {
            return Err(D::Error::custom(
                "ZRAG retrieve response must be a JSON object",
            ));
        };
        validate_response_shape(&object).map_err(D::Error::custom)?;
        let wire =
            serde_json::from_value::<ZragRetrieveResponseWire>(serde_json::Value::Object(object))
                .map_err(D::Error::custom)?;
        if wire.data.is_none() && wire.code.is_none() && wire.message.is_none() {
            return Err(D::Error::custom(
                "ZRAG retrieve response contained no documented non-null fields",
            ));
        }
        Ok(Self {
            data: wire.data,
            code: wire.code,
            message: wire.message,
        })
    }
}

fn validate_response_shape(
    response: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), &'static str> {
    let Some(data) = response.get("data").filter(|value| !value.is_null()) else {
        return Ok(());
    };
    let data = data
        .as_object()
        .ok_or("ZRAG retrieve response data must be an object")?;

    if let Some(contents) = data.get("contents").filter(|value| !value.is_null()) {
        let contents = contents
            .as_array()
            .ok_or("ZRAG retrieve response data.contents must be an array")?;
        for content in contents {
            let content = content
                .as_object()
                .ok_or("ZRAG retrieve response content item must be an object")?;
            if let Some(medias) = content.get("medias").filter(|value| !value.is_null()) {
                let medias = medias
                    .as_array()
                    .ok_or("ZRAG retrieve response content.medias must be an array")?;
                if medias.iter().any(|media| !media.is_object()) {
                    return Err("ZRAG retrieve response media item must be an object");
                }
            }
            for field in ["image_url", "video_url", "metadata"] {
                if content
                    .get(field)
                    .filter(|value| !value.is_null())
                    .is_some_and(|value| !value.is_object())
                {
                    return Err("ZRAG retrieve response nested content field must be an object");
                }
            }
        }
    }

    if data
        .get("rewritten_query")
        .filter(|value| !value.is_null())
        .is_some_and(|value| !value.is_object())
    {
        return Err("ZRAG retrieve response rewritten_query must be an object");
    }
    Ok(())
}

impl ZragRetrieveResponse {
    /// Borrow the optional retrieval data payload.
    pub fn data(&self) -> Option<&ZragRetrieveData> {
        self.data.as_ref()
    }

    /// Return the optional business status code.
    pub const fn code(&self) -> Option<i64> {
        self.code
    }

    /// Borrow the optional provider message.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

impl std::fmt::Debug for ZragRetrieveResponse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ZragRetrieveResponse")
            .field("data", &self.data)
            .field("code", &self.code)
            .field("message_configured", &self.message.is_some())
            .finish()
    }
}

fn invalid(message: impl Into<String>) -> crate::ZaiError {
    crate::client::validation::invalid(message)
}

fn require_non_blank(value: &str, field: &'static str) -> ZaiResult<()> {
    if value.trim().is_empty() {
        Err(invalid(format!("{field} must not be blank")))
    } else {
        Ok(())
    }
}

fn require_non_empty_strings(values: &[String], field: &'static str) -> ZaiResult<()> {
    if values.is_empty() {
        return Err(invalid(format!("{field} must not be empty")));
    }
    if values.iter().any(|value| value.trim().is_empty()) {
        return Err(invalid(format!("{field} cannot contain blank values")));
    }
    Ok(())
}

fn require_finite(value: f64, field: &'static str) -> ZaiResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid(format!("{field} must be finite")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn full_request() -> ZragRetrieveRequest {
        let filters = ZragSearchFilters::new()
            .with_index_types(vec![ZragIndexTypeFilter::new("private-index-kb", 7)])
            .with_tags(vec![ZragTagFilter::new(
                "private-tag",
                ZragFilterValueType::Fixed,
                ZragTagFilterOperator::Contains,
                "private-filter-value",
                vec!["private-choice".to_owned()],
            )])
            .with_qa_intervention(ZragQaIntervention::new(
                0.6,
                vec!["private-qa-kb".to_owned()],
            ));
        ZragRetrieveRequest::new(vec![
            ZragKnowledge::new("private-kb").with_document_ids(vec!["private-document".to_owned()]),
        ])
        .with_multimodal(true)
        .with_query("private query")
        .with_image_parts(vec![ZragImagePart::new(
            "https://private.example/image.png",
        )])
        .with_top_k(8)
        .with_top_n(10)
        .with_recall_method(ZragRecallMethod::Mixed)
        .with_recall_ratio(0.8)
        .with_reranking(true)
        .with_rewrite(true)
        .with_expansion(true)
        .with_similarity_threshold(0.2)
        .with_messages(vec![
            ZragRetrieveMessage::user("private user message"),
            ZragRetrieveMessage::assistant("private assistant message"),
        ])
        .with_search_filters(filters)
    }

    #[test]
    fn full_request_matches_the_frozen_wire_schema() {
        let request = full_request();
        request.validate().unwrap();
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            json!({
                "multimodal": true,
                "knows": [{"id": "private-kb", "doc_ids": ["private-document"]}],
                "query": "private query",
                "multimodal_parts": [{
                    "type": "image_url",
                    "url": "https://private.example/image.png"
                }],
                "top_k": 8,
                "top_n": 10,
                "recall_method": "mixed",
                "recall_ratio": 0.8,
                "enable_rerank": true,
                "enable_rewrite": true,
                "enable_expansion": true,
                "similarity_threshold": 0.2,
                "messages": [
                    {"role": "user", "content": "private user message"},
                    {"role": "assistant", "content": "private assistant message"}
                ],
                "search_filters": {
                    "index_types": [{"know_id": "private-index-kb", "index_type_id": 7}],
                    "tags": [{
                        "tag_id": "private-tag",
                        "value_type": "fixed",
                        "filter_type": 3,
                        "filter_value": "private-filter-value",
                        "multiple_value": ["private-choice"]
                    }],
                    "qa_intervention": {
                        "qa_similarity_threshold": 0.6,
                        "qa_intervention_ids": ["private-qa-kb"]
                    }
                }
            })
        );
    }

    #[test]
    fn validation_rejects_invalid_required_and_cross_fields() {
        for request in [
            ZragRetrieveRequest::new(Vec::new()).with_query("query"),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new(" ")]).with_query("query"),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb").with_document_ids(Vec::new())])
                .with_query("query"),
            ZragRetrieveRequest::new(vec![
                ZragKnowledge::new("kb").with_document_ids(vec![" ".to_owned()]),
            ])
            .with_query("query"),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")]),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")]).with_query(" "),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")]).with_image_parts(Vec::new()),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_image_parts(vec![ZragImagePart::new(" ")]),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_top_k(0),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_top_n(0),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_recall_ratio(f64::NAN),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_recall_ratio(1.1),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_recall_ratio(-0.1),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_similarity_threshold(f64::INFINITY),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_messages(Vec::new()),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_messages(vec![ZragRetrieveMessage::user(" ")]),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_search_filters(ZragSearchFilters::new()),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_search_filters(ZragSearchFilters::new().with_index_types(Vec::new())),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_search_filters(
                    ZragSearchFilters::new()
                        .with_index_types(vec![ZragIndexTypeFilter::new(" ", 7)]),
                ),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_search_filters(ZragSearchFilters::new().with_tags(Vec::new())),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_search_filters(ZragSearchFilters::new().with_tags(vec![ZragTagFilter::new(
                    " ",
                    ZragFilterValueType::Fixed,
                    ZragTagFilterOperator::Contains,
                    "value",
                    Vec::new(),
                )])),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_search_filters(ZragSearchFilters::new().with_tags(vec![ZragTagFilter::new(
                    "tag",
                    ZragFilterValueType::Fixed,
                    ZragTagFilterOperator::Contains,
                    "value",
                    vec![" ".to_owned()],
                )])),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_search_filters(ZragSearchFilters::new().with_qa_intervention(
                    ZragQaIntervention::new(f64::NAN, vec!["qa-kb".to_owned()]),
                )),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_search_filters(
                    ZragSearchFilters::new()
                        .with_qa_intervention(ZragQaIntervention::new(0.6, Vec::new())),
                ),
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_query("query")
                .with_search_filters(
                    ZragSearchFilters::new()
                        .with_qa_intervention(ZragQaIntervention::new(0.6, vec![" ".to_owned()])),
                ),
        ] {
            assert!(request.validate().is_err());
        }

        assert!(
            ZragRetrieveRequest::new(vec![ZragKnowledge::new("kb")])
                .with_image_parts(vec![ZragImagePart::new("https://example.com/image")])
                .validate()
                .is_ok()
        );
        assert!(full_request().validate().is_ok());
    }

    #[test]
    fn request_debug_redacts_identifiers_text_urls_and_filter_values() {
        let debug = format!("{:?}", full_request());
        for secret in [
            "private-kb",
            "private-document",
            "private query",
            "private.example",
            "private user message",
            "private assistant message",
            "private-index-kb",
            "private-tag",
            "private-filter-value",
            "private-choice",
            "private-qa-kb",
        ] {
            assert!(!debug.contains(secret), "Debug leaked {secret:?}");
        }
        assert!(debug.contains("knowledge_count: 1"));
        assert!(debug.contains("multimodal_part_count: Some(1)"));

        let helper_debug = format!(
            "{:?} {:?} {:?} {:?} {:?} {:?}",
            ZragKnowledge::new("private-helper-kb")
                .with_document_ids(vec!["private-helper-doc".to_owned()]),
            ZragImagePart::new("https://private-helper.example/image"),
            ZragRetrieveMessage::user("private helper message"),
            ZragIndexTypeFilter::new("private-helper-index-kb", 987_654_321),
            ZragTagFilter::new(
                "private-helper-tag",
                ZragFilterValueType::Fixed,
                ZragTagFilterOperator::Contains,
                "private-helper-filter",
                vec!["private-helper-choice".to_owned()],
            ),
            ZragQaIntervention::new(0.6, vec!["private-helper-qa-kb".to_owned()]),
        );
        for secret in [
            "private-helper-kb",
            "private-helper-doc",
            "private-helper.example",
            "private helper message",
            "private-helper-index-kb",
            "987654321",
            "private-helper-tag",
            "private-helper-filter",
            "private-helper-choice",
            "private-helper-qa-kb",
        ] {
            assert!(!helper_debug.contains(secret), "Debug leaked {secret:?}");
        }
    }

    #[test]
    fn response_is_typed_additive_and_rejects_false_success() {
        let response: ZragRetrieveResponse = serde_json::from_value(json!({
            "code": 200,
            "message": "private provider message",
            "data": {
                "contents": [{
                    "id": "private-slice",
                    "know_id": "private-kb",
                    "doc_id": "private-document",
                    "text": "private retrieved text",
                    "medias": [{
                        "id": "private-media",
                        "url": "https://private.example/media",
                        "description": "private media description",
                        "future_media_field": true
                    }],
                    "image_url": {"url": "https://private.example/image"},
                    "video_url": {"url": "https://private.example/video"},
                    "index": 1,
                    "score": 0.9,
                    "rerank_index": 2,
                    "rerank_score": 0.8,
                    "metadata": {
                        "doc_type": "pdf",
                        "doc_name": "private.pdf",
                        "doc_url": "https://private.example/document",
                        "index": 3,
                        "page_index": 4,
                        "clip_index": 5,
                        "start_time": 6,
                        "end_time": 7,
                        "duration": 1,
                        "frames": ["private-frame"]
                    },
                    "future_content_field": {"nested": true}
                }],
                "rewritten_query": {
                    "original_query": "private original",
                    "multi_queries": ["private rewrite"],
                    "future_rewrite_field": 1
                },
                "elapsed_ms": 12,
                "total_tokens": 34,
                "request_id": "private-request",
                "future_data_field": "new"
            },
            "future_top_level": true
        }))
        .unwrap();

        assert_eq!(response.code(), Some(200));
        let data = response.data().unwrap();
        assert_eq!(data.elapsed_ms(), Some(12));
        assert_eq!(data.total_tokens(), Some(34));
        let content = &data.contents().unwrap()[0];
        assert_eq!(content.text(), Some("private retrieved text"));
        assert_eq!(content.media().unwrap()[0].id(), Some("private-media"));
        assert_eq!(
            content.metadata().unwrap().document_name(),
            Some("private.pdf")
        );

        let debug = format!("{response:?}");
        for secret in [
            "private provider message",
            "private-slice",
            "private-kb",
            "private-document",
            "private retrieved text",
            "private-media",
            "private.example",
            "private media description",
            "private.pdf",
            "private-frame",
            "private original",
            "private rewrite",
            "private-request",
        ] {
            assert!(!debug.contains(secret), "Debug leaked {secret:?}");
        }

        assert!(serde_json::from_value::<ZragRetrieveResponse>(json!({})).is_err());
        assert!(
            serde_json::from_value::<ZragRetrieveResponse>(json!({"future_only": true})).is_err()
        );
        assert!(
            serde_json::from_value::<ZragRetrieveResponse>(json!({
                "data": null,
                "code": null,
                "message": null
            }))
            .is_err()
        );
        for malformed in [
            json!([]),
            json!({"code": "200"}),
            json!({"message": 200}),
            json!({"data": []}),
            json!({"data": {"contents": {}}}),
            json!({"data": {"contents": [[]]}}),
            json!({"data": {"contents": [{"medias": [[]]}]}}),
            json!({"data": {"contents": [{"image_url": []}]}}),
            json!({"data": {"contents": [{"metadata": []}]}}),
            json!({"data": {"rewritten_query": []}}),
        ] {
            assert!(
                serde_json::from_value::<ZragRetrieveResponse>(malformed.clone()).is_err(),
                "accepted malformed response: {malformed}"
            );
        }
        assert!(serde_json::from_value::<ZragRetrieveResponse>(json!({"data": {}})).is_ok());
    }

    #[test]
    fn response_rejects_top_level_nested_and_false_success_duplicates() {
        for payload in [
            r#"{"code":200,"code":200}"#,
            r#"{"data":{"contents":[{"text":"private-retrieve-value","text":"private-retrieve-value"}]}}"#,
            r#"{"future_only":true,"data":null,"data":{}}"#,
        ] {
            let error = serde_json::from_str::<ZragRetrieveResponse>(payload)
                .expect_err("duplicate-key ZRAG retrieve response must fail closed");
            let diagnostic = error.to_string();
            assert!(diagnostic.contains(crate::serde_helpers::DUPLICATE_JSON_KEY_ERROR));
            assert!(!diagnostic.contains("private-retrieve-value"));
        }
    }
}
