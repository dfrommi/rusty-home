use anyhow::Context;

#[derive(Debug, Clone)]
pub struct HaRestClient {
    client: reqwest::Client,
    base_url: String,
}

impl HaRestClient {
    pub fn new(url: &str, token: &str) -> Self {
        use reqwest::header;

        let mut headers = header::HeaderMap::new();
        let mut auth_value =
            header::HeaderValue::from_str(format!("Bearer {}", token).as_str()).unwrap();
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            client,
            base_url: url.to_owned(),
        }
    }

    pub async fn get_all_states(&self) -> anyhow::Result<Vec<serde_json::Value>> {
        let response = self
            .client
            .get(format!("{}/api/states", self.base_url))
            .send()
            .await?;

        response
            .json::<Vec<serde_json::Value>>()
            .await
            .context("Error getting all states")
    }

    pub async fn call_service(
        &self,
        domain: &str,
        service: &str,
        service_data: serde_json::Value,
    ) -> anyhow::Result<()> {
        let url = format!("{}/api/services/{}/{}", self.base_url, domain, service);

        tracing::info!(
            "Calling HA service {}: {:?}",
            url,
            serde_json::to_string(&service_data)?
        );

        let response = self.client.post(url).json(&service_data).send().await;

        tracing::info!("Response: {:?}", response);

        response
            .with_context(|| format!("Error calling HA service {}/{}", domain, service))
            .map(|_| ())
    }
}
