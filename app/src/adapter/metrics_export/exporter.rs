use crate::{
    adapter::metrics_export::{Metric, repository::VictoriaRepository},
    core::timeseries::DataPoint,
    home::state::HomeStateValue,
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
                    //Use now instead of first timestamp to fill gaps
                    let metric: Metric = DataPoint::new(data_point.value, t!(now)).into();
                    buffer.push(metric);

                    if buffer.len() >= MAX_BATCH || last_flush.elapsed() >= t!(15 seconds) {
                        if let Err(e) = self.repo.push(&buffer).await {
                            tracing::error!("Error pushing metrics to VictoriaMetrics: {:?}", e);
                            continue; //keep trying
                        }
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
}
