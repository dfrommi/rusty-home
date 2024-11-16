use api::state::{ChannelTypeInfo, Temperature};
use support::{t, unit::DegreeCelsius};

use support::DataPoint;

use super::{opened::Opened, DataPointAccess, TimeSeriesAccess};

#[derive(Clone, Debug)]
pub enum AutomaticTemperatureIncrease {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

impl ChannelTypeInfo for AutomaticTemperatureIncrease {
    type ValueType = bool;
}

//TODO detect active heating and summer mode
impl<T> DataPointAccess<AutomaticTemperatureIncrease> for T
where
    T: DataPointAccess<Opened> + DataPointAccess<Temperature> + TimeSeriesAccess<Temperature>,
{
    async fn current_data_point(
        &self,
        item: AutomaticTemperatureIncrease,
    ) -> anyhow::Result<DataPoint<bool>> {
        let (window, temp_sensor) = match item {
            AutomaticTemperatureIncrease::LivingRoom => {
                (Opened::LivingRoomWindowOrDoor, Temperature::LivingRoomDoor)
            }
            AutomaticTemperatureIncrease::Bedroom => {
                (Opened::BedroomWindow, Temperature::BedroomDoor)
            }
            AutomaticTemperatureIncrease::Kitchen => {
                (Opened::KitchenWindow, Temperature::KitchenOuterWall)
            }
            AutomaticTemperatureIncrease::RoomOfRequirements => (
                Opened::RoomOfRequirementsWindow,
                Temperature::RoomOfRequirementsDoor,
            ),
        };

        let window_opened = self.current_data_point(window).await?;
        let opened_elapsed = window_opened.timestamp.elapsed();

        if window_opened.value || opened_elapsed > t!(30 minutes) {
            return Ok(window_opened.map_value(|_| false));
        }

        if opened_elapsed < t!(5 minutes) {
            return Ok(window_opened.map_value(|_| true));
        }

        let temperature = self.series_since(temp_sensor, t!(5 minutes ago)).await?;

        //temperature increase settled
        Ok(temperature
            .last()
            .map_value(|current| current - &temperature.min().value > DegreeCelsius(0.1)))
    }
}
