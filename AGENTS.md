# Agent Guidelines for Rusty Home

## Build/Test Commands

- `cargo build` - Build the project
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test
- `cargo check` - Fast compile check without building
- `cargo clippy` - Run linter
- `cargo fmt` - Format code

## Architecture Overview

This is a smart home automation system with event-driven planning and execution:

### Core Modules

- **`core`** - Central API, persistence, planning engine, time series data
- **`home`** - Domain logic: actions, goals, state definitions, commands
- **`adapter`** - External integrations: HomeAssistant, Homekit, Tasmota, Z2M, Grafana

### Data Flow

1. **Incoming Data**: Adapters receive MQTT/HTTP events → `IncomingDataSource` trait → Core API
2. **State Management**: Time series data cached in `HomeApi`, persisted to PostgreSQL
3. **Planning**: Event-driven planner evaluates goals → generates actions → creates commands
4. **Command Execution**: Commands queued → `CommandExecutor` trait → sent to adapters

### Key Patterns

- **Goal-Action Configuration**: `home/config.rs` maps `HomeGoal` → `Vec<HomeAction>`
- **Adapter Structure**: Each adapter has `incoming.rs`, `outgoing.rs`, `config.rs`, `mod.rs`
- **State Types**: Enums with `#[persistent]` attribute for database storage
- **Event System**: Database triggers → `AppEventListener` → broadcast channels

## Code Style

- Max line width: 120 characters (rustfmt.toml)
- Use workspace dependencies from root Cargo.toml
- Follow Rust 2024 edition conventions
- Use `derive_more` for common trait implementations
- Prefer `anyhow::Result` for error handling
- Use `tracing` for logging, not `println!`

## Imports & Modules

- Group imports: std, external crates, local modules
- Use `pub use` for re-exports in mod.rs files
- Prefer explicit imports over glob imports

## Naming Conventions

- Snake_case for functions, variables, modules
- PascalCase for types, structs, enums
- SCREAMING_SNAKE_CASE for constants
- Use descriptive names (e.g., `HomeAction`, `CommandExecutor`)

## Implementation Guidelines

- **New Adapters**: Follow homeassistant pattern with IncomingDataSource + CommandExecutor traits
- **New Actions**: Implement `Action` trait, add to `HomeAction` enum, configure in `home/config.rs`
- **New State Types**: Add to `HomeStateValue` enum with `#[persistent]` if stored
- **Configuration**: Use `DeviceConfig<T>` for device mappings, settings in `config.toml`
- **Testing**: Use `HomeApi::for_testing()` with mocked data points

