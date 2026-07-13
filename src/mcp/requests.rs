//! Strongly typed request models for every supported MCP tool.

use serde::{Deserialize, Serialize};

use crate::ZaiResult;

mod validation;

use validation::{validate_optional, validate_required};

pub(crate) trait McpRequest {
    fn validate(&self) -> ZaiResult<()>;
}

macro_rules! impl_redacted_debug {
    ($request:ty, redacted[$($redacted:ident),* $(,)?], visible[$($visible:ident),* $(,)?]) => {
        impl std::fmt::Debug for $request {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut debug = formatter.debug_struct(stringify!($request));
                $(debug.field(stringify!($redacted), &"[REDACTED]");)*
                $(debug.field(stringify!($visible), &self.$visible);)*
                debug.finish()
            }
        }
    };
}

/// Web-search result summary size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum SearchContentSize {
    /// Balanced summaries, typically 400–600 words.
    Medium,
    /// Maximum context, typically up to 2,500 words.
    High,
}

/// Search region hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SearchLocation {
    /// Chinese-region results (`cn`).
    #[serde(rename = "cn")]
    China,
    /// Non-Chinese-region results (`us`).
    #[serde(rename = "us")]
    International,
}

/// Search recency filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SearchRecency {
    /// Limit results to the previous day.
    #[serde(rename = "oneDay")]
    OneDay,
    /// Limit results to the previous week.
    #[serde(rename = "oneWeek")]
    OneWeek,
    /// Limit results to the previous month.
    #[serde(rename = "oneMonth")]
    OneMonth,
    /// Limit results to the previous year.
    #[serde(rename = "oneYear")]
    OneYear,
    /// Do not apply a recency limit.
    #[serde(rename = "noLimit")]
    NoLimit,
}

/// Complete request for `web_search_prime`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSearchRequest {
    #[serde(rename = "search_query")]
    query: String,
    #[serde(
        rename = "search_domain_filter",
        skip_serializing_if = "Option::is_none"
    )]
    domain: Option<String>,
    #[serde(
        rename = "search_recency_filter",
        skip_serializing_if = "Option::is_none"
    )]
    recency: Option<SearchRecency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_size: Option<SearchContentSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<SearchLocation>,
}

impl_redacted_debug!(
    WebSearchRequest,
    redacted[query, domain],
    visible[recency, content_size, location]
);

impl WebSearchRequest {
    /// Create a web-search request using server defaults for optional fields.
    ///
    /// Search queries should normally be no longer than 70 characters.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            domain: None,
            recency: None,
            content_size: None,
            location: None,
        }
    }

    /// Prefer results from the supplied domain.
    ///
    /// The upstream service treats this as a search constraint, but callers
    /// should still validate returned URLs when strict domain isolation matters.
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Restrict results to a relative time window.
    pub fn recency(mut self, recency: SearchRecency) -> Self {
        self.recency = Some(recency);
        self
    }

    /// Select the amount of summary text returned for each result.
    pub fn content_size(mut self, content_size: SearchContentSize) -> Self {
        self.content_size = Some(content_size);
        self
    }

    /// Supply the search-region hint used to rank results.
    pub fn location(mut self, location: SearchLocation) -> Self {
        self.location = Some(location);
        self
    }
}

impl McpRequest for WebSearchRequest {
    fn validate(&self) -> ZaiResult<()> {
        validate_required(&[("search_query", &self.query)])?;
        if self.query.chars().count() > 70 {
            return Err(crate::client::validation::invalid(
                "MCP search_query must not exceed 70 characters",
            ));
        }
        validate_optional("search_domain_filter", self.domain.as_deref())
    }
}

/// Web-reader output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum WebReaderFormat {
    /// Return GitHub-Flavored Markdown unless separately disabled.
    Markdown,
    /// Return plain text.
    Text,
}

/// Complete request for `webReader`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebReaderRequest {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    return_format: Option<WebReaderFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retain_images: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    with_links_summary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    with_images_summary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    keep_img_data_url: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_gfm: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_cache: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<u32>,
}

impl_redacted_debug!(
    WebReaderRequest,
    redacted[url],
    visible[
        return_format,
        retain_images,
        with_links_summary,
        with_images_summary,
        keep_img_data_url,
        no_gfm,
        no_cache,
        timeout,
    ]
);

