---
name: command
description: Use when a user asks to add a new command, add a device to an existing command, or wire a command to a backend executor (Tasmota, Z2M, Nuki, HomeAssistant).
---

# Command Skill

You are adding or updating a command in the rusty-home project. Follow this workflow precisely.

## Reference architecture

`CommandModule` follows the module + client + service pattern.

`CommandService` is the explicit command dispatcher. It exhaustively matches each `Command` and calls one backend's capability method with the command's physical target ID. Backends do not implement a shared command executor interface, and there is no fallback or `Ok(false)` “not mine” result.

Command routing and device-state collection are independent mappings. A physical ID used for a command may match a state ID, but this is not assumed; commands and state may use different devices or have no state feedback.

Before re-executing a command, the planner checks two things in `app/src/command/domain/command_state.rs`:

- **`is_reflected_in_state()`** — is the desired effect already visible in home state?
- **`min_wait_duration_between_executions()`** — per-command-type cooldown

For heating commands (`SetHeating`) and the `Z2mSensorSyncRunner`, read `docs/heating-control.md` — the Sonoff TRV is a dumb binary valve (no PID); the command sets setpoint + valve opening/closing limits, not a valve position.

## Step 1: Gather Requirements

If the user has not already provided all of the following, ask using AskUserQuestion:

- **What the command does** (e.g., toggle power, set temperature, open a lock)
- **Which backend** handles it: Tasmota (MQTT), Z2M (Zigbee2MQTT via MQTT), Nuki (HTTP REST, door locks), or HomeAssistant (HTTP REST)
- **External device identifier**:
  - Tasmota: MQTT device ID (e.g., `irheater`)
  - Z2M: friendly name path (e.g., `bathroom/dehumidifier_plug`)
  - Nuki: Nuki opener ID (e.g., `1CC90CCA`)
  - HomeAssistant: entity ID (e.g., `light.hue_go`, `lock.nuki_nuki_lock`)
- **Payload / protocol details**: what exactly to send to the backend. **CRITICAL: Never guess the payload or API interface. Always ask the user.**
  - Tasmota: MQTT topic pattern and payload (e.g., `cmnd/{id}/Power1` with `ON`/`OFF`)
  - Z2M: JSON payload to publish to `{device_id}/set` (e.g., `{"state": "ON"}`)
  - HomeAssistant: service domain, service name, and service data JSON (e.g., domain `lock`, service `open`, data `{"entity_id": ["lock.xxx"]}`)
- **State reflection**: how to check if the command effect is already visible in home state, or if it's a one-shot action with no persistent state (like `OpenDoor`)
- **Cooldown**: minimum wait duration between repeated executions (e.g., 1 minute for fast toggles, 3 minutes for fans, `None` for one-shot actions)

## Step 2: Classify the Work

Read `app/src/command/domain/mod.rs` to inspect the current `Command` enum.

- If a `Command` variant already exists for this action type (e.g., `SetPower` for a new power switch) → follow the **Existing Command** path (Step 4b)
- If no variant exists (e.g., adding a completely new action) → follow the **New Command Type** path (Step 4a)

## Step 3: Confirm Names

**CRITICAL: Never guess or auto-pick enum variant names.** Always:

1. Suggest names for all new types and present to the user using AskUserQuestion
2. Wait for confirmation before writing any code

Names to confirm (as applicable):
- `Command` enum variant name and its fields
- `CommandTarget` enum variant name
- Device enum name and variant(s) (e.g., `Lock::BuildingEntrance`)
- Backend-specific capability method, if a new one is needed

## Step 4a: New Command Type Flow

Execute ALL of these steps in order:

### 4a.1: Add Domain Enums

In `app/src/command/domain/mod.rs`:

1. Add variant to `Command` enum:
   ```rust
   MyCommand {
       device: MyDevice,
       // additional payload fields
   },
   ```

2. Add matching variant to `CommandTarget` enum:
   ```rust
   #[display("MyCommand[{}]", device)]
   MyCommand { device: MyDevice },
   ```

3. Add match arm to `impl From<&Command> for CommandTarget`:
   ```rust
   Command::MyCommand { device, .. } => CommandTarget::MyCommand { device: device.clone() },
   ```

4. Create the device enum (if new — place after existing device enums, grouped with a comment):
   ```rust
   //
   // MY COMMAND
   //
   #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, Id, EnumVariants)]
   #[serde(rename_all = "snake_case")]
   pub enum MyDevice {
       VariantName,
   }
   ```

5. Add any needed imports at the top of the file (unit types, etc.)

### 4a.2: Add State Reflection

In `app/src/command/domain/command_state.rs`:

1. Add match arm to `is_reflected_in_state()`. Choose the appropriate pattern:

   - **State-based** (compare against home state snapshot):
     ```rust
     Command::MyCommand { device, .. } => {
         // Map command device to home state item and compare
         Ok(false) // implement comparison
     }
     ```

   - **Command-history-based** (for transient actions like notifications):
     ```rust
     Command::MyCommand { .. } => {
         // Use command_client.get_latest_command() to check recent history
         Ok(false)
     }
     ```

   - **No reflection** (one-shot triggers with no persistent state):
     ```rust
     Command::MyCommand { .. } => Ok(false),
     ```

2. Add match arm to `min_wait_duration_between_executions()`:
   ```rust
   Command::MyCommand { .. } => Some(t!(N minutes)),  // or None for no cooldown
   ```

### 4a.3: Add Executor — continue to Step 4c

## Step 4b: Existing Command — New Device Flow

1. **Add device variant** to the existing device enum in `app/src/command/domain/mod.rs`:
   ```rust
   pub enum ExistingDevice {
       // ... existing variants
       NewVariant,
   }
   ```

2. **Extend state reflection** in `app/src/command/domain/command_state.rs` if the reflection function has a device-to-state mapping (e.g., `is_set_power_reflected_in_state` maps `PowerToggle` → `PowerAvailable`):
   ```rust
   ExistingDevice::NewVariant => StateItem::NewVariant,
   ```

3. **Continue to Step 4c** to wire the command

## Step 4c: Wire the command

Add a capability method to the selected backend adapter. The method should accept the physical command target ID and command-specific values, perform the backend protocol operation, record command metrics, and return `anyhow::Result<()>`.

Then add an exhaustive arm to the dispatcher in `app/src/command/service.rs`:

```rust
Command::MyCommand {
    device: MyDevice::Variant,
    value,
} => self.backend.my_capability("physical-command-id", value).await,
```

Do not add a wildcard arm. The dispatcher must produce a compile-time error when a new command or device variant has no route. Do not add command routing tables to backend adapters.

Keep command physical IDs independent from device-state adapter IDs. A command may target a different physical device than the one used to derive state.

## Step 5: Verify

Run all three checks:

1. `cargo build` — must compile
2. `cargo test` — all tests must pass
3. `cargo clippy` — no new warnings

Fix any issues before considering the task complete.
