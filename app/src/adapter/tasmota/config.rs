use crate::device_state::{CurrentPowerUsage, PowerAvailable, TotalEnergyConsumption};

use super::TasmotaChannel;

pub fn default_tasmota_state_config() -> Vec<(&'static str, TasmotaChannel)> {
    vec![
        //
        // POWER PLUGS
        //
        (
            "appletv",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::AppleTv, TotalEnergyConsumption::AppleTv),
        ),
        (
            "tv",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::Tv, TotalEnergyConsumption::Tv),
        ),
        (
            "fridge",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::Fridge, TotalEnergyConsumption::Fridge),
        ),
        (
            "dehumidifier",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::Dehumidifier, TotalEnergyConsumption::Dehumidifier),
        ),
        ("dehumidifier", TasmotaChannel::PowerToggle(PowerAvailable::Dehumidifier)),
        (
            "airpurifier",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::AirPurifier, TotalEnergyConsumption::AirPurifier),
        ),
        (
            "kettle",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::Kettle, TotalEnergyConsumption::Kettle),
        ),
        (
            "washer",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::WashingMachine, TotalEnergyConsumption::WashingMachine),
        ),
        (
            "couchlight",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::CouchLight, TotalEnergyConsumption::CouchLight),
        ),
        (
            "dishwasher",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::Dishwasher, TotalEnergyConsumption::Dishwasher),
        ),
        (
            "nuc",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::Nuc, TotalEnergyConsumption::Nuc),
        ),
        (
            "dslmodem",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::DslModem, TotalEnergyConsumption::DslModem),
        ),
        (
            "unifi-usg",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::InternetGateway, TotalEnergyConsumption::InternetGateway),
        ),
        (
            "unifi-switch",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::NetworkSwitch, TotalEnergyConsumption::NetworkSwitch),
        ),
        (
            "irheater",
            TasmotaChannel::EnergyMeter(CurrentPowerUsage::InfraredHeater, TotalEnergyConsumption::InfraredHeater),
        ),
        ("irheater", TasmotaChannel::PowerToggle(PowerAvailable::InfraredHeater)),
    ]
}