impl WebReaderRequest {
    /// Create a page-reader request using server defaults.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            return_format: None,
            retain_images: None,
            with_links_summary: None,
            with_images_summary: None,
            keep_img_data_url: None,
            no_gfm: None,
            no_cache: None,
            timeout: None,
        }
    }

    /// Set the returned content format.
    pub fn format(mut self, format: WebReaderFormat) -> Self {
        self.return_format = Some(format);
        self
    }

    /// Choose whether image references remain in the extracted content.
    pub fn retain_images(mut self, retain: bool) -> Self {
        self.retain_images = Some(retain);
        self
    }

    /// Include the page's link summary.
    pub fn links_summary(mut self, include: bool) -> Self {
        self.with_links_summary = Some(include);
        self
    }

    /// Include the page's image summary.
    pub fn images_summary(mut self, include: bool) -> Self {
        self.with_images_summary = Some(include);
        self
    }

    /// Preserve inline image data URLs instead of removing them.
    pub fn keep_image_data_urls(mut self, keep: bool) -> Self {
        self.keep_img_data_url = Some(keep);
        self
    }

    /// Enable or disable GitHub-Flavored Markdown conversion.
    pub fn github_flavored_markdown(mut self, enabled: bool) -> Self {
        self.no_gfm = Some(!enabled);
        self
    }

    /// Enable or bypass the upstream page cache.
    pub fn cache(mut self, enabled: bool) -> Self {
        self.no_cache = Some(!enabled);
        self
    }

    /// Set the upstream page-fetch timeout in seconds.
    ///
    /// This is distinct from [`McpClient::with_tool_timeout`](super::McpClient::with_tool_timeout),
    /// which bounds the complete MCP operation on the client side.
    pub fn timeout_seconds(mut self, timeout: u32) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl McpRequest for WebReaderRequest {
    fn validate(&self) -> ZaiResult<()> {
        validate_required(&[("url", &self.url)])?;
        let url = url::Url::parse(&self.url)
            .map_err(|_| crate::client::validation::invalid("invalid web-reader URL"))?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(crate::client::validation::invalid(
                "web-reader URL must use the http or https scheme",
            ));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(crate::client::validation::invalid(
                "web-reader URL must not contain user information",
            ));
        }
        if self.timeout == Some(0) {
            return Err(crate::client::validation::invalid(
                "web-reader timeout must be greater than zero",
            ));
        }
        Ok(())
    }
}

/// Repository-search response language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum RepositoryLanguage {
    /// Request a Chinese response.
    Zh,
    /// Request an English response.
    En,
}

/// Complete request for `search_doc`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDocRequest {
    repo_name: String,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<RepositoryLanguage>,
}

impl_redacted_debug!(
    SearchDocRequest,
    redacted[repo_name, query],
    visible[language]
);

impl SearchDocRequest {
    /// Create a repository search for `owner/repository`.
    pub fn new(repository: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            repo_name: repository.into(),
            query: query.into(),
            language: None,
        }
    }

    /// Select the response language.
    pub fn language(mut self, language: RepositoryLanguage) -> Self {
        self.language = Some(language);
        self
    }
}

impl McpRequest for SearchDocRequest {
    fn validate(&self) -> ZaiResult<()> {
        validate_required(&[("repo_name", &self.repo_name), ("query", &self.query)])
    }
}

/// Complete request for `get_repo_structure`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoStructureRequest {
    repo_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dir_path: Option<String>,
}

impl_redacted_debug!(RepoStructureRequest, redacted[repo_name, dir_path], visible[]);

impl RepoStructureRequest {
    /// Create a request for the root tree of `owner/repository`.
    pub fn new(repository: impl Into<String>) -> Self {
        Self {
            repo_name: repository.into(),
            dir_path: None,
        }
    }

    /// Inspect a repository-relative directory instead of the root.
    pub fn directory(mut self, directory: impl Into<String>) -> Self {
        self.dir_path = Some(directory.into());
        self
    }
}

impl McpRequest for RepoStructureRequest {
    fn validate(&self) -> ZaiResult<()> {
        validate_required(&[("repo_name", &self.repo_name)])?;
        validate_optional("dir_path", self.dir_path.as_deref())
    }
}

/// Complete request for `read_file`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadRepoFileRequest {
    repo_name: String,
    file_path: String,
}

