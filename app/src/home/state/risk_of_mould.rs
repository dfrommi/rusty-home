use crate::core::HomeApi;
use crate::core::time::DateTimeRange;
use crate::core::timeseries::DataFrame;
use crate::core::timeseries::interpolate::{self, Estimatable};
use crate::core::unit::{DegreeCelsius, Percent};
use crate::port::DataFrameAccess;
use crate::t;
use crate::{core::timeseries::DataPoint, home::state::RelativeHumidity};
use anyhow::Result;
use r#macro::{EnumVariants, Id, mockable};

use crate::home::state::macros::result;

use super::{DataPointAccess, TimeSeriesAccess, dewpoint::DewPoint, sampled_data_frame};

#[derive(Clone, Debug, PartialEq, Eq, Hash, EnumVariants, Id)]
pub enum RiskOfMould {
    Bathroom,
}

impl DataPointAccess<RiskOfMould> for RiskOfMould {
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> Result<DataPoint<bool>> {
        let humidity = match self {
            RiskOfMould::Bathroom => RelativeHumidity::BathroomShower,
        }
        .current_data_point(api)
        .await?;

        if humidity.value < Percent(70.0) {
            result!(false, humidity.timestamp, self,
                @humidity,
                "Humidity of shower-sensor is below 70%, no risk of mould"
            );
        }

        let this_dp = match self {
            RiskOfMould::Bathroom => DewPoint::BathroomShower,
        }
        .current_data_point(api)
        .await?;

        let ref_dp = self.get_reference_dewpoint(api).await?;

        let risk = this_dp.value.0 - ref_dp.0 > 3.0;

        result!(risk, this_dp.timestamp, self,
            @humidity,
            dewpoint_item = %this_dp.value,
            dewpoint_reference = %ref_dp,
            "Risk of mould is {}",
            if risk { "high" } else { "low" }
        );
    }
}

impl RiskOfMould {
    async fn get_reference_dewpoint(&self, api: &HomeApi) -> Result<DegreeCelsius> {
        let ref_dewpoints = match self {
            RiskOfMould::Bathroom => vec![
                DewPoint::LivingRoomDoor,
                //DewPoint::KitchenOuterWall, //TODO fix data collection
                DewPoint::RoomOfRequirementDoor,
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

impl DataFrameAccess<RiskOfMould> for RiskOfMould {
    #[mockable]
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<bool>> {
        sampled_data_frame(self, range, t!(30 seconds), api).await
    }
}
