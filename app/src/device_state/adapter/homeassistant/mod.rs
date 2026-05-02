mod config;

use infrastructure::{Mqtt, MqttSubscription};
use serde::Deserialize;

use anyhow::{Context, bail};
use infrastructure::HttpClientConfig;
use reqwest_middleware::ClientWithMiddleware;

use crate::device_state::adapter::{IncomingData, IncomingDataSource};
use crate::device_state::{
    AllergenIndex, DeviceAvailability, FanActivity, LightLevel, ParticulateMatter, PowerAvailable, Presence,
    RelativeHumidity, Temperature,
};
use std::collections::HashMap;

use crate::core::time::DateTime;
use serde::Deserializer;
use serde_json::Value;

use crate::core::timeseries::DataPoint;
use crate::core::unit::{
    AllergenIndexValue, DegreeCelsius, FanAirflow, FanSpeed, Lux, MicrogramsPerCubicMeter, Percent,
};
use crate::device_state::DeviceStateValue;

use crate::core::DeviceConfig;
use std::sync::Mutex;

#[derive(Debug, Default, Clone)]
struct ComfeeFanCache {
    powered: Option<bool>,
    fan_speed: Option<FanSpeed>,
}

#[derive(Debug, Clone)]
pub enum HaChannel {
    AllergenIndex(AllergenIndex),
    ParticulateMatter(ParticulateMatter),
    Temperature(Temperature),
    RelativeHumidity(RelativeHumidity),
    Powered(PowerAvailable),
    PresenceFromEsp(Presence),
    PresenceFromDeviceTracker(Presence),
    PresenceFromFP2(Presence),
    ComfeeDehumidifierFanPowerState(FanActivity),
    ComfeeDehumidifierFanSpeed(FanActivity),
    PhilipsAirPurifierFan(FanActivity),
    LightLevel(LightLevel),
}

#[derive(Deserialize, Debug)]
pub struct StateChangedEvent {
    pub entity_id: String,
    pub state: StateValue,
    pub last_changed: DateTime,
    pub last_updated: DateTime,
    pub attributes: HashMap<String, Value>,
}

#[derive(Debug)]
pub enum StateValue {
    Available(String),
    Unavailable,
}

//TODO can deserialization of event be in the adapter?
impl<'de> Deserialize<'de> for StateValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "unavailable" => Ok(StateValue::Unavailable),
            _ => Ok(StateValue::Available(value)),
        }
    }
}

pub struct HomeAssistantIncomingDataSource {
    client: HaHttpClient,
    listener: HaMqttClient,
    config: DeviceConfig<HaChannel>,
    initial_load: Option<Vec<StateChangedEvent>>,
    comfee_cache: Mutex<HashMap<FanActivity, ComfeeFanCache>>,
}

impl HomeAssistantIncomingDataSource {
    #[allow(clippy::expect_used)]
    pub async fn new(mqtt: &mut Mqtt, event_topic: &str, url: &str, token: &str) -> Self {
        let config = DeviceConfig::new(&config::default_ha_state_config());
        let rx = mqtt
            .subscribe(event_topic, "")
            .await
            .expect("Error subscribing to MQTT topic");

        let mqtt_client = HaMqttClient::new(rx);
        let http_client = HaHttpClient::new(url, token).expect("Error creating HA HTTP client");

        Self {
            client: http_client,
            listener: mqtt_client,
            config,
            initial_load: None,
            comfee_cache: Mutex::new(HashMap::new()),
        }
    }
}

impl IncomingDataSource<StateChangedEvent, HaChannel> for HomeAssistantIncomingDataSource {
    fn ds_name(&self) -> &str {
        "HomeAssistant"
    }

    async fn recv(&mut self) -> Option<StateChangedEvent> {
        if self.initial_load.is_none() {
            self.initial_load = match self.client.get_current_state().await {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::error!("Error loading initial state for HA: {:?}", e);
                    Some(vec![])
                }
            };
        }

