---
name: command
description: Use when a user asks to add a new command, add a device to an existing command, or wire a command to a backend executor (Tasmota, Z2M, HomeAssistant).
---

# Command Skill

You are adding or updating a command in the rusty-home project. Follow this workflow precisely.

## Step 1: Gather Requirements

If the user has not already provided all of the following, ask using AskUserQuestion:

- **What the command does** (e.g., toggle power, set temperature, open a lock)
- **Which backend executor** handles it: Tasmota (MQTT), Z2M (Zigbee2MQTT via MQTT), or HomeAssistant (HTTP REST)
- **External device identifier**:
  - Tasmota: MQTT device ID (e.g., `irheater`)
  - Z2M: friendly name path (e.g., `bathroom/dehumidifier_plug`)
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
- Executor-internal target type variant name (e.g., `HaServiceTarget::NukiLock`)

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

3. **Continue to Step 4c** to wire the executor

## Step 4c: Wire the Executor

Based on the chosen backend, modify the appropriate adapter files:

### Tasmota (`app/src/command/adapter/tasmota/`)

1. **Target type** in `mod.rs` — add variant to `TasmotaCommandTarget` if the existing ones don't cover this device shape:
   ```rust
   enum TasmotaCommandTarget {
       PowerSwitch(&'static str),
       NewTarget(&'static str),  // add if needed
   }
   ```

2. **Execution logic** in `mod.rs` — add match arm in `execute_command()`:
   ```rust
   (Command::MyCommand { field, .. }, TasmotaCommandTarget::NewTarget(device_id)) => {
       self.sender
           .send_transient(
               format!("cmnd/{}/Topic", device_id),
               "payload".to_string(),
           )
           .await?;

       CommandMetric::Executed {
           device_id: device_id.to_string(),
           system: CommandTargetSystem::Tasmota,
       }
       .record();

       Ok(true)
   }
   ```

3. **Config mapping** in `config.rs` — add entry to `default_tasmota_command_config()`:
   ```rust
   (
       CommandTarget::MyCommand { device: MyDevice::Variant },
       TasmotaCommandTarget::NewTarget("device_id"),
   ),
   ```

### Z2M (`app/src/command/adapter/z2m/`)

1. **Target type** in `mod.rs` — add variant to `Z2mCommandTarget` if needed:
   ```rust
   pub enum Z2mCommandTarget {
       SonoffThermostat(&'static str),
       PowerPlug(&'static str),
       NewTarget(&'static str),  // add if needed
   }
   ```

2. **Execution logic** in `mod.rs` — add match arm in `execute_command()` and implement handler method:
   ```rust
   // In execute_command match:
   (Command::MyCommand { field, .. }, Z2mCommandTarget::NewTarget(device_id)) => {
       self.my_command_handler(device_id, field).await?;
       device_id
   }

   // Handler method on Z2mCommandExecutor:
   pub async fn my_command_handler(&self, device_id: &str, /* params */) -> anyhow::Result<()> {
       let set_topic = format!("{}/set", device_id);
       self.sender
           .send_transient(set_topic, json!({ /* payload */ }).to_string())
           .await?;
       Ok(())
   }
   ```

3. **Config mapping** in `config.rs` — add entry to `default_z2m_command_config()`:
   ```rust
   (
       CommandTarget::MyCommand { device: MyDevice::Variant },
       Z2mCommandTarget::NewTarget("friendly_name/device"),
   ),
   ```

### HomeAssistant (`app/src/command/adapter/homeassistant/`)

1. **Target type** in `mod.rs` — add variant to `HaServiceTarget`:
   ```rust
   enum HaServiceTarget {
       // ... existing variants
       NewTarget(&'static str),
   }
   ```

2. **Execution logic** in `mod.rs` — add match arm in `dispatch_service_call()` and implement handler method:
   ```rust
   // In dispatch_service_call match:
   (NewTarget(id), Command::MyCommand { .. }) => self.my_command_handler(id).await,

   // Handler method:
   async fn my_command_handler(&self, id: &str) -> anyhow::Result<()> {
       self.client
           .call_service(
               "domain",
               "service",
               json!({
                   "entity_id": vec![id.to_string()],
                   // additional service data
               }),
           )
           .await?;
       record_executed(id);
       Ok(())
   }
   ```

3. **Config mapping** in `config.rs` — add entry to `default_ha_command_config()`:
   ```rust
   (
       CommandTarget::MyCommand { device: MyDevice::Variant },
       HaServiceTarget::NewTarget("entity.id"),
   ),
   ```

## Step 5: Verify

Run all three checks:

1. `cargo build` — must compile
2. `cargo test` — all tests must pass
3. `cargo clippy` — no new warnings

Fix any issues before considering the task complete.
