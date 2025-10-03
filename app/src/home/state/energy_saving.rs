use crate::core::HomeApi;
use crate::core::ValueObject;
use crate::core::time::DateTimeRange;
use crate::core::timeseries::DataFrame;
use crate::core::timeseries::interpolate::{self, Estimatable};
use crate::home::command::CommandTarget;
use crate::port::DataFrameAccess;
use crate::t;
use crate::{core::timeseries::DataPoint, home::state::Powered};
use r#macro::{EnumVariants, Id, mockable};

use crate::home::{
    command::{Command, CommandExecution, EnergySavingDevice},
    state::macros::result,
};

use super::{DataPointAccess, sampled_data_frame};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum EnergySaving {
    LivingRoomTv,
}

impl DataPointAccess<EnergySaving> for EnergySaving {
    //energy saving assumed to be reset when device is turned on. Device off means energy saving
    #[mockable]
    async fn current_data_point(
        &self,
        api: &HomeApi,
    ) -> anyhow::Result<DataPoint<<EnergySaving as ValueObject>::ValueType>> {
        let is_tv_on = match self {
            EnergySaving::LivingRoomTv => Powered::LivingRoomTv.current_data_point(api).await,
        }?;

        //if device is off, then we save energy
        if !is_tv_on.value {
            result!(true, is_tv_on.timestamp, self,
                @is_tv_on,
                "Energy saving active, because TV is off"
            );
        }

        let target = match self {
            EnergySaving::LivingRoomTv => CommandTarget::SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
            },
        };

        let latest_command = api.get_latest_command(target, is_tv_on.timestamp).await?;

        match latest_command {
            Some(CommandExecution {
                command: Command::SetEnergySaving { on, .. },
                created,
                ..
            }) => {
                result!(on, created, self,
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
                result!(false, is_tv_on.timestamp, self,
                    @is_tv_on,
                    "Energy saving not active, because no energy saving command was received since TV was turned on"
                );
            }
        }
    }
}

impl Estimatable for EnergySaving {
    fn interpolate(&self, at: crate::core::time::DateTime, df: &DataFrame<Self::ValueType>) -> Option<Self::ValueType> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataFrameAccess<EnergySaving> for EnergySaving {
    #[mockable]
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<bool>> {
        sampled_data_frame(self, range, t!(30 seconds), api).await
    }
}
