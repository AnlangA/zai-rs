//! Exercise all eight Vision MCP tools.
//!
//! The same source image can be reused across the specialized tools. An
//! optional third argument supplies a different actual screenshot for the UI
//! comparison.
//!
//! Run with:
//! cargo run --example mcp_vision --features mcp -- source.png video.mp4 [actual.png]

use std::env;

use zai_rs::mcp::{
    AnalyzeImageRequest, AnalyzeVideoRequest, AnalyzeVisualizationRequest, DiagnoseErrorRequest,
    ExtractTextRequest, McpClient, McpTextResponse, UiArtifactOutput, UiDiffRequest,
    UiToArtifactRequest, UnderstandDiagramRequest,
};

fn print_response(label: &str, response: McpTextResponse) {
    println!("\n=== {label} ===\n{response}");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let image = args
        .next()
        .ok_or("usage: mcp_vision <image> <video> [actual-ui-image]")?;
    let video = args
        .next()
        .ok_or("usage: mcp_vision <image> <video> [actual-ui-image]")?;
    let actual_ui = args.next().unwrap_or_else(|| image.clone());

    let client = McpClient::from_env()?;
    let mut failures = 0_u8;

    macro_rules! run {
        ($label:literal, $future:expr) => {
            match $future.await {
                Ok(response) => print_response($label, response),
                Err(error) => {
                    failures += 1;
                    eprintln!("{} failed: {error}", $label);
                },
            }
        };
    }

    run!(
        "ui_to_artifact",
        client.ui_to_artifact_with(UiToArtifactRequest::new(
            &image,
            UiArtifactOutput::Specification,
            "Create a detailed design specification for this interface.",
        ))
    );
    run!(
        "extract_text_from_screenshot",
        client.extract_text_with(
            ExtractTextRequest::new(&image, "Extract every visible string and preserve layout.")
                .programming_language("rust")
        )
    );
    run!(
        "diagnose_error_screenshot",
        client.diagnose_error_with(
            DiagnoseErrorRequest::new(&image, "Identify the error and propose a concrete fix.")
                .context("while compiling a Rust MCP client")
        )
    );
    run!(
        "understand_technical_diagram",
        client.understand_diagram_with(
            UnderstandDiagramRequest::new(&image, "Explain components and data flow.")
                .diagram_type("architecture")
        )
    );
    run!(
        "analyze_data_visualization",
        client.analyze_visualization_with(
            AnalyzeVisualizationRequest::new(&image, "Summarize the main quantitative insights.")
                .focus("trends, anomalies, and comparisons")
        )
    );
    run!(
        "ui_diff_check",
        client.compare_ui_with(UiDiffRequest::new(
            &image,
            actual_ui,
            "List every visible difference and its severity.",
        ))
    );
    run!(
        "analyze_image",
        client.analyze_image_with(AnalyzeImageRequest::new(
            &image,
            "Describe the image comprehensively.",
        ))
    );
    run!(
        "analyze_video",
        client.analyze_video_with(AnalyzeVideoRequest::new(
            video,
            "Summarize the video and identify its key moments.",
        ))
    );

    client.close().await?;
    if failures == 0 {
        Ok(())
    } else {
        Err(format!("{failures} Vision MCP tool call(s) failed").into())
    }
}
