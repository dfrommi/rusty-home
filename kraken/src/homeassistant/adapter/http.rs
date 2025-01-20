use anyhow::Context;
use reqwest_middleware::ClientWithMiddleware;
use reqwest_tracing::TracingMiddleware;

use crate::homeassistant::domain::{CallServicePort, GetAllEntityStatesPort, StateChangedEvent};

#[derive(Debug, Clone)]
pub struct HaRestClient {
    client: ClientWithMiddleware,
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

        let client = reqwest_middleware::ClientBuilder::new(client)
            .with(TracingMiddleware::default())
            .build();

        Self {
            client,
            base_url: url.to_owned(),
        }
    }
}

impl GetAllEntityStatesPort for HaRestClient {
    async fn get_current_state(&self) -> anyhow::Result<Vec<StateChangedEvent>> {
        let response = self
            .client
            .get(format!("{}/api/states", self.base_url))
            .send()
            .await?;

        response
            .json::<Vec<StateChangedEvent>>()
            .await
            .context("Error getting all states")
    }
}

impl CallServicePort for HaRestClient {
    #[tracing::instrument(skip(self))]
    async fn call_service(
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

        let response = self.client.post(url).json(&service_data).send().await?;
        tracing::info!(
            "Response: {} - {}",
            response.status(),
            response.text().await?
        );

        Ok(())
    }
}
