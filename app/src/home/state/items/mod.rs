mod absolute_humidity;
mod automatic_temp_inc;
mod cold_air_coming_in;
mod current_power_usage;
mod dewpoint;
mod energy_saving;
mod fan_activity;
mod felt_temperature;
mod heating_demand;
mod is_running;
mod light_level;
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
mod total_energy_consumption;
mod total_radiator_consumption;
mod total_water_consumption;

use std::fmt::Debug;

pub use absolute_humidity::AbsoluteHumidity;
pub use automatic_temp_inc::AutomaticTemperatureIncrease;
pub use cold_air_coming_in::ColdAirComingIn;
pub use current_power_usage::CurrentPowerUsage;
pub use dewpoint::DewPoint;
pub use energy_saving::EnergySaving;
pub use fan_activity::*;
pub use felt_temperature::FeltTemperature;
pub use heating_demand::HeatingDemand;
pub use is_running::IsRunning;
pub use light_level::LightLevel;
pub use load::Load;
pub use occupancy::Occupancy;
pub use opened::Opened;
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
pub use total_energy_consumption::TotalEnergyConsumption;
pub use total_radiator_consumption::TotalRadiatorConsumption;
pub use total_water_consumption::TotalWaterConsumption;

use super::DerivedStateProvider;
use super::StateCalculationContext;
use crate::core::timeseries::DataPoint;
use crate::core::unit::*;
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

    #[persistent]
    EnergySaving(EnergySaving, bool),
    #[persistent]
    CurrentPowerUsage(CurrentPowerUsage, Watt),
    #[persistent]
    FanActivity(FanActivity, FanAirflow),
    #[persistent]
    HeatingDemand(HeatingDemand, Percent),
    #[persistent]
    LightLevel(LightLevel, Lux),
    #[persistent]
    Opened(Opened, bool),
    #[persistent]
    PowerAvailable(PowerAvailable, bool),
    #[persistent]
    Presence(Presence, bool),
    #[persistent]
    RawVendorValue(RawVendorValue, RawValue),
    #[persistent]
    RelativeHumidity(RelativeHumidity, Percent),
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

#[derive(Debug, Clone, PartialEq)]
pub enum PersistentStateValue {
    Boolean(bool),
    DegreeCelsius(DegreeCelsius),
    Watt(Watt),
    Percent(Percent),
    KiloWattHours(KiloWattHours),
    HeatingUnit(HeatingUnit),
    KiloCubicMeter(KiloCubicMeter),
    FanAirflow(FanAirflow),
    RawValue(RawValue),
    Lux(Lux),
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
            HomeState::TargetHeatingMode(id) => {
                TargetHeatingModeStateProvider
                    .calculate_current(id, ctx)
                    .map(|dp| DataPoint {
                        value: dp.value.into(),
                        timestamp: dp.timestamp,
                    })
            }
            _ if id.is_persistent() => {
                tracing::warn!("Trying to calculate persistent state {:?}", id);
                None
            }
            //to be changed when HomeState gets catch-all for persistent states
            _ => unreachable!("This can't happen as all persistent states are covered above"),
        }
    }
}

pub trait PersistentHomeStateTypeInfo {
    type ValueType: Clone;

    fn to_f64(&self, value: &Self::ValueType) -> f64;
    fn from_f64(&self, value: f64) -> Self::ValueType;
}

impl From<PersistentStateValue> for StateValue {
    fn from(persistent: PersistentStateValue) -> Self {
        match persistent {
            PersistentStateValue::Boolean(bool) => StateValue::Boolean(bool),
            PersistentStateValue::DegreeCelsius(degree_celsius) => StateValue::DegreeCelsius(degree_celsius),
            PersistentStateValue::Watt(watt) => StateValue::Watt(watt),
            PersistentStateValue::Percent(percent) => StateValue::Percent(percent),
            PersistentStateValue::KiloWattHours(kilo_watt_hours) => StateValue::KiloWattHours(kilo_watt_hours),
            PersistentStateValue::HeatingUnit(heating_unit) => StateValue::HeatingUnit(heating_unit),
            PersistentStateValue::KiloCubicMeter(kilo_cubic_meter) => StateValue::KiloCubicMeter(kilo_cubic_meter),
            PersistentStateValue::FanAirflow(fan_airflow) => StateValue::FanAirflow(fan_airflow),
            PersistentStateValue::RawValue(raw_value) => StateValue::RawValue(raw_value),
            PersistentStateValue::Lux(lux) => StateValue::Lux(lux),
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

impl std::fmt::Display for PersistentStateValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistentStateValue::Boolean(bool) => write!(f, "{}", bool),
            PersistentStateValue::DegreeCelsius(degree_celsius) => write!(f, "{}", degree_celsius),
            PersistentStateValue::Watt(watt) => write!(f, "{}", watt),
            PersistentStateValue::Percent(percent) => write!(f, "{}", percent),
            PersistentStateValue::KiloWattHours(kilo_watt_hours) => write!(f, "{}", kilo_watt_hours),
            PersistentStateValue::HeatingUnit(heating_unit) => write!(f, "{}", heating_unit),
            PersistentStateValue::KiloCubicMeter(kilo_cubic_meter) => write!(f, "{}", kilo_cubic_meter),
            PersistentStateValue::FanAirflow(fan_airflow) => write!(f, "{}", fan_airflow),
            PersistentStateValue::RawValue(raw_value) => write!(f, "{}", raw_value),
            PersistentStateValue::Lux(lux) => write!(f, "{}", lux),
        }
    }
}
