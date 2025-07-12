use crate::core::HomeApi;
use crate::core::timeseries::DataPoint;
use crate::core::unit::DegreeCelsius;
use crate::home::state::Temperature;
use crate::t;
use r#macro::{EnumVariants, Id};

use crate::home::state::macros::result;

use super::{DataPointAccess, TimeSeriesAccess, opened::Opened};

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum AutomaticTemperatureIncrease {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
}

//TODO detect active heating and summer mode
impl DataPointAccess<AutomaticTemperatureIncrease> for AutomaticTemperatureIncrease {
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<bool>> {
        //TODO define heating schedule lookup and test outside > schedule + 1.0
        let outside_temp = Temperature::Outside.current_data_point(api).await?;

        if outside_temp.value > DegreeCelsius(22.0) {
            result!(false, outside_temp.timestamp, self,
                @outside_temp,
                "No automatic increase, temperature outside is too high"
            );
        }

        let (window, temp_sensor) = match self {
            AutomaticTemperatureIncrease::LivingRoom => (Opened::LivingRoomWindowOrDoor, Temperature::LivingRoomDoor),
            AutomaticTemperatureIncrease::Bedroom => (Opened::BedroomWindow, Temperature::BedroomDoor),
            AutomaticTemperatureIncrease::Kitchen => (Opened::KitchenWindow, Temperature::KitchenOuterWall),
            AutomaticTemperatureIncrease::RoomOfRequirements => {
                (Opened::RoomOfRequirementsWindow, Temperature::RoomOfRequirementsDoor)
            }
        };

        let window_opened = window.current_data_point(api).await?;
        if window_opened.value {
            result!(false, window_opened.timestamp, self,
                @window_opened,
                "No automatic temperature increase, because window is open"
            );
        }

        let opened_elapsed = window_opened.timestamp.elapsed();

        if opened_elapsed > t!(30 minutes) {
            result!(false, window_opened.timestamp, self,
                @window_opened,
                "No automatic temperature increase anymore, because window is closed for more than 30 minutes"
            );
        }

        if opened_elapsed < t!(5 minutes) {
            result!(true, window_opened.timestamp, self,
                @window_opened,
                "Automatic temperature increase assumed, because window is open for less than 5 minutes"
            );
        }

        let temperature = temp_sensor.series_since(window_opened.timestamp, api).await?;

        //wait for a measurement. until then assume opened window still has effect
        if temperature.len_non_estimated() < 2 {
            result!(true, window_opened.timestamp, self,
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
                result!(significant_increase, current_temperature.timestamp, self,
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
                result!(true, any_timestamp, self,
                    @window_opened,
                    "Automatic temperature increase assumed, because there are not enough temperature measurements"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{core::HomeApi, home::state::opened::Opened};

    use super::*;

    #[tokio::test]
    async fn no_increase_when_window_open() {
        let mut api = api_with_defaults();
        api.with_fixed_current_dp(Opened::LivingRoomWindowOrDoor, true, t!(5 minutes ago));

        assert!(!increasing(api).await);
    }

    #[tokio::test]
    async fn increasing_when_window_just_opened() {
        let mut api = api_with_defaults();
        api.with_fixed_current_dp(Opened::LivingRoomWindowOrDoor, false, t!(4 minutes ago));

        assert!(increasing(api).await);
    }

    #[tokio::test]
    async fn not_increasing_when_window_closed_for_long_time() {
        let mut api = api_with_defaults();
        api.with_fixed_current_dp(Opened::LivingRoomWindowOrDoor, false, t!(35 minutes ago));

        assert!(!increasing(api).await);
    }

    #[tokio::test]
    async fn increasing_when_not_enough_data_points() {
        let mut api = api_with_defaults();
        api.with_fixed_current_dp(Opened::LivingRoomWindowOrDoor, false, t!(8 minutes ago));
        api.with_fixed_ts(
            Temperature::LivingRoomDoor,
            &[(19.0, t!(10 minutes ago)), (17.0, t!(6 minutes ago))],
        );

        assert!(increasing(api).await);
    }

    #[tokio::test]
    async fn increasing_when_temperature_difference_big() {
        let mut api = api_with_defaults();
        api.with_fixed_current_dp(Opened::LivingRoomWindowOrDoor, false, t!(15 minutes ago));
        api.with_fixed_ts(
            Temperature::LivingRoomDoor,
            &[
                (17.0, t!(10 minutes ago)),
                (17.5, t!(6 minutes ago)),
                (17.9, t!(2 minutes ago)),
            ],
        );

        assert!(increasing(api).await);
    }

    #[tokio::test]
    async fn not_increasing_when_temperature_change_small() {
        let mut api = api_with_defaults();
        api.with_fixed_current_dp(Opened::LivingRoomWindowOrDoor, false, t!(15 minutes ago));
        api.with_fixed_ts(
            Temperature::LivingRoomDoor,
            &[
                (17.0, t!(10 minutes ago)),
                (17.5, t!(6 minutes ago)),
                (17.6, t!(2 minutes ago)),
            ],
        );

        assert!(!increasing(api).await);
    }

    fn api_with_defaults() -> HomeApi {
        let mut api = HomeApi::for_testing();
        api.with_fixed_current_dp(Temperature::Outside, 18.0, t!(now));
        api.with_fixed_current_dp(Opened::LivingRoomWindowOrDoor, false, t!(15 minutes ago));
        api.with_fixed_ts(
            Temperature::LivingRoomDoor,
            &[
                (17.0, t!(10 minutes ago)),
                (17.5, t!(6 minutes ago)),
                (17.6, t!(2 minutes ago)),
            ],
        );
        api
    }

    async fn increasing(api: HomeApi) -> bool {
        AutomaticTemperatureIncrease::LivingRoom.current(&api).await.unwrap()
    }
}
