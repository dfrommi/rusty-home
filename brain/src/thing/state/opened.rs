use support::unit::OpenedState;

use crate::{adapter::persistence::DataPoint, home_api};

use super::DataPointAccess;

#[derive(Debug, Clone)]
pub enum Opened {
    LivingRoomWindowOrDoor,
    BedroomWindow,
    KitchenWindow,
    RoomOfRequirementsWindow,
}

impl DataPointAccess<OpenedState> for Opened {
    async fn current_data_point(&self) -> anyhow::Result<DataPoint<OpenedState>> {
        let api = home_api();
        match self {
            Opened::LivingRoomWindowOrDoor => {
                any_of(vec![
                    api::state::Opened::LivingRoomWindowLeft,
                    api::state::Opened::LivingRoomWindowRight,
                    api::state::Opened::LivingRoomWindowSide,
                    api::state::Opened::LivingRoomBalconyDoor,
                ])
                .await
            }
            Opened::BedroomWindow => api.get_latest(&api::state::Opened::BedroomWindow).await,
            Opened::KitchenWindow => api.get_latest(&api::state::Opened::KitchenWindow).await,
            Opened::RoomOfRequirementsWindow => {
                any_of(vec![
                    api::state::Opened::RoomOfRequirementsWindowLeft,
                    api::state::Opened::RoomOfRequirementsWindowRight,
                    api::state::Opened::RoomOfRequirementsWindowSide,
                ])
                .await
            }
        }
    }
}

async fn any_of(opened_states: Vec<api::state::Opened>) -> anyhow::Result<DataPoint<OpenedState>> {
    let api = home_api();
    let futures: Vec<_> = opened_states.iter().map(|o| api.get_latest(o)).collect();
    let res: Result<Vec<_>, _> = futures::future::try_join_all(futures).await;

    match res {
        Ok(values) => {
            let timestamp = values.iter().map(|v| v.timestamp).max().unwrap_or_default();
            let value = OpenedState::any(&values.iter().map(|v| v.value).collect::<Vec<_>>());

            Ok(DataPoint { value, timestamp })
        }
        Err(e) => Err(e),
    }
}
