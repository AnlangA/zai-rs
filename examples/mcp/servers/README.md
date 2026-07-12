# MCP counter server

This example exposes counter tools and reusable prompts over MCP Streamable
HTTP at `http://127.0.0.1:8000/mcp`.

From the repository root, start the server with:

```shell
cargo run -p mcp_server
```

Then run the low-level interoperability client in another terminal:

```shell
ZHIPU_API_KEY=your-key cargo run -p zai-rmcp-test
```
