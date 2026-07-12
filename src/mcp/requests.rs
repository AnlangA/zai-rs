//! Strongly typed request models for every supported MCP tool.

use serde::{Deserialize, Serialize};

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
    #[serde(rename = "cn")]
    China,
    #[serde(rename = "us")]
    International,
}

/// Search recency filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SearchRecency {
    #[serde(rename = "oneDay")]
    OneDay,
    #[serde(rename = "oneWeek")]
    OneWeek,
    #[serde(rename = "oneMonth")]
    OneMonth,
    #[serde(rename = "oneYear")]
    OneYear,
    #[serde(rename = "noLimit")]
    NoLimit,
}

/// Complete request for `web_search_prime`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl WebSearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            domain: None,
            recency: None,
            content_size: None,
            location: None,
        }
    }

    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    pub fn recency(mut self, recency: SearchRecency) -> Self {
        self.recency = Some(recency);
        self
    }

    pub fn content_size(mut self, content_size: SearchContentSize) -> Self {
        self.content_size = Some(content_size);
        self
    }

    pub fn location(mut self, location: SearchLocation) -> Self {
        self.location = Some(location);
        self
    }
}

/// Web-reader output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum WebReaderFormat {
    Markdown,
    Text,
}

/// Complete request for `webReader`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl WebReaderRequest {
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

    pub fn format(mut self, format: WebReaderFormat) -> Self {
        self.return_format = Some(format);
        self
    }

    pub fn retain_images(mut self, retain: bool) -> Self {
        self.retain_images = Some(retain);
        self
    }

    pub fn links_summary(mut self, include: bool) -> Self {
        self.with_links_summary = Some(include);
        self
    }

    pub fn images_summary(mut self, include: bool) -> Self {
        self.with_images_summary = Some(include);
        self
    }

    pub fn keep_image_data_urls(mut self, keep: bool) -> Self {
        self.keep_img_data_url = Some(keep);
        self
    }

    pub fn github_flavored_markdown(mut self, enabled: bool) -> Self {
        self.no_gfm = Some(!enabled);
        self
    }

    pub fn cache(mut self, enabled: bool) -> Self {
        self.no_cache = Some(!enabled);
        self
    }

    pub fn timeout_seconds(mut self, timeout: u32) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

/// Repository-search response language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum RepositoryLanguage {
    Zh,
    En,
}

/// Complete request for `search_doc`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDocRequest {
    repo_name: String,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<RepositoryLanguage>,
}

impl SearchDocRequest {
    pub fn new(repository: impl Into<String>, query: impl Into<String>) -> Self {
        Self {
            repo_name: repository.into(),
            query: query.into(),
            language: None,
        }
    }

    pub fn language(mut self, language: RepositoryLanguage) -> Self {
        self.language = Some(language);
        self
    }
}

/// Complete request for `get_repo_structure`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoStructureRequest {
    repo_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dir_path: Option<String>,
}

impl RepoStructureRequest {
    pub fn new(repository: impl Into<String>) -> Self {
        Self {
            repo_name: repository.into(),
            dir_path: None,
        }
    }

    pub fn directory(mut self, directory: impl Into<String>) -> Self {
        self.dir_path = Some(directory.into());
        self
    }
}

/// Complete request for `read_file`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadRepoFileRequest {
    repo_name: String,
    file_path: String,
}

impl ReadRepoFileRequest {
    pub fn new(repository: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            repo_name: repository.into(),
            file_path: path.into(),
        }
    }
}

/// Output generated by `ui_to_artifact`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum UiArtifactOutput {
    Code,
    Prompt,
    #[serde(rename = "spec")]
    Specification,
    Description,
}

/// Complete request for `ui_to_artifact`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiToArtifactRequest {
    image_source: String,
    output_type: UiArtifactOutput,
    prompt: String,
}

impl UiToArtifactRequest {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractTextRequest {
    image_source: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    programming_language: Option<String>,
}

impl ExtractTextRequest {
    pub fn new(image_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image_source: image_source.into(),
            prompt: prompt.into(),
            programming_language: None,
        }
    }

    pub fn programming_language(mut self, language: impl Into<String>) -> Self {
        self.programming_language = Some(language.into());
        self
    }
}

/// Complete request for `diagnose_error_screenshot`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnoseErrorRequest {
    image_source: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
}

impl DiagnoseErrorRequest {
    pub fn new(image_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image_source: image_source.into(),
            prompt: prompt.into(),
            context: None,
        }
    }

    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
}

/// Complete request for `understand_technical_diagram`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnderstandDiagramRequest {
    image_source: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagram_type: Option<String>,
}

impl UnderstandDiagramRequest {
    pub fn new(image_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image_source: image_source.into(),
            prompt: prompt.into(),
            diagram_type: None,
        }
    }

    pub fn diagram_type(mut self, diagram_type: impl Into<String>) -> Self {
        self.diagram_type = Some(diagram_type.into());
        self
    }
}

/// Complete request for `analyze_data_visualization`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzeVisualizationRequest {
    image_source: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    analysis_focus: Option<String>,
}

impl AnalyzeVisualizationRequest {
    pub fn new(image_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image_source: image_source.into(),
            prompt: prompt.into(),
            analysis_focus: None,
        }
    }

    pub fn focus(mut self, focus: impl Into<String>) -> Self {
        self.analysis_focus = Some(focus.into());
        self
    }
}

/// Complete request for `ui_diff_check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiDiffRequest {
    expected_image_source: String,
    actual_image_source: String,
    prompt: String,
}

impl UiDiffRequest {
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzeImageRequest {
    image_source: String,
    prompt: String,
}

impl AnalyzeImageRequest {
    pub fn new(image_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            image_source: image_source.into(),
            prompt: prompt.into(),
        }
    }
}

/// Complete request for `analyze_video`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzeVideoRequest {
    video_source: String,
    prompt: String,
}

impl AnalyzeVideoRequest {
    pub fn new(video_source: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            video_source: video_source.into(),
            prompt: prompt.into(),
        }
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
}
