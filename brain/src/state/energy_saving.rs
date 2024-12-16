use api::{
    command::{CommandExecution, EnergySavingDevice, SetEnergySaving},
    state::{ChannelTypeInfo, Powered},
};
use support::DataPoint;

use super::{CommandAccess, DataPointAccess};

#[derive(Debug, Clone)]
pub enum EnergySaving {
    LivingRoomTv,
}

impl ChannelTypeInfo for EnergySaving {
    type ValueType = bool;
}

impl<T> DataPointAccess<EnergySaving> for T
where
    T: CommandAccess<EnergySavingDevice> + DataPointAccess<Powered>,
{
    //energy saving assumed to be reset when device is turned on. Device off means energy saving
    async fn current_data_point(
        &self,
        item: EnergySaving,
    ) -> anyhow::Result<DataPoint<<EnergySaving as ChannelTypeInfo>::ValueType>> {
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
                command: SetEnergySaving { on, .. },
                created,
                ..
            }) => Ok(DataPoint::new(on, created)),
            _ => Ok(DataPoint::new(false, is_tv_on.timestamp)),
        }
    }
}
