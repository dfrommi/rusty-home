# System Workflow

Two input paths feed the system; both converge on home-state calculation, which drives planning and command execution.

## Pipeline stages

1. **Input: User triggers** — Frontends (HomeKit, remotes) → MQTT → `TriggerClient` persists a `UserTrigger` → emits `TriggerEvent::TriggerAdded`
2. **Input: Device state** — Adapters (Tasmota, Z2M, HomeAssistant, Tado, energy meters) → MQTT / HTTP polling → `DeviceStateModule` deduplicates → emits `DeviceStateEvent::Changed`
3. **State derivation** — `HomeStateModule` combines raw device state + active user triggers into a `StateSnapshot` (occupancy, mould risk, heating demand, …). Recalculates on `DeviceStateEvent::Changed` (debounced 50 ms), `TriggerEvent::TriggerAdded`, or a 30 s timer. Emits `HomeStateEvent::SnapshotUpdated`.
4. **Planning** — `AutomationModule` runs `plan_for_home(snapshot)` on every `SnapshotUpdated` (and a 30 s timer). `resource_plans()` defines, per `CommandTarget`, a priority-ordered list of `HomeAction` rules. Actions are evaluated in order and the first non-`Skip` wins — it produces a single `Command` (or a `Command` bound to a `UserTriggerId`), which is then executed. `should_execute` applies the in-memory 30-second same-command guard and state-reflection checks before firing.
5. **Command execution** — `CommandClient` sends each command to the exhaustive dispatcher in `CommandService`. The dispatcher selects exactly one backend capability method and supplies its physical command target ID. The automation planner keeps an in-memory last-execution record and waits 30 seconds before retrying the same source and command. `is_reflected_in_state` prevents execution when the desired effect is already visible.
6. **Feedback loop** — Device-state adapters observe external device feedback and return it as `DeviceStateEvent`, returning to stage 2. Notification delivery state is maintained separately by the notification module.

## Key behaviours

- **Trigger activation windows**: `UserTrigger` has `active_from` / `active_until`; only active triggers appear in the snapshot.
- **Deduplication**: `DeviceStateModule` only emits `Changed` when a value actually differs from the previous one.
- **Debounce**: State derivation debounces change-triggered recalculations by 50 ms.
- **Explicit command routing**: `CommandService` owns an exhaustive match over commands and knows all backend implementations and physical command target IDs. There is no backend probing or fallback chain.
- **Independent state mappings**: device-state adapter IDs and command target IDs are maintained independently. Matching IDs do not imply that state gathering and command execution use the same physical device.
- **Execution guard**: The automation planner keeps last-execution state in memory and blocks the same source and command for 30 seconds while device feedback propagates.
