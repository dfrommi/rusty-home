use crate::core::time::DateTimeRange;
use crate::core::timeseries::DataFrame;
use crate::core::timeseries::interpolate::{self, Estimatable};
use crate::core::unit::DegreeCelsius;
use crate::port::DataFrameAccess;
use crate::t;
use crate::{core::timeseries::DataPoint, home::state::Temperature};
use r#macro::{EnumVariants, Id, mockable};

use crate::home::state::macros::result;

use super::{DataPointAccess, OpenedArea, sampled_data_frame};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum ColdAirComingIn {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

impl DataPointAccess<ColdAirComingIn> for ColdAirComingIn {
    #[mockable]
    async fn current_data_point(&self, api: &crate::core::HomeApi) -> anyhow::Result<DataPoint<bool>> {
        let outside_temp = Temperature::Outside.current_data_point(api).await?;

        if outside_temp.value > DegreeCelsius(22.0) {
            result!(false, outside_temp.timestamp, self,
                @outside_temp,
                "No cold air coming in, temperature outside is too high"
            );
        }

        let window_opened = match self {
            ColdAirComingIn::LivingRoom => OpenedArea::LivingRoomWindowOrDoor.current_data_point(api).await,
            ColdAirComingIn::Bedroom => OpenedArea::BedroomWindow.current_data_point(api).await,
            ColdAirComingIn::Kitchen => OpenedArea::KitchenWindow.current_data_point(api).await,
            ColdAirComingIn::RoomOfRequirements => OpenedArea::RoomOfRequirementsWindow.current_data_point(api).await,
        }?;

        result!(window_opened.value, window_opened.timestamp, self,
            @outside_temp,
            @window_opened,
            "{}",
            if window_opened.value {
                "Cold air coming in, because it's cold outside and window is open"
            } else {
                "No cold air coming in, because window is closed"
            },
        );
    }
}

impl Estimatable for ColdAirComingIn {
    fn interpolate(&self, at: crate::core::time::DateTime, df: &DataFrame<Self::ValueType>) -> Option<Self::ValueType> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataFrameAccess<ColdAirComingIn> for ColdAirComingIn {
    #[mockable]
    async fn get_data_frame(
        &self,
        range: DateTimeRange,
        api: &crate::core::HomeApi,
    ) -> anyhow::Result<DataFrame<bool>> {
        sampled_data_frame(self, range, t!(30 seconds), api).await
    }
}
