use api::state::{ChannelTypeInfo, Temperature};
use support::unit::DegreeCelsius;

use crate::{adapter::persistence::DataPoint, thing::state::opened::Opened};

use super::DataPointAccess;

#[derive(Debug, Clone)]
pub enum ColdAirComingIn {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

impl ChannelTypeInfo for ColdAirComingIn {
    type ValueType = bool;
}

impl<T> DataPointAccess<ColdAirComingIn> for T
where
    T: DataPointAccess<Temperature> + DataPointAccess<Opened>,
{
    async fn current_data_point(&self, item: ColdAirComingIn) -> anyhow::Result<DataPoint<bool>> {
        let outside_temp = self.current_data_point(Temperature::Outside).await?;

        if outside_temp.value > DegreeCelsius(22.0) {
            return Ok(outside_temp.map_value(|_| false));
        }

        match item {
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
        }
    }
}
