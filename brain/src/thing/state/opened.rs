use support::unit::OpenedState;

use crate::{adapter::persistence::DataPoint, home_api};

use super::DataPointAccess;

#[derive(Debug, Clone)]
pub enum Opened {
    LivingRoomWindow,
    BalconyDoor,
    BedroomWindow,
    KitchenWindow,
    RoomOfRequirementsWindow,
}

impl DataPointAccess<OpenedState> for Opened {
    async fn current_data_point(&self) -> anyhow::Result<DataPoint<OpenedState>> {
        let api = home_api();
        match self {
            Opened::BalconyDoor => {
                api.get_latest(&api::state::Opened::LivingRoomBalconyDoor)
                    .await
            }
            Opened::LivingRoomWindow => {
                any_of(
                    api::state::Opened::LivingRoomWindowLeft,
                    api::state::Opened::LivingRoomWindowRight,
                    api::state::Opened::LivingRoomWindowSide,
                )
                .await
            }
            Opened::BedroomWindow => api.get_latest(&api::state::Opened::BedroomWindow).await,
            Opened::KitchenWindow => api.get_latest(&api::state::Opened::KitchenWindow).await,
            Opened::RoomOfRequirementsWindow => {
                any_of(
                    api::state::Opened::RoomOfRequirementsWindowLeft,
                    api::state::Opened::RoomOfRequirementsWindowRight,
                    api::state::Opened::RoomOfRequirementsWindowSide,
                )
                .await
            }
        }
    }
}

async fn any_of(
    o1: api::state::Opened,
    o2: api::state::Opened,
    o3: api::state::Opened,
) -> anyhow::Result<DataPoint<OpenedState>> {
    let api = home_api();
    let res = tokio::try_join! {
        api.get_latest(&o1),
        api.get_latest(&o2),
        api.get_latest(&o3)
    };

    match res {
        Ok((v1, v2, v3)) => {
            //min of timestamp of l, r, s
            let timestamp = std::cmp::max(v1.timestamp, std::cmp::max(v2.timestamp, v3.timestamp));
            let value = OpenedState::any(&[v1.value, v2.value, v3.value]);

            Ok(DataPoint { value, timestamp })
        }
        Err(e) => Err(e),
    }
}
