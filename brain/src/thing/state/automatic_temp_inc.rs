use chrono::{Duration, Utc};
use support::unit::DegreeCelsius;

use crate::adapter::persistence::DataPoint;

use super::{opened::Opened, temperature::Temperature, DataPointAccess, TimeSeriesAccess};

#[derive(Clone, Debug)]
pub enum AutomaticTemperatureIncrease {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

//TODO detect active heating and summer mode
impl DataPointAccess<bool> for AutomaticTemperatureIncrease {
    async fn current_data_point(&self) -> anyhow::Result<DataPoint<bool>> {
        let (window, temp_sensor) = match self {
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

        let window_opened = window.current_data_point().await?;
        let opened_elapsed = Utc::now() - window_opened.timestamp;

        if window_opened.value.is_opened() || opened_elapsed > Duration::minutes(30) {
            return Ok(window_opened.map_value(|_| false));
        }

        if opened_elapsed < Duration::minutes(5) {
            return Ok(window_opened.map_value(|_| true));
        }

        let temperature = temp_sensor.series_of_last(Duration::minutes(5)).await?;

        //temperature increase settled
        Ok(temperature
            .last()
            .map_value(|current| current - &temperature.min().value > DegreeCelsius(0.1)))
    }
}