        match &mut self.initial_load {
            Some(data) if !data.is_empty() => data.pop(),
            _ => self.listener.recv().await,
        }
    }

    fn device_id(&self, msg: &StateChangedEvent) -> Option<String> {
        Some(msg.entity_id.clone())
    }

    fn get_channels(&self, device_id: &str) -> &[HaChannel] {
        self.config.get(device_id)
    }

    async fn to_incoming_data(
        &self,
        device_id: &str,
        channel: &HaChannel,
        msg: &StateChangedEvent,
    ) -> anyhow::Result<Vec<IncomingData>> {
        let mut result = match &msg.state {
            StateValue::Available(state_value) => {
                tracing::info!("Received supported event {}", device_id);

                let dp_result = to_persistent_data_point(
                    channel.clone(),
                    state_value,
                    &msg.attributes,
                    msg.last_changed,
                    &self.comfee_cache,
                );

                match dp_result {
                    Ok(Some(dp)) => vec![dp],
                    Ok(None) => vec![],
                    Err(e) => {
                        tracing::error!("Error processing homeassistant event of {}: {:?}", device_id, e);
                        vec![]
                    }
                }
            }
            _ => {
                tracing::warn!("Value of {} is not available", device_id);
                vec![]
            }
        };

        result.push(to_item_availability(msg));

        Ok(result)
    }
}

fn to_persistent_data_point(
    channel: HaChannel,
    ha_value: &str,
    attributes: &HashMap<String, serde_json::Value>,
    timestamp: DateTime,
    comfee_cache: &Mutex<HashMap<FanActivity, ComfeeFanCache>>,
) -> anyhow::Result<Option<IncomingData>> {
    let dp: Option<IncomingData> = match channel {
        HaChannel::AllergenIndex(channel) => Some(
            DataPoint::new(
                DeviceStateValue::AllergenIndex(channel, AllergenIndexValue(ha_value.parse()?)),
                timestamp,
            )
            .into(),
        ),
        HaChannel::ParticulateMatter(channel) => Some(
            DataPoint::new(
                DeviceStateValue::ParticulateMatter(channel, MicrogramsPerCubicMeter(ha_value.parse()?)),
                timestamp,
            )
            .into(),
        ),
        HaChannel::Temperature(channel) => Some(
            DataPoint::new(
                DeviceStateValue::Temperature(channel, DegreeCelsius(ha_value.parse()?)),
                timestamp,
            )
            .into(),
        ),
        HaChannel::RelativeHumidity(channel) => Some(
            DataPoint::new(
                DeviceStateValue::RelativeHumidity(channel, Percent(ha_value.parse()?)),
                timestamp,
            )
            .into(),
        ),
        HaChannel::Powered(channel) => {
            Some(DataPoint::new(DeviceStateValue::PowerAvailable(channel, ha_value == "on"), timestamp).into())
        }
        HaChannel::PresenceFromEsp(channel) => {
            Some(DataPoint::new(DeviceStateValue::Presence(channel, ha_value == "on"), timestamp).into())
        }
        HaChannel::PresenceFromDeviceTracker(channel) => {
            Some(DataPoint::new(DeviceStateValue::Presence(channel, ha_value == "home"), timestamp).into())
        }
        HaChannel::LightLevel(channel) => {
            Some(DataPoint::new(DeviceStateValue::LightLevel(channel, Lux(ha_value.parse()?)), timestamp).into())
        }
        HaChannel::PresenceFromFP2(channel) => {
            Some(DataPoint::new(DeviceStateValue::Presence(channel, ha_value == "on"), timestamp).into())
        }
        HaChannel::ComfeeDehumidifierFanSpeed(channel) => {
            let preset_mode = attributes.get("preset_mode").and_then(|v| v.as_str());
            let fan_speed = match preset_mode {
                Some("Low") => FanSpeed::Low,
                Some("Medium") => FanSpeed::Medium,
                Some("High") => FanSpeed::High,
                _ => bail!("Unknown fan speed value: {:?}", preset_mode),
            };

            update_comfee_state(comfee_cache, channel, ComfeeFanUpdate::FanSpeed(fan_speed), timestamp)?
        }
        HaChannel::ComfeeDehumidifierFanPowerState(channel) => {
            let on = ha_value == "on";

            update_comfee_state(comfee_cache, channel, ComfeeFanUpdate::Powered(on), timestamp)?
        }
        HaChannel::PhilipsAirPurifierFan(channel) => Some(
            DataPoint::new(
                DeviceStateValue::FanActivity(
                    channel,
                    philips_air_purifier_airflow(ha_value, attributes.get("preset_mode").and_then(|v| v.as_str()))?,
                ),
                timestamp,
            )
            .into(),
        ),
    };

    Ok(dp)
}

