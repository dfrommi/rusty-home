use anyhow::Context;
use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;

use crate::homeassistant::domain::{CallServicePort, GetAllEntityStatesPort, StateChangedEvent};

#[derive(Debug, Clone)]
pub struct HaRestClient {
    client: ClientWithMiddleware,
    base_url: String,
}

impl HaRestClient {
    pub fn new(url: &str, token: &str) -> anyhow::Result<Self> {
        let client = HttpClientConfig::new(Some(token.to_owned())).new_tracing_client()?;

        Ok(Self {
            client,
            base_url: url.to_owned(),
        })
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
