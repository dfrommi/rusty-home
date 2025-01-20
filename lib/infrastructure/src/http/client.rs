use reqwest::header::{self, HeaderMap};
use reqwest_middleware::ClientWithMiddleware;
use reqwest_tracing::TracingMiddleware;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HttpClientConfig {
    bearer_token: Option<String>,
}

impl HttpClientConfig {
    pub fn new(bearer_token: Option<String>) -> Self {
        Self { bearer_token }
    }

    pub fn new_tracing_client(&self) -> anyhow::Result<ClientWithMiddleware> {
        let mut headers = HeaderMap::new();

        if let Some(token) = &self.bearer_token {
            let mut auth_value =
                header::HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap();
            auth_value.set_sensitive(true);
            headers.insert(header::AUTHORIZATION, auth_value);
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(reqwest_middleware::ClientBuilder::new(client)
            .with(TracingMiddleware::default())
            .build())
    }
}
