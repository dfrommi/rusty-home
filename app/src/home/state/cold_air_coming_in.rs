use crate::core::HomeApi;
use crate::core::unit::DegreeCelsius;
use crate::{core::timeseries::DataPoint, home::state::Temperature};
use r#macro::{EnumVariants, Id, mockable};

use crate::home::state::macros::result;

use super::{DataPointAccess, Opened};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum ColdAirComingIn {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

impl DataPointAccess<ColdAirComingIn> for ColdAirComingIn {
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        let outside_temp = Temperature::Outside.current_data_point(api).await?;

        if outside_temp.value > DegreeCelsius(22.0) {
            result!(false, outside_temp.timestamp, self,
                @outside_temp,
                "No cold air coming in, temperature outside is too high"
            );
        }

        let window_opened = match self {
            ColdAirComingIn::LivingRoom => Opened::LivingRoomWindowOrDoor.current_data_point(api).await,
            ColdAirComingIn::Bedroom => Opened::BedroomWindow.current_data_point(api).await,
            ColdAirComingIn::Kitchen => Opened::KitchenWindow.current_data_point(api).await,
            ColdAirComingIn::RoomOfRequirements => Opened::RoomOfRequirementsWindow.current_data_point(api).await,
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
