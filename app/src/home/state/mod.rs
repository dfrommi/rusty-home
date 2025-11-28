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

use crate::core::HomeApi;
use crate::core::time::DateTimeRange;
use crate::core::time::Duration;
use crate::core::timeseries::DataFrame;
use crate::core::timeseries::DataPoint;
use crate::core::unit::*;
use crate::home::state::felt_temperature::FeltTemperature;
use crate::port::{DataPointAccess, TimeSeriesAccess, ValueObject};
use crate::t;
use r#macro::StateTypeInfoDerive;

#[derive(Debug, Clone, PartialEq, StateTypeInfoDerive)]
pub enum HomeStateValue {
    AbsoluteHumidity(AbsoluteHumidity, GramPerCubicMeter),
    AutomaticTemperatureIncrease(AutomaticTemperatureIncrease, bool),
    ColdAirComingIn(ColdAirComingIn, bool),
    DewPoint(DewPoint, DegreeCelsius),
    EnergySaving(EnergySaving, bool),
    FeltTemperature(FeltTemperature, DegreeCelsius),
    IsRunning(IsRunning, bool),
    Load(Load, Percent),
    Occupancy(Occupancy, Probability),
    OpenedArea(OpenedArea, bool),
    Resident(Resident, bool),
    RiskOfMould(RiskOfMould, bool),
    TargetHeatingMode(TargetHeatingMode, HeatingMode),

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

#[derive(Debug, Clone, PartialEq)]
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

pub trait PersistentHomeStateTypeInfo {
    type ValueType: Clone;

    fn to_f64(&self, value: &Self::ValueType) -> f64;
    fn from_f64(&self, value: f64) -> Self::ValueType;
}

async fn sampled_data_frame<T>(
    item: &T,
    range: DateTimeRange,
    rate: Duration,
    api: &HomeApi,
) -> anyhow::Result<DataFrame<T::ValueType>>
where
    T: ValueObject + DataPointAccess<T::ValueType>,
    T::ValueType: PartialEq,
{
    let caching_range = DateTimeRange::new(*range.start() - t!(3 hours), *range.end() + t!(3 hours));
    let api = api.for_processing_of_range(caching_range);

    let mut result = vec![];
    let mut previous_value: Option<T::ValueType> = None;

    let mut seen_timestamps = std::collections::BTreeSet::new();

    for dt in range.step_by(rate) {
        let mut dp = dt
            .eval_timeshifted(async { item.current_data_point(&api).await })
            .await?;

        if previous_value.as_ref() != Some(&dp.value) {
            //Timestamp might jump back to an old value, as a consequence of calculation.
            //It could take another path and then take the timestamp from other/older source datapoints.
            //Keeping track of seen timestamps to avoid jumping back and forth. Just assuming the
            //current "now" for such cases.
            if seen_timestamps.contains(&dp.timestamp) {
                dp = DataPoint::new(dp.value, dt);
            }

            result.push(dp.clone());
            previous_value = Some(dp.value.clone());
            seen_timestamps.insert(dp.timestamp);
        }
    }

    DataFrame::new(result)
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
