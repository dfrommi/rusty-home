use crate::core::HomeApi;
use crate::core::time::DateTimeRange;
use crate::core::timeseries::interpolate::{self, Estimatable};
use crate::core::timeseries::{DataFrame, TimeSeries};
use crate::core::unit::{DegreeCelsius, Percent};
use crate::home::state::calc::{DerivedStateProvider, StateCalculationContext};
use crate::port::DataFrameAccess;
use crate::t;
use crate::{core::timeseries::DataPoint, home::state::RelativeHumidity};
use anyhow::Result;
use r#macro::{EnumVariants, Id, trace_state};

use super::{DataPointAccess, TimeSeriesAccess, dewpoint::DewPoint, sampled_data_frame};

#[derive(Clone, Debug, PartialEq, Eq, Hash, EnumVariants, Id)]
pub enum RiskOfMould {
    Bathroom,
}

pub struct RiskOfMouldStateProvider;

impl DerivedStateProvider<RiskOfMould, bool> for RiskOfMouldStateProvider {
    fn calculate_current(&self, id: RiskOfMould, ctx: &StateCalculationContext) -> Option<DataPoint<bool>> {
        let humidity = match id {
            RiskOfMould::Bathroom => ctx.get(RelativeHumidity::BathroomShower)?,
        };

        if humidity.value < Percent(70.0) {
            tracing::trace!("Humidity of shower-sensor is below 70%, no risk of mould");
            return Some(DataPoint::new(false, humidity.timestamp));
        }

        let this_dp = match id {
            RiskOfMould::Bathroom => ctx.get(DewPoint::BathroomShower)?,
        };

        let ref_dp = Self::get_reference_dewpoint(id, ctx)?;

        let risk = this_dp.value.0 - ref_dp.0 > 3.0;

        tracing::trace!(
            "Risk is {}. Dewpoint is {}, reference dewpoint is {}, threashold is 3.0",
            if risk { "high" } else { "low" },
            this_dp.value,
            ref_dp
        );

        Some(DataPoint::new(risk, this_dp.timestamp))
    }
}

impl RiskOfMouldStateProvider {
    fn get_reference_dewpoint(id: RiskOfMould, ctx: &StateCalculationContext) -> Option<DegreeCelsius> {
        let ref_dewpoints = match id {
            RiskOfMould::Bathroom => vec![
                DewPoint::LivingRoom,
                //DewPoint::KitchenOuterWall, //TODO fix data collection
                DewPoint::RoomOfRequirement,
            ],
        };

        let mut ref_sum: f64 = 0.0;
        let ref_len = ref_dewpoints.len();
        for ref_dp in ref_dewpoints.into_iter() {
            let range = DateTimeRange::new(t!(3 hours ago), t!(now));
            let df = ctx.all_since(ref_dp.clone(), *range.start())?;
            let ts = TimeSeries::new(ref_dp.clone(), &df, range)
                .expect("Internal error: TimeSeries empty with non-enpty DataFrame of same range");
            ref_sum += ts.mean().0;
        }

        let ref_mean = ref_sum / ref_len as f64;

        Some(DegreeCelsius(ref_mean))
    }
}

impl DataPointAccess<bool> for RiskOfMould {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> Result<DataPoint<bool>> {
        let humidity = match self {
            RiskOfMould::Bathroom => RelativeHumidity::BathroomShower,
        }
        .current_data_point(api)
        .await?;

        if humidity.value < Percent(70.0) {
            tracing::trace!("Humidity of shower-sensor is below 70%, no risk of mould");
            return Ok(DataPoint::new(false, humidity.timestamp));
        }

        let this_dp = match self {
            RiskOfMould::Bathroom => DewPoint::BathroomShower,
        }
        .current_data_point(api)
        .await?;

        let ref_dp = self.get_reference_dewpoint(api).await?;

        let risk = this_dp.value.0 - ref_dp.0 > 3.0;

        tracing::trace!(
            "Risk is {}. Dewpoint is {}, reference dewpoint is {}, threashold is 3.0",
            if risk { "high" } else { "low" },
            this_dp.value,
            ref_dp
        );

        Ok(DataPoint::new(risk, this_dp.timestamp))
    }
}

impl RiskOfMould {
    async fn get_reference_dewpoint(&self, api: &HomeApi) -> Result<DegreeCelsius> {
        let ref_dewpoints = match self {
            RiskOfMould::Bathroom => vec![
                DewPoint::LivingRoom,
                //DewPoint::KitchenOuterWall, //TODO fix data collection
                DewPoint::RoomOfRequirement,
            ],
        };

        let mut ref_sum: f64 = 0.0;
        for ref_dp in &ref_dewpoints {
            let ts = ref_dp.clone().series_since(t!(3 hours ago), api).await?;
            ref_sum += ts.mean().0;
        }

        let ref_mean = ref_sum / ref_dewpoints.len() as f64;

        Ok(DegreeCelsius(ref_mean))
    }
}

impl Estimatable for RiskOfMould {
    fn interpolate(&self, at: crate::core::time::DateTime, df: &DataFrame<Self::ValueType>) -> Option<Self::ValueType> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataFrameAccess<bool> for RiskOfMould {
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<bool>> {
        sampled_data_frame(self, range, t!(30 seconds), api).await
    }
}
