#![cfg(feature = "mcp")]

use zai_rs::mcp::{
    AnalyzeImageRequest, AnalyzeVideoRequest, AnalyzeVisualizationRequest, DiagnoseErrorRequest,
    ExtractTextRequest, ReadRepoFileRequest, RepoStructureRequest, SearchDocRequest,
    UiArtifactOutput, UiDiffRequest, UiToArtifactRequest, UnderstandDiagramRequest,
    WebReaderRequest, WebSearchRequest,
};

#[test]
fn every_typed_mcp_request_exposes_preflight_validation() {
    WebSearchRequest::new("rust").validate().unwrap();
    WebReaderRequest::new("https://example.com")
        .validate()
        .unwrap();
    SearchDocRequest::new("owner/repository", "transport")
        .validate()
        .unwrap();
    RepoStructureRequest::new("owner/repository")
        .validate()
        .unwrap();
    ReadRepoFileRequest::new("owner/repository", "README.md")
        .validate()
        .unwrap();
    UiToArtifactRequest::new("ui.png", UiArtifactOutput::Code, "recreate")
        .validate()
        .unwrap();
    ExtractTextRequest::new("terminal.png", "extract")
        .validate()
        .unwrap();
    DiagnoseErrorRequest::new("error.png", "diagnose")
        .validate()
        .unwrap();
    UnderstandDiagramRequest::new("diagram.png", "explain")
        .validate()
        .unwrap();
    AnalyzeVisualizationRequest::new("chart.png", "analyze")
        .validate()
        .unwrap();
    UiDiffRequest::new("expected.png", "actual.png", "compare")
        .validate()
        .unwrap();
    AnalyzeImageRequest::new("photo.png", "describe")
        .validate()
        .unwrap();
    AnalyzeVideoRequest::new("clip.mp4", "summarize")
        .validate()
        .unwrap();
}

#[test]
fn preflight_validation_rejects_invalid_inputs_without_a_client() {
    assert!(WebSearchRequest::new(" ").validate().is_err());
    assert!(
        WebReaderRequest::new("file:///etc/passwd")
            .validate()
            .is_err()
    );
    assert!(
        ReadRepoFileRequest::new("owner/repository", " ")
            .validate()
            .is_err()
    );
    assert!(
        AnalyzeImageRequest::new("photo.png", " ")
            .validate()
            .is_err()
    );
}
