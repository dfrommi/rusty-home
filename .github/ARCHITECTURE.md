# System Workflow

Two input paths feed the system; both converge on home-state calculation, which drives planning and command execution.

## Pipeline stages

1. **Input: User triggers** — Frontends (HomeKit, remotes) → MQTT → `TriggerClient` persists a `UserTrigger` → emits `TriggerEvent::TriggerAdded`
2. **Input: Device state** — Adapters (Tasmota, Z2M, HomeAssistant, energy meters) → MQTT / HTTP polling → `DeviceStateModule` deduplicates → emits `DeviceStateEvent::Changed`
3. **State derivation** — `HomeStateModule` combines raw device state + active user triggers into a `StateSnapshot` (occupancy, mould risk, heating demand, …). Recalculates on `DeviceStateEvent::Changed` (debounced 50 ms), `TriggerEvent::TriggerAdded`, or a 30 s timer. Emits `HomeStateEvent::SnapshotUpdated`.
4. **Planning** — `AutomationModule` runs `plan_for_home(snapshot)` on every `SnapshotUpdated` (and a 30 s timer). Evaluates `HomeGoal` → `HomeAction` rules. Each rule returns `Execute(commands)`, `ExecuteTrigger(commands, trigger_id)`, or `Skip`. A sequential resource-lock pass prevents conflicting commands on the same device.
5. **Command execution** — `CommandClient` tries executors in order: Tasmota → Z2M → HA. `is_reflected_in_state` checks and per-type cooldowns prevent redundant re-execution. Emits `CommandEvent::CommandExecuted`.
6. **Feedback loop** — Executed commands feed back as `DeviceStateEvent` via the internal adapter, returning to stage 2.

## Key behaviours

- **Trigger activation windows**: `UserTrigger` has `active_from` / `active_until`; only active triggers appear in the snapshot.
- **Deduplication**: `DeviceStateModule` only emits `Changed` when a value actually differs from the previous one.
- **Debounce**: State derivation debounces change-triggered recalculations by 50 ms.
- **Executor fallback chain**: Tasmota → Z2M → HA (first success wins).
- **Cooldowns**: Per-command-type cooldowns prevent rapid re-execution.
