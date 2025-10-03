use r#macro::{EnumVariants, Id, mockable};

use crate::{
    core::{
        HomeApi,
        time::{DateTime, DateTimeRange},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{self, Estimatable},
        },
    },
    home::state::{AutomaticTemperatureIncrease, OpenedArea, Presence, Resident, macros::result, sampled_data_frame},
    port::{DataFrameAccess, DataPointAccess},
    t,
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, derive_more::Display)]
pub enum HeatingMode {
    EnergySaving,
    Comfort,
    Sleep,

    Ventilation,
    PostVentilation,

    Away,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum ScheduledHeatingMode {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl ScheduledHeatingMode {
    fn window(&self) -> OpenedArea {
        match self {
            ScheduledHeatingMode::RoomOfRequirements => OpenedArea::RoomOfRequirementsWindow,
            ScheduledHeatingMode::LivingRoom => OpenedArea::LivingRoomWindowOrDoor,
            ScheduledHeatingMode::Bedroom | ScheduledHeatingMode::Bathroom => OpenedArea::BedroomWindow,
            ScheduledHeatingMode::Kitchen => OpenedArea::KitchenWindow,
        }
    }

    fn temp_increase(&self) -> AutomaticTemperatureIncrease {
        match self {
            ScheduledHeatingMode::RoomOfRequirements => AutomaticTemperatureIncrease::RoomOfRequirements,
            ScheduledHeatingMode::LivingRoom => AutomaticTemperatureIncrease::LivingRoom,
            ScheduledHeatingMode::Bedroom | ScheduledHeatingMode::Bathroom => AutomaticTemperatureIncrease::Bedroom,
            ScheduledHeatingMode::Kitchen => AutomaticTemperatureIncrease::Kitchen,
        }
    }
}

impl DataPointAccess<ScheduledHeatingMode> for ScheduledHeatingMode {
    #[mockable]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<HeatingMode>> {
        let (window, temp_increase) = (self.window(), self.temp_increase());

        let (away, window_open, temp_increase, sleeping) = tokio::try_join!(
            Presence::away(api),
            window.current_data_point(api),
            temp_increase.current_data_point(api),
            Resident::DennisSleeping.current_data_point(api)
        )?;

        if away.value {
            result!(HeatingMode::Away, away.timestamp, self,
                @away, @window_open, @temp_increase, @sleeping,
                "Heating in away mode as nobody is at home"
            );
        }

        //Or cold-air coming in?
        if window_open.value {
            result!(HeatingMode::Ventilation, window_open.timestamp, self,
                @away, @window_open, @temp_increase, @sleeping,
                "Heating in ventilation mode as window is open"
            );
        }

        if temp_increase.value {
            result!(HeatingMode::PostVentilation, temp_increase.timestamp, self,
                @away, @window_open, @temp_increase, @sleeping,
                "Heating in post-ventilation mode as cold air is coming in after ventilation"
            );
        }

        //TODO more refined per room
        if sleeping.value {
            result!(HeatingMode::Sleep, sleeping.timestamp, self,
                @away, @window_open, @temp_increase, @sleeping,
                "Heating in sleep-mode as Dennis is sleeping"
            );
        }

        let max_ts = &[
            away.timestamp,
            window_open.timestamp,
            temp_increase.timestamp,
            sleeping.timestamp,
        ]
        .into_iter()
        .max()
        .unwrap_or_else(|| t!(now));

        //TODO comfort mode based on time and occupancy

        result!(HeatingMode::EnergySaving, *max_ts , self,
            @away, @window_open, @temp_increase, @sleeping,
            "Heating in energy-saving-mode (fallback) as no higher-prio rule applied"
        );
    }
}

impl Estimatable for ScheduledHeatingMode {
    fn interpolate(&self, at: DateTime, df: &DataFrame<HeatingMode>) -> Option<HeatingMode> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataFrameAccess<ScheduledHeatingMode> for ScheduledHeatingMode {
    #[mockable]
    async fn get_data_frame(&self, range: DateTimeRange, api: &HomeApi) -> anyhow::Result<DataFrame<HeatingMode>> {
        sampled_data_frame(self, range, t!(30 seconds), api).await
    }
}

impl From<&HeatingMode> for f64 {
    fn from(value: &HeatingMode) -> Self {
        match value {
            HeatingMode::Sleep => 10.0,
            HeatingMode::EnergySaving => 11.0,
            HeatingMode::Comfort => 12.0,
            HeatingMode::Ventilation => 1.0,
            HeatingMode::PostVentilation => 2.0,
            HeatingMode::Away => -1.0,
        }
    }
}

impl From<f64> for HeatingMode {
    fn from(value: f64) -> Self {
        if value < 0.0 {
            HeatingMode::Away
        } else if value == 1.0 {
            HeatingMode::Ventilation
        } else if value == 2.0 {
            HeatingMode::PostVentilation
        } else if value == 10.0 {
            HeatingMode::Sleep
        } else if value == 11.0 {
            HeatingMode::EnergySaving
        } else if value == 12.0 {
            HeatingMode::Comfort
        } else {
            tracing::warn!("Trying to convert unsupported value {value} to HeatingMode. Fallback to EnergySaving");
            HeatingMode::EnergySaving
        }
    }
}
