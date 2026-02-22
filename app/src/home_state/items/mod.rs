mod absolute_humidity;
mod cold_air_coming_in;
mod dewpoint;
mod energy_saving;
mod fan_activity;
mod felt_temperature;
mod heating;
mod is_running;
mod occupancy;
mod opened;
mod power_available;
mod presence;
mod relative_humidity;
mod resident;
mod risk_of_mould;
mod temperature;
mod temperature_change;
mod ventilation;

use std::fmt::Debug;

pub use absolute_humidity::AbsoluteHumidity;
pub use cold_air_coming_in::ColdAirComingIn;
pub use dewpoint::DewPoint;
pub use energy_saving::EnergySaving;
pub use fan_activity::*;
pub use felt_temperature::FeltTemperature;
pub use heating::*;
pub use is_running::IsRunning;
pub use occupancy::Occupancy;
pub use opened::Opened;
pub use power_available::PowerAvailable;
pub use presence::Presence;
pub use relative_humidity::RelativeHumidity;
pub use resident::Resident;
pub use risk_of_mould::RiskOfMould;
pub use temperature::Temperature;
pub use temperature_change::TemperatureChange;
pub use ventilation::Ventilation;

use crate::core::range::Range;
use crate::core::time::DateTime;
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
    Occupancy(Occupancy, Probability),
    Opened(Opened, bool),
    Resident(Resident, bool),
    RiskOfMould(RiskOfMould, bool),
    Ventilation(Ventilation, bool),
    TargetHeatingAdjustment(TargetHeatingAdjustment, AdjustmentDirection),
    TargetHeatingMode(TargetHeatingMode, HeatingMode),
    TargetHeatingDemand(TargetHeatingDemand, Percent),

    EnergySaving(EnergySaving, bool),
    FanActivity(FanActivity, FanAirflow),
    HeatingDemand(HeatingDemand, Percent),
    HeatingDemandLimit(HeatingDemandLimit, Range<Percent>),
    PowerAvailable(PowerAvailable, bool),
    Presence(Presence, bool),
    RelativeHumidity(RelativeHumidity, Percent),
    SetPoint(SetPoint, Range<DegreeCelsius>),
    Temperature(Temperature, DegreeCelsius),
    TemperatureChange(TemperatureChange, RateOfChange<DegreeCelsius>),
}

impl HomeStateItem for HomeStateId {
    type Type = HomeStateValue;

    fn try_downcast(&self, value: HomeStateValue) -> anyhow::Result<Self::Type> {
        Ok(value)
    }
}

pub struct HomeStateDerivedStateProvider;

impl DerivedStateProvider<HomeStateId, HomeStateValue> for HomeStateDerivedStateProvider {
    fn calculate_current(&self, id: HomeStateId, ctx: &StateCalculationContext) -> Option<HomeStateValue> {
        match id {
            HomeStateId::AbsoluteHumidity(id) => absolute_humidity::AbsoluteHumidityStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::AbsoluteHumidity(id, value)),
            HomeStateId::ColdAirComingIn(id) => cold_air_coming_in::ColdAirComingInStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::ColdAirComingIn(id, value)),
            HomeStateId::DewPoint(id) => dewpoint::DewPointStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::DewPoint(id, value)),
            HomeStateId::FeltTemperature(id) => felt_temperature::FeltTemperatureStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::FeltTemperature(id, value)),
            HomeStateId::IsRunning(id) => is_running::IsRunningStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::IsRunning(id, value)),
            HomeStateId::Occupancy(id) => occupancy::OccupancyStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::Occupancy(id, value)),
            HomeStateId::Opened(id) => opened::OpenedStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::Opened(id, value)),
            HomeStateId::Ventilation(id) => ventilation::VentilationStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::Ventilation(id, value)),
            HomeStateId::Resident(id) => resident::ResidentStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::Resident(id, value)),
            HomeStateId::RiskOfMould(id) => risk_of_mould::RiskOfMouldStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::RiskOfMould(id, value)),
            HomeStateId::TargetHeatingMode(id) => heating::target_heating_mode::TargetHeatingModeStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::TargetHeatingMode(id, value)),
            HomeStateId::EnergySaving(id) => energy_saving::EnergySavingStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::EnergySaving(id, value)),
            HomeStateId::FanActivity(id) => fan_activity::FanActivityStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::FanActivity(id, value)),
            HomeStateId::HeatingDemand(id) => heating::heating_demand::HeatingDemandStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::HeatingDemand(id, value)),
            HomeStateId::HeatingDemandLimit(id) => heating::heating_demand_limit::HeatingDemandLimitStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::HeatingDemandLimit(id, value)),
            HomeStateId::PowerAvailable(id) => power_available::PowerAvailableStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::PowerAvailable(id, value)),
            HomeStateId::Presence(id) => presence::PresenceStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::Presence(id, value)),
            HomeStateId::RelativeHumidity(id) => relative_humidity::RelativeHumidityStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::RelativeHumidity(id, value)),
            HomeStateId::SetPoint(id) => heating::set_point::SetPointStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::SetPoint(id, value)),
            HomeStateId::Temperature(id) => temperature::TemperatureStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::Temperature(id, value)),
            HomeStateId::TemperatureChange(id) => temperature_change::TemperatureChangeStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::TemperatureChange(id, value)),
            HomeStateId::TargetHeatingDemand(id) => heating::target_heating_demand::HeatingDemandStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::TargetHeatingDemand(id, value)),
            HomeStateId::TargetHeatingAdjustment(id) => {
                heating::target_heating_adjustment::TargetHeatingAdjustmentStateProvider
                .calculate_current(id, ctx)
                .map(|value| HomeStateValue::TargetHeatingAdjustment(id, value))
            }
        }
    }
}

fn from_iso(s: &'static str) -> DateTime {
    DateTime::from_iso(s).expect("Invalid ISO datetime")
}
