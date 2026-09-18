---
name: device-state
description: Use when a user asks to create or update a device state, add a sensor, wire a new device, or add a new measurement type to the device-state module.
---

# Device State Skill

You are adding or updating a device state in the rusty-home project. Follow this workflow precisely.

## Reference architecture

`DeviceStateModule` follows the module + client + service pattern. It emits two event types:

- `DeviceStateEvent::Updated` — emitted on every value received (for observability)
- `DeviceStateEvent::Changed` — emitted only when the value differs from the previous one

Downstream modules (home_state, observability) subscribe to the appropriate event type.

Device-state mappings are independent from command routing. A state source ID can match a command target ID, but this is not assumed: commands may use another physical device to affect the system, or the command may have no usable state feedback.

## Step 1: Gather Requirements

If the user has not already provided all of the following, ask using AskUserQuestion:

- **What is being measured** (e.g., temperature, CO2 level, power usage, presence)
- **Which backend adapter** delivers the data: Tasmota, Z2M (Zigbee2MQTT), HomeAssistant, Tado, EnergyMeter, or Internal
- **External device identifier**: the MQTT topic (for Tasmota/Z2M), HA entity ID (for HomeAssistant), Tado zone id (for Tado), or event source
- **Payload structure**: which JSON fields in the incoming message map to which values (e.g., `{"temperature": 21.5, "humidity": 55.0, "last_seen": "..."}`)

## Step 2: Classify the Work

Read `app/src/device_state/domain/mod.rs` to inspect the current `DeviceStateValue` enum.

- If a variant already exists for this measurement type (e.g., `Temperature` for a new temperature sensor) → follow the **Existing Type** path
- If no variant exists (e.g., adding CO2 tracking for the first time) → follow the **New Measurement Type** path

## Step 3: Suggest Names and Confirm

**CRITICAL: Never guess or auto-pick enum variant names.** Always:

1. Suggest a `DeviceStateValue` variant name (for new types) and a domain enum variant name
2. Present the suggestion to the user using AskUserQuestion
3. Wait for confirmation before writing any code

## Step 4a: New Measurement Type Flow

Execute ALL of these steps in order:

### 4a.1: Create Domain Enum File

Create `app/src/device_state/domain/<snake_case_type>.rs`:

```rust
use r#macro::{EnumVariants, Id};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum TypeName {
    VariantName,
    // Add confirmed variants here
}
```

If a variant wraps another enum (like `Radiator`), use tuple variant: `VariantName(OtherEnum)`.

### 4a.2: Update Domain mod.rs

In `app/src/device_state/domain/mod.rs`:

1. Add `mod <snake_case_type>;` (alphabetical order)
2. Add `pub use <snake_case_type>::TypeName;` (alphabetical order)
3. Add variant to `DeviceStateValue` enum: `TypeName(<snake_case_type>::TypeName, ValueUnit)`
4. Add match arm to `impl From<&DeviceStateValue> for f64`:
   - For numeric types: `DeviceStateValue::TypeName(_, v) => v.into()`
   - For boolean types: add to the existing `if *v { 1.0 } else { 0.0 }` group

### 4a.3: Update DB Reconstruction

In `app/src/device_state/adapter/db/mod.rs`, add match arm to `from_f64_value()`:

- For numeric types: `DeviceStateId::TypeName(id) => DeviceStateValue::TypeName(id, value.into())`
- For boolean types: `DeviceStateId::TypeName(id) => DeviceStateValue::TypeName(id, bool_of(value))`

### 4a.4: Create Unit Type (if needed)

Check if a suitable unit already exists in `app/src/core/unit/`:

| Unit | Type | File |
|------|------|------|
| Temperature | `DegreeCelsius` | `degree_celsius.rs` |
| Percentage | `Percent` | `percent.rs` |
| Power | `Watt` | `watt.rs` |
| Energy | `KiloWattHours` | `kwh.rs` |
| Light | `Lux` | `light.rs` |
| Heating | `HeatingUnit` | `heating.rs` |
| Water volume | `KiloCubicMeter` | `liquid.rs` |
| Fan | `FanAirflow`, `FanSpeed` | `fan.rs` |
| Boolean | `bool` | (built-in) |

