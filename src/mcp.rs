//! Unified API for MCP capabilities.
//!
//! Callers use capability-oriented methods such as
//! [`McpClient::web_search`](crate::mcp::McpClient::web_search)
//! and
//! [`McpClient::analyze_image`](crate::mcp::McpClient::analyze_image).
//! Server selection, Streamable HTTP
//! versus stdio transport, connection initialization, and connection reuse are
//! handled internally.

use rmcp::{
    RoleClient, ServiceExt,
    model::{CallToolRequestParams, CallToolResult, ClientInfo, Tool},
    service::RunningService,
    transport::{
        ConfigureCommandExt, StreamableHttpClientTransport, TokioChildProcess,
        streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

mod requests;
mod responses;

pub use requests::*;
pub use responses::{McpTextResponse, WebReaderResponse, WebSearchResponse, WebSearchResult};

const VISION_MCP_PACKAGE: &str = "@z_ai/mcp-server@0.1.2";
/// Node.js ships `npx` as a `npx.cmd` shim on Windows, which
/// `CreateProcess` cannot resolve from the bare `npx` name.
const NPX_PROGRAM: &str = if cfg!(windows) { "npx.cmd" } else { "npx" };
const DEFAULT_TOOL_TIMEOUT: Duration = Duration::from_secs(300);
const TOOL_WEB_SEARCH: &str = "web_search_prime";
const TOOL_WEB_READER: &str = "webReader";
const TOOL_SEARCH_DOC: &str = "search_doc";
const TOOL_REPO_STRUCTURE: &str = "get_repo_structure";
const TOOL_READ_FILE: &str = "read_file";
const TOOL_UI_TO_ARTIFACT: &str = "ui_to_artifact";
const TOOL_EXTRACT_TEXT: &str = "extract_text_from_screenshot";
const TOOL_DIAGNOSE_ERROR: &str = "diagnose_error_screenshot";
const TOOL_UNDERSTAND_DIAGRAM: &str = "understand_technical_diagram";
const TOOL_ANALYZE_VISUALIZATION: &str = "analyze_data_visualization";
const TOOL_UI_DIFF: &str = "ui_diff_check";
const TOOL_ANALYZE_IMAGE: &str = "analyze_image";
const TOOL_ANALYZE_VIDEO: &str = "analyze_video";

use crate::{
    ZaiResult,
    client::{
        error::{ZaiError, codes},
        secret::ApiSecret,
    },
    model::{
        chat_message_types::VisionMessage,
        traits::{Bounded, ModelName},
    },
};

/// Service region used to select the official MCP endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpRegion {
    /// China service at `open.bigmodel.cn`.
    Zhipu,
    /// International service at `api.z.ai`.
    Zai,
}

impl McpRegion {
    const fn origin(self) -> &'static str {
        match self {
            Self::Zhipu => "https://open.bigmodel.cn",
            Self::Zai => "https://api.z.ai",
        }
    }

    const fn vision_mode(self) -> &'static str {
        match self {
            Self::Zhipu => "ZHIPU",
            Self::Zai => "ZAI",
        }
    }
}

/// Internal target selected from a capability or advertised tool name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpBackend {
    WebSearch,
    WebReader,
    Zread,
    Vision,
}

impl McpBackend {
    fn endpoint(self, region: McpRegion) -> Option<String> {
        let path = match self {
            Self::WebSearch => "web_search_prime",
            Self::WebReader => "web_reader",
            Self::Zread => "zread",
            Self::Vision => return None,
        };
        Some(format!("{}/api/mcp/{path}/mcp", region.origin()))
    }

    fn for_tool(name: &str) -> Option<Self> {
        match name {
            TOOL_WEB_SEARCH => Some(Self::WebSearch),
            TOOL_WEB_READER => Some(Self::WebReader),
            TOOL_SEARCH_DOC | TOOL_REPO_STRUCTURE | TOOL_READ_FILE => Some(Self::Zread),
            TOOL_UI_TO_ARTIFACT
            | TOOL_EXTRACT_TEXT
            | TOOL_DIAGNOSE_ERROR
            | TOOL_UNDERSTAND_DIAGRAM
            | TOOL_ANALYZE_VISUALIZATION
            | TOOL_UI_DIFF
            | TOOL_ANALYZE_IMAGE
            | TOOL_ANALYZE_VIDEO => Some(Self::Vision),
            _ => None,
        }
    }
}

