use r#macro::{EnumVariants, Id, trace_state};

use crate::{
    core::{
        HomeApi,
        time::{DateTime, DateTimeRange, Duration},
        timeseries::{
            DataFrame, DataPoint,
            interpolate::{self, Estimatable},
        },
        unit::p,
    },
    home::state::{AutomaticTemperatureIncrease, Occupancy, OpenedArea, Presence, Resident, sampled_data_frame},
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
            Resident::AnyoneSleeping.current_data_point(api)
        )?;

        let occupancy_item = match self {
            ScheduledHeatingMode::LivingRoom => Some(Occupancy::LivingRoomCouch),
            ScheduledHeatingMode::RoomOfRequirements => Some(Occupancy::RoomOfRequirementsDesk),
            ScheduledHeatingMode::Bedroom => Some(Occupancy::BedroomBed),
            _ => None,
        };

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

        //Use negative occupancy in living room to detect sleep-mode, but only after is was
        //occupied for a while
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

        if let Some(occupancy_item) = occupancy_item {
            let threshold_high = p(0.7);
            let threshold_low = p(0.5);
            let current_occupancy = occupancy_item.current_data_point(api).await?;

            //On with hysteresis
            if current_occupancy.value >= threshold_high {
                tracing::trace!("Heating in comfort-mode as room is highly occupied");
                return Ok(current_occupancy.map_value(|_| HeatingMode::Comfort));
            } else if current_occupancy.value >= threshold_low {
                let occupancy_ts = occupancy_item
                    .get_data_frame(DateTimeRange::since(t!(1 hours ago)), api)
                    .await?;
                let last_outlier =
                    occupancy_ts.latest_where(|dp| dp.value >= threshold_high || dp.value <= threshold_low);

                if let Some(last_outlier) = last_outlier
                    && last_outlier.value >= threshold_high
                {
                    tracing::trace!(
                        "Heating in comfort-mode as room was highly occupied recently and is now moderately occupied"
                    );
                    return Ok(current_occupancy.map_value(|_| HeatingMode::Comfort));
                } else {
                    tracing::trace!(
                        "Room occupancy is moderate, but no high occupancy recently - not switching to comfort mode"
                    );
                }
            } else {
                tracing::trace!("Room occupancy is low - not switching to comfort mode");
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
