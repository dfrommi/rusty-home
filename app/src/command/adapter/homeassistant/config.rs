use crate::command::{CommandTarget, EnergySavingDevice, Fan, Notification, NotificationRecipient, PowerToggle};

use super::HaServiceTarget;

pub fn default_ha_command_config() -> Vec<(CommandTarget, HaServiceTarget)> {
    vec![
        (
            CommandTarget::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
            },
            HaServiceTarget::LightTurnOnOff("light.hue_go"),
        ),
        (
            CommandTarget::PushNotify {
                recipient: NotificationRecipient::Dennis,
                notification: Notification::WindowOpened,
            },
            HaServiceTarget::PushNotification("mobile_app_jarvis"),
        ),
        (
            CommandTarget::PushNotify {
                recipient: NotificationRecipient::Sabine,
                notification: Notification::WindowOpened,
            },
            HaServiceTarget::PushNotification("mobile_app_simi_2"),
        ),
        (
            CommandTarget::SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
            },
            HaServiceTarget::LgWebosSmartTv("media_player.lg_webos_smart_tv"),
        ),
        (
            CommandTarget::ControlFan {
                device: Fan::LivingRoomCeilingFan,
            },
            HaServiceTarget::WindcalmFanSpeed("fan.ceiling_fan_living_room"),
        ),
        (
            CommandTarget::ControlFan {
                device: Fan::BedroomCeilingFan,
            },
            HaServiceTarget::WindcalmFanSpeed("fan.ceiling_fan_bedroom"),
        ),
        (
            CommandTarget::ControlFan {
                device: Fan::BedroomDehumidifier,
            },
            HaServiceTarget::ComfeeDehumidifier {
                humidifier_id: "humidifier.dehumidifier_34e8",
                fan_id: "fan.dehumidifier_34e8_fan",
            },
        ),
    ]
}
