use serde::{Deserialize, Serialize};

// https://github.com/homebridge/HAP-NodeJS/blob/latest/src/lib/definitions/ServiceDefinitions.ts
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HomekitService {
    ContactSensor,
    Fanv2,
    GarageDoorOpener,
    HumiditySensor,
    Lightbulb,
    LockMechanism,
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
    CurrentDoorState,
    CurrentHeatingCoolingState,
    CurrentRelativeHumidity,
    CurrentTemperature,
    LockCurrentState,
    LockTargetState,
    On,
    RotationDirection,
    RotationSpeed,
    TargetDoorState,
    TargetHeatingCoolingState,
    TargetTemperature,
    TemperatureDisplayUnits,
}
