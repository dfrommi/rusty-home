use crate::{
    command::{
        Command, ControlFan, EnergySavingDevice, Fan, Lock, NotificationLight, OpenDoor, PowerToggle, PushNotify,
        PushRecipient, SetEnergySaving, SetHeating, SetLightNotification, SetPower,
    },
    core::domain::Radiator,
};

mod homeassistant;
mod lgtv;
pub mod nuki;
mod persistence;
mod tasmota;
pub mod z2m;

pub use homeassistant::HomeAssistantCommandExecutor;
pub use lgtv::LgTvCommandExecutor;
pub use nuki::NukiCommandExecutor;
pub use persistence::{CommandExecution, CommandExecutionRepository};
pub use tasmota::TasmotaCommandExecutor;
pub use z2m::Z2mCommandExecutor;

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
            Command::SetPower(SetPower {
                device: PowerToggle::BedroomAirPurifier,
                power_on,
            }) => {
                self.homeassistant
                    .set_power("switch.zhimi_de_974125313_rmb1_on_p_2_1", *power_on)
                    .await
            }
            Command::SetPower(SetPower {
                device: PowerToggle::InfraredHeater,
                power_on,
            }) => self.tasmota.set_power("irheater", *power_on).await,
            Command::SetPower(SetPower {
                device: PowerToggle::Dehumidifier,
                power_on,
            }) => self.z2m.set_power("bathroom/dehumidifier_plug", *power_on).await,
            Command::SetPower(SetPower {
                device: PowerToggle::LivingRoomTvAmbilight,
                power_on,
            }) => self.lgtv.set_piccap_power(*power_on).await,
            Command::SetHeating(SetHeating { device, target_state }) => {
                let device_id = match device {
                    Radiator::RoomOfRequirements => "room_of_requirements/radiator_thermostat_sonoff",
                    Radiator::Bathroom => "bathroom/radiator_thermostat_sonoff",
                    Radiator::LivingRoomBig => "living_room/radiator_thermostat_big_sonoff",
                    Radiator::LivingRoomSmall => "living_room/radiator_thermostat_small_sonoff_v2",
                    Radiator::Bedroom => "bedroom/radiator_thermostat_sonoff",
                    Radiator::Kitchen => "kitchen/radiator_thermostat_sonoff_v2",
                };
                self.z2m.set_heating(device_id, target_state.clone()).await
            }
            Command::PushNotify(PushNotify {
                notification,
                recipient,
                operation,
            }) => {
                self.homeassistant
                    .push_notify(push_recipient(*recipient), *notification, *operation)
                    .await
            }
            Command::SetLightNotification(SetLightNotification { device, signal, on }) => {
                self.homeassistant
                    .set_light_notification(notification_light_entity(*device), *signal, *on)
                    .await
            }
            Command::SetEnergySaving(SetEnergySaving {
                device: EnergySavingDevice::LivingRoomTv,
                on,
            }) => self.lgtv.set_energy_saving(*on).await,
            Command::ControlFan(ControlFan {
                device: Fan::BedroomDehumidifier,
                speed,
            }) => {
                self.homeassistant
                    .set_comfee_fan_speed("humidifier.dehumidifier_34e8", "fan.dehumidifier_34e8_fan", speed)
                    .await
            }
            Command::ControlFan(ControlFan {
                device: Fan::LivingRoomAirPurifier,
                speed,
            }) => {
                self.homeassistant
                    .set_philips_air_purifier_speed("fan.wohnzimmer", speed)
                    .await
            }
            Command::OpenDoor(OpenDoor {
                device: Lock::BuildingEntrance,
                open,
            }) => {
                if *open {
                    self.nuki.open_door("1CC90CCA").await
                } else {
                    //Nothing to do to close it, just leave as is
                    Ok(())
                }
            }
        }
    }
}

fn push_recipient(recipient: PushRecipient) -> &'static str {
    match recipient {
        PushRecipient::Dennis => "mobile_app_jarvis",
        PushRecipient::Sabine => "mobile_app_simi",
    }
}

fn notification_light_entity(device: NotificationLight) -> &'static str {
    match device {
        NotificationLight::LivingRoom => "light.hue_go",
    }
}

mod metrics {
    use crate::observability::system_metric_increment;

    #[derive(Clone, Copy, derive_more::Display)]
    pub enum CommandTargetSystem {
        #[display("TASMOTA")]
        Tasmota,
        #[display("Z2M")]
        Z2M,
        #[display("NUKI")]
        Nuki,
        #[display("HA")]
        HomeAssistant,
        #[allow(clippy::upper_case_acronyms)]
        #[display("LGTV")]
        LGTV,
    }

    pub enum CommandMetric {
        Executed {
            device_id: String,
            system: CommandTargetSystem,
        },
    }

    impl CommandMetric {
        pub fn record(&self) {
            match self {
                CommandMetric::Executed { device_id, system } => {
                    let system = system.to_string();
                    system_metric_increment(
                        "command_executed",
                        &[("device_id", device_id.as_str()), ("system", system.as_str())],
                    );
                }
            }
        }
    }
}
