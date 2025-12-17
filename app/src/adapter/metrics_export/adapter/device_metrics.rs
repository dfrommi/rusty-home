use crate::{
    core::timeseries::DataPoint,
    device_state::{CurrentPowerUsage, DeviceStateId, DeviceStateValue, HeatingDemand, TotalEnergyConsumption},
};

use super::{Metric, MetricId, MetricLabel};

pub struct DeviceMetricsAdapter;

impl super::MetricsAdapter<DataPoint<DeviceStateValue>> for DeviceMetricsAdapter {
    fn to_metrics(&self, dp: DataPoint<DeviceStateValue>) -> Vec<Metric> {
        let base_metric: Metric = dp.clone().into();
        let state: DeviceStateId = dp.value.into();

        let mut metrics = vec![base_metric.clone()];
        let derived = derived_metrics(&base_metric, state);
        metrics.extend(derived);

        metrics
    }
}

impl From<DataPoint<DeviceStateValue>> for Metric {
    fn from(dp: DataPoint<DeviceStateValue>) -> Self {
        Metric {
            id: MetricId::from(&dp.value),
            value: (&dp.value).into(),
            timestamp: dp.timestamp,
        }
    }
}

impl From<&DeviceStateValue> for MetricId {
    fn from(value: &DeviceStateValue) -> Self {
        let id = DeviceStateId::from(value.clone());
        let ext_id = id.ext_id();
        let mut tags = super::get_common_tags(&ext_id);
        tags.extend(get_tags_for_device(value));

        MetricId {
            name: format!("device_{}", ext_id.type_name()),
            labels: tags,
        }
    }
}

fn derived_metrics(metric: &Metric, state: DeviceStateId) -> Vec<Metric> {
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

pub fn get_tags_for_device(value: &DeviceStateValue) -> Vec<MetricLabel> {
    let mut tags = vec![];

    if let Some(friendly_name) = friendly_name(value.into()) {
        tags.push(MetricLabel::FriendlyName(friendly_name.to_owned()));
    }

    tags
}

fn friendly_name(state: DeviceStateId) -> Option<&'static str> {
    match state {
        DeviceStateId::CurrentPowerUsage(s) => Some(match s {
            CurrentPowerUsage::Fridge => "Kühlschrank",
            CurrentPowerUsage::Dehumidifier => "Blasi",
            CurrentPowerUsage::AppleTv => "Apple TV",
            CurrentPowerUsage::Tv => "TV",
            CurrentPowerUsage::AirPurifier => "Luftfilter",
            CurrentPowerUsage::CouchLight => "Couchlicht",
            CurrentPowerUsage::Dishwasher => "Geschirrspüler",
            CurrentPowerUsage::Kettle => "Wasserkocher",
            CurrentPowerUsage::WashingMachine => "Waschmaschine",
            CurrentPowerUsage::Nuc => "Nuc",
            CurrentPowerUsage::DslModem => "DSL Modem",
            CurrentPowerUsage::InternetGateway => "Internet Gateway",
            CurrentPowerUsage::NetworkSwitch => "Network Switch",
            CurrentPowerUsage::InfraredHeater => "Infrarot-Heizung",
            CurrentPowerUsage::KitchenMultiPlug => "Küche Arbeitsplatte",
            CurrentPowerUsage::CouchPlug => "Couch-Stecker",
            CurrentPowerUsage::RoomOfRequirementsDesk => "Schreibtisch",
            CurrentPowerUsage::RoomOfRequirementsMonitor => "Monitor",
        }),
        DeviceStateId::TotalEnergyConsumption(s) => Some(match s {
            TotalEnergyConsumption::Fridge => "Kühlschrank",
            TotalEnergyConsumption::Dehumidifier => "Blasi",
            TotalEnergyConsumption::AppleTv => "Apple TV",
            TotalEnergyConsumption::Tv => "TV",
            TotalEnergyConsumption::AirPurifier => "Luftfilter",
            TotalEnergyConsumption::CouchLight => "Couchlicht",
            TotalEnergyConsumption::Dishwasher => "Geschirrspüler",
            TotalEnergyConsumption::Kettle => "Wasserkocher",
            TotalEnergyConsumption::WashingMachine => "Waschmaschine",
            TotalEnergyConsumption::Nuc => "Nuc",
            TotalEnergyConsumption::DslModem => "DSL Modem",
            TotalEnergyConsumption::InternetGateway => "Internet Gateway",
            TotalEnergyConsumption::NetworkSwitch => "Network Switch",
            TotalEnergyConsumption::InfraredHeater => "Infrarot-Heizung",
            TotalEnergyConsumption::KitchenMultiPlug => "Küche Arbeitsplatte",
            TotalEnergyConsumption::CouchPlug => "Couch-Stecker",
            TotalEnergyConsumption::RoomOfRequirementsDesk => "Schreibtisch",
            TotalEnergyConsumption::RoomOfRequirementsMonitor => "Monitor",
        }),
        DeviceStateId::HeatingDemand(s) => Some(match s {
            HeatingDemand::LivingRoomBig => "Wohnzimmer (groß)",
            HeatingDemand::LivingRoomSmall => "Wohnzimmer (klein)",
            HeatingDemand::Bedroom => "Schlafzimmer",
            HeatingDemand::RoomOfRequirements => "Room of Requirements",
            HeatingDemand::Kitchen => "Küche",
            HeatingDemand::Bathroom => "Bad",
        }),
        _ => None,
    }
}
