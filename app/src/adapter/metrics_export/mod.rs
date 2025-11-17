mod api;
mod exporter;
mod repository;
mod tags;

pub use exporter::HomeStateMetricsExporter;

use crate::core::HomeApi;
use crate::core::id::ExternalId;
use crate::core::time::DateTime;
use crate::core::timeseries::DataPoint;
use crate::home::state::{HomeState, HomeStateValue, StateValue};
use serde::Deserialize;
use tokio::sync::broadcast::Receiver;

#[derive(Clone, Debug, Deserialize)]
pub struct MetricsExport {
    pub victoria_url: String,
}

impl MetricsExport {
    pub fn new_routes(&self, api: HomeApi) -> actix_web::Scope {
        let repo = repository::VictoriaRepository::new(self.victoria_url.clone());
        api::routes(repo, api)
    }

    pub fn new_exporter(&self, rx: Receiver<DataPoint<HomeStateValue>>) -> HomeStateMetricsExporter {
        let repo = repository::VictoriaRepository::new(self.victoria_url.clone());
        HomeStateMetricsExporter::new(rx, repo)
    }
}

#[derive(Debug, Clone)]
struct Metric {
    id: MetricId,
    value: f64,
    timestamp: DateTime,
}

#[derive(Debug, Clone)]
struct MetricId {
    name: String,
    labels: Vec<MetricLabel>,
}

#[derive(Debug, Clone)]
enum MetricLabel {
    Variant(String),
    Room(String),
    FriendlyName(String),
    EnumVariant(String),
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

        MetricId {
            name: state.ext_id().type_name().to_string(),
            labels: tags::get_tags(value),
        }
    }
}

impl From<&ExternalId> for MetricId {
    fn from(ext_id: &ExternalId) -> Self {
        MetricId {
            name: ext_id.type_name().to_string(),
            labels: vec![MetricLabel::Variant(ext_id.variant_name().to_string())],
        }
    }
}

impl std::fmt::Display for Metric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.id, self.value, self.timestamp.millis())
    }
}

impl std::fmt::Display for MetricId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let labels: Vec<String> = self.labels.iter().map(|label| label.to_string()).collect();
        if labels.is_empty() {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}{{{}}}", self.name, labels.join(", "))
        }
    }
}

impl std::fmt::Display for MetricLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetricLabel::Variant(v) => write!(f, "item=\"{}\"", v),
            MetricLabel::Room(r) => write!(f, "room=\"{}\"", r),
            MetricLabel::FriendlyName(n) => write!(f, "friendly_name=\"{}\"", n),
            MetricLabel::EnumVariant(ev) => write!(f, "enum_variant=\"{}\"", ev),
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
        StateValue::HeatingMode(heating_mode) => f64::from(&heating_mode),
        StateValue::RawValue(raw) => f64::from(&raw),
        StateValue::Lux(lux) => f64::from(&lux),
        StateValue::Probability(probability) => f64::from(&probability),
    }
}
