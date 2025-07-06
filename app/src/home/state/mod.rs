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

use crate::core::{ValueObject, unit::*};
use crate::port::*;
use r#macro::{DbMapped, PersistentStateDerive};

#[derive(Debug, Clone, PersistentStateDerive, DbMapped)]
pub enum PersistentStateValue {
    Temperature(Temperature, DegreeCelsius),
    RelativeHumidity(RelativeHumidity, Percent),
    Opened(OpenedRaw, bool),
    Powered(Powered, bool),
    CurrentPowerUsage(CurrentPowerUsage, Watt),
    TotalEnergyConsumption(TotalEnergyConsumption, KiloWattHours),
    SetPoint(SetPoint, DegreeCelsius),
    HeatingDemand(HeatingDemand, Percent),
    ExternalAutoControl(ExternalAutoControl, bool),
    Presence(Presence, bool),
    TotalRadiatorConsumption(TotalRadiatorConsumption, HeatingUnit),
    TotalWaterConsumption(TotalWaterConsumption, KiloCubicMeter),
    FanActivity(FanActivity, FanAirflow),
}

impl PersistentStateValue {
    pub fn value_to_string(&self) -> String {
        match self {
            PersistentStateValue::Temperature(_, value) => value.to_string(),
            PersistentStateValue::RelativeHumidity(_, value) => value.to_string(),
            PersistentStateValue::Opened(_, value) => value.to_string(),
            PersistentStateValue::Powered(_, value) => value.to_string(),
            PersistentStateValue::CurrentPowerUsage(_, value) => value.to_string(),
            PersistentStateValue::TotalEnergyConsumption(_, value) => value.to_string(),
            PersistentStateValue::SetPoint(_, value) => value.to_string(),
            PersistentStateValue::HeatingDemand(_, value) => value.to_string(),
            PersistentStateValue::ExternalAutoControl(_, value) => value.to_string(),
            PersistentStateValue::Presence(_, value) => value.to_string(),
            PersistentStateValue::TotalRadiatorConsumption(_, value) => value.to_string(),
            PersistentStateValue::TotalWaterConsumption(_, value) => value.to_string(),
            PersistentStateValue::FanActivity(_, value) => value.to_string(),
        }
    }
}

impl From<&PersistentStateValue> for f64 {
    fn from(val: &PersistentStateValue) -> Self {
        match val {
            PersistentStateValue::Temperature(_, value) => Temperature::to_f64(value),
            PersistentStateValue::RelativeHumidity(_, value) => RelativeHumidity::to_f64(value),
            PersistentStateValue::Opened(_, value) => Opened::to_f64(value),
            PersistentStateValue::Powered(_, value) => Powered::to_f64(value),
            PersistentStateValue::CurrentPowerUsage(_, value) => CurrentPowerUsage::to_f64(value),
            PersistentStateValue::TotalEnergyConsumption(_, value) => {
                TotalEnergyConsumption::to_f64(value)
            }
            PersistentStateValue::SetPoint(_, value) => SetPoint::to_f64(value),
            PersistentStateValue::HeatingDemand(_, value) => HeatingDemand::to_f64(value),
            PersistentStateValue::ExternalAutoControl(_, value) => {
                ExternalAutoControl::to_f64(value)
            }
            PersistentStateValue::Presence(_, value) => Presence::to_f64(value),
            PersistentStateValue::TotalRadiatorConsumption(_, value) => {
                TotalRadiatorConsumption::to_f64(value)
            }
            PersistentStateValue::TotalWaterConsumption(_, value) => {
                TotalWaterConsumption::to_f64(value)
            }
            PersistentStateValue::FanActivity(_, value) => FanActivity::to_f64(value),
        }
    }
}

