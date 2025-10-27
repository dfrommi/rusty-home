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
mod absolute_humidity;
mod current_power_usage;
mod energy_saving;
mod fan_activity;
mod heating_demand;
mod powered;
mod presence;
mod relative_humidity;
mod scheduled_heating_mode;
mod set_point;
mod temperature;
mod total_energy_consumption;
mod total_radiator_consumption;
mod total_water_consumption;
mod user_controlled;

use std::fmt::Debug;

pub use absolute_humidity::AbsoluteHumidity;
pub use automatic_temp_inc::AutomaticTemperatureIncrease;
pub use cold_air_coming_in::ColdAirComingIn;
pub use current_power_usage::CurrentPowerUsage;
pub use dewpoint::DewPoint;
pub use energy_saving::EnergySaving;
pub use fan_activity::*;
pub use heating_demand::HeatingDemand;
pub use opened::Opened;
pub use opened::OpenedArea;
pub use powered::Powered;
pub use presence::Presence;
pub use relative_humidity::RelativeHumidity;
pub use resident::Resident;
pub use risk_of_mould::RiskOfMould;
pub use scheduled_heating_mode::*;
pub use set_point::SetPoint;
pub use temperature::Temperature;
pub use total_energy_consumption::TotalEnergyConsumption;
pub use total_radiator_consumption::TotalRadiatorConsumption;
pub use total_water_consumption::TotalWaterConsumption;
pub use user_controlled::UserControlled;

use crate::core::HomeApi;
use crate::core::time::DateTimeRange;
use crate::core::time::Duration;
use crate::core::timeseries::DataFrame;
use crate::core::timeseries::DataPoint;
use crate::core::unit::*;
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
    OpenedArea(OpenedArea, bool),
    Resident(Resident, bool),
    RiskOfMould(RiskOfMould, bool),
    ScheduledHeatingMode(ScheduledHeatingMode, HeatingMode),
    UserControlled(UserControlled, bool),

    #[persistent]
    CurrentPowerUsage(CurrentPowerUsage, Watt),
    #[persistent]
    FanActivity(FanActivity, FanAirflow),
    #[persistent]
    HeatingDemand(HeatingDemand, Percent),
    #[persistent]
    Opened(Opened, bool),
    #[persistent]
    Powered(Powered, bool),
    #[persistent]
    Presence(Presence, bool),
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
            //Timestamp might jump back to an old value, as a consequence of calculation and taking the
            //timestamp from the datapoints.
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
        }
    }
}
