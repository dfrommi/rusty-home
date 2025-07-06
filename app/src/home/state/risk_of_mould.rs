use crate::core::ValueObject;
use crate::core::unit::{DegreeCelsius, Percent};
use crate::t;
use crate::{core::timeseries::DataPoint, home::state::RelativeHumidity};
use anyhow::Result;
use r#macro::Id;

use crate::home::state::macros::result;

use super::{DataPointAccess, TimeSeriesAccess, dewpoint::DewPoint};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Id)]
pub enum RiskOfMould {
    Bathroom,
}

impl ValueObject for RiskOfMould {
    type ValueType = bool;

    fn to_f64(value: &Self::ValueType) -> f64 {
        if *value { 1.0 } else { 0.0 }
    }

    fn from_f64(value: f64) -> Self::ValueType {
        value > 0.0
    }
}

impl<T> DataPointAccess<RiskOfMould> for T
where
    T: DataPointAccess<RelativeHumidity> + DataPointAccess<DewPoint> + TimeSeriesAccess<DewPoint>,
{
    async fn current_data_point(&self, item: RiskOfMould) -> Result<DataPoint<bool>> {
        let humidity = self
            .current_data_point(match item {
                RiskOfMould::Bathroom => RelativeHumidity::BathroomShower,
            })
            .await?;

        if humidity.value < Percent(70.0) {
            result!(false, humidity.timestamp, item,
                @humidity,
                "Humidity of shower-sensor is below 70%, no risk of mould"
            );
        }

        let this_dp = self
            .current_data_point(match item {
                RiskOfMould::Bathroom => DewPoint::BathroomShower,
            })
            .await?;

        let ref_dp = item.get_reference_dewpoint(self).await?;

        let risk = this_dp.value.0 - ref_dp.0 > 3.0;

        result!(risk, this_dp.timestamp, item,
            @humidity,
            dewpoint_item = %this_dp.value,
            dewpoint_reference = %ref_dp,
            "Risk of mould is {}",
            if risk { "high" } else { "low" }
        );
    }
}

impl RiskOfMould {
    async fn get_reference_dewpoint(
        &self,
        api: &impl TimeSeriesAccess<DewPoint>,
    ) -> Result<DegreeCelsius> {
        let ref_dewpoints = match self {
            RiskOfMould::Bathroom => vec![
                DewPoint::LivingRoomDoor,
                //DewPoint::KitchenOuterWall, //TODO fix data collection
                DewPoint::RoomOfRequirementDoor,
            ],
        };

        let mut ref_sum: f64 = 0.0;
        for ref_dp in &ref_dewpoints {
            let ts = api.series_since(ref_dp.clone(), t!(3 hours ago)).await?;
            ref_sum += ts.mean().0;
        }

        let ref_mean = ref_sum / ref_dewpoints.len() as f64;

        Ok(DegreeCelsius(ref_mean))
    }
}