impl_redacted_debug!(ReadRepoFileRequest, redacted[repo_name, file_path], visible[]);

impl ReadRepoFileRequest {
    /// Create a request for a repository-relative file path.
    pub fn new(repository: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            repo_name: repository.into(),
            file_path: path.into(),
        }
    }
}

impl McpRequest for ReadRepoFileRequest {
    fn validate(&self) -> ZaiResult<()> {
        validate_required(&[
            ("repo_name", &self.repo_name),
            ("file_path", &self.file_path),
        ])
    }
}

/// Output generated by `ui_to_artifact`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum UiArtifactOutput {
    /// Generate frontend implementation code.
    Code,
    /// Generate an AI prompt for recreating the interface.
    Prompt,
    /// Generate a design specification.
    #[serde(rename = "spec")]
    Specification,
    /// Generate a natural-language description.
    Description,
}

/// Complete request for `ui_to_artifact`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiToArtifactRequest {
    image_source: String,
    output_type: UiArtifactOutput,
    prompt: String,
}

impl_redacted_debug!(
    UiToArtifactRequest,
    redacted[image_source, prompt],
    visible[output_type]
);

impl UiToArtifactRequest {
    /// Create a UI-conversion request from a local image path or remote URL.
    pub fn new(
        image_source: impl Into<String>,
        output_type: UiArtifactOutput,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            image_source: image_source.into(),
            output_type,
            prompt: prompt.into(),
        }
    }
}

/// Complete request for `extract_text_from_screenshot`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractTextRequest {
    image_source: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    programming_language: Option<String>,
}

impl_redacted_debug!(
    ExtractTextRequest,
    redacted[image_source, prompt, programming_language],
    visible[]
);

impl ExtractTextRequest {
    /// Create an OCR request from a local image path or remote URL.
    pub fn new(image_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image_source: image_source.into(),
            prompt: prompt.into(),
            programming_language: None,
        }
    }

    /// Hint at the programming language shown in a code screenshot.
    pub fn programming_language(mut self, language: impl Into<String>) -> Self {
        self.programming_language = Some(language.into());
        self
    }
}

/// Complete request for `diagnose_error_screenshot`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnoseErrorRequest {
    image_source: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
}

impl_redacted_debug!(
    DiagnoseErrorRequest,
    redacted[image_source, prompt, context],
    visible[]
);

impl DiagnoseErrorRequest {
    /// Create an error-diagnosis request from a local image path or remote URL.
    pub fn new(image_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image_source: image_source.into(),
            prompt: prompt.into(),
            context: None,
        }
    }

    /// Describe when or where the error occurred.
    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

/// Complete request for `understand_technical_diagram`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnderstandDiagramRequest {
    image_source: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagram_type: Option<String>,
}

impl_redacted_debug!(
    UnderstandDiagramRequest,
    redacted[image_source, prompt, diagram_type],
    visible[]
);

impl UnderstandDiagramRequest {
    /// Create a technical-diagram request from a local image path or remote URL.
    pub fn new(image_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image_source: image_source.into(),
            prompt: prompt.into(),
            diagram_type: None,
        }
    }

    /// Hint at the diagram type, such as `architecture`, `uml`, or `sequence`.
    pub fn diagram_type(mut self, diagram_type: impl Into<String>) -> Self {
        self.diagram_type = Some(diagram_type.into());
        self
    }
}

/// Complete request for `analyze_data_visualization`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzeVisualizationRequest {
    image_source: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    analysis_focus: Option<String>,
}

impl_redacted_debug!(
    AnalyzeVisualizationRequest,
    redacted[image_source, prompt, analysis_focus],
    visible[]
);

impl AnalyzeVisualizationRequest {
    /// Create a visualization-analysis request from a local image path or URL.
    pub fn new(image_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image_source: image_source.into(),
            prompt: prompt.into(),
            analysis_focus: None,
        }
    }

    /// Narrow the analysis to an area such as trends, anomalies, or comparisons.
    pub fn focus(mut self, focus: impl Into<String>) -> Self {
        self.analysis_focus = Some(focus.into());
        self
    }
}

/// Complete request for `ui_diff_check`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiDiffRequest {
    expected_image_source: String,
    actual_image_source: String,
    prompt: String,
}

impl_redacted_debug!(
    UiDiffRequest,
    redacted[expected_image_source, actual_image_source, prompt],
    visible[]
);

