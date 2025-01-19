use api::state::Temperature;
use r#macro::Id;
use support::{unit::DegreeCelsius, ValueObject};

use crate::home::state::macros::result;

use super::{DataPointAccess, Opened};
use support::DataPoint;

#[derive(Debug, Clone, Id)]
pub enum ColdAirComingIn {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

impl ValueObject for ColdAirComingIn {
    type ValueType = bool;
}

impl<T> DataPointAccess<ColdAirComingIn> for T
where
    T: DataPointAccess<Temperature> + DataPointAccess<Opened>,
{
    async fn current_data_point(&self, item: ColdAirComingIn) -> anyhow::Result<DataPoint<bool>> {
        let outside_temp = self.current_data_point(Temperature::Outside).await?;

        if outside_temp.value > DegreeCelsius(22.0) {
            result!(false, outside_temp.timestamp, item,
                @outside_temp,
                "No cold air coming in, temperature outside is too high"
            );
        }

        let window_opened = match item {
            ColdAirComingIn::LivingRoom => {
                self.current_data_point(Opened::LivingRoomWindowOrDoor)
                    .await
            }
            ColdAirComingIn::Bedroom => self.current_data_point(Opened::BedroomWindow).await,
            ColdAirComingIn::Kitchen => self.current_data_point(Opened::KitchenWindow).await,
            ColdAirComingIn::RoomOfRequirements => {
                self.current_data_point(Opened::RoomOfRequirementsWindow)
                    .await
            }
        }?;

        result!(window_opened.value, window_opened.timestamp, item,
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
