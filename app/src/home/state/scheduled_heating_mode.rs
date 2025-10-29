use r#macro::{EnumVariants, Id, trace_state};

use crate::{
    core::{
        HomeApi,
        time::{DateTime, DateTimeRange, Duration},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{self, Estimatable},
        },
    },
    home::state::{AutomaticTemperatureIncrease, OpenedArea, Presence, Resident, sampled_data_frame},
    port::{DataFrameAccess, DataPointAccess},
    t,
};

#[derive(Debug, Clone, Hash, Eq, PartialEq, derive_more::Display, Id, EnumVariants)]
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

    pub fn post_ventilation_duration() -> Duration {
        t!(30 minutes)
    }
}

impl DataPointAccess<HeatingMode> for ScheduledHeatingMode {
    #[trace_state]
    async fn current_data_point(&self, api: &HomeApi) -> anyhow::Result<DataPoint<HeatingMode>> {
        let (window, temp_increase) = (self.window(), self.temp_increase());

        let (away, window_open, temp_increase, sleeping) = tokio::try_join!(
            Presence::away(api),
            window.current_data_point(api),
            temp_increase.current_data_point(api),
            Resident::DennisSleeping.current_data_point(api)
        )?;

        if away.value {
            tracing::trace!("Heating in away mode as nobody is at home");
            return Ok(DataPoint::new(HeatingMode::Away, away.timestamp));
        }

        //Or cold-air coming in?
        if window_open.value {
            tracing::trace!("Heating in ventilation mode as window is open");
            return Ok(DataPoint::new(HeatingMode::Ventilation, window_open.timestamp));
        }

        //TODO take more factors like cold air coming in after ventilation into account
        //possible improvement: compare room temperature with setpoint and stop post-ventilation
        //mode when setpoint is reached, but make sure it won't toggle on/off.
        //Maybe use a hysteresis for that and don't enter mode unless room is below
        //default-temperature of thermostat
        if !window_open.value && window_open.timestamp.elapsed() < Self::post_ventilation_duration() {
            tracing::trace!("Heating in post-ventilation mode as cold air is coming in after ventilation");
            return Ok(DataPoint::new(HeatingMode::PostVentilation, temp_increase.timestamp));
        }

        //TODO more refined per room
        if sleeping.value {
            tracing::trace!("Heating in sleep-mode as Dennis is sleeping");
            return Ok(DataPoint::new(HeatingMode::Sleep, sleeping.timestamp));
        }

        //sleeping preseved until ventilation in that room
        if let Some(morning_timerange) = t!(5:30 - 12:30).active() {
            //some tampering with window, but not in morning hours
            if !morning_timerange.contains(&window_open.timestamp) {
                tracing::trace!("Heating in sleep-mode as not yet ventilated");
                return Ok(DataPoint::new(HeatingMode::Sleep, sleeping.timestamp));
            }
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

        //TODO block comfort mode based on time and occupancy in most rooms
        if self == &ScheduledHeatingMode::LivingRoom {
            //in ventilation check range
            if let Some(ventilation_range) = t!(17:30 - 19:45).active()
                && ventilation_range.contains(&window_open.timestamp)
            {
                let max_ts = t!(17:30).today().max(*max_ts);
                tracing::trace!("Heating in comfort-mode as living room was ventilated in the evening");
                return Ok(DataPoint::new(HeatingMode::Comfort, max_ts));
            }

            if t!(19:00 - 23:00).is_now() {
                let max_ts = t!(19:00).today().max(*max_ts);
                tracing::trace!("Heating in comfort-mode as it's evening time in the living room");
                return Ok(DataPoint::new(HeatingMode::Comfort, max_ts));
            }
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
        tracing::trace!("Heating in energy-saving-mode (fallback) as no higher-prio rule applied");
        Ok(DataPoint::new(HeatingMode::EnergySaving, *max_ts))
    }
}

impl Estimatable for ScheduledHeatingMode {
    fn interpolate(&self, at: DateTime, df: &DataFrame<HeatingMode>) -> Option<HeatingMode> {
        interpolate::algo::last_seen(at, df)
    }
}

impl DataFrameAccess<HeatingMode> for ScheduledHeatingMode {
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
