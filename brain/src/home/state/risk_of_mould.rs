use api::state::{ChannelTypeInfo, RelativeHumidity};
use support::{
    t,
    unit::{DegreeCelsius, Percent},
    DataPoint,
};

use anyhow::Result;

use super::{dewpoint::DewPoint, DataPointAccess, TimeSeriesAccess};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RiskOfMould {
    Bathroom,
}

impl ChannelTypeInfo for RiskOfMould {
    type ValueType = bool;
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
            return Ok(DataPoint {
                timestamp: humidity.timestamp,
                value: false,
            });
        }

        let this_dp = self
            .current_data_point(match item {
                RiskOfMould::Bathroom => DewPoint::BathroomShower,
            })
            .await?;

        let ref_dp = item.get_reference_dewpoint(self).await?;

        let risk = this_dp.value.0 - ref_dp.0 > 3.0;

        Ok(DataPoint {
            timestamp: this_dp.timestamp,
            value: risk, //TODO avoid jumping on and off (different
                         //thresholds
        })
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