impl From<(PersistentState, f64)> for PersistentStateValue {
    fn from(val: (PersistentState, f64)) -> Self {
        let (channel, value) = val;
        match channel {
            PersistentState::Temperature(item) => {
                PersistentStateValue::Temperature(item, Temperature::from_f64(value))
            }
            PersistentState::RelativeHumidity(item) => {
                PersistentStateValue::RelativeHumidity(item, RelativeHumidity::from_f64(value))
            }
            PersistentState::Opened(item) => {
                PersistentStateValue::Opened(item, Opened::from_f64(value))
            }
            PersistentState::Powered(item) => {
                PersistentStateValue::Powered(item, Powered::from_f64(value))
            }
            PersistentState::CurrentPowerUsage(item) => {
                PersistentStateValue::CurrentPowerUsage(item, CurrentPowerUsage::from_f64(value))
            }
            PersistentState::TotalEnergyConsumption(item) => {
                PersistentStateValue::TotalEnergyConsumption(
                    item,
                    TotalEnergyConsumption::from_f64(value),
                )
            }
            PersistentState::SetPoint(item) => {
                PersistentStateValue::SetPoint(item, SetPoint::from_f64(value))
            }
            PersistentState::HeatingDemand(item) => {
                PersistentStateValue::HeatingDemand(item, HeatingDemand::from_f64(value))
            }
            PersistentState::ExternalAutoControl(item) => {
                PersistentStateValue::ExternalAutoControl(
                    item,
                    ExternalAutoControl::from_f64(value),
                )
            }
            PersistentState::Presence(item) => {
                PersistentStateValue::Presence(item, Presence::from_f64(value))
            }
            PersistentState::TotalRadiatorConsumption(item) => {
                PersistentStateValue::TotalRadiatorConsumption(
                    item,
                    TotalRadiatorConsumption::from_f64(value),
                )
            }
            PersistentState::TotalWaterConsumption(item) => {
                PersistentStateValue::TotalWaterConsumption(
                    item,
                    TotalWaterConsumption::from_f64(value),
                )
            }
            PersistentState::FanActivity(item) => {
                PersistentStateValue::FanActivity(item, FanActivity::from_f64(value))
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use crate::core::time::DateTime;
    use crate::t;
    use crate::{
        core::timeseries::{DataFrame, DataPoint},
        home::state::*,
    };

    use crate::core::timeseries::TimeSeries;

    use super::{DataPointAccess, TimeSeriesAccess};

    #[derive(Clone, Default)]
    pub struct Api {
        opened: Option<DataPoint<bool>>,
        temperature_dp: Option<DataPoint<DegreeCelsius>>,
        temperature_df: Option<DataFrame<DegreeCelsius>>,
    }

    impl Api {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn opened(&mut self, value: bool, at: DateTime) -> &mut Self {
            self.opened = Some(DataPoint::new(value, at));
            self
        }

        pub fn current_temperature(&mut self, value: f64) -> &mut Self {
            self.temperature_dp = Some(DataPoint::new(DegreeCelsius(value), t!(now)));
            self
        }

        pub fn temperature_series(&mut self, values: &[(f64, DateTime)]) -> &mut Self {
            self.temperature_df = Some(
                DataFrame::new(
                    values
                        .iter()
                        .map(|(v, t)| DataPoint::new(DegreeCelsius(*v), *t)),
                )
                .unwrap(),
            );

            self
        }
    }

    impl DataPointAccess<Opened> for Api {
        async fn current_data_point(&self, _: Opened) -> anyhow::Result<DataPoint<bool>> {
            Ok(self.opened.clone().unwrap())
        }
    }

    impl DataPointAccess<Temperature> for Api {
        async fn current_data_point(
            &self,
            _: Temperature,
        ) -> anyhow::Result<DataPoint<DegreeCelsius>> {
            Ok(self.temperature_dp.clone().unwrap())
        }
    }

    impl TimeSeriesAccess<Temperature> for Api {
        async fn series(
            &self,
            item: Temperature,
            range: crate::core::time::DateTimeRange,
        ) -> anyhow::Result<TimeSeries<Temperature>> {
            TimeSeries::new(item, &self.temperature_df.clone().unwrap(), range)
        }
    }
}
