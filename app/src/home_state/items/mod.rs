mod absolute_humidity;
mod automatic_temp_inc;
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
pub use automatic_temp_inc::AutomaticTemperatureIncrease;
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

use super::StateCalculationContext;
use crate::core::timeseries::DataPoint;
use crate::core::unit::*;
use crate::home_state::calc::DerivedStateProvider;
use crate::t;
use r#macro::StateTypeInfoDerive;

#[derive(Debug, Clone, PartialEq, StateTypeInfoDerive)]
pub enum HomeStateValue {
    AbsoluteHumidity(AbsoluteHumidity, GramPerCubicMeter),
    AutomaticTemperatureIncrease(AutomaticTemperatureIncrease, bool),
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

#[derive(Debug, Clone, PartialEq, derive_more::From)]
pub enum StateValue {
    Boolean(bool),
    DegreeCelsius(DegreeCelsius),
    Watt(Watt),
    Percent(Percent),
    GramPerCubicMeter(GramPerCubicMeter),
    KiloWattHours(KiloWattHours),
    HeatingUnit(HeatingUnit),
    KiloCubicMeter(KiloCubicMeter),
    FanAirflow(FanAirflow),
    HeatingMode(HeatingMode),
    RawValue(RawValue),
    Lux(Lux),
    Probability(Probability),
}

pub struct HomeStateDerivedStateProvider;

impl DerivedStateProvider<HomeState, StateValue> for HomeStateDerivedStateProvider {
    fn calculate_current(&self, id: HomeState, ctx: &StateCalculationContext) -> Option<DataPoint<StateValue>> {
        match id {
            HomeState::AbsoluteHumidity(id) => absolute_humidity::AbsoluteHumidityStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::AutomaticTemperatureIncrease(id) => {
                automatic_temp_inc::AutomaticTemperatureIncreaseStateProvider
                    .calculate_current(id, ctx)
                    .map(|dp| DataPoint {
                        value: dp.value.into(),
                        timestamp: dp.timestamp,
                    })
            }
            HomeState::ColdAirComingIn(id) => cold_air_coming_in::ColdAirComingInStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::DewPoint(id) => dewpoint::DewPointStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::FeltTemperature(id) => felt_temperature::FeltTemperatureStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::IsRunning(id) => {
                is_running::IsRunningStateProvider
                    .calculate_current(id, ctx)
                    .map(|dp| DataPoint {
                        value: dp.value.into(),
                        timestamp: dp.timestamp,
                    })
            }
            HomeState::Load(id) => load::LoadStateProvider.calculate_current(id, ctx).map(|dp| DataPoint {
                value: dp.value.into(),
                timestamp: dp.timestamp,
            }),
            HomeState::Occupancy(id) => {
                occupancy::OccupancyStateProvider
                    .calculate_current(id, ctx)
                    .map(|dp| DataPoint {
                        value: dp.value.into(),
                        timestamp: dp.timestamp,
                    })
            }
            HomeState::OpenedArea(id) => {
                opened::OpenedAreaStateProvider
                    .calculate_current(id, ctx)
                    .map(|dp| DataPoint {
                        value: dp.value.into(),
                        timestamp: dp.timestamp,
                    })
            }
            HomeState::Resident(id) => resident::ResidentStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::RiskOfMould(id) => {
                risk_of_mould::RiskOfMouldStateProvider
                    .calculate_current(id, ctx)
                    .map(|dp| DataPoint {
                        value: dp.value.into(),
                        timestamp: dp.timestamp,
                    })
            }
            HomeState::TargetHeatingMode(id) => target_heating_mode::TargetHeatingModeStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::EnergySaving(id) => {
                energy_saving::EnergySavingStateProvider
                    .calculate_current(id, ctx)
                    .map(|dp| DataPoint {
                        value: dp.value.into(),
                        timestamp: dp.timestamp,
                    })
            }
            HomeState::FanActivity(id) => fan_activity::FanActivityStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::HeatingDemand(id) => heating_demand::HeatingDemandStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::PowerAvailable(id) => power_available::PowerAvailableStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::Presence(id) => presence::PresenceStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::RawVendorValue(id) => raw_vendor_value::RawVendorValueStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::RelativeHumidity(id) => relative_humidity::RelativeHumidityStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
            HomeState::SetPoint(id) => {
                set_point::SetPointStateProvider
                    .calculate_current(id, ctx)
                    .map(|dp| DataPoint {
                        value: dp.value.into(),
                        timestamp: dp.timestamp,
                    })
            }
            HomeState::Temperature(id) => temperature::TemperatureStateProvider
                .calculate_current(id, ctx)
                .map(|dp| DataPoint {
                    value: dp.value.into(),
                    timestamp: dp.timestamp,
                }),
        }
    }
}

impl std::fmt::Display for StateValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateValue::Boolean(bool) => write!(f, "{}", bool),
            StateValue::DegreeCelsius(degree_celsius) => write!(f, "{}", degree_celsius),
            StateValue::Watt(watt) => write!(f, "{}", watt),
            StateValue::Percent(percent) => write!(f, "{}", percent),
            StateValue::GramPerCubicMeter(gram_per_cubic_meter) => write!(f, "{}", gram_per_cubic_meter),
            StateValue::KiloWattHours(kilo_watt_hours) => write!(f, "{}", kilo_watt_hours),
            StateValue::HeatingUnit(heating_unit) => write!(f, "{}", heating_unit),
            StateValue::KiloCubicMeter(kilo_cubic_meter) => write!(f, "{}", kilo_cubic_meter),
            StateValue::FanAirflow(fan_airflow) => write!(f, "{}", fan_airflow),
            StateValue::HeatingMode(heating_mode) => write!(f, "{}", heating_mode),
            StateValue::RawValue(raw_value) => write!(f, "{}", raw_value),
            StateValue::Lux(lux) => write!(f, "{}", lux),
            StateValue::Probability(probability) => write!(f, "{}", probability),
        }
    }
}
