# Home State

Derives higher-level home state from raw device state. No service — module owns the calculation loop directly.

**Before changing calculation logic**, read `docs/calculations.md` — it documents the physics/rationale behind items like `RiskOfMould`, `Temperature::BedroomCorner`, `DewPoint`, `AbsoluteHumidity` (f_Rsi mould model, 3-Kelvin rule, dewpoint vs absolute humidity, known TODOs).

## Adding or updating a home state item

Use the `home-state` skill (structure/wiring) and `implement-home-state` skill (calculation logic).

