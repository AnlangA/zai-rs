use log::info;

pub trait HttpClient {
    // Associated types
    type Body: serde::Serialize;
    type ApiUrl: AsRef<str>;
    type ApiKey: AsRef<str>;

    // Accessors
    fn api_url(&self) -> &Self::ApiUrl;
    fn api_key(&self) -> &Self::ApiKey;
    fn body(&self) -> &Self::Body;

    // POST using anyhow for error handling; serialize body here
    fn post(&self) -> impl std::future::Future<Output = anyhow::Result<reqwest::Response>> + Send {
        let body_compact = serde_json::to_string(self.body());
        let body_pretty = serde_json::to_string_pretty(self.body());
        let url = self.api_url().as_ref().to_owned();
        let key = self.api_key().as_ref().to_owned();
        async move {
            let body = body_compact?;
            let pretty = body_pretty.unwrap_or_else(|_| body.clone());
            info!("Request body: {}", pretty);
            let resp = reqwest::Client::new()
                .post(url)
                .header("Authorization", format!("Bearer {}", key))
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await?;
            Ok(resp)
        }
    }
}
