use crate::core::ValueObject;
use crate::core::timeseries::DataPoint;
use crate::home::state::{HomeState, HomeStateValue};
use infrastructure::meter::set;
use tokio::sync::broadcast::Receiver;

const ITEM_TYPE: &str = "item_type";
const ITEM_NAME: &str = "item_name";

pub struct HomeStateMetricsExporter {
    state_updated_rx: tokio::sync::broadcast::Receiver<DataPoint<HomeStateValue>>,
}

impl HomeStateMetricsExporter {
    pub fn new(rx: Receiver<DataPoint<HomeStateValue>>) -> Self {
        Self { state_updated_rx: rx }
    }

    pub async fn run(&mut self) {
        loop {
            match self.state_updated_rx.recv().await {
                Ok(data_point) => {
                    //TODO direct support of to_f64 on HomeStateValue
                    let state_value = HomeState::from(&data_point.value);
                    let value = state_value.to_f64(&data_point.value);
                    let external_id = state_value.ext_id();

                    set(
                        "home_state_value",
                        value,
                        &[
                            (ITEM_TYPE, external_id.type_name()),
                            (ITEM_NAME, external_id.variant_name()),
                        ],
                    );
                }

                Err(e) => {
                    tracing::error!("Error receiving home state updated event: {:?}", e);
                }
            }
        }
    }
}
