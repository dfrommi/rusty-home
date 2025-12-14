use crate::{
    adapter::metrics_export::{Metric, repository::VictoriaRepository},
    core::timeseries::DataPoint,
    device_state::{DeviceStateEvent, DeviceStateId},
    home::state::HomeStateValue,
    t,
};

pub struct HomeStateMetricsExporter {
    device_state_updated_rx: tokio::sync::broadcast::Receiver<DeviceStateEvent>,
    home_state_updated_rx: tokio::sync::broadcast::Receiver<DataPoint<HomeStateValue>>,
    repo: VictoriaRepository,
}

impl HomeStateMetricsExporter {
    pub(super) fn new(
        rx_device: tokio::sync::broadcast::Receiver<DeviceStateEvent>,
        rx_home: tokio::sync::broadcast::Receiver<DataPoint<HomeStateValue>>,
        repo: VictoriaRepository,
    ) -> Self {
        Self {
            device_state_updated_rx: rx_device,
            home_state_updated_rx: rx_home,
            repo,
        }
    }

    pub async fn run(&mut self) {
        const MAX_BATCH: usize = 2000;

        let mut buffer = Vec::with_capacity(MAX_BATCH);
        let mut last_flush = t!(now);

        loop {
            tokio::select! {
                Ok(DeviceStateEvent::Updated(data_point)) = self.device_state_updated_rx.recv() => {
                    //Use now instead of first timestamp to fill gaps
                    let metric: Metric = DataPoint::new(data_point.value.clone(), t!(now)).into();
                    //Derived metrics
                    let derived = self.derived_metrics(&metric, data_point.value.into());
                    for dm in derived {
                        buffer.push(dm);
                    }

                    buffer.push(metric);
                }

                Ok(data_point) = self.home_state_updated_rx.recv() => {
                    //Use now instead of first timestamp to fill gaps
                    let metric: Metric = DataPoint::new(data_point.value, t!(now)).into();

                    buffer.push(metric);
                }
            };

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

    fn derived_metrics(&self, metric: &Metric, state: DeviceStateId) -> Vec<Metric> {
        let mut metrics = Vec::new();

        match state {
            DeviceStateId::HeatingDemand(demand) => {
                let mut scaled_metric = metric.clone();
                scaled_metric.id.name = format!("{}_scaled", metric.id.name);
                scaled_metric.value = metric.value * demand.scaling_factor();
                metrics.push(scaled_metric);
            }
            DeviceStateId::TotalRadiatorConsumption(consumption) => {
                let mut scaled_metric = metric.clone();
                scaled_metric.id.name = format!("{}_scaled", metric.id.name);
                scaled_metric.value = metric.value * consumption.scaling_factor();
                metrics.push(scaled_metric);
            }
            _ => (),
        }

        metrics
    }
}
