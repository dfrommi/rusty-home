# HomeKit Frontend

Bridges the home automation system to Apple HomeKit via
[homebridge-mqtt](https://github.com/cflurin/homebridge-mqtt). All communication goes over MQTT.

```
HomeKit app
  ↕ Homebridge + homebridge-mqtt
MQTT broker
  ↕ HomekitRunner (runtime.rs)
HomekitRegistry (accessory/mod.rs)
  ↕ accessories (accessory/*.rs)
```

## MQTT topics

| Direction | Topic | Payload |
|---|---|---|
| State to HomeKit | `<base>/to/set` | `{ "name", "service_name", "characteristic", "value" }` |
| Trigger from HomeKit | `<base>/from/set` | same shape |
| Register first service | `<base>/to/add` | service definition + initial values |
| Register additional | `<base>/to/add/service` | same |

## Runtime notes

- **Debounce**: 2s after the last event per target before firing the trigger.
- **State reset**: `export_state` is the only way to push state back. Write-only accessories stay in triggered state until the next home state event.
- **Multi-service**: multiple targets from `get_all_targets()` auto-register via `to/add/service` with a 100ms sleep between registrations.

## Adding a new accessory

Use the `homekit-accessory` skill.
