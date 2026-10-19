use reqwest::Client;

use crate::observability::domain::Metric;

pub struct VictoriaRepository {
    client: Client,
    base_url: String,
}

impl VictoriaRepository {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }

    pub async fn push(&self, metrics: &[Metric]) -> anyhow::Result<()> {
        if metrics.is_empty() {
            return Ok(());
        }

        let mut body = String::new();
        for metric in metrics {
            body.push_str(&metric.to_string());
            body.push('\n');
        }

        let resp = self
            .client
            .post(format!("{}/api/v1/import/prometheus", self.base_url))
            .body(body)
            .send()
            .await?;

        resp.error_for_status_ref()?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::{
        core::time::DateTime,
        observability::domain::{MetricId, MetricLabel},
    };
    use mockito::Server;

    fn metric(name: &str, variant: Option<&str>, value: f64, timestamp: DateTime) -> Metric {
        Metric {
            id: MetricId {
                name: name.to_string(),
                labels: variant
                    .map(|v| vec![MetricLabel::Variant(v.to_string())])
                    .unwrap_or_default(),
            },
            value,
            timestamp,
        }
    }

    #[tokio::test]
    async fn push_sends_all_metric_lines() {
        let mut server = Server::new_async().await;

        let ts_one = DateTime::from_static_iso("2024-01-01T00:00:00Z");
        let ts_two = DateTime::from_static_iso("2024-01-01T01:00:00Z");

        let metric_one = metric("consumption_watts", None, 42.0, ts_one);
        let metric_two = metric("consumption_watts", Some("heat_pump"), 41.5, ts_two);
        let metrics = vec![metric_one, metric_two];

        let expected_body = format!(
            "{}\n{}\n",
            "consumption_watts 42 1704067200000", "consumption_watts{item=\"heat_pump\"} 41.5 1704070800000"
        );

        let mock = server
            .mock("POST", "/api/v1/import/prometheus")
            .match_body(expected_body.as_str())
            .with_status(204)
            .create_async()
            .await;

        let repository = VictoriaRepository::new(server.url());
        repository.push(&metrics).await.unwrap();
        mock.assert_async().await;
    }
}
