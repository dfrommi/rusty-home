use r#macro::{EnumVariants, Id};

use crate::{
    automation::Thermostat,
    home_state::{
        Occupancy, OpenedArea, Presence, Resident,
        calc::{DerivedStateProvider, StateCalculationContext},
    },
};
use crate::{
    core::{
        time::DateTime,
        timeseries::{DataFrame, DataPoint},
        unit::{DegreeCelsius, Probability, p},
    },
    frontends::homekit::{HomekitCommand, HomekitCommandTarget, HomekitHeatingState},
    t,
    trigger::{UserTrigger, UserTriggerId, UserTriggerTarget},
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

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TargetHeatingMode {
    LivingRoom,
    Bedroom,
    Kitchen,
    RoomOfRequirements,
    Bathroom,
}

impl TargetHeatingMode {
    pub fn from_thermostat(thermostat: Thermostat) -> Self {
        match thermostat {
            Thermostat::LivingRoomSmall => TargetHeatingMode::LivingRoom,
            Thermostat::LivingRoomBig => TargetHeatingMode::LivingRoom,
            Thermostat::Bedroom => TargetHeatingMode::Bedroom,
            Thermostat::Kitchen => TargetHeatingMode::Kitchen,
            Thermostat::RoomOfRequirements => TargetHeatingMode::RoomOfRequirements,
            Thermostat::Bathroom => TargetHeatingMode::Bathroom,
        }
    }
}

pub struct TargetHeatingModeStateProvider;

impl DerivedStateProvider<TargetHeatingMode, HeatingMode> for TargetHeatingModeStateProvider {
    fn calculate_current(&self, id: TargetHeatingMode, ctx: &StateCalculationContext) -> Option<HeatingMode> {
        let occupancy_item = match id {
            TargetHeatingMode::LivingRoom => Some(Occupancy::LivingRoomCouch),
            TargetHeatingMode::RoomOfRequirements => Some(Occupancy::RoomOfRequirementsDesk),
            _ => None,
        };

        let occupancy_1h = occupancy_item
            .and_then(|item| ctx.all_since(item, t!(1 hours ago)))
            .unwrap_or(DataFrame::empty());

        let result = calculate_heating_mode(
            &id,
            !ctx.get(Presence::AtHomeDennis)? & !ctx.get(Presence::AtHomeSabine)?,
            ctx.get(id.window())?,
            occupancy_1h,
            self.get_user_override(id, ctx),
        );

        //derive sleep mode from other rooms for kitchen and bathroom
        if result == HeatingMode::EnergySaving
            && (id == TargetHeatingMode::Kitchen || id == TargetHeatingMode::Bathroom)
        {
            let bedroom_sleep = ctx
                .get(TargetHeatingMode::Bedroom)
                .map(|mode| mode.value == HeatingMode::Sleep)
                .unwrap_or(false);
            let livingroom_sleep = ctx
                .get(TargetHeatingMode::LivingRoom)
                .map(|mode| mode.value == HeatingMode::Sleep)
                .unwrap_or(false);
            let room_of_requirements_sleep = ctx
                .get(TargetHeatingMode::RoomOfRequirements)
                .map(|mode| mode.value == HeatingMode::Sleep)
                .unwrap_or(false);

            if bedroom_sleep && livingroom_sleep && room_of_requirements_sleep {
                tracing::trace!("Setting heating mode to Sleep as all other rooms are in Sleep mode");
                return Some(HeatingMode::Sleep);
            }
        }

        Some(result)
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
                HomekitHeatingState::Heat(target_temperature) => Some(*target_temperature),
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
}

fn calculate_heating_mode(
    id: &TargetHeatingMode,
    away: DataPoint<bool>,
    window_open: DataPoint<bool>,
    occupancy_1h: DataFrame<Probability>,
    user_override: Option<UserHeatingOverride>,
) -> HeatingMode {
    //away and no later override
    if away.value && user_override.clone().is_none_or(|o| o.timestamp < away.timestamp) {
        tracing::trace!("Heating in away mode as nobody is at home");
        return HeatingMode::Away;
    }

    //Or cold-air coming in?
    if window_open.value && window_open.timestamp.elapsed() > t!(20 seconds) {
        tracing::trace!("Heating in ventilation mode as window is open");
        return HeatingMode::Ventilation;
    }

    if let Some(user_override) = user_override {
        if user_override.timestamp.elapsed() > t!(1 hours) {
            tracing::trace!(
                "User override expired ({} minutes) - ignoring",
                user_override.timestamp.elapsed().as_minutes()
            );
        } else {
            tracing::trace!(
                "Heating in manual mode as user override is active to {}°C",
                user_override.target_temperature.0
            );
            return HeatingMode::Manual(user_override.target_temperature, user_override.trigger_id);
        }
    }

    //TODO take more factors like cold air coming in after ventilation into account
    //possible improvement: compare room temperature with setpoint and stop post-ventilation
    //mode when setpoint is reached, but make sure it won't toggle on/off.
    //Maybe use a hysteresis for that and don't enter mode unless room is below
    //default-temperature of thermostat
    if !window_open.value && window_open.timestamp.elapsed() < t!(30 minutes) {
        tracing::trace!("Heating in post-ventilation mode as cold air is coming in after ventilation");
        return HeatingMode::PostVentilation;
    }

    //sleeping preserved until ventilation in that room
    if let Some(morning_timerange) = t!(5:30 - 12:30).active() {
        //some tampering with window, but not in morning hours
        if !morning_timerange.contains(&window_open.timestamp) {
            tracing::trace!("Heating in sleep-mode as not yet ventilated");
            return HeatingMode::Sleep;
        }
    }

    if let Some(current_occupancy) = occupancy_1h.last() {
        let threshold_high = p(0.7);
        let threshold_low = p(0.5);

        //On with hysteresis
        if current_occupancy.value >= threshold_high {
            tracing::trace!("Heating in comfort-mode as room is highly occupied");
            return HeatingMode::Comfort;
        } else if current_occupancy.value >= threshold_low {
            let last_outlier = occupancy_1h.latest_where(|dp| dp.value >= threshold_high || dp.value <= threshold_low);

            if let Some(last_outlier) = last_outlier
                && last_outlier.value >= threshold_high
            {
                tracing::trace!(
                    "Heating in comfort-mode as room was highly occupied recently and is now moderately occupied"
                );
                return HeatingMode::Comfort;
            } else {
                tracing::trace!(
                    "Room occupancy is moderate, but no high occupancy recently - not switching to comfort mode"
                );
            }
        } else {
            tracing::trace!("Room occupancy is low - not switching to comfort mode");
        }
    }

    //Starting sleep mode if no higher-prio, like comfort, applies. Overrides in-bed detection in
    //some zones
    //TODO "last ventilation of the day" concept for RoR
    if (id == &TargetHeatingMode::Bedroom && t!(22:00 - 5:00).is_now())
        || (id == &TargetHeatingMode::LivingRoom && t!(22:00 - 5:00).is_now())
        || (id == &TargetHeatingMode::RoomOfRequirements && t!(20:00 - 5:00).is_now())
    {
        tracing::trace!("Heating in sleep-mode in preparation of going to bed");
        return HeatingMode::Sleep;
    }

    tracing::trace!("Heating in energy-saving-mode (fallback) as no higher-prio rule applied");

    HeatingMode::EnergySaving
}

#[derive(Debug, Clone)]
struct UserHeatingOverride {
    timestamp: DateTime,
    target_temperature: DegreeCelsius,
    trigger_id: UserTriggerId,
}
