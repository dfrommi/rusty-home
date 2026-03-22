# Infrastructure

Cross-cutting infrastructure services. Separate crate at `lib/infrastructure/`.

## Non-obvious behaviors

- **MQTT topic stripping**: subscriptions strip the base topic from incoming messages — subscribers see relative paths only.
- **MQTT QoS**: all publishes use `QoS::ExactlyOnce`.
- **Event bus**: lagged subscribers skip missed messages and return `None` — the channel logs a warning but does not crash. Lagged messages are lost.
