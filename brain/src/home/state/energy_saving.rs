use api::{
    command::{Command, CommandExecution, EnergySavingDevice},
    state::Powered,
};
use r#macro::Id;
use support::{DataPoint, ValueObject};

use super::{CommandAccess, DataPointAccess};

#[derive(Debug, Clone, Id)]
pub enum EnergySaving {
    LivingRoomTv,
}

impl ValueObject for EnergySaving {
    type ValueType = bool;
}

impl<T> DataPointAccess<EnergySaving> for T
where
    T: CommandAccess + DataPointAccess<Powered>,
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
            return Ok(DataPoint::new(true, is_tv_on.timestamp));
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
            }) => Ok(DataPoint::new(on, created)),
            _ => Ok(DataPoint::new(false, is_tv_on.timestamp)),
        }
    }
}
