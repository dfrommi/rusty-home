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

        let temperature = self
            .series_since(temp_sensor, window_opened.timestamp)
            .await?;

        //wait for a mearurement. until then assume opened window still has effect
        if temperature.len() < 2 {
            return Ok(window_opened.map_value(|_| true));
        }

        let current_temperature = temperature.at(t!(now));
        let start_temperature = temperature.at(t!(5 minutes ago));
        let any_timestamp = current_temperature
            .as_ref()
            .or(start_temperature.as_ref())
            .map(|v| v.timestamp)
            .unwrap_or(window_opened.timestamp);

        match (current_temperature, start_temperature) {
            (Some(current_temperature), Some(start_temperature)) => {
                let diff = current_temperature.value - start_temperature.value;
                //temperature still increasing significantly
                Ok(current_temperature.map_value(|current| current - &diff > DegreeCelsius(0.1)))
            }
            _ => Ok(DataPoint::new(true, any_timestamp)),
        }
    }
}
