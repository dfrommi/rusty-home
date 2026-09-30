# Energy Meter Frontend

HTTP API for receiving manually entered current-year radiator readings.

## Non-obvious conventions

- HTTP labels use **German room names** (e.g., "Wohnzimmer (groß)", "Küche") — mapped to domain enums in the handler.
- Stores raw yearly readings and adds earlier `year_end` readings to calculate cumulative values. Year-end flags are maintained manually in the database.
- HTTP success follows raw-reading persistence and emits a timestamped cumulative reading to `device_state`; it does not wait for device-state history to be saved.
- Startup publishes the latest cumulative readings after the device-state adapter subscribes. Initial-load errors are logged and startup continues.

