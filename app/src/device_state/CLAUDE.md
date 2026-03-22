# Device State

Collects raw device readings from multiple sources and stores them as time-series. Follows module + client + service pattern.

## Dual event semantics

- `DeviceStateEvent::Updated` — emitted on every value received (for observability)
- `DeviceStateEvent::Changed` — emitted only when the value differs from the previous one

Downstream modules (home_state, observability) subscribe to the appropriate event type.

## Adding a new device state

Use the `device-state` skill.

