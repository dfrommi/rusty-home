# Agent Guidelines for Rusty Home

## Build/Test Commands

- `cargo build` - Build the project
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test
- `cargo check` - Fast compile check without building
- `cargo clippy` - Run linter
- `cargo fmt` - Format code

## Logging Style Guide (Automation Actions)

- **Purpose**: Logs should let readers infer why an action ran or was skipped without reading code.
- **Always log the decision**: Every execution or skip should produce an info log describing the decision; do not mention specific commands.
- **Levels**:
  - **info**: decision outcomes and skip reasons.
  - **debug**: intermediate calculations only when they add understanding beyond the decision logs.
  - Avoid **trace** for rule decisions; adjust existing trace to the guideline.
- **Scope**: Don’t add common/prefix text; rely on tracing scope/module context.
- **Minimality**: Prefer minimal code changes; small structural adjustments are OK (early-return with logs is fine).
- **Placement**: Log at the point where the decision is made.
- **No input dumps**: Don’t log full input snapshots or large structured objects.
- **Delegation**: In delegating rules, log the workflow decisions there; assume delegated rules already log their own decisions.
- **Threshold phrasing**: Use human-readable wording (e.g., “more than 3 minutes”, “within 3 minutes”).
- **Complex conditions**: Prefer multiple clear decision branches over a single compound `if` so logs don’t read like “this or that”.

## Top-Level Module Structure (module struct + client + service)

### Where it lives
- Composition root is `app/src/main.rs`: wires modules, event buses, clients, and background runners.
- Each domain module typically lives in `app/src/<module>/` with:
  - `mod.rs` for the **module struct** (wiring + public API),
  - `service.rs` for core **business logic**,
  - `domain/` for types/logic,
  - `adapter/` for external IO (DB, MQTT, HTTP, etc.).

### The “module struct” pattern
- The module struct in `mod.rs` owns wiring: repositories/adapters, event buses, and any input listeners.
- `new(...)` builds dependencies and usually creates an `EventBus`.
- `subscribe()` exposes an `EventListener` for outbound events.
- `run()` (if present) is the async loop that consumes inbound events or data sources and emits domain events.

### The client pattern
- The client is a thin, `Clone`-able wrapper around `Arc<Service>` for cross-module calls.
- The client exposes async methods that mirror service capabilities; it never does IO wiring.
- Typical flow in `main.rs`: `Module::new(...)` → `module.client()` for other modules.

### The service pattern
- Service holds the core business logic + repositories/adapters.
- Service is constructed in the module struct and shared via `Arc`.
- Service emits events via `EventEmitter` and is called by the client and module runner.

### Examples in this repo
- `command/`: `CommandModule` + `CommandClient` + `CommandService` (`app/src/command/mod.rs`, `app/src/command/service.rs`).
- `trigger/`: `TriggerModule` + `TriggerClient` + `TriggerService` (`app/src/trigger/mod.rs`, `app/src/trigger/service.rs`).
- `device_state/`: `DeviceStateModule` + `DeviceStateClient` + `DeviceStateService` + multiple incoming data sources (`app/src/device_state/mod.rs`).
- `home_state/`: `HomeStateModule` + `HomeStateClient` (no separate service; module owns the calculation loop).
- `automation/`: `AutomationModule` is a pure runner reacting to `HomeStateEvent`; no client/service.
- `observability/`: `ObservabilityModule` wires repo/adapters and exposes `api()`; no client/service.

### When adding a new module
- Default to module struct + client + service unless it’s a pure runner or stateless adapter.
- Keep wiring in `mod.rs`, logic in `service.rs`, domain types in `domain/`, and IO in `adapter/`.
- Expose the client from the module, not the service, to keep cross-module calls consistent.
