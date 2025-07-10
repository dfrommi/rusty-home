use crate::core::ValueObject;
use crate::{core::timeseries::DataPoint, home::state::Powered};
use r#macro::{EnumVariants, Id};

use crate::home::{
    command::{Command, CommandExecution, EnergySavingDevice},
    state::macros::result,
};

use super::{CommandExecutionAccess, DataPointAccess};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum EnergySaving {
    LivingRoomTv,
}

impl<T> DataPointAccess<EnergySaving> for T
where
    T: DataPointAccess<Powered> + CommandExecutionAccess,
{
    //energy saving assumed to be reset when device is turned on. Device off means energy saving
    async fn current_data_point(
        &self,
        item: EnergySaving,
    ) -> anyhow::Result<DataPoint<<EnergySaving as ValueObject>::ValueType>> {
        let is_tv_on = match item {
            EnergySaving::LivingRoomTv => self.current_data_point(Powered::LivingRoomTv).await,
        }?;

        //if device is off, then we save energy
        if !is_tv_on.value {
            result!(true, is_tv_on.timestamp, item,
                @is_tv_on,
                "Energy saving active, because TV is off"
            );
        }

        let target = match item {
            EnergySaving::LivingRoomTv => EnergySavingDevice::LivingRoomTv,
        };

        let latest_command = self.get_latest_command(target, is_tv_on.timestamp).await?;

        match latest_command {
            Some(CommandExecution {
                command: Command::SetEnergySaving { on, .. },
                created,
                ..
            }) => {
                result!(on, created, item,
                    @is_tv_on,
                    latest_command.timestamp = %created,
                    latest_command.elapsed = %created.elapsed(),
                    "{}",
                    if on {
                        "Energy saving active, because energy saving command was received since TV was turned on"
                    } else {
                        "Energy saving not active, because no energy saving command was received since TV was turned on"
                    },
                );
            }
            _ => {
                result!(false, is_tv_on.timestamp, item,
                    @is_tv_on,
                    "Energy saving not active, because no energy saving command was received since TV was turned on"
                );
            }
        }
    }
}