struct McpConnection {
    service: RunningService<RoleClient, ClientInfo>,
}

impl McpConnection {
    async fn connect_with_key(
        backend: McpBackend,
        region: McpRegion,
        api_key: &str,
        vision_model: Option<&str>,
    ) -> ZaiResult<Self> {
        crate::client::error::validate_api_key(api_key)?;
        if backend == McpBackend::Vision {
            return Self::connect_vision(region, api_key, vision_model).await;
        }
        Self::connect_remote(backend, region, api_key).await
    }

    async fn connect_remote(
        backend: McpBackend,
        region: McpRegion,
        api_key: &str,
    ) -> ZaiResult<Self> {
        let endpoint = backend.endpoint(region).ok_or_else(|| ZaiError::Unknown {
            code: codes::SDK_EXTERNAL_TOOL,
            message: "vision MCP does not have a remote endpoint".to_owned(),
        })?;
        let config = StreamableHttpClientTransportConfig::with_uri(endpoint)
            .auth_header(api_key.to_owned())
            .reinit_on_expired_session(true);
        let transport = StreamableHttpClientTransport::from_config(config);
        let service = ClientInfo::default()
            .serve(transport)
            .await
            .map_err(external_error("connect to MCP"))?;
        Ok(Self { service })
    }

    async fn connect_vision(
        region: McpRegion,
        api_key: &str,
        vision_model: Option<&str>,
    ) -> ZaiResult<Self> {
        let transport =
            TokioChildProcess::new(tokio::process::Command::new(NPX_PROGRAM).configure(|cmd| {
                cmd.args(["-y", VISION_MCP_PACKAGE])
                    .env("Z_AI_API_KEY", api_key)
                    .env("Z_AI_MODE", region.vision_mode());
                if let Some(model) = vision_model {
                    cmd.env("Z_AI_VISION_MODEL", model);
                }
            }))
            .map_err(vision_start_error)?;
        let service = ClientInfo::default()
            .serve(transport)
            .await
            .map_err(external_error("connect to vision MCP"))?;
        Ok(Self { service })
    }

    async fn tools(&self) -> ZaiResult<Vec<Tool>> {
        self.service
            .peer()
            .list_all_tools()
            .await
            .map_err(external_error("list MCP tools"))
    }

    async fn call(
        &self,
        name: &str,
        arguments: serde_json::Map<String, Value>,
    ) -> ZaiResult<CallToolResult> {
        self.service
            .peer()
            .call_tool(CallToolRequestParams::new(name.to_owned()).with_arguments(arguments))
            .await
            .map_err(external_error("call MCP tool"))
    }

    async fn close(self) -> ZaiResult<()> {
        self.service
            .cancel()
            .await
            .map(|_| ())
            .map_err(external_error("close MCP"))
    }
}

/// Unified client for MCP capabilities.
///
/// Connections are created lazily on the first capability call and reused
/// afterwards. The user never needs to select an MCP server or transport.
/// Remote capabilities use Streamable HTTP. Vision capabilities start the
/// pinned `@z_ai/mcp-server` package through `npx`, so Node.js and `npx` must be
/// available when a vision method or [`McpClient::tools`] is first called.
pub struct McpClient {
    region: McpRegion,
    api_key: ApiSecret,
    web_search: tokio::sync::OnceCell<McpConnection>,
    web_reader: tokio::sync::OnceCell<McpConnection>,
    zread: tokio::sync::OnceCell<McpConnection>,
    vision: tokio::sync::OnceCell<McpConnection>,
    tool_timeout: Duration,
    vision_model: Option<String>,
}

impl McpClient {
    /// Build a client from `Z_AI_API_KEY` or `ZHIPU_API_KEY`.
    ///
    /// `Z_AI_MODE=ZAI` selects the international service; otherwise the China
    /// service is used. No network connection is made until a capability is
    /// called.
    pub fn from_env() -> ZaiResult<Self> {
        let api_key = api_key_from_env()?;
        Self::new(api_key)
    }

    /// Build a lazily connected client. The service region is inferred from
    /// `Z_AI_MODE`, defaulting to the China service. Invalid credentials are
    /// rejected before any connection or child process can be started.
    pub fn new(api_key: impl Into<String>) -> ZaiResult<Self> {
        Self::with_region(api_key, region_from_env())
    }