fn philips_air_purifier_airflow(ha_value: &str, preset_mode: Option<&str>) -> anyhow::Result<FanAirflow> {
    if ha_value == "off" {
        return Ok(FanAirflow::Off);
    }

    let fan_speed = match preset_mode {
        Some("auto" | "allergen" | "bacteria" | "sleep" | "speed_1") => FanSpeed::Low,
        Some("speed_2") => FanSpeed::Medium,
        Some("speed_3" | "turbo") => FanSpeed::High,
        _ => bail!("Unknown living room air purifier fan speed value: {:?}", preset_mode),
    };

    Ok(FanAirflow::Forward(fan_speed))
}

#[derive(Debug, Clone)]
enum ComfeeFanUpdate {
    FanSpeed(FanSpeed),
    Powered(bool),
}

fn update_comfee_state(
    comfee_cache: &Mutex<HashMap<FanActivity, ComfeeFanCache>>,
    channel: FanActivity,
    update: ComfeeFanUpdate,
    timestamp: DateTime,
) -> anyhow::Result<Option<IncomingData>> {
    tracing::info!(
        "Trying to update Comfee state for channel {:?} with update {:?}",
        channel,
        update
    );

    let mut cache = comfee_cache
        .lock()
        .map_err(|e| anyhow::anyhow!("Error locking Comfee cache: {:?}", e))?;
    let state = cache.entry(channel).or_default();

    tracing::info!("Current Comfee state for channel {:?} is {:?}", channel, state);

    match update {
        ComfeeFanUpdate::FanSpeed(fan_speed) => state.fan_speed = Some(fan_speed),
        ComfeeFanUpdate::Powered(on) => state.powered = Some(on),
    }

    let dp = match (state.powered, state.fan_speed.clone()) {
        (Some(false), Some(_)) => {
            tracing::info!("Comfee fan is powered off, setting airflow to Off");
            Some(DataPoint::new(DeviceStateValue::FanActivity(channel, FanAirflow::Off), timestamp).into())
        }
        (Some(true), Some(fan_speed)) => {
            tracing::info!("Comfee fan is powered on, setting airflow to Forward({:?})", fan_speed);
            Some(
                DataPoint::new(
                    DeviceStateValue::FanActivity(channel, FanAirflow::Forward(fan_speed)),
                    timestamp,
                )
                .into(),
            )
        }
        _ => None,
    };

    Ok(dp)
}

fn to_item_availability(new_state: &StateChangedEvent) -> IncomingData {
    let entity_id: &str = &new_state.entity_id;

    DeviceAvailability {
        source: "HA".to_string(),
        device_id: entity_id.to_string(),
        last_seen: new_state.last_updated,
        marked_offline: matches!(new_state.state, StateValue::Unavailable),
    }
    .into()
}

#[derive(Debug, Clone)]
pub struct HaHttpClient {
    client: ClientWithMiddleware,
    base_url: String,
}

impl HaHttpClient {
    pub fn new(url: &str, token: &str) -> anyhow::Result<Self> {
        let client = HttpClientConfig::new(Some(token.to_owned())).new_tracing_client()?;

        Ok(Self {
            client,
            base_url: url.to_owned(),
        })
    }
}

impl HaHttpClient {
    pub async fn get_current_state(&self) -> anyhow::Result<Vec<StateChangedEvent>> {
        let response = self.client.get(format!("{}/api/states", self.base_url)).send().await?;

        response
            .json::<Vec<StateChangedEvent>>()
            .await
            .context("Error getting all states")
    }
}

pub struct HaMqttClient {
    state_rx: MqttSubscription,
}

impl HaMqttClient {
    pub fn new(rx: MqttSubscription) -> Self {
        Self { state_rx: rx }
    }
}

