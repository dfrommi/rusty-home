use crate::{
    core::timeseries::DataPoint,
    home_state::{HeatingMode, HomeStateId, HomeStateValue},
    observability::domain::{Metric, MetricId, MetricLabel},
    t,
};

pub struct HomeMetricsAdapter;

impl super::MetricsAdapter<DataPoint<HomeStateValue>> for HomeMetricsAdapter {
    fn to_metrics(&self, dp: DataPoint<HomeStateValue>) -> Vec<Metric> {
        let home_state_id = HomeStateId::from(&dp.value);
        let timestamp = dp.timestamp;

        let default_with = |v: f64| {
            vec![Metric {
                id: MetricId::from(&home_state_id),
                timestamp,
                value: v,
            }]
        };

        match dp.value {
            HomeStateValue::AbsoluteHumidity(_, v) => default_with(f64::from(&v)),
            HomeStateValue::ColdAirComingIn(_, v) => default_with(v.into()),
            HomeStateValue::DewPoint(_, v) => default_with(f64::from(&v)),
            HomeStateValue::FeltTemperature(_, v) => default_with(f64::from(&v)),
            HomeStateValue::IsRunning(_, v) => default_with(v.into()),
            HomeStateValue::Occupancy(_, v) => default_with(f64::from(&v)),
            HomeStateValue::OpenedArea(_, v) => default_with(v.into()),
            HomeStateValue::Resident(_, v) => default_with(v.into()),
            HomeStateValue::RiskOfMould(_, v) => default_with(v.into()),
            HomeStateValue::EnergySaving(_, v) => default_with(v.into()),
            HomeStateValue::FanActivity(_, v) => default_with(f64::from(&v)),
            HomeStateValue::HeatingDemand(_, v) => default_with(f64::from(&v)),
            HomeStateValue::PowerAvailable(_, v) => default_with(v.into()),
            HomeStateValue::Presence(_, v) => default_with(v.into()),
            HomeStateValue::RelativeHumidity(_, v) => default_with(f64::from(&v)),
            HomeStateValue::SetPoint(_, v) => default_with(f64::from(&v)),
            HomeStateValue::Temperature(_, v) => default_with(f64::from(&v)),
            HomeStateValue::TemperatureChange(_, v) => [
                ("1m", t!(1 minutes)),
                ("10m", t!(10 minutes)),
                ("15m", t!(15 minutes)),
                ("1h", t!(1 hours)),
            ]
            .into_iter()
            .map(|(suffix, duration)| Metric {
                id: metric_id(&home_state_id, suffix, vec![]),
                timestamp,
                value: f64::from(v.per(duration)),
            })
            .collect(),
            HomeStateValue::TargetHeatingDemand(_, v) => default_with(f64::from(&v)),
            HomeStateValue::TargetHeatingAdjustment(_, v) => {
                use crate::home_state::AdjustmentDirection::*;
                let value = match v {
                    MustIncrease => 2.0,
                    ShouldIncrease => 1.0,
                    MustDecrease => -2.0,
                    ShouldDecrease => -1.0,
                    MustOff => -4.0,
                    Hold => 0.0,
                };

                default_with(value)
            }
            HomeStateValue::PidOutput(_, v) => {
                vec![
                    Metric {
                        id: metric_id(&home_state_id, "p", vec![]),
                        timestamp,
                        value: v.p().into(),
                    },
                    Metric {
                        id: metric_id(&home_state_id, "i", vec![]),
                        timestamp,
                        value: v.i().into(),
                    },
                    Metric {
                        id: metric_id(&home_state_id, "d", vec![]),
                        timestamp,
                        value: v.d().into(),
                    },
                    Metric {
                        id: metric_id(&home_state_id, "total", vec![]),
                        timestamp,
                        value: v.total().into(),
                    },
                ]
            }
            HomeStateValue::TargetHeatingMode(_, v) => {
                //::variants not possible because of manual parameters
                let modes_and_values = vec![
                    ("energy_saving", v == HeatingMode::EnergySaving),
                    ("comfort", v == HeatingMode::Comfort),
                    ("sleep", v == HeatingMode::Sleep),
                    ("ventilation", v == HeatingMode::Ventilation),
                    ("post_ventilation", v == HeatingMode::PostVentilation),
                    ("away", v == HeatingMode::Away),
                    ("manual", matches!(v, HeatingMode::Manual(_, _))),
                ];

                modes_and_values
                    .into_iter()
                    .map(|(mode, value)| Metric {
                        id: metric_id(&home_state_id, "", vec![MetricLabel::EnumVariant(mode.to_string())]),
                        timestamp,
                        value: if value { 1.0 } else { 0.0 },
                    })
                    .collect()
            }
        }
    }
}

fn metric_id(state: &HomeStateId, suffix: &str, extra_tags: Vec<MetricLabel>) -> MetricId {
    let external_id = state.ext_id();

    let name = if suffix.is_empty() {
        external_id.type_name().to_string()
    } else {
        format!("{}_{}", external_id.type_name(), suffix)
    };
    let labels = {
        let mut common_tags = super::get_common_tags(&external_id);
        common_tags.extend(extra_tags);
        common_tags
    };

    MetricId { name, labels }
}

impl From<&HomeStateId> for MetricId {
    fn from(state: &HomeStateId) -> Self {
        let external_id = state.ext_id();

        MetricId {
            name: external_id.type_name().to_string(),
            labels: super::get_common_tags(&external_id),
        }
    }
}
