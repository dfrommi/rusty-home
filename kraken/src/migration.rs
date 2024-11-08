use std::env;

use adapter::persistence::BackendApi;
use settings::Settings;
use tracing::info;

mod adapter;
mod settings;

#[tokio::main]
pub async fn main() {
    unsafe { env::set_var("RUST_LOG", "warn,kraken_migration=debug") };
    tracing_subscriber::fmt::init();

    let settings = Settings::new().expect("Error reading configuration");
    info!("Starting with settings: {:?}", settings);

    let source_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&settings.migration_db.url)
        .await
        .unwrap();

    let target_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&settings.database.url)
        .await
        .unwrap();

    let api = BackendApi::new(target_pool.clone());

    tracing::info!("Migrating data");

    thing_values::migrate(&source_pool, &api).await;
}

mod thing_values {
    use api::state::{
        Channel, ChannelValue, CurrentPowerUsage, ExternalAutoControl, HeatingDemand, Opened,
        Powered, Presence, RelativeHumidity, SetPoint, Temperature, TotalEnergyConsumption,
    };
    use support::unit::Watt;

    use crate::adapter::persistence::BackendApi;

    pub async fn migrate(source_pool: &sqlx::PgPool, api: &BackendApi) {
        let mut source_tags: Vec<SourceTags> =
            sqlx::query_as("SELECT * FROM tags where channel is not null")
                .fetch_all(source_pool)
                .await
                .unwrap();

        source_tags.sort_by(|a, b| a.channel.cmp(&b.channel));

        for source in source_tags {
            let target = into_target(&source);
            if target.is_none() {
                tracing::debug!(
                    "Skipping {} {:?} {:?}",
                    source.channel,
                    source.room,
                    source.position
                );
                continue;
            }

            migrate_channel(source.id, target.unwrap(), source_pool, api).await;
        }
    }

    async fn migrate_channel(
        source_id: i32,
        target: Channel,
        source_pool: &sqlx::PgPool,
        api: &BackendApi,
    ) {
        tracing::info!("Migrating channel {:?}", target);

        let souce_thing_values: Vec<SourceThingValues> = sqlx::query_as(
            "SELECT * FROM thing_values WHERE tag_id = $1 order by timestamp asc, id asc",
        )
        .bind(source_id)
        .fetch_all(source_pool)
        .await
        .unwrap();

        for source_value in souce_thing_values {
            let target_value = into_channel_value(&target, source_value.value);
            api.add_thing_value(&target_value, &source_value.timestamp)
                .await
                .unwrap();
        }
    }

    fn into_channel_value(channel: &Channel, value: f64) -> ChannelValue {
        match channel.clone() {
            Channel::Temperature(temperature) => {
                ChannelValue::Temperature(temperature, value.into())
            }
            Channel::RelativeHumidity(relative_humidity) => {
                ChannelValue::RelativeHumidity(relative_humidity, value.into())
            }
            Channel::Opened(opened) => ChannelValue::Opened(opened, value > 0.0),
            Channel::Powered(powered) => ChannelValue::Powered(powered, value > 0.0),
            Channel::CurrentPowerUsage(current_power_usage) => {
                ChannelValue::CurrentPowerUsage(current_power_usage, Watt(value / 1000.0))
            }
            Channel::TotalEnergyConsumption(total_energy_consumption) => {
                ChannelValue::TotalEnergyConsumption(total_energy_consumption, value.into())
            }
            Channel::SetPoint(set_point) => ChannelValue::SetPoint(set_point, value.into()),
            Channel::HeatingDemand(heating_demand) => {
                ChannelValue::HeatingDemand(heating_demand, value.into())
            }
            Channel::ExternalAutoControl(external_auto_control) => {
                ChannelValue::ExternalAutoControl(external_auto_control, value == 0.0)
                //Inverted flag
            }
            Channel::Presence(presence) => ChannelValue::Presence(presence, value > 0.0),
        }
    }

