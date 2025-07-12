mod automatic_temp_inc;
mod cold_air_coming_in;
//mod current_power_usage;
mod dewpoint;
//mod external_auto_control;
mod opened;
//mod powered;
//mod relative_humidity;
mod resident;
mod risk_of_mould;
//mod set_point;
//mod temperature;
//mod total_energy_consumption;
mod current_power_usage;
mod energy_saving;
mod external_auto_control;
mod fan_activity;
mod heating_demand;
mod powered;
mod presence;
mod relative_humidity;
mod set_point;
mod temperature;
mod total_energy_consumption;
mod total_radiator_consumption;
mod total_water_consumption;
mod user_controlled;

pub use automatic_temp_inc::AutomaticTemperatureIncrease;
pub use cold_air_coming_in::ColdAirComingIn;
pub use current_power_usage::CurrentPowerUsage;
pub use dewpoint::DewPoint;
pub use energy_saving::EnergySaving;
pub use external_auto_control::ExternalAutoControl;
pub use fan_activity::*;
pub use heating_demand::HeatingDemand;
pub use opened::Opened;
pub use opened::raw::Opened as OpenedRaw;
pub use powered::Powered;
pub use presence::Presence;
pub use relative_humidity::RelativeHumidity;
pub use resident::Resident;
pub use risk_of_mould::RiskOfMould;
pub use set_point::SetPoint;
pub use temperature::Temperature;
pub use total_energy_consumption::TotalEnergyConsumption;
pub use total_radiator_consumption::TotalRadiatorConsumption;
pub use total_water_consumption::TotalWaterConsumption;
pub use user_controlled::UserControlled;

use crate::core::unit::*;
use crate::port::*;
use r#macro::{EnumWithValue, StateTypeInfoDerive};

#[derive(Debug, Clone, EnumWithValue, StateTypeInfoDerive)]
pub enum HomeStateValue {
    AutomaticTemperatureIncrease(AutomaticTemperatureIncrease, bool),
    ColdAirComingIn(ColdAirComingIn, bool),
    #[persistent]
    CurrentPowerUsage(CurrentPowerUsage, Watt),
    DewPoint(DewPoint, DegreeCelsius),
    EnergySaving(EnergySaving, bool),
    #[persistent]
    ExternalAutoControl(ExternalAutoControl, bool),
    #[persistent]
    FanActivity(FanActivity, FanAirflow),
    #[persistent]
    HeatingDemand(HeatingDemand, Percent),
    Opened(Opened, bool),
    #[persistent]
    OpenedRaw(OpenedRaw, bool),
    #[persistent]
    Powered(Powered, bool),
    #[persistent]
    Presence(Presence, bool),
    #[persistent]
    RelativeHumidity(RelativeHumidity, Percent),
    Resident(Resident, bool),
    RiskOfMould(RiskOfMould, bool),
    #[persistent]
    SetPoint(SetPoint, DegreeCelsius),
    #[persistent]
    Temperature(Temperature, DegreeCelsius),
    #[persistent]
    TotalEnergyConsumption(TotalEnergyConsumption, KiloWattHours),
    #[persistent]
    TotalRadiatorConsumption(TotalRadiatorConsumption, HeatingUnit),
    #[persistent]
    TotalWaterConsumption(TotalWaterConsumption, KiloCubicMeter),
    UserControlled(UserControlled, bool),
}

mod macros {
    macro_rules! result {
        ($result:expr, $timestamp:expr, $item:expr, { $(,)* $($dps:ident),* }, @$dp:ident, $($arg:tt)+ ) => {
            result!($result, $timestamp, $item, { $($dps),*, $dp }, $($arg)+)
        };

        ($result:expr, $timestamp:expr, $item:expr, { $(,)* $($dps:ident),* }, $($arg:tt)+ ) => {
            let result = crate::core::timeseries::DataPoint::new($result, $timestamp);

            tracing::trace!(
                timestamp = %crate::t!(now),
                item.r#type = %$item.int_type(),
                item.name = %$item.int_name(),
                result.value = %result.value,
                result.timestamp = %result.timestamp,
                $(
                    $dps.value = %$dps.value,
                    $dps.timestamp = %$dps.timestamp,
                    $dps.elapsed = %$dps.timestamp.elapsed(),
                )*
                $($arg)+
            );

            return Ok(result);
        };

        ($result:expr, $timestamp:expr, $item:expr, $($arg:tt)+ ) => {
            result!($result, $timestamp, $item, {}, $($arg)+)
        };
    }

    pub(super) use result;
}
