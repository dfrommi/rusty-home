---
name: homekit-accessory
description: Use when a user asks to create or update a HomeKit accessory, binding, sensor, switch, or anything exposed to the Apple Home app. Also triggers for keywords like homekit, accessory, home-app, homebridge.
---

# HomeKit Accessory Skill

You are adding or updating a HomeKit accessory in the rusty-home project. The HomeKit integration uses [homebridge-mqtt](https://github.com/cflurin/homebridge-mqtt) which communicates via MQTT using the HAP (HomeKit Accessory Protocol) at a low level.

## Step 1: Gather Requirements

If not already provided by the user, ask using AskUserQuestion:

- **What to expose**: which `HomeStateValue` variant(s) should be visible in the Home app?
- **Direction**: read-only (sensor), writable (switch/control), or write-only trigger (momentary action)?
- **Display name**: German convention is used (e.g., "Klimasensor Wohnzimmer", "Thermostat Küche"). Suggest one and confirm.

Read `app/src/home_state/items/mod.rs` to check which `HomeStateValue` variants exist and are available for export.

## Step 2: Choose HAP Service and Characteristic Types

**This is the hardest part.** HomeKit often doesn't directly offer what is needed. You must present options to the user.

### Process

1. Read `app/src/frontends/homekit/hap.rs` to see which `HomekitService` and `HomekitCharacteristic` variants already exist
2. Consult the HAP-NodeJS definitions for the full list of available types:
   - Services: https://github.com/homebridge/HAP-NodeJS/blob/latest/src/lib/definitions/ServiceDefinitions.ts
   - Characteristics: https://github.com/homebridge/HAP-NodeJS/blob/latest/src/lib/definitions/CharacteristicDefinitions.ts
3. **Present 2-3 candidate service types** to the user with:
   - What it looks like in the Home app
   - Which characteristics it supports (required vs optional)
   - Pros and cons for the specific use case
   - Any workarounds needed (e.g., using `GarageDoorOpener` for a door buzzer)
4. **Let the user decide** — never auto-pick a HAP type
5. If the chosen service/characteristic doesn't exist in `hap.rs` yet, note that it needs to be added

### Currently available services and characteristics

**Services** (in `hap.rs`):
`ContactSensor`, `Fanv2`, `GarageDoorOpener`, `HumiditySensor`, `Lightbulb`, `LockMechanism`, `TemperatureSensor`, `Switch`, `Thermostat`

**Characteristics** (in `hap.rs`):
`Active`, `Brightness`, `ContactSensorState`, `CurrentDoorState`, `CurrentHeatingCoolingState`, `CurrentRelativeHumidity`, `CurrentTemperature`, `LockCurrentState`, `LockTargetState`, `On`, `RotationDirection`, `RotationSpeed`, `TargetDoorState`, `TargetHeatingCoolingState`, `TargetTemperature`, `TemperatureDisplayUnits`

## Step 3: Confirm Names

**Never guess or auto-pick.** Present suggestions and confirm with the user:

- Rust struct name for the accessory (e.g., `PowerSwitch`, `ClimateSensor`)
- `HomekitAccessory` enum variant name
- Display name (German, shown in Home app)
- Any new `UserTrigger` / `UserTriggerTarget` variant names (if writable)

## Step 4: Implement the Accessory

Create `app/src/frontends/homekit/accessory/<snake_case_name>.rs`.

Every accessory implements three methods. Choose the right template based on direction:

### Read-only sensor pattern (e.g., `climate_sensor.rs`, `window_sensor.rs`)

```rust
use crate::{
    frontends::homekit::{HomekitCharacteristic, HomekitEvent, HomekitService, HomekitTarget, HomekitTargetConfig},
    home_state::HomeStateValue,
    trigger::UserTrigger,
};

pub struct MySensor {
    name: &'static str,
    // fields matching HomeStateValue identifiers to filter on
}

impl MySensor {
    pub fn new(name: &'static str, /* identifier fields */) -> Self {
        Self { name, /* ... */ }
    }

    pub fn get_all_targets(&self) -> Vec<HomekitTargetConfig> {
        // Register characteristics with homebridge-mqtt
        // Use .into_config() for defaults, .with_config(json!(...)) for constraints
        vec![
            HomekitTarget::new(self.name.to_string(), HomekitService::Xxx, HomekitCharacteristic::Xxx)
                .into_config(),
        ]
    }

    pub fn export_state(&self, state: &HomeStateValue) -> Vec<HomekitEvent> {
        // Match against relevant HomeStateValue variant
        // Return HomekitEvent with the value to push to HomeKit
        match state {
            HomeStateValue::Xxx(id, value) if *id == self.field => {
                vec![HomekitEvent {
                    target: HomekitTarget::new(self.name.to_string(), HomekitService::Xxx, HomekitCharacteristic::Xxx),
                    value: serde_json::json!(value.0),
                }]
            }
            _ => Vec::new(),
        }
    }

    pub fn process_trigger(&self, _trigger: &HomekitEvent) -> Option<UserTrigger> {
        None // Read-only
    }
}
```

### Writable switch pattern (e.g., `power_switch.rs`, `energy_saving_switch.rs`)

Same as read-only but `process_trigger` returns `Some(UserTrigger::...)`:

```rust
pub fn process_trigger(&self, trigger: &HomekitEvent) -> Option<UserTrigger> {
    if trigger.target == HomekitTarget::new(self.name.to_string(), HomekitService::Switch, HomekitCharacteristic::On)
        && let Some(value) = trigger.value.as_bool()
    {
        return Some(UserTrigger::MyTrigger { /* ... */ });
    }
    None
}
```

### Write-only trigger pattern (e.g., `door_lock.rs`)

- Uses `pending_reset: bool` to snap state back after trigger fires
- `export_state` only emits reset events, doesn't track real state
- `process_trigger` returns the trigger and sets `pending_reset = true`

### Complex bidirectional pattern (e.g., `thermostat.rs`, `fan.rs`)

- Internal status struct to track multi-characteristic state
- Multiple characteristics registered with `get_all_targets()`
- `export_state` matches multiple `HomeStateValue` variants
- `process_trigger` handles multiple characteristics
- Helper methods: `target(characteristic)` and `event(characteristic, value)` to reduce repetition

### Registration config

- `into_config()`: no extra config, HAP uses defaults. For simple sensors and switches.
- `with_config(serde_json::json!(...))`: attach constraints. Examples:
  - `{"validValues": [0, 1]}` — restrict enum options
  - `{"minStep": 20.0}` — step size for sliders
  - `serde_json::json!(1)` — set initial/default value (e.g., door starts closed)

## Step 5: Wire the Registry

In `app/src/frontends/homekit/accessory/mod.rs`:

1. Add `mod <snake_case_name>;` (alphabetical)
2. Add import in the `use` block at top
3. Add variant to `HomekitAccessory` enum
4. Add match arm in ALL THREE dispatch methods:
   - `get_device_config()` → `HomekitAccessory::MyType(x) => x.get_all_targets()`
   - `export_state()` → `HomekitAccessory::MyType(x) => x.export_state(state)`
   - `process_trigger()` → `HomekitAccessory::MyType(x) => x.process_trigger(trigger)`
5. Add instance(s) to `config()` function with the confirmed display name

## Step 6: Add HAP Types (if needed)

If the chosen service or characteristic doesn't exist in `app/src/frontends/homekit/hap.rs`:

1. Add variant to `HomekitService` enum — the variant name must match the HAP-NodeJS service name exactly (PascalCase serialization)
2. Add variant to `HomekitCharacteristic` enum — same rule

Both enums derive `Serialize, Deserialize` without `rename_all`, so variants serialize as-is (PascalCase).

## Step 7: Add Triggers (if writable)

**Only needed if the accessory receives commands from HomeKit.** Skip for read-only sensors.

In `app/src/trigger/domain/mod.rs`:

1. **Add `UserTrigger` variant** — follows the existing pattern:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(tag = "type", rename_all = "snake_case")]
   pub enum UserTrigger {
       // ... existing variants
       MyNewTrigger { field: Type, /* ... */ },
   }
   ```

2. **Add `UserTriggerTarget` variant** — must correspond to the new trigger:
   ```rust
   pub enum UserTriggerTarget {
       // ... existing variants
       #[display("MyNewTrigger[{}]", _0)]
       MyNewTrigger(IdentifierType),
   }
   ```

3. **Add match arm to `UserTrigger::target()`**:
   ```rust
   UserTrigger::MyNewTrigger { field, .. } => UserTriggerTarget::MyNewTrigger(field.clone()),
   ```

4. If the trigger needs a new identifier enum (like `OnOffDevice` or `Door`), create it with:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, derive_more::Display, Id, EnumVariants)]
   #[serde(rename_all = "snake_case")]
   pub enum MyIdentifier {
       Variant1,
   }
   ```

**Processing the trigger (downstream effects) is out of scope** — only the enum variant, serialization, and `target()` mapping are needed here.

## Step 8: Verify

Run all three checks:

1. `cargo build` — must compile
2. `cargo test` — all tests must pass
3. `cargo clippy` — no new warnings

Fix any issues before considering the task complete.
