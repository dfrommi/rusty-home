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

use crate::{core::persistence::DbValue, port::*};
use r#macro::{DbMapped, StateChannel};
use crate::core::unit::*;

#[derive(Debug, Clone, StateChannel, DbMapped)]
pub enum ChannelValue {
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

impl ChannelValue {
    pub fn value_to_string(&self) -> String {
        match self {
            ChannelValue::Temperature(_, value) => value.to_string(),
            ChannelValue::RelativeHumidity(_, value) => value.to_string(),
            ChannelValue::Opened(_, value) => value.to_string(),
            ChannelValue::Powered(_, value) => value.to_string(),
            ChannelValue::CurrentPowerUsage(_, value) => value.to_string(),
            ChannelValue::TotalEnergyConsumption(_, value) => value.to_string(),
            ChannelValue::SetPoint(_, value) => value.to_string(),
            ChannelValue::HeatingDemand(_, value) => value.to_string(),
            ChannelValue::ExternalAutoControl(_, value) => value.to_string(),
            ChannelValue::Presence(_, value) => value.to_string(),
            ChannelValue::TotalRadiatorConsumption(_, value) => value.to_string(),
            ChannelValue::TotalWaterConsumption(_, value) => value.to_string(),
            ChannelValue::FanActivity(_, value) => value.to_string(),
        }
    }
}

//TODO macro
impl From<(Channel, DbValue)> for ChannelValue {
    fn from(val: (Channel, DbValue)) -> Self {
        let (channel, value) = val;
        match channel {
            Channel::Temperature(item) => ChannelValue::Temperature(item, value.into()),
            Channel::RelativeHumidity(item) => ChannelValue::RelativeHumidity(item, value.into()),
            Channel::Opened(item) => ChannelValue::Opened(item, value.into()),
            Channel::Powered(item) => ChannelValue::Powered(item, value.into()),
            Channel::CurrentPowerUsage(item) => ChannelValue::CurrentPowerUsage(item, value.into()),
            Channel::TotalEnergyConsumption(item) => {
                ChannelValue::TotalEnergyConsumption(item, value.into())
            }
            Channel::SetPoint(item) => ChannelValue::SetPoint(item, value.into()),
            Channel::HeatingDemand(item) => ChannelValue::HeatingDemand(item, value.into()),
            Channel::ExternalAutoControl(item) => {
                ChannelValue::ExternalAutoControl(item, value.into())
            }
            Channel::Presence(item) => ChannelValue::Presence(item, value.into()),
            Channel::TotalRadiatorConsumption(item) => {
                ChannelValue::TotalRadiatorConsumption(item, value.into())
            }
            Channel::TotalWaterConsumption(item) => {
                ChannelValue::TotalWaterConsumption(item, value.into())
            }
            Channel::FanActivity(item) => ChannelValue::FanActivity(item, value.into()),
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
    use crate::{
        core::timeseries::{DataFrame, DataPoint},
        home::state::*,
    };
    use crate::t;
    use crate::core::time::DateTime;

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
