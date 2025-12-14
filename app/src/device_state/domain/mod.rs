use r#macro::StateEnumDerive;

use crate::core::unit::*;

mod current_power_usage;
mod energy_saving;
mod fan_activity;
mod heating_demand;
mod light_level;
mod opened;
mod power_available;
mod presence;
mod raw_vendor_value;
mod relative_humidity;
mod set_point;
mod temperature;
mod total_energy_consumption;
mod total_radiator_consumption;
mod total_water_consumption;

pub use current_power_usage::CurrentPowerUsage;
pub use energy_saving::EnergySaving;
pub use fan_activity::FanActivity;
pub use heating_demand::HeatingDemand;
pub use light_level::LightLevel;
pub use opened::Opened;
pub use power_available::PowerAvailable;
pub use presence::Presence;
pub use raw_vendor_value::RawVendorValue;
pub use relative_humidity::RelativeHumidity;
pub use set_point::SetPoint;
pub use temperature::Temperature;
pub use total_energy_consumption::TotalEnergyConsumption;
pub use total_radiator_consumption::TotalRadiatorConsumption;
pub use total_water_consumption::TotalWaterConsumption;

#[derive(Debug, Clone, PartialEq, StateEnumDerive)]
pub enum DeviceStateValue {
    EnergySaving(energy_saving::EnergySaving, bool),
    CurrentPowerUsage(current_power_usage::CurrentPowerUsage, Watt),
    FanActivity(fan_activity::FanActivity, FanAirflow),
    HeatingDemand(heating_demand::HeatingDemand, Percent),
    LightLevel(light_level::LightLevel, Lux),
    Opened(opened::Opened, bool),
    PowerAvailable(power_available::PowerAvailable, bool),
    Presence(presence::Presence, bool),
    RawVendorValue(raw_vendor_value::RawVendorValue, RawValue),
    RelativeHumidity(relative_humidity::RelativeHumidity, Percent),
    SetPoint(set_point::SetPoint, DegreeCelsius),
    Temperature(temperature::Temperature, DegreeCelsius),
    TotalEnergyConsumption(total_energy_consumption::TotalEnergyConsumption, KiloWattHours),
    TotalRadiatorConsumption(total_radiator_consumption::TotalRadiatorConsumption, HeatingUnit),
    TotalWaterConsumption(total_water_consumption::TotalWaterConsumption, KiloCubicMeter),
}

impl From<&DeviceStateValue> for f64 {
    fn from(value: &DeviceStateValue) -> Self {
        match value {
            DeviceStateValue::CurrentPowerUsage(_, v) => v.into(),
            DeviceStateValue::FanActivity(_, v) => v.into(),
            DeviceStateValue::HeatingDemand(_, v) => v.into(),
            DeviceStateValue::LightLevel(_, v) => v.into(),
            DeviceStateValue::RawVendorValue(_, v) => v.into(),
            DeviceStateValue::RelativeHumidity(_, v) => v.into(),
            DeviceStateValue::SetPoint(_, v) => v.into(),
            DeviceStateValue::Temperature(_, v) => v.into(),
            DeviceStateValue::TotalEnergyConsumption(_, v) => v.into(),
            DeviceStateValue::TotalRadiatorConsumption(_, v) => v.into(),
            DeviceStateValue::TotalWaterConsumption(_, v) => v.into(),
            DeviceStateValue::EnergySaving(_, v)
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