impl HaMqttClient {
    pub async fn recv(&mut self) -> Option<StateChangedEvent> {
        match self.state_rx.recv().await {
            Some(msg) => {
                match serde_json::from_str::<HaEvent>(&msg.payload) {
                    Ok(HaEvent::StateChanged { new_state: event, .. }) => Some(event),
                    Ok(HaEvent::Unknown(_)) => {
                        tracing::trace!("Received unsupported event: {:?}", msg.payload);
                        None
                    }

                    //json parsing error
                    Err(e) => {
                        tracing::error!("Error parsing MQTT message: {}", e);
                        None
                    }
                }
            }

            None => {
                tracing::error!("Error parsing MQTT message: channel closed");
                None
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "event_type", content = "event_data")]
pub enum HaEvent {
    #[serde(rename = "state_changed")]
    StateChanged {
        #[allow(dead_code)]
        entity_id: String,
        new_state: StateChangedEvent,
    },

    #[serde(untagged)]
    Unknown(#[allow(dead_code)] serde_json::Value),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::unit::AllergenIndexValue;
    use crate::t;
    use serde_json::json;

    fn extract_state_value(value: IncomingData) -> DeviceStateValue {
        match value {
            IncomingData::StateValue(dp) => dp.value,
            IncomingData::ItemAvailability(_) => panic!("Expected state value"),
        }
    }

    #[test]
    fn parses_allergen_index() {
        let value = to_persistent_data_point(
            HaChannel::AllergenIndex(AllergenIndex::LivingRoom),
            "7",
            &HashMap::new(),
            t!(now),
            &Mutex::new(HashMap::new()),
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            extract_state_value(value),
            DeviceStateValue::AllergenIndex(AllergenIndex::LivingRoom, AllergenIndexValue(7),)
        );
    }

    #[test]
    fn parses_pm25() {
        let value = to_persistent_data_point(
            HaChannel::ParticulateMatter(ParticulateMatter::LivingRoomPM25),
            "3.25",
            &HashMap::new(),
            t!(now),
            &Mutex::new(HashMap::new()),
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            extract_state_value(value),
            DeviceStateValue::ParticulateMatter(ParticulateMatter::LivingRoomPM25, MicrogramsPerCubicMeter(3.25),)
        );
    }

    #[test]
    fn maps_philips_air_purifier_presets() {
        assert_eq!(
            philips_air_purifier_airflow("on", Some("auto")).unwrap(),
            FanAirflow::Forward(FanSpeed::Low)
        );
        assert_eq!(
            philips_air_purifier_airflow("on", Some("allergen")).unwrap(),
            FanAirflow::Forward(FanSpeed::Low)
        );
        assert_eq!(
            philips_air_purifier_airflow("on", Some("bacteria")).unwrap(),
            FanAirflow::Forward(FanSpeed::Low)
        );
        assert_eq!(
            philips_air_purifier_airflow("on", Some("sleep")).unwrap(),
            FanAirflow::Forward(FanSpeed::Low)
        );
        assert_eq!(
            philips_air_purifier_airflow("on", Some("speed_1")).unwrap(),
            FanAirflow::Forward(FanSpeed::Low)
        );
        assert_eq!(
            philips_air_purifier_airflow("on", Some("speed_2")).unwrap(),
            FanAirflow::Forward(FanSpeed::Medium)
        );
        assert_eq!(
            philips_air_purifier_airflow("on", Some("speed_3")).unwrap(),
            FanAirflow::Forward(FanSpeed::High)
        );
        assert_eq!(
            philips_air_purifier_airflow("on", Some("turbo")).unwrap(),
            FanAirflow::Forward(FanSpeed::High)
        );
    }

    #[test]
    fn maps_philips_air_purifier_off_without_preset() {
        assert_eq!(philips_air_purifier_airflow("off", None).unwrap(), FanAirflow::Off);
    }

    #[test]
    fn rejects_unknown_philips_air_purifier_preset() {
        assert!(philips_air_purifier_airflow("on", Some("unknown")).is_err());
    }

    #[test]
    fn parses_philips_air_purifier_fan_state() {
        let attributes = HashMap::from([("preset_mode".to_string(), json!("turbo"))]);

        let value = to_persistent_data_point(
            HaChannel::PhilipsAirPurifierFan(FanActivity::LivingRoomAirPurifier),
            "on",
            &attributes,
            t!(now),
            &Mutex::new(HashMap::new()),
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            extract_state_value(value),
            DeviceStateValue::FanActivity(FanActivity::LivingRoomAirPurifier, FanAirflow::Forward(FanSpeed::High),)
        );
    }
}
