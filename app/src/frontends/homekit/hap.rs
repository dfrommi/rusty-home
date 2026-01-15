use serde::{Deserialize, Serialize};

// https://github.com/homebridge/HAP-NodeJS/blob/latest/src/lib/definitions/ServiceDefinitions.ts
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HomekitService {
    ContactSensor,
    Fanv2,
    HumiditySensor,
    Lightbulb,
    TemperatureSensor,
    Switch,
    Thermostat,
}

// https://github.com/homebridge/HAP-NodeJS/blob/latest/src/lib/definitions/CharacteristicDefinitions.ts
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HomekitCharacteristic {
    Active,
    Brightness,
    ContactSensorState,
    CurrentHeatingCoolingState,
    CurrentRelativeHumidity,
    CurrentTemperature,
    On,
    RotationDirection,
    RotationSpeed,
    TargetHeatingCoolingState,
    TargetTemperature,
    TemperatureDisplayUnits,
}
