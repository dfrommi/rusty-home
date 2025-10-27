use crate::core::timeseries::DataPoint;
use crate::home::state::{HomeState, HomeStateValue, StateValue};
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
                    let value = match data_point.value.value() {
                        StateValue::Boolean(b) => b.into(),
                        StateValue::DegreeCelsius(degree_celsius) => f64::from(&degree_celsius),
                        StateValue::Watt(watt) => f64::from(&watt),
                        StateValue::Percent(percent) => f64::from(&percent),
                        StateValue::GramPerCubicMeter(gram_per_cubic_meter) => f64::from(&gram_per_cubic_meter),
                        StateValue::KiloWattHours(kilo_watt_hours) => f64::from(&kilo_watt_hours),
                        StateValue::HeatingUnit(heating_unit) => f64::from(&heating_unit),
                        StateValue::KiloCubicMeter(kilo_cubic_meter) => f64::from(&kilo_cubic_meter),
                        StateValue::FanAirflow(fan_airflow) => f64::from(&fan_airflow),
                        StateValue::HeatingMode(heating_mode) => f64::from(&heating_mode),
                    };

                    let external_id = HomeState::from(&data_point.value).ext_id();

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