    /// Build a client with an explicit service region.
    pub fn with_region(api_key: impl Into<String>, region: McpRegion) -> ZaiResult<Self> {
        let api_key = api_key.into();
        crate::client::error::validate_api_key(&api_key)?;
        Ok(Self {
            region,
            api_key: ApiSecret::new(api_key),
            web_search: tokio::sync::OnceCell::new(),
            web_reader: tokio::sync::OnceCell::new(),
            zread: tokio::sync::OnceCell::new(),
            vision: tokio::sync::OnceCell::new(),
            tool_timeout: DEFAULT_TOOL_TIMEOUT,
            vision_model: None,
        })
    }

    /// Set the maximum duration of connection setup plus one MCP tool call.
    ///
    /// The default is five minutes because vision operations can take longer
    /// than ordinary remote tools.
    pub fn with_tool_timeout(mut self, timeout: Duration) -> Self {
        self.tool_timeout = timeout;
        self
    }

    /// Override the vision model used by the local vision MCP server.
    ///
    /// Only models with image recognition capability — those bound to
    /// [`VisionMessage`] — are accepted at compile time. The model id is
    /// passed as `Z_AI_VISION_MODEL` to the spawned `@z_ai/mcp-server`
    /// process, replacing its built-in default (`glm-4.6v`). It takes effect
    /// when the vision backend is first started; setting it after a vision
    /// capability has connected has no effect on the already-running server.
    ///
    /// ```
    /// use zai_rs::{mcp::McpClient, model::chat_models::GLM5V_turbo};
    ///
    /// let client = McpClient::new("test.12345678901234567890")
    ///     .unwrap()
    ///     .with_vision_model(GLM5V_turbo {});
    /// ```
    pub fn with_vision_model<M>(mut self, _model: M) -> Self
    where
        M: ModelName,
        (M, VisionMessage): Bounded,
    {
        self.vision_model = Some(M::NAME.to_owned());
        self
    }

    async fn connection(&self, backend: McpBackend) -> ZaiResult<&McpConnection> {
        let cell = match backend {
            McpBackend::WebSearch => &self.web_search,
            McpBackend::WebReader => &self.web_reader,
            McpBackend::Zread => &self.zread,
            McpBackend::Vision => &self.vision,
        };
        cell.get_or_try_init(|| {
            McpConnection::connect_with_key(
                backend,
                self.region,
                self.api_key.expose(),
                self.vision_model.as_deref(),
            )
        })
        .await
    }

    /// Return all available MCP tools.
    ///
    /// This initializes each capability backend, including the local vision
    /// runtime, and follows MCP pagination internally.
    pub async fn tools(&self) -> ZaiResult<Vec<Tool>> {
        let (search, reader, zread, vision) = tokio::try_join!(
            self.connection(McpBackend::WebSearch),
            self.connection(McpBackend::WebReader),
            self.connection(McpBackend::Zread),
            self.connection(McpBackend::Vision),
        )?;
        let (mut tools, reader_tools, zread_tools, vision_tools) = tokio::try_join!(
            search.tools(),
            reader.tools(),
            zread.tools(),
            vision.tools(),
        )?;
        tools.extend(reader_tools);
        tools.extend(zread_tools);
        tools.extend(vision_tools);
        Ok(tools)
    }

    /// Invoke a tool supported by this SDK with raw JSON arguments.
    ///
    /// Most users should use the typed capability methods instead. The correct
    /// backend and transport are still selected automatically. `arguments` must
    /// be a JSON object, and the returned value is the complete MCP
    /// `CallToolResult` envelope rather than only its text or structured content.
    pub async fn call_raw(&self, name: &str, arguments: Value) -> ZaiResult<Value> {
        Ok(serde_json::to_value(
            self.call_result(name, arguments).await?,
        )?)
    }

