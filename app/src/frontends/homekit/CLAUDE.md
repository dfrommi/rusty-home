# HomeKit Frontend — Architecture Guide

## Overview

This frontend bridges the home automation system to Apple HomeKit via
[homebridge-mqtt](https://github.com/cflurin/homebridge-mqtt). All communication
goes over MQTT. There is no direct HAP implementation here.

```
HomeKit app
    ↕ (Homebridge + homebridge-mqtt plugin)
MQTT broker
    ↕
HomekitRunner  (runtime.rs)
    ↕
HomekitRegistry  (accessory/mod.rs)
    ↕
Individual accessories  (accessory/*.rs)
```

## MQTT message format

**Outbound (to HomeKit):** `<base_topic>/to/set`
**Inbound (from HomeKit):** `<base_topic>/from/set`

Both directions use this JSON shape:
```json
{ "name": "Haustür", "service_name": "LockMechanism", "characteristic": "LockTargetState", "value": 0 }
```

**Registration** is sent on startup via `<base_topic>/to/add` (first service) and
`<base_topic>/to/add/service` (additional services on the same accessory):
```json
{ "name": "Haustür", "service_name": "LockMechanism", "service": "LockMechanism", "LockCurrentState": 1, "LockTargetState": 1 }
```
Initial characteristic values in registration come from `HomekitTargetConfig::config`.
`into_config()` sends `"default"`, `with_config(json!(value))` sends a specific initial value.

## Key types (mod.rs, hap.rs)

- **`HomekitTarget`** — identifies a single characteristic: `(name, service, characteristic)`
- **`HomekitTargetConfig`** — a target plus optional initial registration value
- **`HomekitEvent`** — a target plus a runtime value (used both for inbound triggers and outbound state)
- **`HomekitService`** / **`HomekitCharacteristic`** — HAP enum wrappers in `hap.rs`;
  add new variants here when a new service/characteristic is needed.
  Reference: [ServiceDefinitions](https://github.com/homebridge/HAP-NodeJS/blob/latest/src/lib/definitions/ServiceDefinitions.ts),
  [CharacteristicDefinitions](https://github.com/homebridge/HAP-NodeJS/blob/latest/src/lib/definitions/CharacteristicDefinitions.ts)

## Accessory pattern

Every accessory is a plain struct in `accessory/<name>.rs` with three methods:

```rust
// Called once on startup — registers the accessory/service with HomeKit.
// Use into_config() for "default" initial value, with_config(json!(...)) for a specific one.
pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig>

// Called on every HomeStateEvent::Changed. Map HomeStateValue variants to HomekitEvents.
// Return empty vec for state you don't care about (including if the accessory is write-only).
pub fn export_state(&self/&mut self, state: &HomeStateValue) -> Vec<HomekitEvent>

// Called when HomeKit sends a value change. Return the UserTrigger to fire, or None.
// The runtime debounces 2 s before firing the trigger.
pub fn process_trigger(&self/&mut self, trigger: &HomekitEvent) -> Option<UserTrigger>
```

`process_trigger` returns `Option<UserTrigger>` directly using the flat domain variants:
- `UserTrigger::DevicePower { device: OnOffDevice::X, on: bool }`
- `UserTrigger::FanSpeed { fan: FanActivity::X, airflow: FanAirflow::X }`
- `UserTrigger::Heating { zone: HeatingZone::X, request: HeatingRequest::X }`
- `UserTrigger::OpenDoor(Door::X)`

## Adding a new accessory — checklist

1. **`hap.rs`** — add any new `HomekitService` or `HomekitCharacteristic` variants needed.

2. **`accessory/<name>.rs`** — implement the three methods above. Use an existing accessory
   as a template:
   - Read-only sensor → `window_sensor.rs` or `climate_sensor.rs`
   - Writable switch → `power_switch.rs` or `energy_saving_switch.rs`
   - Bidirectional with state → `thermostat.rs` or `fan.rs`
   - Write-only trigger (no persistent state) → `door_lock.rs`

3. **`accessory/mod.rs`** — four places to touch:
   - `mod <name>;`
   - Add variant to `enum HomekitAccessory`
   - Add arm to all three match blocks (`get_device_config`, `export_state`, `process_trigger`)
   - Add instance to `fn config()`

4. **`trigger/domain/mod.rs`** — if the new action doesn't map to an existing flat `UserTrigger`
   variant, add a new variant there and update all match sites.

## Runtime behaviour notes

- **Debounce:** the runtime waits 2 s after the last HomeKit event for a given target before
  firing the trigger. Rapid repeated taps reset the timer.
- **State reset:** `export_state` is the only way to push state back to HomeKit. Accessories
  with no corresponding `HomeStateValue` (e.g. door lock) will not auto-reset their HomeKit
  state after a trigger — the UI stays in the triggered state until the next home state event
  or a manual refresh. If immediate reset is needed, it must be handled explicitly.
- **Multi-service accessories:** if one physical accessory needs multiple HomeKit services
  (e.g. climate sensor with `TemperatureSensor` + `HumiditySensor`), all targets are returned
  from `get_all_targets()`. The runtime automatically uses `to/add/service` for the second
  and subsequent services on the same name.
