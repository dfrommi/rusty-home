---
applyTo: app/src/frontends/energy_meter/**
---
# Energy Meter Frontend

HTTP API for receiving energy consumption readings from heating and water meters.

## Non-obvious conventions

- HTTP labels use **German room names** (e.g., "Wohnzimmer groß", "Küche") — mapped to domain enums in the handler.
- Water meter values are **divided by 1000** before storage.
- Emits `EnergyReading` events consumed by the `device_state` module.

