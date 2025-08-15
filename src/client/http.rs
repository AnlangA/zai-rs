
pub struct HttpClient {
    api_key: String,
    api_url: String,
    body: String
}

impl HttpClient {
    pub fn new(api_key: impl Into<String>, api_url: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_url: api_url.into(),
            body: body.into(),
        }
    }
    pub async fn post<'a>(&self) -> Result<reqwest::Response, reqwest::Error> {
        let client = reqwest::Client::new();
        let response = client
            .post(self.api_url.as_str())
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(self.body.clone())
            .send()
            .await;
        response
    }
}
