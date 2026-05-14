# Rusty Home

Smart-home collection and automation software.

## Commands

- Build: `cargo build`
- Test: `cargo test` / `cargo test <test_name>`
- Format: `cargo fmt`
- Lint: `cargo clippy`

## Workspace Layout

- `app/` — main application crate (modules in `app/src/`)
- `lib/macro/` — procedural derive macros (`StateEnumDerive`, `Id`, etc.)
- `lib/infrastructure/` — cross-cutting infra (MQTT, event bus)

## Development Lifecycle

At the end of every task, run `cargo fmt` and `cargo clippy` and fix any new violations, then run `cargo test` and make sure all tests pass.

## Core Types & Time DSL

The `t!` macro is used throughout the codebase as a time DSL:

```rust
t!(now)            // DateTime::now()
t!(10:30)          // Time::at(10, 30)
t!(10:00 - 14:00)  // DailyTimeRange
t!(5 minutes)      // Duration
t!(10 minutes ago) // DateTime
t!(in 5 hours)     // DateTime
```

Three non-obvious behaviors in `app/src/core/`:
- `DateTime::now()` uses a task-local override — use it in tests for deterministic time.
- `DataFrame` deduplicates on insert: consecutive identical values are silently dropped. Timestamps mark when a value *became* active, not when the last message arrived.
- Unit types (`DegreeCelsius`, `Percent`, etc.) divided by `Duration` produce `RateOfChange<T>`.

## System Architecture

See [ARCHITECTURE.md](.agents/ARCHITECTURE.md) for the full data-flow and module responsibilities.

## Module Reference

When editing files in a module, read its reference doc before making changes:

| Path | Reference |
|---|---|
| `app/src/**` | [Module structure & wiring](.agents/instructions/app.md) |
| `app/src/automation/**` | [Automation planner & rules](.agents/instructions/automation.md) |
| `app/src/command/**` | [Command executor chain](.agents/instructions/command.md) |
| `app/src/device_state/**` | [Device state module](.agents/instructions/device-state.md) |
| `app/src/frontends/energy_meter/**` | [Energy meter frontend](.agents/instructions/energy-meter.md) |
| `app/src/home_state/**` | [Home state module](.agents/instructions/home-state.md) |
| `app/src/frontends/homekit/**` | [HomeKit frontend](.agents/instructions/homekit.md) |
| `lib/infrastructure/**` | [Infrastructure (MQTT, event bus)](.agents/instructions/infrastructure.md) |
| `lib/macro/**` | [Procedural macros](.agents/instructions/macro.md) |
| `app/src/frontends/remote/**` | [Remote frontend](.agents/instructions/remote.md) |
| `app/src/trigger/**` | [Trigger module](.agents/instructions/trigger.md) |
