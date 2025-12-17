use crate::{
    adapter::metrics_export::domain::{Metric, MetricId},
    core::timeseries::DataPoint,
    home_state::{HeatingMode, HomeState, HomeStateValue, StateValue},
};

pub struct HomeMetricsAdapter;

impl super::MetricsAdapter<DataPoint<HomeStateValue>> for HomeMetricsAdapter {
    fn to_metrics(&self, dp: DataPoint<HomeStateValue>) -> Vec<Metric> {
        vec![dp.into()]
    }
}

impl From<DataPoint<HomeStateValue>> for Metric {
    fn from(dp: DataPoint<HomeStateValue>) -> Self {
        Metric {
            id: MetricId::from(&dp.value),
            value: to_metrics_value(dp.value.value()),
            timestamp: dp.timestamp,
        }
    }
}

impl From<&HomeStateValue> for MetricId {
    fn from(value: &HomeStateValue) -> Self {
        let state = HomeState::from(value);
        let external_id = state.ext_id();

        MetricId {
            name: external_id.type_name().to_string(),
            labels: super::get_common_tags(&external_id),
        }
    }
}

fn to_metrics_value(value: StateValue) -> f64 {
    match value {
        StateValue::Boolean(b) => b.into(),
        StateValue::DegreeCelsius(degree_celsius) => f64::from(&degree_celsius),
        StateValue::Watt(watt) => f64::from(&watt),
        StateValue::Percent(percent) => f64::from(&percent),
        StateValue::GramPerCubicMeter(gram_per_cubic_meter) => f64::from(&gram_per_cubic_meter),
        StateValue::KiloWattHours(kilo_watt_hours) => f64::from(&kilo_watt_hours),
        StateValue::HeatingUnit(heating_unit) => f64::from(&heating_unit),
        StateValue::KiloCubicMeter(kilo_cubic_meter) => f64::from(&kilo_cubic_meter),
        StateValue::FanAirflow(fan_airflow) => f64::from(&fan_airflow),
        StateValue::RawValue(raw) => f64::from(&raw),
        StateValue::Lux(lux) => f64::from(&lux),
        StateValue::Probability(probability) => f64::from(&probability),
        StateValue::HeatingMode(heating_mode) => match heating_mode {
            HeatingMode::Sleep => 10.0,
            HeatingMode::EnergySaving => 11.0,
            HeatingMode::Comfort => 12.0,
            HeatingMode::Manual(_, _) => 13.0,
            HeatingMode::Ventilation => 1.0,
            HeatingMode::PostVentilation => 2.0,
            HeatingMode::Away => -1.0,
        },
    }
}
