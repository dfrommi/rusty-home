use r#macro::StateEnumDerive;

use crate::core::{time::DateTime, unit::*};

mod allergen_index;
mod completion_time;
mod current_power_usage;
mod energy_saving;
mod fan_activity;
mod heating_demand;
mod heating_demand_limit;
mod is_running;
mod light_level;
mod opened;
mod particulate_matter;
mod power_available;
mod presence;
mod relative_humidity;
mod set_point;
mod temperature;
mod total_energy_consumption;
mod total_radiator_consumption;

pub use allergen_index::AllergenIndex;
pub use completion_time::CompletionTime;
pub use current_power_usage::CurrentPowerUsage;
pub use energy_saving::EnergySaving;
pub use fan_activity::FanActivity;
#[allow(unused_imports)]
pub use heating_demand::HeatingDemand;
pub use heating_demand_limit::HeatingDemandLimit;
pub use is_running::IsRunning;
pub use light_level::LightLevel;
pub use opened::Opened;
pub use particulate_matter::ParticulateMatter;
pub use power_available::PowerAvailable;
pub use presence::Presence;
pub use relative_humidity::RelativeHumidity;
pub use set_point::SetPoint;
pub use temperature::Temperature;
pub use total_energy_consumption::TotalEnergyConsumption;
pub use total_radiator_consumption::TotalRadiatorConsumption;

#[derive(Debug, Clone, PartialEq, StateEnumDerive)]
pub enum DeviceStateValue {
    AllergenIndex(allergen_index::AllergenIndex, AllergenIndexValue),
    CompletionTime(completion_time::CompletionTime, DateTime),
    EnergySaving(energy_saving::EnergySaving, bool),
    CurrentPowerUsage(current_power_usage::CurrentPowerUsage, Watt),
    FanActivity(fan_activity::FanActivity, FanAirflow),
    HeatingDemand(heating_demand::HeatingDemand, Percent),
    HeatingDemandLimit(heating_demand_limit::HeatingDemandLimit, Percent),
    IsRunning(is_running::IsRunning, bool),
    LightLevel(light_level::LightLevel, Lux),
    Opened(opened::Opened, bool),
    ParticulateMatter(particulate_matter::ParticulateMatter, MicrogramsPerCubicMeter),
    PowerAvailable(power_available::PowerAvailable, bool),
    Presence(presence::Presence, bool),
    RelativeHumidity(relative_humidity::RelativeHumidity, Percent),
    SetPoint(set_point::SetPoint, DegreeCelsius),
    Temperature(temperature::Temperature, DegreeCelsius),
    TotalEnergyConsumption(total_energy_consumption::TotalEnergyConsumption, KiloWattHours),
    TotalRadiatorConsumption(total_radiator_consumption::TotalRadiatorConsumption, HeatingUnit),
}

impl DeviceStateValue {
    /// Returns the typed minimum change for this incoming value, if this state opts into filtering.
    pub fn minimum_storage_delta(&self) -> Option<f64> {
        match self {
            DeviceStateValue::CurrentPowerUsage(id, value) => id.minimum_storage_delta(*value).as_ref().map(f64::from),
            DeviceStateValue::LightLevel(id, value) => id.minimum_storage_delta(*value).as_ref().map(f64::from),
            _ => None,
        }
    }
}

impl From<&DeviceStateValue> for f64 {
    fn from(value: &DeviceStateValue) -> Self {
        match value {
            DeviceStateValue::AllergenIndex(_, v) => v.into(),
            DeviceStateValue::CurrentPowerUsage(_, v) => v.into(),
            DeviceStateValue::FanActivity(_, v) => v.into(),
            DeviceStateValue::HeatingDemand(_, v) => v.into(),
            DeviceStateValue::HeatingDemandLimit(_, v) => v.into(),
            DeviceStateValue::LightLevel(_, v) => v.into(),
            DeviceStateValue::ParticulateMatter(_, v) => v.into(),
            DeviceStateValue::RelativeHumidity(_, v) => v.into(),
            DeviceStateValue::SetPoint(_, v) => v.into(),
            DeviceStateValue::Temperature(_, v) => v.into(),
            DeviceStateValue::TotalEnergyConsumption(_, v) => v.into(),
            DeviceStateValue::TotalRadiatorConsumption(_, v) => v.into(),
            DeviceStateValue::CompletionTime(_, v) => v.unix_millis() as f64,
            DeviceStateValue::EnergySaving(_, v)
            | DeviceStateValue::IsRunning(_, v)
            | DeviceStateValue::Opened(_, v)
            | DeviceStateValue::PowerAvailable(_, v)
            | DeviceStateValue::Presence(_, v) => {
                if *v {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
}
