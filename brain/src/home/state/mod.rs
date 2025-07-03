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
mod energy_saving;
mod heating_demand;
mod presence;
mod relative_humidity;
mod set_point;
mod temperature;
mod total_energy_consumption;
mod user_controlled;

pub use api::state::Powered;
pub use automatic_temp_inc::AutomaticTemperatureIncrease;
pub use cold_air_coming_in::ColdAirComingIn;
pub use dewpoint::DewPoint;
pub use energy_saving::EnergySaving;
pub use opened::Opened;
pub use resident::Resident;
pub use risk_of_mould::RiskOfMould;
pub use user_controlled::UserControlled;

use crate::port::*;

mod macros {
    macro_rules! result {
        ($result:expr, $timestamp:expr, $item:expr, { $(,)* $($dps:ident),* }, @$dp:ident, $($arg:tt)+ ) => {
            result!($result, $timestamp, $item, { $($dps),*, $dp }, $($arg)+)
        };

        ($result:expr, $timestamp:expr, $item:expr, { $(,)* $($dps:ident),* }, $($arg:tt)+ ) => {
            let result = support::DataPoint::new($result, $timestamp);

            tracing::trace!(
                timestamp = %support::t!(now),
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
    use api::state::*;
    use support::{DataFrame, DataPoint, t, time::DateTime, unit::*};

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
            range: support::time::DateTimeRange,
        ) -> anyhow::Result<TimeSeries<Temperature>> {
            TimeSeries::new(item, &self.temperature_df.clone().unwrap(), range)
        }
    }
}
