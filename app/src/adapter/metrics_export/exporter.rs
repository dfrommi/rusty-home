use crate::{
    adapter::metrics_export::{Metric, repository::VictoriaRepository},
    core::timeseries::DataPoint,
    home::state::{HomeState, HomeStateValue},
    t,
};

pub struct HomeStateMetricsExporter {
    state_updated_rx: tokio::sync::broadcast::Receiver<DataPoint<HomeStateValue>>,
    repo: VictoriaRepository,
}

impl HomeStateMetricsExporter {
    pub(super) fn new(
        rx: tokio::sync::broadcast::Receiver<DataPoint<HomeStateValue>>,
        repo: VictoriaRepository,
    ) -> Self {
        Self {
            state_updated_rx: rx,
            repo,
        }
    }

    pub async fn run(&mut self) {
        const MAX_BATCH: usize = 2000;

        let mut buffer = Vec::with_capacity(MAX_BATCH);
        let mut last_flush = t!(now);

        loop {
            match self.state_updated_rx.recv().await {
                Ok(data_point) => {
                    let home_state = HomeState::from(&data_point.value);

                    //Use now instead of first timestamp to fill gaps
                    let metric: Metric = DataPoint::new(data_point.value, t!(now)).into();

                    //Derived metrics
                    let derived = self.derived_metrics(&metric, home_state);
                    for dm in derived {
                        buffer.push(dm);
                    }

                    buffer.push(metric);

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

                Err(e) => {
                    tracing::error!("Error receiving home state updated event: {:?}", e);
                }
            }
        }
    }

    fn derived_metrics(&self, metric: &Metric, state: HomeState) -> Vec<Metric> {
        let mut metrics = Vec::new();

        match state {
            HomeState::HeatingDemand(demand) => {
                let mut scaled_metric = metric.clone();
                scaled_metric.id.name = format!("{}_scaled", metric.id.name);
                scaled_metric.value = metric.value * demand.scaling_factor();
                metrics.push(scaled_metric);
            }
            HomeState::TotalRadiatorConsumption(consumption) => {
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