impl UiDiffRequest {
    /// Create a comparison between expected and actual UI screenshots.
    ///
    /// Each image source may be a local path or a remote URL.
    pub fn new(
        expected_image_source: impl Into<String>,
        actual_image_source: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            expected_image_source: expected_image_source.into(),
            actual_image_source: actual_image_source.into(),
            prompt: prompt.into(),
        }
    }
}

/// Complete request for `analyze_image`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzeImageRequest {
    image_source: String,
    prompt: String,
}

impl_redacted_debug!(AnalyzeImageRequest, redacted[image_source, prompt], visible[]);

impl AnalyzeImageRequest {
    /// Create a general image-analysis request from a local path or remote URL.
    pub fn new(image_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image_source: image_source.into(),
            prompt: prompt.into(),
        }
    }
}

/// Complete request for `analyze_video`.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzeVideoRequest {
    video_source: String,
    prompt: String,
}

impl_redacted_debug!(AnalyzeVideoRequest, redacted[video_source, prompt], visible[]);

impl AnalyzeVideoRequest {
    /// Create a video-analysis request from a local path or remote URL.
    ///
    /// The Vision MCP accepts MP4, MOV, and M4V inputs up to 8 MB.
    pub fn new(video_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            video_source: video_source.into(),
            prompt: prompt.into(),
        }
    }
}

macro_rules! impl_required_mcp_request {
    ($request:ty => $($field:ident),+ $(,)?) => {
        impl McpRequest for $request {
            fn validate(&self) -> ZaiResult<()> {
                validate_required(&[$((stringify!($field), &self.$field)),+])
            }
        }
    };
}

impl_required_mcp_request!(UiToArtifactRequest => image_source, prompt);
impl_required_mcp_request!(UiDiffRequest => expected_image_source, actual_image_source, prompt);
impl_required_mcp_request!(AnalyzeImageRequest => image_source, prompt);
impl_required_mcp_request!(AnalyzeVideoRequest => video_source, prompt);

impl McpRequest for ExtractTextRequest {
    fn validate(&self) -> ZaiResult<()> {
        validate_required(&[
            ("image_source", &self.image_source),
            ("prompt", &self.prompt),
        ])?;
        validate_optional("programming_language", self.programming_language.as_deref())
    }
}

impl McpRequest for DiagnoseErrorRequest {
    fn validate(&self) -> ZaiResult<()> {
        validate_required(&[
            ("image_source", &self.image_source),
            ("prompt", &self.prompt),
        ])?;
        validate_optional("context", self.context.as_deref())
    }
}

impl McpRequest for UnderstandDiagramRequest {
    fn validate(&self) -> ZaiResult<()> {
        validate_required(&[
            ("image_source", &self.image_source),
            ("prompt", &self.prompt),
        ])?;
        validate_optional("diagram_type", self.diagram_type.as_deref())
    }
}

