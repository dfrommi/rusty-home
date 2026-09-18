use crate::command::HeatingTargetState;
use crate::core::range::Range;
use crate::core::time::Duration;
use crate::core::unit::{DegreeCelsius, FanAirflow, Percent};
use crate::home_state::{FanActivity, HeatingDemandLimit, PowerAvailable, SetPoint, StateSnapshot};
use crate::notification::NotificationClient;
use crate::t;
use anyhow::Result;

use crate::home_state::EnergySaving;

use super::{
    Command, EnergySavingDevice, Fan, Notification, NotificationAction, NotificationRecipient, PowerToggle, Radiator,
};

impl Command {
    pub async fn is_reflected_in_state(
        &self,
        snapshot: &StateSnapshot,
        notification_client: &NotificationClient,
    ) -> Result<bool> {
        match self {
            Command::SetPower { device, power_on } => is_set_power_reflected_in_state(device, *power_on, snapshot),
            Command::SetHeating {
                device,
                target_state: HeatingTargetState::Off,
            } => is_set_heating_reflected_in_state(
                device,
                &Range::new(DegreeCelsius(0.0), DegreeCelsius(0.0)),
                &Range::new(Percent(0.0), Percent(0.0)),
                snapshot,
            ),
            Command::SetHeating {
                device,
                target_state:
                    HeatingTargetState::Heat {
                        target_temperature,
                        demand_limit,
                    },
            } => is_set_heating_reflected_in_state(device, target_temperature, demand_limit, snapshot),
            Command::PushNotify {
                recipient,
                notification,
                action,
            } => Ok(is_push_notify_reflected_in_state(
                recipient,
                notification,
                action,
                notification_client,
            )),
            Command::SetEnergySaving { device, on } => is_set_energy_saving_reflected_in_state(device, *on, snapshot),
            Command::ControlFan { device, speed } => is_fan_control_reflected_in_state(device, speed, snapshot),
            Command::OpenDoor { .. } => {
                //Only a short trigger, no permanent state change
                Ok(false)
            }
        }
    }

    pub fn min_wait_duration_between_executions(&self) -> Option<Duration> {
        match self {
            Command::SetHeating { .. } => Some(t!(2 minutes)),
            Command::SetPower { .. } => Some(t!(1 minutes)),
            Command::SetEnergySaving { .. } => Some(t!(2 minutes)),
            Command::ControlFan { .. } => Some(t!(3 minutes)),
            Command::PushNotify { .. } => None,
            Command::OpenDoor { .. } => None,
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
        PowerToggle::LivingRoomNotificationLight => PowerAvailable::LivingRoomNotificationLight,
        PowerToggle::InfraredHeater => PowerAvailable::InfraredHeater,
    };

    let powered = snapshot.try_get(powered_item)?.value;
    Ok(powered == power_on)
}

fn is_push_notify_reflected_in_state(
    recipient: &NotificationRecipient,
    notification: &Notification,
    action: &NotificationAction,
    notification_client: &NotificationClient,
) -> bool {
    let is_delivered = notification_client.is_delivered(recipient, notification);

    match action {
        NotificationAction::Notify => is_delivered,
        NotificationAction::Dismiss => !is_delivered,
    }
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
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use mockito::Server;

    use super::*;
    use crate::notification::{NotificationId, NotificationModule};

    #[tokio::test]
    async fn push_notification_reflection_uses_notification_delivery_state() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/api/services/notify/mobile_app_jarvis")
            .with_status(200)
            .expect(2)
            .create_async()
            .await;
        let module = NotificationModule::new(&server.url(), "token");
        let client = module.client();
        let recipient = NotificationRecipient::Dennis;
        let notification = Notification::WindowOpened;
        let snapshot = StateSnapshot::default();
        let notify = Command::PushNotify {
            action: NotificationAction::Notify,
            notification: notification.clone(),
            recipient: recipient.clone(),
        };
        let dismiss = Command::PushNotify {
            action: NotificationAction::Dismiss,
            notification: notification.clone(),
            recipient: recipient.clone(),
        };

        assert!(!notify.is_reflected_in_state(&snapshot, &client).await.unwrap());
        client.notify(&recipient, &notification).await.unwrap();
        assert!(notify.is_reflected_in_state(&snapshot, &client).await.unwrap());
        assert!(!dismiss.is_reflected_in_state(&snapshot, &client).await.unwrap());
        client.dismiss(&recipient, NotificationId::WindowOpened).await.unwrap();
        assert!(dismiss.is_reflected_in_state(&snapshot, &client).await.unwrap());
    }
}
