use crate::core::timeseries::DataPoint;
use crate::home::state::Temperature;
use r#macro::Id;
use support::{t, unit::DegreeCelsius};

use support::ValueObject;

use crate::home::state::macros::result;

use super::{DataPointAccess, TimeSeriesAccess, opened::Opened};

#[derive(Clone, Debug, Id)]
pub enum AutomaticTemperatureIncrease {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

impl ValueObject for AutomaticTemperatureIncrease {
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
        //TODO define heating schedule lookup and test outside > schedule + 1.0
        let outside_temp = self.current_data_point(Temperature::Outside).await?;

        if outside_temp.value > DegreeCelsius(22.0) {
            result!(false, outside_temp.timestamp, item,
                @outside_temp,
                "No automatic increase, temperature outside is too high"
            );
        }

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
        if window_opened.value {
            result!(false, window_opened.timestamp, item,
                @window_opened,
                "No automatic temperature increase, because window is open"
            );
        }

        let opened_elapsed = window_opened.timestamp.elapsed();

        if opened_elapsed > t!(30 minutes) {
            result!(false, window_opened.timestamp, item,
                @window_opened,
                "No automatic temperature increase anymore, because window is closed for more than 30 minutes"
            );
        }

        if opened_elapsed < t!(5 minutes) {
            result!(true, window_opened.timestamp, item,
                @window_opened,
                "Automatic temperature increase assumed, because window is open for less than 5 minutes"
            );
        }

        let temperature = self
            .series_since(temp_sensor, window_opened.timestamp)
            .await?;

        //wait for a measurement. until then assume opened window still has effect
        if temperature.len_non_estimated() < 2 {
            result!(true, window_opened.timestamp, item,
                @window_opened,
                "Automatic temperature increase assumed, because not enough temperature measurements exist after window was opened"
            );
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
                let significant_increase = diff >= DegreeCelsius(0.1);
                result!(significant_increase, current_temperature.timestamp, item,
                    @window_opened,
                    @current_temperature,
                    @start_temperature,
                    temperature_increase = %diff,
                    "{}",
                    if significant_increase {
                        "Automatic temperature increase active, because temperature increased by more than 0.1 degree in last 5 minutes"
                    } else {
                        "Automatic temperature increase not active, because temperature increased by less than 0.1 degree in last 5 minutes"
                    },
                );
            }
            _ => {
                //Should not happen, covered before
                result!(true, any_timestamp, item,
                    @window_opened,
                    "Automatic temperature increase assumed, because there are not enough temperature measurements"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::tests::Api;

    #[tokio::test]
    async fn no_increase_when_window_open() {
        let mut api = Api::default();
        api.opened(true, t!(5 minutes ago));
        api.current_temperature(12.0);

        assert!(!increasing(api).await);
    }

    #[tokio::test]
    async fn increasing_when_window_just_opened() {
        let mut api = Api::default();
        api.opened(false, t!(4 minutes ago));
        api.current_temperature(12.0);

        assert!(increasing(api).await);
    }

    #[tokio::test]
    async fn not_increasing_when_window_closed_for_long_time() {
        let mut api = Api::default();
        api.opened(false, t!(35 minutes ago));
        api.current_temperature(12.0);

        assert!(!increasing(api).await);
    }

    #[tokio::test]
    async fn increasing_when_not_enough_data_points() {
        let mut api = Api::default();
        api.current_temperature(12.0);
        api.opened(false, t!(8 minutes ago))
            .temperature_series(&[(19.0, t!(10 minutes ago)), (17.0, t!(6 minutes ago))]);

        assert!(increasing(api).await);
    }

    #[tokio::test]
    async fn increasing_when_temperature_difference_big() {
        let mut api = Api::default();
        api.current_temperature(12.0);
        api.opened(false, t!(15 minutes ago)).temperature_series(&[
            (17.0, t!(10 minutes ago)),
            (17.5, t!(6 minutes ago)),
            (17.9, t!(2 minutes ago)),
        ]);

        assert!(increasing(api).await);
    }

    #[tokio::test]
    async fn not_increasing_when_temperature_change_small() {
        let mut api = Api::default();
        api.current_temperature(12.0);
        api.opened(false, t!(15 minutes ago)).temperature_series(&[
            (17.0, t!(10 minutes ago)),
            (17.5, t!(6 minutes ago)),
            (17.6, t!(2 minutes ago)),
        ]);

        assert!(!increasing(api).await);
    }

    async fn increasing(api: Api) -> bool {
        api.current(AutomaticTemperatureIncrease::LivingRoom)
            .await
            .unwrap()
    }
}
