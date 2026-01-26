mod adapter;
mod domain;

pub use infrastructure::meter::increment as system_metric_increment;
pub use infrastructure::meter::set as system_metric_set;

use std::sync::Arc;

use infrastructure::EventListener;

use crate::{
    command::CommandClient,
    device_state::{DeviceStateClient, DeviceStateEvent},
    home_state::{HomeStateClient, HomeStateEvent},
    observability::adapter::{MetricsAdapter as _, api::MetricsExportApi, repository::VictoriaRepository},
    t,
};

use crate::observability::adapter::{device_metrics::DeviceMetricsAdapter, home_metrics::HomeMetricsAdapter};

pub struct ObservabilityModule {
    repo: Arc<VictoriaRepository>,
    device_state_events: EventListener<DeviceStateEvent>,
    home_state_events: EventListener<HomeStateEvent>,
    device_state_client: DeviceStateClient,
    home_state_client: HomeStateClient,
    command_client: CommandClient,
    home_metrics_adapter: HomeMetricsAdapter,
    device_metrics_adapter: DeviceMetricsAdapter,
}

impl ObservabilityModule {
    pub fn new(
        victoria_url: String,
        device_state_events: EventListener<DeviceStateEvent>,
        home_state_events: EventListener<HomeStateEvent>,
        device_state_client: DeviceStateClient,
        home_state_client: HomeStateClient,
        command_client: CommandClient,
    ) -> Self {
        let repo = Arc::new(VictoriaRepository::new(victoria_url));

        Self {
            repo,
            device_state_events,
            home_state_events,
            device_state_client,
            home_state_client,
            command_client,
            home_metrics_adapter: HomeMetricsAdapter,
            device_metrics_adapter: DeviceMetricsAdapter,
        }
    }

    pub fn api(&self) -> MetricsExportApi {
        MetricsExportApi::new(
            self.repo.clone(),
            self.command_client.clone(),
            self.device_state_client.clone(),
            self.home_state_client.clone(),
        )
    }

    pub async fn run(mut self) {
        const MAX_BATCH: usize = 500;

        let mut device_state_timer = tokio::time::interval(std::time::Duration::from_secs(30));
        let mut buffer = Vec::with_capacity(MAX_BATCH);
        let mut last_flush = t!(now);

        loop {
            let metrics = tokio::select! {
                event = self.device_state_events.recv() => match event {
                    Some(DeviceStateEvent::Updated(data_point)) => self.device_metrics_adapter.to_metrics(data_point.clone()),
                    _ => vec![],
                },

                _ = device_state_timer.tick() => match self.device_state_client.get_current_for_all().await {
                    Ok(states) => {
                        states.into_iter().flat_map(|(_, dp)| {
                            self.device_metrics_adapter.to_metrics(dp)
                        }).collect()
                    },
                    Err(e) => {
                        tracing::error!("Error fetching current device states for metrics export: {:?}", e);
                        vec![]
                    }
                },

                event = self.home_state_events.recv() => match event {
                    Some(HomeStateEvent::Updated(data_point)) => self.home_metrics_adapter.to_metrics(data_point.clone()),
                    _ => vec![],
                }
            };

            for mut metric in metrics.into_iter() {
                //ensure a consitent flow of datapoints
                metric.timestamp = t!(now);
                buffer.push(metric);
            }

            if buffer.len() >= MAX_BATCH || last_flush.elapsed() >= t!(15 seconds) {
                if let Err(e) = self.repo.push(&buffer).await {
                    tracing::error!("Error pushing metrics to VictoriaMetrics: {:?}", e);
                    continue; //keep trying
                }
                tracing::info!("Flushed {} metrics to VictoriaMetrics", buffer.len());
                buffer.clear();
                last_flush = t!(now);
            }
        }
    }
}
