use api::state::Temperature;
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

impl DataPointAccess<bool> for ColdAirComingIn {
    async fn current_data_point(&self) -> anyhow::Result<DataPoint<bool>> {
        let outside_temp = Temperature::Outside.current_data_point().await?;

        if outside_temp.value > DegreeCelsius(22.0) {
            return Ok(outside_temp.map_value(|_| false));
        }

        match self {
            ColdAirComingIn::LivingRoom => {
                Opened::LivingRoomWindowOrDoor.current_data_point().await
            }
            ColdAirComingIn::Bedroom => Opened::BedroomWindow.current_data_point().await,
            ColdAirComingIn::Kitchen => Opened::KitchenWindow.current_data_point().await,
            ColdAirComingIn::RoomOfRequirements => {
                Opened::RoomOfRequirementsWindow.current_data_point().await
            }
        }
    }
}
