use api::state::{DataPoint, RelativeHumidity};
use chrono::Duration;
use support::unit::{DegreeCelsius, Percent};

use crate::error::Result;

use super::{dewpoint::DewPoint, DataPointAccess, TimeSeriesAccess};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RiskOfMould {
    Bathroom,
}

impl DataPointAccess<bool> for RiskOfMould {
    fn current_data_point(&self) -> Result<DataPoint<bool>> {
        let humidity = match self {
            RiskOfMould::Bathroom => RelativeHumidity::BathroomShower,
        }
        .current_data_point()?;

        if humidity.value < Percent(60.0) {
            return Ok(DataPoint {
                timestamp: humidity.timestamp,
                value: false,
            });
        }

        let this_dp = match self {
            RiskOfMould::Bathroom => DewPoint::BathroomShower,
        }
        .current_data_point()?;

        let ref_dp = self.get_reference_dewpoint()?;

        let risk = this_dp.value.0 - ref_dp.0 > 3.0;

        Ok(DataPoint {
            timestamp: this_dp.timestamp,
            value: risk, //TODO avoid jumping on and off (different
                         //thresholds
        })
    }
}

impl RiskOfMould {
    fn get_reference_dewpoint(&self) -> Result<DegreeCelsius> {
        let ref_dewpoints = match self {
            RiskOfMould::Bathroom => vec![
                DewPoint::LivingRoomDoor,
                //DewPoint::KitchenOuterWall, //TODO fix data collection
                DewPoint::RoomOfRequirementDoor,
            ],
        };

        let mut ref_sum: f64 = 0.0;
        for ref_dp in &ref_dewpoints {
            let ts = ref_dp.series_of_last(Duration::hours(3))?;
            ref_sum += ts.mean().0;
        }

        let ref_mean = ref_sum / ref_dewpoints.len() as f64;

        Ok(DegreeCelsius(ref_mean))
    }
}