    fn into_target(source: &SourceTags) -> Option<Channel> {
        match (
            source.channel.as_str(),
            source.room.as_deref(),
            source.position.as_deref(),
        ) {
            ("TEMPERATURE", Some("KITCHEN"), Some("OUTER_WALL")) => {
                Some(Channel::Temperature(Temperature::KitchenOuterWall))
            }
            ("TEMPERATURE", Some("BEDROOM"), Some("OUTER_WALL")) => {
                Some(Channel::Temperature(Temperature::BedroomOuterWall))
            }
            ("TEMPERATURE", Some("LIVING_ROOM"), Some("DOOR")) => {
                Some(Channel::Temperature(Temperature::LivingRoomDoor))
            }
            ("TEMPERATURE", Some("BEDROOM"), Some("DOOR")) => {
                Some(Channel::Temperature(Temperature::BedroomDoor))
            }
            ("TEMPERATURE", Some("BATHROOM"), None) => {
                Some(Channel::Temperature(Temperature::BathroomShower))
            }
            ("TEMPERATURE", Some("ROOM_OF_REQUIREMENTS"), Some("DOOR")) => {
                Some(Channel::Temperature(Temperature::RoomOfRequirementsDoor))
            }
            ("TEMPERATURE", None, Some("OUTDOOR")) => {
                Some(Channel::Temperature(Temperature::Outside))
            }

            ("HUMIDITY", Some("KITCHEN"), Some("OUTER_WALL")) => Some(Channel::RelativeHumidity(
                RelativeHumidity::KitchenOuterWall,
            )),
            ("HUMIDITY", Some("BEDROOM"), Some("OUTER_WALL")) => Some(Channel::RelativeHumidity(
                RelativeHumidity::BedroomOuterWall,
            )),
            ("HUMIDITY", Some("LIVING_ROOM"), Some("DOOR")) => {
                Some(Channel::RelativeHumidity(RelativeHumidity::LivingRoomDoor))
            }
            ("HUMIDITY", Some("BEDROOM"), Some("DOOR")) => {
                Some(Channel::RelativeHumidity(RelativeHumidity::BedroomDoor))
            }
            ("HUMIDITY", Some("BATHROOM"), None) => {
                Some(Channel::RelativeHumidity(RelativeHumidity::BathroomShower))
            }
            ("HUMIDITY", Some("ROOM_OF_REQUIREMENTS"), Some("DOOR")) => Some(
                Channel::RelativeHumidity(RelativeHumidity::RoomOfRequirementsDoor),
            ),
            ("HUMIDITY", None, Some("OUTDOOR")) => {
                Some(Channel::RelativeHumidity(RelativeHumidity::Outside))
            }

            ("OPEN", Some("KITCHEN"), Some("WINDOW")) => {
                Some(Channel::Opened(Opened::KitchenWindow))
            }
            ("OPEN", Some("BEDROOM"), Some("WINDOW")) => {
                Some(Channel::Opened(Opened::BedroomWindow))
            }
            ("OPEN", Some("LIVING_ROOM"), Some("WINDOW_LEFT")) => {
                Some(Channel::Opened(Opened::LivingRoomWindowLeft))
            }
            ("OPEN", Some("LIVING_ROOM"), Some("WINDOW_RIGHT")) => {
                Some(Channel::Opened(Opened::LivingRoomWindowRight))
            }
            ("OPEN", Some("LIVING_ROOM"), Some("WINDOW_SIDE")) => {
                Some(Channel::Opened(Opened::LivingRoomWindowSide))
            }
            ("OPEN", Some("LIVING_ROOM"), Some("BALCONY_DOOR")) => {
                Some(Channel::Opened(Opened::LivingRoomBalconyDoor))
            }
            ("OPEN", Some("ROOM_OF_REQUIREMENTS"), Some("WINDOW_LEFT")) => {
                Some(Channel::Opened(Opened::RoomOfRequirementsWindowLeft))
            }
            ("OPEN", Some("ROOM_OF_REQUIREMENTS"), Some("WINDOW_RIGHT")) => {
                Some(Channel::Opened(Opened::RoomOfRequirementsWindowRight))
            }
            ("OPEN", Some("ROOM_OF_REQUIREMENTS"), Some("WINDOW_SIDE")) => {
                Some(Channel::Opened(Opened::RoomOfRequirementsWindowSide))
            }

            ("HEATING_POWER", Some("LIVING_ROOM"), None) => {
                Some(Channel::HeatingDemand(HeatingDemand::LivingRoom))
            }
            ("HEATING_POWER", Some("BEDROOM"), None) => {
                Some(Channel::HeatingDemand(HeatingDemand::Bedroom))
            }
            ("HEATING_POWER", Some("ROOM_OF_REQUIREMENTS"), None) => {
                Some(Channel::HeatingDemand(HeatingDemand::RoomOfRequirements))
            }
            ("HEATING_POWER", Some("KITCHEN"), None) => {
                Some(Channel::HeatingDemand(HeatingDemand::Kitchen))
            }
            ("HEATING_POWER", Some("BATHROOM"), None) => {
                Some(Channel::HeatingDemand(HeatingDemand::Bathroom))
            }

            ("MANUAL_CONTROL", Some("LIVING_ROOM"), None) => Some(Channel::ExternalAutoControl(
                ExternalAutoControl::LivingRoomThermostat,
            )),
            ("MANUAL_CONTROL", Some("BEDROOM"), None) => Some(Channel::ExternalAutoControl(
                ExternalAutoControl::BedroomThermostat,
            )),
            ("MANUAL_CONTROL", Some("ROOM_OF_REQUIREMENTS"), None) => Some(
                Channel::ExternalAutoControl(ExternalAutoControl::RoomOfRequirementsThermostat),
            ),
            ("MANUAL_CONTROL", Some("KITCHEN"), None) => Some(Channel::ExternalAutoControl(
                ExternalAutoControl::KitchenThermostat,
            )),
            ("MANUAL_CONTROL", Some("BATHROOM"), None) => Some(Channel::ExternalAutoControl(
                ExternalAutoControl::BathroomThermostat,
            )),

            ("TARGET_TEMPERATURE", Some("LIVING_ROOM"), None) => {
                Some(Channel::SetPoint(SetPoint::LivingRoom))
            }
            ("TARGET_TEMPERATURE", Some("BEDROOM"), None) => {
                Some(Channel::SetPoint(SetPoint::Bedroom))
            }
            ("TARGET_TEMPERATURE", Some("ROOM_OF_REQUIREMENTS"), None) => {
                Some(Channel::SetPoint(SetPoint::RoomOfRequirements))
            }
            ("TARGET_TEMPERATURE", Some("KITCHEN"), None) => {
                Some(Channel::SetPoint(SetPoint::Kitchen))
            }
            ("TARGET_TEMPERATURE", Some("BATHROOM"), None) => {
                Some(Channel::SetPoint(SetPoint::Bathroom))
            }

            ("ENERGY_TOTAL", Some("KITCHEN"), Some("FRIDGE")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::Fridge),
            ),
            ("ENERGY_TOTAL", Some("BATHROOM"), Some("DEHUMIDIFIER")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::Dehumidifier),
            ),
            ("ENERGY_TOTAL", Some("LIVING_ROOM"), Some("APPLE_TV")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::AppleTv),
            ),
            ("ENERGY_TOTAL", Some("LIVING_ROOM"), Some("TV")) => {
                Some(Channel::TotalEnergyConsumption(TotalEnergyConsumption::Tv))
            }
            ("ENERGY_TOTAL", Some("LIVING_ROOM"), Some("AIRPURIFIER")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::AirPurifier),
            ),
            ("ENERGY_TOTAL", Some("LIVING_ROOM"), Some("COUCH")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::CouchLight),
            ),
            ("ENERGY_TOTAL", Some("KITCHEN"), Some("DISHWASHER")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::Dishwasher),
            ),
            ("ENERGY_TOTAL", Some("KITCHEN"), Some("KETTLE")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::Kettle),
            ),
            ("ENERGY_TOTAL", Some("BATHROOM"), Some("WASHER")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::WashingMachine),
            ),
            ("ENERGY_TOTAL", Some("ROOM_OF_REQUIREMENTS"), Some("NUC")) => {
                Some(Channel::TotalEnergyConsumption(TotalEnergyConsumption::Nuc))
            }
            ("ENERGY_TOTAL", Some("ROOM_OF_REQUIREMENTS"), Some("DSL_MODEM")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::DslModem),
            ),
            ("ENERGY_TOTAL", Some("ROOM_OF_REQUIREMENTS"), Some("UNIFI_USG")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::InternetGateway),
            ),
            ("ENERGY_TOTAL", Some("ROOM_OF_REQUIREMENTS"), Some("UNIFI_SWITCH")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::NetworkSwitch),
            ),
            ("ENERGY_TOTAL", Some("KITCHEN"), Some("WORKSPACE")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::KitchenMultiPlug),
            ),
            ("ENERGY_TOTAL", Some("LIVING_ROOM"), Some("BEHIND_COUCH")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::CouchPlug),
            ),
            ("ENERGY_TOTAL", Some("ROOM_OF_REQUIREMENTS"), Some("WORKSPACE")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::RoomOfRequirementsDesk),
            ),
            ("ENERGY_TOTAL", Some("BEDROOM"), Some("IR_HEATER")) => Some(
                Channel::TotalEnergyConsumption(TotalEnergyConsumption::InfraredHeater),
            ),

            ("ENERGY_CURRENT", Some("KITCHEN"), Some("FRIDGE")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::Fridge))
            }
            ("ENERGY_CURRENT", Some("BATHROOM"), Some("DEHUMIDIFIER")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::Dehumidifier))
            }
            ("ENERGY_CURRENT", Some("LIVING_ROOM"), Some("APPLE_TV")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::AppleTv))
            }
            ("ENERGY_CURRENT", Some("LIVING_ROOM"), Some("TV")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::Tv))
            }
            ("ENERGY_CURRENT", Some("LIVING_ROOM"), Some("AIRPURIFIER")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::AirPurifier))
            }
            ("ENERGY_CURRENT", Some("LIVING_ROOM"), Some("COUCH")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::CouchLight))
            }
            ("ENERGY_CURRENT", Some("KITCHEN"), Some("DISHWASHER")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::Dishwasher))
            }
            ("ENERGY_CURRENT", Some("KITCHEN"), Some("KETTLE")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::Kettle))
            }
            ("ENERGY_CURRENT", Some("BATHROOM"), Some("WASHER")) => Some(
                Channel::CurrentPowerUsage(CurrentPowerUsage::WashingMachine),
            ),
            ("ENERGY_CURRENT", Some("ROOM_OF_REQUIREMENTS"), Some("NUC")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::Nuc))
            }
            ("ENERGY_CURRENT", Some("ROOM_OF_REQUIREMENTS"), Some("DSL_MODEM")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::DslModem))
            }
            ("ENERGY_CURRENT", Some("ROOM_OF_REQUIREMENTS"), Some("UNIFI_USG")) => Some(
                Channel::CurrentPowerUsage(CurrentPowerUsage::InternetGateway),
            ),
            ("ENERGY_CURRENT", Some("ROOM_OF_REQUIREMENTS"), Some("UNIFI_SWITCH")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::NetworkSwitch))
            }
            ("ENERGY_CURRENT", Some("KITCHEN"), Some("WORKSPACE")) => Some(
                Channel::CurrentPowerUsage(CurrentPowerUsage::KitchenMultiPlug),
            ),
            ("ENERGY_CURRENT", Some("LIVING_ROOM"), Some("BEHIND_COUCH")) => {
                Some(Channel::CurrentPowerUsage(CurrentPowerUsage::CouchPlug))
            }
            ("ENERGY_CURRENT", Some("ROOM_OF_REQUIREMENTS"), Some("WORKSPACE")) => Some(
                Channel::CurrentPowerUsage(CurrentPowerUsage::RoomOfRequirementsDesk),
            ),
            ("ENERGY_CURRENT", Some("BEDROOM"), Some("IR_HEATER")) => Some(
                Channel::CurrentPowerUsage(CurrentPowerUsage::InfraredHeater),
            ),

            ("AT_HOME", None, Some("DENNIS")) => Some(Channel::Presence(Presence::AtHomeDennis)),
            ("AT_HOME", None, Some("SABINE")) => Some(Channel::Presence(Presence::AtHomeSabine)),
            ("PRESENT", Some("BEDROOM"), Some("DENNIS")) => {
                Some(Channel::Presence(Presence::BedDennis))
            }
            ("PRESENT", Some("BEDROOM"), Some("SABINE")) => {
                Some(Channel::Presence(Presence::BedSabine))
            }

            ("POWER", Some("BATHROOM"), None) => Some(Channel::Powered(Powered::Dehumidifier)),

            _ => None,
        }
    }

    #[derive(Debug, sqlx::FromRow)]
    struct SourceTags {
        id: i32,
        channel: String,
        room: Option<String>,
        position: Option<String>,
        r#type: String,
    }

    #[derive(Debug, sqlx::FromRow)]
    struct SourceThingValues {
        id: i64,
        tag_id: i32,
        value: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    }
}
