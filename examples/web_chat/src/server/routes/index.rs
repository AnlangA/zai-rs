//! Index page handler

use axum::response::Html;

/// Serve the main HTML page
pub async fn index_handler() -> Html<&'static str> {
    Html(include_str!("../../../static/index.html"))
}

/// Serve a minimal HTML page for testing
pub async fn minimal_handler() -> Html<&'static str> {
    Html(r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>AI Chat - Testing</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .status { color: green; font-weight: bold; }
    </style>
</head>
<body>
    <h1>🚀 AI Chat Server is Running</h1>
    <p class="status">✅ Server is healthy and ready to accept connections</p>
    <p>Visit <a href="/">the main page</a> to start chatting</p>
    <hr>
    <p><small>Server time: <span id="time"></span></small></p>
    <script>
        document.getElementById('time').textContent = new Date().toLocaleString('zh-CN');
    </script>
</body>
</html>
    "#)
}