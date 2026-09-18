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
- Unit types (`DegreeCelsius`, `Percent`) divided by `Duration` produce `RateOfChange<T>`.

## Code Style

- Use `pub` instead of `pub(crate)`

## System Architecture

See [ARCHITECTURE.md](.agents/ARCHITECTURE.md) for the full data-flow and module responsibilities.

## Observability

See [Observability Reference](.agents/observability.md) for the full Grafana datasource inventory, telemetry pipelines (OTLP + VictoriaMetrics), metric-to-code mappings, trace structure, and query patterns.

## Module Reference

For automation rules, commands, device states, home states, and HomeKit accessories, use the matching skill — the reference architecture is inlined there. The short docs below cover the remaining modules and only document what isn't self-evident from the code:

| Path | Reference |
| --- | --- |
| `app/src/**` | [Module structure & wiring](.agents/instructions/app.md) |
| `app/src/frontends/energy_meter/**` | [Energy meter frontend](.agents/instructions/energy-meter.md) |
| `app/src/observability/**` | [Observability module](.agents/instructions/observability.md) |
| `lib/infrastructure/**` | [Infrastructure (MQTT, event bus)](.agents/instructions/infrastructure.md) |
| `lib/macro/**` | [Procedural macros](.agents/instructions/macro.md) |
| `app/src/frontends/remote/**` | [Remote frontend](.agents/instructions/remote.md) |
| `app/src/trigger/**` | [Trigger module](.agents/instructions/trigger.md) |

## Domain Knowledge

- [Calculation physics](docs/calculations.md) — theory/rationale behind home-state calculations (f_Rsi mould model, 3-Kelvin rule, dewpoint vs absolute humidity). **Read before touching any calculation item** (`RiskOfMould`, `DewPoint`, `AbsoluteHumidity`, `Temperature::BedroomCorner`) or the `dehumidify` rule.
- [Heating control](docs/heating-control.md) — the physical setup, the TRV model, the full control chain, the environment constraints, and the design dead ends. **Read before touching any heating item** (`TargetHeatingMode`, `SetPoint`, `TargetHeatingAdjustment`, `TargetHeatingDemand`, `HeatingDemand`, `HeatingDemandLimit`), the `FollowTargetHeatingDemand` rule, the Z2M heating executor, or the `Z2mSensorSyncRunner`.