    async fn call_result(&self, name: &str, arguments: Value) -> ZaiResult<CallToolResult> {
        let Value::Object(arguments) = arguments else {
            return Err(ZaiError::ApiError {
                code: codes::SDK_VALIDATION,
                message: "MCP tool arguments must be a JSON object".to_owned(),
            });
        };
        let backend = McpBackend::for_tool(name).ok_or_else(|| ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: format!("unknown MCP tool: {name}"),
        })?;
        tokio::time::timeout(self.tool_timeout, async {
            self.connection(backend).await?.call(name, arguments).await
        })
        .await
        .map_err(|_| ZaiError::ApiError {
            code: codes::SDK_TIMEOUT,
            message: format!("MCP tool {name} timed out after {:?}", self.tool_timeout),
        })?
    }

    async fn call_request<R>(&self, name: &str, request: &R) -> ZaiResult<CallToolResult>
    where
        R: Serialize + requests::McpRequest + ?Sized,
    {
        request.validate()?;
        self.call_result(name, serde_json::to_value(request)?).await
    }

    async fn call_text_request<R>(&self, name: &str, request: &R) -> ZaiResult<McpTextResponse>
    where
        R: Serialize + requests::McpRequest + ?Sized,
    {
        responses::text_response(self.call_request(name, request).await?)
    }

    /// Search the web with server defaults.
    pub async fn web_search(&self, query: impl Into<String>) -> ZaiResult<WebSearchResponse> {
        self.web_search_with(WebSearchRequest::new(query)).await
    }

    /// Search the web with complete typed options.
    pub async fn web_search_with(&self, request: WebSearchRequest) -> ZaiResult<WebSearchResponse> {
        responses::web_search_response(self.call_request(TOOL_WEB_SEARCH, &request).await?)
    }

    /// Read a web page with server defaults.
    pub async fn read_web_page(&self, url: impl Into<String>) -> ZaiResult<WebReaderResponse> {
        self.read_web_page_with(WebReaderRequest::new(url)).await
    }

    /// Read a web page with complete typed options.
    pub async fn read_web_page_with(
        &self,
        request: WebReaderRequest,
    ) -> ZaiResult<WebReaderResponse> {
        responses::web_reader_response(self.call_request(TOOL_WEB_READER, &request).await?)
    }

    /// Search documentation and project knowledge for a public GitHub repo.
    pub async fn search_repo(
        &self,
        repository: impl Into<String>,
        query: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.search_repo_with(SearchDocRequest::new(repository, query))
            .await
    }

    /// Search repository knowledge with a typed language option.
    pub async fn search_repo_with(&self, request: SearchDocRequest) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_SEARCH_DOC, &request).await
    }

    /// Get the root directory tree of a public GitHub repository.
    pub async fn repo_structure(
        &self,
        repository: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.repo_structure_with(RepoStructureRequest::new(repository))
            .await
    }

    /// Get a repository tree at a typed directory path.
    pub async fn repo_structure_with(
        &self,
        request: RepoStructureRequest,
    ) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_REPO_STRUCTURE, &request).await
    }

    /// Read one file from a public GitHub repository.
    pub async fn read_repo_file(
        &self,
        repository: impl Into<String>,
        path: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.read_repo_file_with(ReadRepoFileRequest::new(repository, path))
            .await
    }

    /// Read one repository file using a typed request.
    pub async fn read_repo_file_with(
        &self,
        request: ReadRepoFileRequest,
    ) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_READ_FILE, &request).await
    }

    /// Perform general image analysis.
    pub async fn analyze_image(
        &self,
        image_source: impl Into<String>,
        prompt: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.analyze_image_with(AnalyzeImageRequest::new(image_source, prompt))
            .await
    }

    /// Perform general image analysis using a typed request.
    pub async fn analyze_image_with(
        &self,
        request: AnalyzeImageRequest,
    ) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_ANALYZE_IMAGE, &request).await
    }

    /// Extract text from a screenshot with automatic language detection.
    pub async fn extract_text(
        &self,
        image_source: impl Into<String>,
        prompt: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.extract_text_with(ExtractTextRequest::new(image_source, prompt))
            .await
    }

    /// Extract text with a typed programming-language hint.
    pub async fn extract_text_with(
        &self,
        request: ExtractTextRequest,
    ) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_EXTRACT_TEXT, &request).await
    }

    /// Diagnose an error screenshot.
    pub async fn diagnose_error(
        &self,
        image_source: impl Into<String>,
        prompt: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.diagnose_error_with(DiagnoseErrorRequest::new(image_source, prompt))
            .await
    }

    /// Diagnose an error screenshot with typed execution context.
    pub async fn diagnose_error_with(
        &self,
        request: DiagnoseErrorRequest,
    ) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_DIAGNOSE_ERROR, &request).await
    }

    /// Explain a technical diagram with automatic type detection.
    pub async fn understand_diagram(
        &self,
        image_source: impl Into<String>,
        prompt: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.understand_diagram_with(UnderstandDiagramRequest::new(image_source, prompt))
            .await
    }

    /// Explain a technical diagram with a typed diagram hint.
    pub async fn understand_diagram_with(
        &self,
        request: UnderstandDiagramRequest,
    ) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_UNDERSTAND_DIAGRAM, &request)
            .await
    }

    /// Analyze a chart or dashboard comprehensively.
    pub async fn analyze_visualization(
        &self,
        image_source: impl Into<String>,
        prompt: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.analyze_visualization_with(AnalyzeVisualizationRequest::new(image_source, prompt))
            .await
    }

    /// Analyze a visualization with a typed focus hint.
    pub async fn analyze_visualization_with(
        &self,
        request: AnalyzeVisualizationRequest,
    ) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_ANALYZE_VISUALIZATION, &request)
            .await
    }

    /// Convert a UI screenshot into code, a prompt, a specification, or a
    /// natural-language description.
    pub async fn ui_to_artifact(
        &self,
        image_source: impl Into<String>,
        output: UiArtifactOutput,
        prompt: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.ui_to_artifact_with(UiToArtifactRequest::new(image_source, output, prompt))
            .await
    }

    /// Convert a UI screenshot using a typed request.
    pub async fn ui_to_artifact_with(
        &self,
        request: UiToArtifactRequest,
    ) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_UI_TO_ARTIFACT, &request).await
    }

    /// Compare an expected UI screenshot with an actual screenshot.
    pub async fn compare_ui(
        &self,
        expected_image_source: impl Into<String>,
        actual_image_source: impl Into<String>,
        prompt: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.compare_ui_with(UiDiffRequest::new(
            expected_image_source,
            actual_image_source,
            prompt,
        ))
        .await
    }

    /// Compare UI screenshots using a typed request.
    pub async fn compare_ui_with(&self, request: UiDiffRequest) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_UI_DIFF, &request).await
    }

    /// Analyze a local or remote MP4/MOV/M4V video with the vision server.
    ///
    /// The Vision MCP accepts files up to 8 MB.
    pub async fn analyze_video(
        &self,
        video_source: impl Into<String>,
        prompt: impl Into<String>,
    ) -> ZaiResult<McpTextResponse> {
        self.analyze_video_with(AnalyzeVideoRequest::new(video_source, prompt))
            .await
    }

    /// Analyze a video using a typed request.
    pub async fn analyze_video_with(
        &self,
        request: AnalyzeVideoRequest,
    ) -> ZaiResult<McpTextResponse> {
        self.call_text_request(TOOL_ANALYZE_VIDEO, &request).await
    }

    /// Shut down every connection that was initialized by this client.
    pub async fn close(self) -> ZaiResult<()> {
        let connections = [
            self.web_search.into_inner(),
            self.web_reader.into_inner(),
            self.zread.into_inner(),
            self.vision.into_inner(),
        ];
        let mut first_error = None;
        for connection in connections.into_iter().flatten() {
            if let Err(error) = connection.close().await
                && first_error.is_none()
            {
                first_error = Some(error);
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

fn api_key_from_env() -> ZaiResult<String> {
    ["Z_AI_API_KEY", "ZHIPU_API_KEY"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
        .ok_or_else(|| ZaiError::ApiError {
            code: codes::SDK_VALIDATION,
            message: "set a non-empty Z_AI_API_KEY or ZHIPU_API_KEY".to_owned(),
        })
}

fn region_from_env() -> McpRegion {
    match std::env::var("Z_AI_MODE") {
        Ok(mode) if mode.trim().eq_ignore_ascii_case("ZAI") => McpRegion::Zai,
        _ => McpRegion::Zhipu,
    }
}

fn external_error<E>(operation: &'static str) -> impl FnOnce(E) -> ZaiError {
    move |_error| ZaiError::Unknown {
        code: codes::SDK_EXTERNAL_TOOL,
        // External transports may include authorization headers, URLs, tool
        // arguments, or provider text in Display. Preserve a stable operation
        // label without copying that untrusted detail into the public error.
        message: format!("failed to {operation}"),
    }
}

fn vision_start_error<E: std::fmt::Display>(error: E) -> ZaiError {
    ZaiError::Unknown {
        code: codes::SDK_EXTERNAL_TOOL,
        // Spawn failures come from the OS (for example a missing executable),
        // never from provider payloads, so the detail is safe to surface.
        message: format!(
            "failed to start vision MCP: {error}; the vision backend runs \
             `{VISION_MCP_PACKAGE}` through `{NPX_PROGRAM}`, so install Node.js \
             and make sure `{NPX_PROGRAM}` is on PATH"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_remote_endpoints_are_region_aware() {
        assert_eq!(
            McpBackend::WebSearch.endpoint(McpRegion::Zhipu),
            Some("https://open.bigmodel.cn/api/mcp/web_search_prime/mcp".to_owned())
        );
        assert_eq!(
            McpBackend::WebReader.endpoint(McpRegion::Zai),
            Some("https://api.z.ai/api/mcp/web_reader/mcp".to_owned())
        );
        assert_eq!(
            McpBackend::Zread.endpoint(McpRegion::Zhipu),
            Some("https://open.bigmodel.cn/api/mcp/zread/mcp".to_owned())
        );
        assert_eq!(McpBackend::Vision.endpoint(McpRegion::Zhipu), None);
    }

    #[test]
    fn tool_names_route_to_their_backends_internally() {
        let cases = [
            ("web_search_prime", McpBackend::WebSearch),
            ("webReader", McpBackend::WebReader),
            ("search_doc", McpBackend::Zread),
            ("get_repo_structure", McpBackend::Zread),
            ("read_file", McpBackend::Zread),
            ("ui_to_artifact", McpBackend::Vision),
            ("extract_text_from_screenshot", McpBackend::Vision),
            ("diagnose_error_screenshot", McpBackend::Vision),
            ("understand_technical_diagram", McpBackend::Vision),
            ("analyze_data_visualization", McpBackend::Vision),
            ("ui_diff_check", McpBackend::Vision),
            ("analyze_image", McpBackend::Vision),
            ("analyze_video", McpBackend::Vision),
        ];
        for (tool, backend) in cases {
            assert_eq!(McpBackend::for_tool(tool), Some(backend), "{tool}");
        }
        assert_eq!(McpBackend::for_tool("not_a_tool"), None);
    }

    #[test]
    fn client_does_not_connect_until_a_capability_is_used() {
        let client = McpClient::with_region("test.12345678901234567890", McpRegion::Zhipu).unwrap();
        assert!(client.web_search.get().is_none());
        assert!(client.web_reader.get().is_none());
        assert!(client.zread.get().is_none());
        assert!(client.vision.get().is_none());
        assert!(McpClient::with_region("secret", McpRegion::Zhipu).is_err());
    }

    #[tokio::test]
    async fn raw_calls_reject_non_object_arguments_before_connecting() {
        let client = McpClient::with_region("test.12345678901234567890", McpRegion::Zhipu).unwrap();
        assert!(
            client
                .call_raw(TOOL_WEB_SEARCH, serde_json::json!([]))
                .await
                .is_err()
        );
        assert!(client.web_search.get().is_none());
    }

    #[tokio::test]
    async fn malformed_credentials_are_rejected_before_mcp_startup() {
        let error = McpConnection::connect_with_key(
            McpBackend::WebSearch,
            McpRegion::Zhipu,
            "bad\ncredential",
            None,
        )
        .await
        .err()
        .expect("invalid header data must fail before a connection is attempted");
        assert_eq!(error.code(), Some(codes::SDK_VALIDATION));
    }

    #[test]
    fn external_errors_do_not_copy_provider_details() {
        let secret = "customer prompt and test.12345678901234567890";
        let error = external_error("call MCP tool")(secret);
        assert_eq!(error.message(), "failed to call MCP tool");
        assert!(!error.to_string().contains(secret));
    }

    #[test]
    fn vision_start_errors_explain_the_node_requirement() {
        let error = vision_start_error(std::io::Error::from(std::io::ErrorKind::NotFound));
        assert_eq!(error.code(), Some(codes::SDK_EXTERNAL_TOOL));
        assert!(error.message().contains("failed to start vision MCP"));
        assert!(error.message().contains(NPX_PROGRAM));
        assert!(error.message().contains("Node.js"));
    }

    #[test]
    fn vision_model_override_is_stored_for_the_vision_backend() {
        let client = McpClient::with_region("test.12345678901234567890", McpRegion::Zhipu)
            .unwrap()
            .with_vision_model("glm-5v-turbo");
        assert_eq!(client.vision_model.as_deref(), Some("glm-5v-turbo"));
        assert!(client.vision.get().is_none());
    }
}
