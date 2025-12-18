mod absolute_humidity;
mod cold_air_coming_in;
mod dewpoint;
mod energy_saving;
mod fan_activity;
mod felt_temperature;
mod heating_demand;
mod is_running;
mod load;
mod occupancy;
mod opened;
mod power_available;
mod presence;
mod raw_vendor_value;
mod relative_humidity;
mod resident;
mod risk_of_mould;
mod set_point;
mod target_heating_mode;
mod temperature;

use std::fmt::Debug;

pub use absolute_humidity::AbsoluteHumidity;
pub use cold_air_coming_in::ColdAirComingIn;
pub use dewpoint::DewPoint;
pub use energy_saving::EnergySaving;
pub use fan_activity::*;
pub use felt_temperature::FeltTemperature;
pub use heating_demand::HeatingDemand;
pub use is_running::IsRunning;
pub use load::Load;
pub use occupancy::Occupancy;
pub use opened::OpenedArea;
pub use power_available::PowerAvailable;
pub use presence::Presence;
pub use raw_vendor_value::RawVendorValue;
pub use relative_humidity::RelativeHumidity;
pub use resident::Resident;
pub use risk_of_mould::RiskOfMould;
pub use set_point::SetPoint;
pub use target_heating_mode::*;
pub use temperature::Temperature;

use crate::core::timeseries::DataPoint;
use crate::core::unit::*;
use crate::home_state::calc::DerivedStateProvider;
use crate::home_state::calc::StateCalculationContext;
use crate::t;
use r#macro::StateEnumDerive;

#[derive(Debug, Clone, PartialEq, StateEnumDerive)]
pub enum HomeStateValue {
    AbsoluteHumidity(AbsoluteHumidity, GramPerCubicMeter),
    ColdAirComingIn(ColdAirComingIn, bool),
    DewPoint(DewPoint, DegreeCelsius),
    FeltTemperature(FeltTemperature, DegreeCelsius),
    IsRunning(IsRunning, bool),
    Load(Load, Percent),
    Occupancy(Occupancy, Probability),
    OpenedArea(OpenedArea, bool),
    Resident(Resident, bool),
    RiskOfMould(RiskOfMould, bool),
    TargetHeatingMode(TargetHeatingMode, HeatingMode),

    EnergySaving(EnergySaving, bool),
    FanActivity(FanActivity, FanAirflow),
    HeatingDemand(HeatingDemand, Percent),
    PowerAvailable(PowerAvailable, bool),
    Presence(Presence, bool),
    RawVendorValue(RawVendorValue, RawValue),
    RelativeHumidity(RelativeHumidity, Percent),
    SetPoint(SetPoint, DegreeCelsius),
    Temperature(Temperature, DegreeCelsius),
}

impl HomeStateItem for HomeStateId {
    type Type = HomeStateValue;

    fn try_downcast(&self, value: HomeStateValue) -> anyhow::Result<Self::Type> {
        Ok(value)
    }
}

pub struct HomeStateDerivedStateProvider;

impl DerivedStateProvider<HomeStateId, HomeStateValue> for HomeStateDerivedStateProvider {
    fn calculate_current(&self, id: HomeStateId, ctx: &StateCalculationContext) -> Option<DataPoint<HomeStateValue>> {
        match id {
            HomeStateId::AbsoluteHumidity(id) => absolute_humidity::AbsoluteHumidityStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::AbsoluteHumidity(id, dp.value))),
            HomeStateId::ColdAirComingIn(id) => cold_air_coming_in::ColdAirComingInStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::ColdAirComingIn(id, dp.value))),
            HomeStateId::DewPoint(id) => dewpoint::DewPointStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::DewPoint(id, dp.value))),
            HomeStateId::FeltTemperature(id) => felt_temperature::FeltTemperatureStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::FeltTemperature(id, dp.value))),
            HomeStateId::IsRunning(id) => is_running::IsRunningStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::IsRunning(id, dp.value))),
            HomeStateId::Load(id) => load::LoadStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::Load(id, dp.value))),
            HomeStateId::Occupancy(id) => occupancy::OccupancyStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::Occupancy(id, dp.value))),
            HomeStateId::OpenedArea(id) => opened::OpenedAreaStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::OpenedArea(id, dp.value))),
            HomeStateId::Resident(id) => resident::ResidentStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::Resident(id, dp.value))),
            HomeStateId::RiskOfMould(id) => risk_of_mould::RiskOfMouldStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::RiskOfMould(id, dp.value))),
            HomeStateId::TargetHeatingMode(id) => target_heating_mode::TargetHeatingModeStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::TargetHeatingMode(id, dp.value.clone()))),
            HomeStateId::EnergySaving(id) => energy_saving::EnergySavingStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::EnergySaving(id, dp.value))),
            HomeStateId::FanActivity(id) => fan_activity::FanActivityStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::FanActivity(id, dp.value.clone()))),
            HomeStateId::HeatingDemand(id) => heating_demand::HeatingDemandStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::HeatingDemand(id, dp.value))),
            HomeStateId::PowerAvailable(id) => power_available::PowerAvailableStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::PowerAvailable(id, dp.value))),
            HomeStateId::Presence(id) => presence::PresenceStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::Presence(id, dp.value))),
            HomeStateId::RawVendorValue(id) => raw_vendor_value::RawVendorValueStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::RawVendorValue(id, dp.value))),
            HomeStateId::RelativeHumidity(id) => relative_humidity::RelativeHumidityStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::RelativeHumidity(id, dp.value))),
            HomeStateId::SetPoint(id) => set_point::SetPointStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::SetPoint(id, dp.value))),
            HomeStateId::Temperature(id) => temperature::TemperatureStateProvider
                .calculate_current(id, ctx)
                .map(|dp| dp.with(HomeStateValue::Temperature(id, dp.value))),
        }
    }
}
