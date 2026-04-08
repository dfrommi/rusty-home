---
applyTo: app/src/**
---
# Module Structure

Composition root: `main.rs`. Each domain module lives in `<module>/`:

- `mod.rs` — **module struct**: owns wiring (repos, adapters, event bus). Provides `new(...)`, `subscribe()` → `EventListener`, `run()` async loop.
- `service.rs` — **business logic**: shared via `Arc`, emits events via `EventEmitter`.
- `domain/` — types and logic.
- `adapter/` — external IO (DB, MQTT, HTTP).

**Client**: thin `Clone`-able `Arc<Service>` wrapper for cross-module calls. Exposed from module, not the service.

## Existing modules

| Module | Pattern |
|---|---|
| `command/` | module + client + service |
| `trigger/` | module + client + service |
| `device_state/` | module + client + service |
| `home_state/` | module + client (no service; module owns calculation loop) |
| `automation/` | pure runner (reacts to `HomeStateEvent`) |
| `observability/` | module + api (`api()` method) |
| `frontends/remote/` | module + service (no client) |
| `frontends/homekit/` | runner factory (`new_runner()` on config struct) |

## Adding a new module

Default to module + client + service unless it's a pure runner or stateless adapter.
Keep wiring in `mod.rs`, logic in `service.rs`, types in `domain/`, IO in `adapter/`.

