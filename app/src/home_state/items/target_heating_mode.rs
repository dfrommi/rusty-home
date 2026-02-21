use r#macro::{EnumVariants, Id};

use crate::{
    automation::{HeatingZone, Radiator, RoomWithWindow},
    home_state::{
        Occupancy, Presence, Ventilation,
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

    Away,

    #[display("Manual[{}]", _0)]
    Manual(DegreeCelsius, UserTriggerId),
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TargetHeatingMode {
    HeatingZone(HeatingZone),
}

impl TargetHeatingMode {
    pub fn from_radiator(radiator: Radiator) -> Self {
        TargetHeatingMode::HeatingZone(radiator.heating_zone())
    }
}

pub struct TargetHeatingModeStateProvider;

impl DerivedStateProvider<TargetHeatingMode, HeatingMode> for TargetHeatingModeStateProvider {
    fn calculate_current(&self, id: TargetHeatingMode, ctx: &StateCalculationContext) -> Option<HeatingMode> {
        let TargetHeatingMode::HeatingZone(heating_zone) = id;

        let occupancy_item = match id {
            TargetHeatingMode::HeatingZone(HeatingZone::LivingRoom) => Some(Occupancy::LivingRoomCouch),
            TargetHeatingMode::HeatingZone(HeatingZone::RoomOfRequirements) => Some(Occupancy::RoomOfRequirementsDesk),
            _ => None,
        };

        let occupancy_1h = occupancy_item
            .and_then(|item| ctx.all_since(item, t!(1 hours ago)))
            .unwrap_or(DataFrame::empty());

        let ventilation_item = Ventilation::Room(match heating_zone {
            HeatingZone::LivingRoom => RoomWithWindow::LivingRoom,
            HeatingZone::Bedroom | HeatingZone::Bathroom => RoomWithWindow::Bedroom,
            HeatingZone::Kitchen => RoomWithWindow::Kitchen,
            HeatingZone::RoomOfRequirements => RoomWithWindow::RoomOfRequirements,
        });

        let result = calculate_heating_mode(
            &id,
            !ctx.get(Presence::AtHomeDennis)? & !ctx.get(Presence::AtHomeSabine)?,
            ctx.get(ventilation_item)?,
            occupancy_1h,
            self.get_user_override(id, ctx),
        );

        //derive sleep mode from other rooms for kitchen and bathroom
        if result == HeatingMode::EnergySaving
            && (id == TargetHeatingMode::HeatingZone(HeatingZone::Kitchen)
                || id == TargetHeatingMode::HeatingZone(HeatingZone::Bathroom))
        {
            let bedroom_sleep = ctx
                .get(TargetHeatingMode::HeatingZone(HeatingZone::Bedroom))
                .map(|mode| mode.value == HeatingMode::Sleep)
                .unwrap_or(false);
            let livingroom_sleep = ctx
                .get(TargetHeatingMode::HeatingZone(HeatingZone::LivingRoom))
                .map(|mode| mode.value == HeatingMode::Sleep)
                .unwrap_or(false);
            let room_of_requirements_sleep = ctx
                .get(TargetHeatingMode::HeatingZone(HeatingZone::RoomOfRequirements))
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
            TargetHeatingMode::HeatingZone(HeatingZone::LivingRoom) => HomekitCommandTarget::LivingRoomHeatingState,
            TargetHeatingMode::HeatingZone(HeatingZone::Bedroom) => HomekitCommandTarget::BedroomHeatingState,
            TargetHeatingMode::HeatingZone(HeatingZone::Kitchen) => HomekitCommandTarget::KitchenHeatingState,
            TargetHeatingMode::HeatingZone(HeatingZone::RoomOfRequirements) => {
                HomekitCommandTarget::RoomOfRequirementsHeatingState
            }
            TargetHeatingMode::HeatingZone(HeatingZone::Bathroom) => HomekitCommandTarget::BathroomHeatingState,
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

fn calculate_heating_mode(
    id: &TargetHeatingMode,
    away: DataPoint<bool>,
    ventilation: DataPoint<bool>,
    occupancy_1h: DataFrame<Probability>,
    user_override: Option<UserHeatingOverride>,
) -> HeatingMode {
    //away and no later override
    if away.value && user_override.clone().is_none_or(|o| o.timestamp < away.timestamp) {
        tracing::trace!("Heating in away mode as nobody is at home");
        return HeatingMode::Away;
    }

    //Or cold-air coming in?
    if ventilation.value {
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

    //sleeping preserved until ventilation in that room
    if let Some(morning_timerange) = t!(5:20 - 12:30).active() {
        //some tampering with window, but not in morning hours
        if !morning_timerange.contains(&ventilation.timestamp) {
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
    if (id == &TargetHeatingMode::HeatingZone(HeatingZone::Bedroom) && t!(21:00 - 5:30).is_now())
        || (id == &TargetHeatingMode::HeatingZone(HeatingZone::LivingRoom) && t!(22:00 - 5:30).is_now())
        || (id == &TargetHeatingMode::HeatingZone(HeatingZone::RoomOfRequirements) && t!(20:00 - 5:30).is_now())
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
