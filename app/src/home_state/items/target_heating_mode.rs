use r#macro::{EnumVariants, Id};

use crate::home_state::{
    AutomaticTemperatureIncrease, Occupancy, OpenedArea, Presence, Resident,
    calc::{DerivedStateProvider, StateCalculationContext},
};
use crate::{
    adapter::homekit::{HomekitCommand, HomekitCommandTarget, HomekitHeatingState},
    core::{
        time::{DateTime, Duration},
        timeseries::{DataFrame, DataPoint},
        unit::{DegreeCelsius, Probability, p},
    },
    home::trigger::{UserTrigger, UserTriggerId, UserTriggerTarget},
    t,
};

#[derive(Debug, Clone, PartialEq, derive_more::Display)]
pub enum HeatingMode {
    EnergySaving,
    Comfort,
    Sleep,

    Ventilation,
    PostVentilation,

    Away,

    #[display("Manual[{}]", _0)]
    Manual(DegreeCelsius, UserTriggerId),
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TargetHeatingMode {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

pub struct TargetHeatingModeStateProvider;

impl DerivedStateProvider<TargetHeatingMode, HeatingMode> for TargetHeatingModeStateProvider {
    fn calculate_current(
        &self,
        id: TargetHeatingMode,
        ctx: &StateCalculationContext,
    ) -> Option<DataPoint<HeatingMode>> {
        let occupancy_item = match id {
            TargetHeatingMode::LivingRoom => Some(Occupancy::LivingRoomCouch),
            TargetHeatingMode::RoomOfRequirements => Some(Occupancy::RoomOfRequirementsDesk),
            TargetHeatingMode::Bedroom => Some(Occupancy::BedroomBed),
            _ => None,
        };

        let occupancy_1h = occupancy_item.and_then(|item| ctx.all_since(item, t!(1 hours ago)));

        Some(calculate_heating_mode(
            !ctx.get(Presence::AtHomeDennis)? & !ctx.get(Presence::AtHomeSabine)?,
            ctx.get(id.window())?,
            ctx.get(id.temp_increase())?,
            ctx.get(Resident::AnyoneSleeping)?,
            occupancy_1h,
            self.get_user_override(id, ctx),
        ))
    }
}

impl TargetHeatingModeStateProvider {
    fn get_user_override(&self, id: TargetHeatingMode, ctx: &StateCalculationContext) -> Option<UserHeatingOverride> {
        let user_trigger = ctx.user_trigger(UserTriggerTarget::Homekit(match id {
            TargetHeatingMode::LivingRoom => HomekitCommandTarget::LivingRoomHeatingState,
            TargetHeatingMode::Bedroom => HomekitCommandTarget::BedroomHeatingState,
            TargetHeatingMode::Kitchen => HomekitCommandTarget::KitchenHeatingState,
            TargetHeatingMode::RoomOfRequirements => HomekitCommandTarget::RoomOfRequirementsHeatingState,
            TargetHeatingMode::Bathroom => HomekitCommandTarget::BathroomHeatingState,
        }))?;

        let target_temperature = match &user_trigger.trigger {
            UserTrigger::Homekit(HomekitCommand::LivingRoomHeatingState(state))
            | UserTrigger::Homekit(HomekitCommand::BedroomHeatingState(state))
            | UserTrigger::Homekit(HomekitCommand::KitchenHeatingState(state))
            | UserTrigger::Homekit(HomekitCommand::RoomOfRequirementsHeatingState(state))
            | UserTrigger::Homekit(HomekitCommand::BathroomHeatingState(state)) => match state {
                HomekitHeatingState::Off => Some(DegreeCelsius(0.0)),
                HomekitHeatingState::Heat(target_temperature) => Some(target_temperature).cloned(),
                HomekitHeatingState::Auto => None,
            },
            _ => None,
        };

        if let Some(target_temperature) = target_temperature {
            return Some(UserHeatingOverride {
                timestamp: user_trigger.timestamp,
                target_temperature,
                trigger_id: user_trigger.id.clone(),
            });
        }

        None
    }
}

impl TargetHeatingMode {
    fn window(&self) -> OpenedArea {
        match self {
            TargetHeatingMode::RoomOfRequirements => OpenedArea::RoomOfRequirementsWindow,
            TargetHeatingMode::LivingRoom => OpenedArea::LivingRoomWindowOrDoor,
            TargetHeatingMode::Bedroom | TargetHeatingMode::Bathroom => OpenedArea::BedroomWindow,
            TargetHeatingMode::Kitchen => OpenedArea::KitchenWindow,
        }
    }

    fn temp_increase(&self) -> AutomaticTemperatureIncrease {
        match self {
            TargetHeatingMode::RoomOfRequirements => AutomaticTemperatureIncrease::RoomOfRequirements,
            TargetHeatingMode::LivingRoom => AutomaticTemperatureIncrease::LivingRoom,
            TargetHeatingMode::Bedroom | TargetHeatingMode::Bathroom => AutomaticTemperatureIncrease::Bedroom,
            TargetHeatingMode::Kitchen => AutomaticTemperatureIncrease::Kitchen,
        }
    }

    pub fn post_ventilation_duration() -> Duration {
        t!(30 minutes)
    }
}

fn calculate_heating_mode(
    away: DataPoint<bool>,
    window_open: DataPoint<bool>,
    temp_increase: DataPoint<bool>,
    sleeping: DataPoint<bool>,
    occupancy_1h: Option<DataFrame<Probability>>,
    user_override: Option<UserHeatingOverride>,
) -> DataPoint<HeatingMode> {
    //away and no later override
    if away.value && user_override.clone().is_none_or(|o| o.timestamp < away.timestamp) {
        tracing::trace!("Heating in away mode as nobody is at home");
        return DataPoint::new(HeatingMode::Away, away.timestamp);
    }

    //Or cold-air coming in?
    if window_open.value && window_open.timestamp.elapsed() > t!(20 seconds) {
        tracing::trace!("Heating in ventilation mode as window is open");
        return DataPoint::new(HeatingMode::Ventilation, window_open.timestamp);
    }

    if let Some(user_override) = user_override {
        tracing::trace!(
            "Heating in manual mode as user override is active to {}°C",
            user_override.target_temperature.0
        );
        return DataPoint::new(
            HeatingMode::Manual(user_override.target_temperature, user_override.trigger_id),
            user_override.timestamp,
        );
    }

    //TODO take more factors like cold air coming in after ventilation into account
    //possible improvement: compare room temperature with setpoint and stop post-ventilation
    //mode when setpoint is reached, but make sure it won't toggle on/off.
    //Maybe use a hysteresis for that and don't enter mode unless room is below
    //default-temperature of thermostat
    if !window_open.value && window_open.timestamp.elapsed() < TargetHeatingMode::post_ventilation_duration() {
        tracing::trace!("Heating in post-ventilation mode as cold air is coming in after ventilation");
        return DataPoint::new(HeatingMode::PostVentilation, temp_increase.timestamp);
    }

    //Use negative occupancy in living room to detect sleep-mode, but only after is was
    //occupied for a while
    if sleeping.value {
        tracing::trace!("Heating in sleep-mode as Dennis is sleeping");
        return DataPoint::new(HeatingMode::Sleep, sleeping.timestamp);
    }

    //sleeping preseved until ventilation in that room
    if let Some(morning_timerange) = t!(5:30 - 12:30).active() {
        //some tampering with window, but not in morning hours
        if !morning_timerange.contains(&window_open.timestamp) {
            tracing::trace!("Heating in sleep-mode as not yet ventilated");
            return DataPoint::new(HeatingMode::Sleep, sleeping.timestamp);
        }
    }

    if let Some(occupancy_ts) = occupancy_1h {
        let threshold_high = p(0.7);
        let threshold_low = p(0.5);
        let current_occupancy = occupancy_ts.last();

        //On with hysteresis
        if current_occupancy.value >= threshold_high {
            tracing::trace!("Heating in comfort-mode as room is highly occupied");
            return current_occupancy.map_value(|_| HeatingMode::Comfort);
        } else if current_occupancy.value >= threshold_low {
            let last_outlier = occupancy_ts.latest_where(|dp| dp.value >= threshold_high || dp.value <= threshold_low);

            if let Some(last_outlier) = last_outlier
                && last_outlier.value >= threshold_high
            {
                tracing::trace!(
                    "Heating in comfort-mode as room was highly occupied recently and is now moderately occupied"
                );
                return current_occupancy.map_value(|_| HeatingMode::Comfort);
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

    DataPoint::new(HeatingMode::EnergySaving, *max_ts)
}

#[derive(Debug, Clone)]
struct UserHeatingOverride {
    timestamp: DateTime,
    target_temperature: DegreeCelsius,
    trigger_id: UserTriggerId,
}