If no existing unit fits, create a new one in `app/src/core/unit/<name>.rs` following the pattern of existing unit files, and export it from `app/src/core/unit/mod.rs`.

### 4a.5–4a.7: Wire the Adapter

Continue to Step 4b below (same for both paths).

## Step 4b: Existing Type Flow (or adapter wiring for new types)

### Wire the Backend Adapter

Based on the adapter, modify the appropriate files:

#### Tasmota (`app/src/device_state/adapter/tasmota/`)

- **Channel enum** in `mod.rs`: Add or extend `TasmotaChannel` variant if the existing ones don't cover this data shape
- **Config** in `config.rs`: Add entry to `default_tasmota_state_config()`:
  ```rust
  ("device_id", TasmotaChannel::ChannelVariant(DomainEnum::Variant, ...)),
  ```
- **Parsing** in `mod.rs`: Add a match arm in `parse_tasmota_channel()` if new channel type. Create serde `Deserialize` struct if payload shape is novel

#### Z2M (`app/src/device_state/adapter/z2m/`)

- **Channel enum** in `mod.rs`: Add or extend `Z2mChannel` variant
- **Config** in `config.rs`: Add entry to `default_z2m_state_config()`:
  ```rust
  ("friendly_name/device", Z2mChannel::ChannelVariant(DomainEnum::Variant, ...)),
  ```
- **Parsing** in `mod.rs`: Add a match arm in `parse_z2m_channel()` if new channel type

#### HomeAssistant (`app/src/device_state/adapter/homeassistant/`)

- **Channel enum** in `mod.rs`: Add or extend `HaChannel` variant
- **Config** in `config.rs`: Add entry to `default_ha_state_config()`:
  ```rust
  ("sensor.entity_id", HaChannel::ChannelVariant(DomainEnum::Variant)),
  ```
- **Parsing** in `mod.rs`: Add a match arm in `parse_ha_channel()` if new channel type

#### EnergyMeter (`app/src/device_state/adapter/energy_meter/mod.rs`)

- Extend the `From<&EnergyReading> for DeviceStateValue` implementation

#### Internal (`app/src/device_state/adapter/internal/mod.rs`)

- Extend the `incoming_data_from_command_event()` match on `CommandEvent`

#### Tado (`app/src/device_state/adapter/tado/`)

- **Channel enum** in `mod.rs`: Add or extend `TadoChannel` variant if the existing ones don't cover this data shape
- **Config** in `config.rs`: Add entry to `default_tado_state_config()` keyed by the Tado zone id (string):
  ```rust
  ("4", TadoChannel::Temperature(Temperature::NewRoomTado)),
  ("4", TadoChannel::RelativeHumidity(RelativeHumidity::NewRoomTado)),
  ```
- **Parsing** in `mod.rs`: The adapter polls Tado's HTTP API (`poll_api` / `poll_zones`); add a match arm in `poll_api()` if a new channel type needs special handling

### Payload Parsing Patterns

When creating a new payload struct, follow this pattern:

```rust
#[derive(Debug, Clone, serde::Deserialize)]
struct PayloadName {
    field_name: f64,
    last_seen: DateTime,  // from crate::core::time::DateTime
}
```

When creating DataPoints:

```rust
DataPoint::new(
    DeviceStateValue::TypeName(DomainEnum::Variant, UnitType(payload.field)),
    payload.last_seen,
).into()
```

When emitting availability:

```rust
DeviceAvailability {
    source: "AdapterName".to_string(),
    device_id: device_id.to_string(),
    last_seen: payload.last_seen,
    marked_offline: false,
}.into()
```

## Step 5: Verify

Run all three checks:

1. `cargo build` — must compile
2. `cargo test` — all tests must pass
3. `cargo clippy` — no new warnings

Fix any issues before considering the task complete.
