use crate::{
    core::timeseries::DataPoint,
    home_state::{HeatingMode, HomeStateId, HomeStateValue},
    observability::domain::{Metric, MetricId},
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
            value: to_metrics_value(dp.value),
            timestamp: dp.timestamp,
        }
    }
}

impl From<&HomeStateValue> for MetricId {
    fn from(value: &HomeStateValue) -> Self {
        let state = HomeStateId::from(value);
        let external_id = state.ext_id();

        MetricId {
            name: external_id.type_name().to_string(),
            labels: super::get_common_tags(&external_id),
        }
    }
}

fn to_metrics_value(value: HomeStateValue) -> f64 {
    match value {
        HomeStateValue::AbsoluteHumidity(_, v) => f64::from(&v),
        HomeStateValue::ColdAirComingIn(_, v) => v.into(),
        HomeStateValue::DewPoint(_, v) => f64::from(&v),
        HomeStateValue::FeltTemperature(_, v) => f64::from(&v),
        HomeStateValue::IsRunning(_, v) => v.into(),
        HomeStateValue::Load(_, v) => f64::from(&v),
        HomeStateValue::Occupancy(_, v) => f64::from(&v),
        HomeStateValue::OpenedArea(_, v) => v.into(),
        HomeStateValue::Resident(_, v) => v.into(),
        HomeStateValue::RiskOfMould(_, v) => v.into(),
        HomeStateValue::TargetHeatingMode(_, v) => match v {
            HeatingMode::Sleep => 10.0,
            HeatingMode::EnergySaving => 11.0,
            HeatingMode::Comfort => 12.0,
            HeatingMode::Manual(_, _) => 13.0,
            HeatingMode::Ventilation => 1.0,
            HeatingMode::PostVentilation => 2.0,
            HeatingMode::Away => -1.0,
        },
        HomeStateValue::EnergySaving(_, v) => v.into(),
        HomeStateValue::FanActivity(_, v) => f64::from(&v),
        HomeStateValue::HeatingDemand(_, v) => f64::from(&v),
        HomeStateValue::PowerAvailable(_, v) => v.into(),
        HomeStateValue::Presence(_, v) => v.into(),
        HomeStateValue::RawVendorValue(_, v) => f64::from(&v),
        HomeStateValue::RelativeHumidity(_, v) => f64::from(&v),
        HomeStateValue::SetPoint(_, v) => f64::from(&v),
        HomeStateValue::Temperature(_, v) => f64::from(&v),
    }
}
