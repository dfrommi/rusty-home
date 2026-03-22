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

## Development lifecycle

- Run linting and fix new violations at the end of each task
- Run tests and make sure they pass

## System Workflow

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full data-flow and module responsibilities.
