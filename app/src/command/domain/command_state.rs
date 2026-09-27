use crate::command::HeatingTargetState;
use crate::core::range::Range;
use crate::core::unit::{DegreeCelsius, FanAirflow, Percent};
use crate::home_state::{FanActivity, HeatingDemandLimit, PowerAvailable, SetPoint, StateSnapshot};
use anyhow::Result;

use crate::home_state::EnergySaving;

use super::{Command, EnergySavingDevice, Fan, NotificationDestination, NotificationOperation, PowerToggle, Radiator};

impl Command {
    pub fn is_reflected_in_state(&self, snapshot: &StateSnapshot) -> Result<Option<bool>> {
        match self {
            Command::SetPower { device, power_on } => {
                is_set_power_reflected_in_state(device, *power_on, snapshot).map(Some)
            }
            Command::SetHeating {
                device,
                target_state: HeatingTargetState::Off,
            } => is_set_heating_reflected_in_state(
                device,
                &Range::new(DegreeCelsius(0.0), DegreeCelsius(0.0)),
                &Range::new(Percent(0.0), Percent(0.0)),
                snapshot,
            )
            .map(Some),
            Command::SetHeating {
                device,
                target_state:
                    HeatingTargetState::Heat {
                        target_temperature,
                        demand_limit,
                    },
            } => is_set_heating_reflected_in_state(device, target_temperature, demand_limit, snapshot).map(Some),
            Command::Notify { target, operation, .. } => {
                is_notification_reflected_in_state(target, *operation, snapshot)
            }
            Command::SetEnergySaving { device, on } => {
                is_set_energy_saving_reflected_in_state(device, *on, snapshot).map(Some)
            }
            Command::ControlFan { device, speed } => {
                is_fan_control_reflected_in_state(device, speed, snapshot).map(Some)
            }
            Command::OpenDoor { .. } => {
                // One-shot action with no observable persistent state.
                Ok(None)
            }
        }
    }
}

fn is_set_heating_reflected_in_state(
    device: &Radiator,
    target_temperature: &Range<DegreeCelsius>,
    demand_limit: &Range<Percent>,
    snapshot: &StateSnapshot,
) -> Result<bool> {
    let current_setpoint_range = snapshot.try_get(SetPoint::Current(*device))?.value;
    let current_demand_limit = snapshot.try_get(HeatingDemandLimit::Current(*device))?.value;
    let is_reflected = current_setpoint_range == *target_temperature && current_demand_limit == *demand_limit;

    tracing::debug!(
        "Checking if SetHeating command for device {:?} is reflected in state: current setpoint range: {:?}, target setpoint range: {:?}, current demand limit: {:?}, target demand limit: {:?}, is_reflected: {}",
        device,
        current_setpoint_range,
        target_temperature,
        current_demand_limit,
        demand_limit,
        is_reflected
    );

    Ok(is_reflected)
}

fn is_set_power_reflected_in_state(device: &PowerToggle, power_on: bool, snapshot: &StateSnapshot) -> Result<bool> {
    let powered_item = match device {
        PowerToggle::Dehumidifier => PowerAvailable::Dehumidifier,
        PowerToggle::InfraredHeater => PowerAvailable::InfraredHeater,
        PowerToggle::LivingRoomTvAmbilight => PowerAvailable::LivingRoomTvAmbilight,
    };

    let powered = snapshot.try_get(powered_item)?.value;
    Ok(powered == power_on)
}

fn is_notification_reflected_in_state(
    target: &NotificationDestination,
    operation: NotificationOperation,
    snapshot: &StateSnapshot,
) -> Result<Option<bool>> {
    match target {
        NotificationDestination::Phone { .. } => Ok(None),
        NotificationDestination::Light { device } => {
            let state_item = match device {
                super::NotificationLight::LivingRoom => PowerAvailable::LivingRoomNotificationLight,
            };
            let powered = snapshot.try_get(state_item)?.value;
            Ok(Some(notification_light_is_reflected(operation, powered)))
        }
    }
}

fn notification_light_is_reflected(operation: NotificationOperation, powered: bool) -> bool {
    powered == (operation == NotificationOperation::Show)
}

fn is_set_energy_saving_reflected_in_state(
    device: &EnergySavingDevice,
    on: bool,
    snapshot: &StateSnapshot,
) -> Result<bool> {
    let state_device = match device {
        EnergySavingDevice::LivingRoomTv => EnergySaving::LivingRoomTv,
    };
    Ok(snapshot.try_get(state_device)?.value == on)
}

fn is_fan_control_reflected_in_state(device: &Fan, airflow: &FanAirflow, snapshot: &StateSnapshot) -> Result<bool> {
    let state_device = match device {
        Fan::BedroomDehumidifier => FanActivity::BedroomDehumidifier,
        Fan::LivingRoomAirPurifier => FanActivity::LivingRoomAirPurifier,
    };

    let current_flow = snapshot.try_get(state_device)?.value;

    Ok(current_flow == *airflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_notification_reflection_is_unknown() {
        let command = Command::Notify {
            notification: super::super::NotificationKind::WindowOpened,
            target: NotificationDestination::Phone {
                recipient: super::super::NotificationRecipient::Dennis,
            },
            operation: NotificationOperation::Show,
        };

        assert_eq!(command.is_reflected_in_state(&StateSnapshot::default()).unwrap(), None);
    }

    #[test]
    fn light_notification_reflection_checks_power_state() {
        assert!(notification_light_is_reflected(NotificationOperation::Show, true));
        assert!(!notification_light_is_reflected(NotificationOperation::Show, false));
        assert!(notification_light_is_reflected(NotificationOperation::Dismiss, false));
        assert!(!notification_light_is_reflected(NotificationOperation::Dismiss, true));
    }

    #[test]
    fn light_notification_requires_observed_power_state() {
        let command = Command::Notify {
            notification: super::super::NotificationKind::WindowOpened,
            target: NotificationDestination::Light {
                device: super::super::NotificationLight::LivingRoom,
            },
            operation: NotificationOperation::Show,
        };

        assert!(command.is_reflected_in_state(&StateSnapshot::default()).is_err());
    }
}
