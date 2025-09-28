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
mod scheduled_heating_mode;
mod set_point;
mod temperature;
mod total_energy_consumption;
mod total_radiator_consumption;
mod total_water_consumption;
mod user_controlled;

use std::fmt::Debug;

pub use automatic_temp_inc::AutomaticTemperatureIncrease;
pub use cold_air_coming_in::ColdAirComingIn;
pub use current_power_usage::CurrentPowerUsage;
pub use dewpoint::DewPoint;
pub use energy_saving::EnergySaving;
pub use external_auto_control::ExternalAutoControl;
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
use crate::core::ValueObject;
use crate::core::time::DateTimeRange;
use crate::core::time::Duration;
use crate::core::timeseries::DataFrame;
use crate::core::timeseries::DataPoint;
use crate::core::unit::*;
use crate::port::{DataPointAccess, TimeSeriesAccess};
use crate::t;
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
    OpenedArea(OpenedArea, bool),
    #[persistent]
    Opened(Opened, bool),
    #[persistent]
    Powered(Powered, bool),
    #[persistent]
    Presence(Presence, bool),
    #[persistent]
    RelativeHumidity(RelativeHumidity, Percent),
    Resident(Resident, bool),
    RiskOfMould(RiskOfMould, bool),
    ScheduledHeatingMode(ScheduledHeatingMode, HeatingMode),
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

async fn sampled_data_frame<T>(
    item: &T,
    range: DateTimeRange,
    rate: Duration,
    api: &HomeApi,
) -> anyhow::Result<DataFrame<T::ValueType>>
where
    T: ValueObject + DataPointAccess<T>,
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
