use crate::{
    command::{
        Command, EnergySavingDevice, Fan, Lock, Notification, NotificationAction, NotificationRecipient, PowerToggle,
        adapter::{
            HomeAssistantCommandExecutor, LgTvCommandExecutor, NukiCommandExecutor, TasmotaCommandExecutor,
            Z2mCommandExecutor,
        },
    },
    core::domain::Radiator,
};

pub struct CommandDispatcher {
    tasmota: TasmotaCommandExecutor,
    z2m: Z2mCommandExecutor,
    lgtv: LgTvCommandExecutor,
    nuki: NukiCommandExecutor,
    homeassistant: HomeAssistantCommandExecutor,
}

impl CommandDispatcher {
    pub fn new(
        tasmota: TasmotaCommandExecutor,
        z2m: Z2mCommandExecutor,
        lgtv: LgTvCommandExecutor,
        nuki: NukiCommandExecutor,
        homeassistant: HomeAssistantCommandExecutor,
    ) -> Self {
        Self {
            tasmota,
            z2m,
            lgtv,
            nuki,
            homeassistant,
        }
    }

    pub async fn dispatch(&self, command: &Command) -> anyhow::Result<()> {
        match command {
            Command::SetPower {
                device: PowerToggle::InfraredHeater,
                power_on,
            } => self.tasmota.set_power("irheater", *power_on).await,
            Command::SetPower {
                device: PowerToggle::Dehumidifier,
                power_on,
            } => self.z2m.set_power("bathroom/dehumidifier_plug", *power_on).await,
            Command::SetPower {
                device: PowerToggle::LivingRoomNotificationLight,
                power_on,
            } => self.homeassistant.set_light_power("light.hue_go", *power_on).await,
            Command::SetHeating { device, target_state } => {
                let device_id = match device {
                    Radiator::RoomOfRequirements => "room_of_requirements/radiator_thermostat_sonoff",
                    Radiator::Bathroom => "bathroom/radiator_thermostat_sonoff",
                    Radiator::LivingRoomBig => "living_room/radiator_thermostat_big_sonoff",
                    Radiator::LivingRoomSmall => "living_room/radiator_thermostat_small_sonoff",
                    Radiator::Bedroom => "bedroom/radiator_thermostat_sonoff",
                    Radiator::Kitchen => "kitchen/radiator_thermostat_sonoff",
                };
                self.z2m.set_heating(device_id, target_state.clone()).await
            }
            Command::PushNotify {
                action: NotificationAction::Notify,
                notification: Notification::WindowOpened,
                recipient: NotificationRecipient::Dennis,
            } => self.homeassistant.notify_window_opened("mobile_app_jarvis").await,
            Command::PushNotify {
                action: NotificationAction::Notify,
                notification: Notification::WindowOpened,
                recipient: NotificationRecipient::Sabine,
            } => self.homeassistant.notify_window_opened("mobile_app_simi_2").await,
            Command::PushNotify {
                action: NotificationAction::Dismiss,
                notification: Notification::WindowOpened,
                recipient: NotificationRecipient::Dennis,
            } => {
                self.homeassistant
                    .dismiss_window_opened_notification("mobile_app_jarvis")
                    .await
            }
            Command::PushNotify {
                action: NotificationAction::Dismiss,
                notification: Notification::WindowOpened,
                recipient: NotificationRecipient::Sabine,
            } => {
                self.homeassistant
                    .dismiss_window_opened_notification("mobile_app_simi_2")
                    .await
            }
            Command::SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
                on,
            } => self.lgtv.set_energy_saving(*on).await,
            Command::ControlFan {
                device: Fan::BedroomDehumidifier,
                speed,
            } => {
                self.homeassistant
                    .set_comfee_fan_speed("humidifier.dehumidifier_34e8", "fan.dehumidifier_34e8_fan", speed)
                    .await
            }
            Command::ControlFan {
                device: Fan::LivingRoomAirPurifier,
                speed,
            } => {
                self.homeassistant
                    .set_philips_air_purifier_speed("fan.wohnzimmer", speed)
                    .await
            }
            Command::OpenDoor {
                device: Lock::BuildingEntrance,
            } => self.nuki.open_door("1CC90CCA").await,
        }
    }
}