impl McpRequest for AnalyzeVisualizationRequest {
    fn validate(&self) -> ZaiResult<()> {
        validate_required(&[
            ("image_source", &self.image_source),
            ("prompt", &self.prompt),
        ])?;
        validate_optional("analysis_focus", self.analysis_focus.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json, to_value};

    #[test]
    fn default_requests_omit_every_optional_field() {
        assert_eq!(
            to_value(WebSearchRequest::new("rust")).unwrap(),
            json!({"search_query": "rust"})
        );
        assert_eq!(
            to_value(WebReaderRequest::new("https://example.com")).unwrap(),
            json!({"url": "https://example.com"})
        );
        assert_eq!(
            to_value(SearchDocRequest::new("owner/repo", "query")).unwrap(),
            json!({"repo_name": "owner/repo", "query": "query"})
        );
        assert_eq!(
            to_value(RepoStructureRequest::new("owner/repo")).unwrap(),
            json!({"repo_name": "owner/repo"})
        );
    }

    #[test]
    fn web_search_serializes_every_captured_field() {
        let request = WebSearchRequest::new("Rust rmcp")
            .domain("docs.rs")
            .recency(SearchRecency::OneMonth)
            .content_size(SearchContentSize::High)
            .location(SearchLocation::International);
        assert_eq!(
            to_value(request).unwrap(),
            json!({
                "search_query": "Rust rmcp",
                "search_domain_filter": "docs.rs",
                "search_recency_filter": "oneMonth",
                "content_size": "high",
                "location": "us"
            })
        );

        let recencies = [
            (SearchRecency::OneDay, "oneDay"),
            (SearchRecency::OneWeek, "oneWeek"),
            (SearchRecency::OneMonth, "oneMonth"),
            (SearchRecency::OneYear, "oneYear"),
            (SearchRecency::NoLimit, "noLimit"),
        ];
        for (value, expected) in recencies {
            assert_eq!(to_value(value).unwrap(), Value::String(expected.to_owned()));
        }

        let content_sizes = [
            (SearchContentSize::Medium, "medium"),
            (SearchContentSize::High, "high"),
        ];
        for (value, expected) in content_sizes {
            assert_eq!(to_value(value).unwrap(), Value::String(expected.to_owned()));
        }

        let locations = [
            (SearchLocation::China, "cn"),
            (SearchLocation::International, "us"),
        ];
        for (value, expected) in locations {
            assert_eq!(to_value(value).unwrap(), Value::String(expected.to_owned()));
        }
    }

    #[test]
    fn web_reader_serializes_every_captured_field() {
        let request = WebReaderRequest::new("https://example.com")
            .format(WebReaderFormat::Text)
            .retain_images(false)
            .links_summary(true)
            .images_summary(true)
            .keep_image_data_urls(true)
            .github_flavored_markdown(false)
            .cache(false)
            .timeout_seconds(30);
        assert_eq!(
            to_value(request).unwrap(),
            json!({
                "url": "https://example.com",
                "return_format": "text",
                "retain_images": false,
                "with_links_summary": true,
                "with_images_summary": true,
                "keep_img_data_url": true,
                "no_gfm": true,
                "no_cache": true,
                "timeout": 30
            })
        );

        assert_eq!(
            to_value(WebReaderFormat::Markdown).unwrap(),
            Value::String("markdown".to_owned())
        );
        assert_eq!(
            to_value(WebReaderFormat::Text).unwrap(),
            Value::String("text".to_owned())
        );
    }

    #[test]
    fn zread_requests_match_all_three_live_schemas() {
        assert_eq!(
            to_value(
                SearchDocRequest::new("owner/repo", "transport").language(RepositoryLanguage::En)
            )
            .unwrap(),
            json!({"repo_name": "owner/repo", "query": "transport", "language": "en"})
        );
        assert_eq!(
            to_value(RepoStructureRequest::new("owner/repo").directory("src")).unwrap(),
            json!({"repo_name": "owner/repo", "dir_path": "src"})
        );
        assert_eq!(
            to_value(ReadRepoFileRequest::new("owner/repo", "README.md")).unwrap(),
            json!({"repo_name": "owner/repo", "file_path": "README.md"})
        );

        assert_eq!(
            to_value(RepositoryLanguage::Zh).unwrap(),
            Value::String("zh".to_owned())
        );
        assert_eq!(
            to_value(RepositoryLanguage::En).unwrap(),
            Value::String("en".to_owned())
        );
    }

    #[test]
    fn all_eight_vision_requests_match_live_schemas() {
        assert_eq!(
            to_value(UiToArtifactRequest::new(
                "ui.png",
                UiArtifactOutput::Specification,
                "write a spec"
            ))
            .unwrap(),
            json!({"image_source": "ui.png", "output_type": "spec", "prompt": "write a spec"})
        );
        assert_eq!(
            to_value(
                ExtractTextRequest::new("terminal.png", "extract").programming_language("rust")
            )
            .unwrap(),
            json!({"image_source": "terminal.png", "prompt": "extract", "programming_language": "rust"})
        );
        assert_eq!(
            to_value(DiagnoseErrorRequest::new("error.png", "diagnose").context("cargo build"))
                .unwrap(),
            json!({"image_source": "error.png", "prompt": "diagnose", "context": "cargo build"})
        );
        assert_eq!(
            to_value(
                UnderstandDiagramRequest::new("diagram.png", "explain")
                    .diagram_type("architecture")
            )
            .unwrap(),
            json!({"image_source": "diagram.png", "prompt": "explain", "diagram_type": "architecture"})
        );
        assert_eq!(
            to_value(AnalyzeVisualizationRequest::new("chart.png", "analyze").focus("trends"))
                .unwrap(),
            json!({"image_source": "chart.png", "prompt": "analyze", "analysis_focus": "trends"})
        );
        assert_eq!(
            to_value(UiDiffRequest::new("expected.png", "actual.png", "compare")).unwrap(),
            json!({
                "expected_image_source": "expected.png",
                "actual_image_source": "actual.png",
                "prompt": "compare"
            })
        );
        assert_eq!(
            to_value(AnalyzeImageRequest::new("photo.png", "describe")).unwrap(),
            json!({"image_source": "photo.png", "prompt": "describe"})
        );
        assert_eq!(
            to_value(AnalyzeVideoRequest::new("clip.mp4", "summarize")).unwrap(),
            json!({"video_source": "clip.mp4", "prompt": "summarize"})
        );
    }

    #[test]
    fn vision_defaults_omit_every_optional_field() {
        assert_eq!(
            to_value(ExtractTextRequest::new("terminal.png", "extract")).unwrap(),
            json!({"image_source": "terminal.png", "prompt": "extract"})
        );
        assert_eq!(
            to_value(DiagnoseErrorRequest::new("error.png", "diagnose")).unwrap(),
            json!({"image_source": "error.png", "prompt": "diagnose"})
        );
        assert_eq!(
            to_value(UnderstandDiagramRequest::new("diagram.png", "explain")).unwrap(),
            json!({"image_source": "diagram.png", "prompt": "explain"})
        );
        assert_eq!(
            to_value(AnalyzeVisualizationRequest::new("chart.png", "analyze")).unwrap(),
            json!({"image_source": "chart.png", "prompt": "analyze"})
        );
    }

    #[test]
    fn ui_artifact_output_serializes_all_schema_enum_values() {
        let cases = [
            (UiArtifactOutput::Code, "code"),
            (UiArtifactOutput::Prompt, "prompt"),
            (UiArtifactOutput::Specification, "spec"),
            (UiArtifactOutput::Description, "description"),
        ];
        for (value, expected) in cases {
            assert_eq!(to_value(value).unwrap(), Value::String(expected.to_owned()));
        }
    }

    #[test]
    fn request_validation_rejects_blank_required_fields() {
        assert!(WebSearchRequest::new(" ").validate().is_err());
        assert!(WebSearchRequest::new("x".repeat(71)).validate().is_err());
        assert!(SearchDocRequest::new("owner/repo", "").validate().is_err());
        assert!(AnalyzeImageRequest::new("", "describe").validate().is_err());
        assert!(
            ExtractTextRequest::new("image.png", "extract")
                .programming_language(" ")
                .validate()
                .is_err()
        );
    }

    #[test]
    fn web_reader_requires_http_url_and_positive_timeout() {
        assert!(
            WebReaderRequest::new("file:///tmp/page.html")
                .validate()
                .is_err()
        );
        assert!(
            WebReaderRequest::new("https://example.com")
                .timeout_seconds(0)
                .validate()
                .is_err()
        );
        assert!(
            WebReaderRequest::new("https://example.com")
                .timeout_seconds(30)
                .validate()
                .is_ok()
        );
        assert!(
            WebReaderRequest::new("https://user:password@example.com/private")
                .validate()
                .is_err()
        );
    }

    #[test]
    fn request_debug_output_redacts_queries_paths_and_media_sources() {
        let requests = [
            format!(
                "{:?}",
                WebSearchRequest::new("private-query").domain("private.example")
            ),
            format!(
                "{:?}",
                WebReaderRequest::new("https://private.example/page")
            ),
            format!(
                "{:?}",
                SearchDocRequest::new("private/repository", "private-doc-query")
            ),
            format!(
                "{:?}",
                DiagnoseErrorRequest::new("private-image.png", "private-prompt")
                    .context("private-context")
            ),
            format!(
                "{:?}",
                UiDiffRequest::new("private-expected.png", "private-actual.png", "compare")
            ),
            format!(
                "{:?}",
                AnalyzeVideoRequest::new("private-video.mp4", "private-video-prompt")
            ),
        ];
        for debug in requests {
            for secret in [
                "private-query",
                "private.example",
                "private/repository",
                "private-doc-query",
                "private-image.png",
                "private-prompt",
                "private-context",
                "private-expected.png",
                "private-actual.png",
                "private-video.mp4",
                "private-video-prompt",
            ] {
                assert!(!debug.contains(secret), "Debug leaked {secret}");
            }
        }
    }
}
