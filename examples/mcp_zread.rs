//! Exercise all three ZRead MCP tools.
//!
//! Run with:
//! cargo run --example mcp_zread --features mcp -- modelcontextprotocol/rust-sdk CallToolResult crates/rmcp/src README.md

use std::env;

use zai_rs::mcp::{
    McpClient, McpTextResponse, ReadRepoFileRequest, RepoStructureRequest, RepositoryLanguage,
    SearchDocRequest,
};

fn print_response(label: &str, response: McpTextResponse) {
    println!("\n=== {label} ===\n{response}");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let repository = args
        .next()
        .unwrap_or_else(|| "modelcontextprotocol/rust-sdk".to_owned());
    let query = args.next().unwrap_or_else(|| "CallToolResult".to_owned());
    let directory = args.next().unwrap_or_else(|| "crates/rmcp/src".to_owned());
    let file = args.next().unwrap_or_else(|| "README.md".to_owned());

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
        "search_doc",
        client.search_repo_with(
            SearchDocRequest::new(&repository, query).language(RepositoryLanguage::En)
        )
    );
    run!(
        "get_repo_structure",
        client.repo_structure_with(RepoStructureRequest::new(&repository).directory(directory))
    );
    run!(
        "read_file",
        client.read_repo_file_with(ReadRepoFileRequest::new(&repository, file))
    );

    client.close().await?;
    if failures == 0 {
        Ok(())
    } else {
        Err(format!("{failures} ZRead MCP tool call(s) failed").into())
    }
}
