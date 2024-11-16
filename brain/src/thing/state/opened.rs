use api::state::ChannelTypeInfo;
use support::{t, DataPoint};

use super::DataPointAccess;

#[derive(Debug, Clone)]
pub enum Opened {
    LivingRoomWindowOrDoor,
    BedroomWindow,
    KitchenWindow,
    RoomOfRequirementsWindow,
}

impl ChannelTypeInfo for Opened {
    type ValueType = bool;
}

impl<T> DataPointAccess<Opened> for T
where
    T: DataPointAccess<api::state::Opened>,
{
    async fn current_data_point(&self, item: Opened) -> anyhow::Result<DataPoint<bool>> {
        match item {
            Opened::LivingRoomWindowOrDoor => {
                any_of(
                    self,
                    vec![
                        api::state::Opened::LivingRoomWindowLeft,
                        api::state::Opened::LivingRoomWindowRight,
                        api::state::Opened::LivingRoomWindowSide,
                        api::state::Opened::LivingRoomBalconyDoor,
                    ],
                )
                .await
            }
            Opened::BedroomWindow => {
                self.current_data_point(api::state::Opened::BedroomWindow)
                    .await
            }
            Opened::KitchenWindow => {
                self.current_data_point(api::state::Opened::KitchenWindow)
                    .await
            }
            Opened::RoomOfRequirementsWindow => {
                any_of(
                    self,
                    vec![
                        api::state::Opened::RoomOfRequirementsWindowLeft,
                        api::state::Opened::RoomOfRequirementsWindowRight,
                        api::state::Opened::RoomOfRequirementsWindowSide,
                    ],
                )
                .await
            }
        }
    }
}

async fn any_of(
    api: &impl DataPointAccess<api::state::Opened>,
    opened_states: Vec<api::state::Opened>,
) -> anyhow::Result<DataPoint<bool>> {
    let futures: Vec<_> = opened_states
        .into_iter()
        .map(|o| api.current_data_point(o))
        .collect();
    let res: Result<Vec<_>, _> = futures::future::try_join_all(futures).await;

    match res {
        Ok(values) => {
            let timestamp = values.iter().map(|v| v.timestamp).max().unwrap_or(t!(now));
            let value = values.iter().any(|v| v.value);

            Ok(DataPoint { value, timestamp })
        }
        Err(e) => Err(e),
    }
}
