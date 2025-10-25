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

## Custom Macros

The project uses several custom derive macros in `lib/macro/` that generate boilerplate code:

### Core Macros

#### `#[derive(Id)]`

- **Purpose**: Generates ID conversion methods for enum types
- **Generated Helpers**:
  - `int_id()` / `ext_id()` returning `&'static InternalId` / `&'static ExternalId`
  - `TryFrom<InternalId>` and `TryFrom<ExternalId>` implementations
  - `Display` implementation showing `Type[Variant]`
- **Usage**: Applied to all state enums (Temperature, Presence, etc.)
- **Example**: `Temperature::Outside.int_id().variant_name()` returns `"Outside"`

#### `#[derive(IdDelegation)]`

- **Purpose**: Delegates ID methods to wrapped enum variants
- **Usage**: Applied to compound enums that contain other ID-implementing enums
- **Generated**: Same helpers/impls as `Id`, delegating to the inner enum's `int_id()` / `ext_id()`
- **Example**: Used by `HomeState` enum generated from `HomeStateValue`

#### `#[derive(EnumVariants)]`

- **Purpose**: Generates `variants()` method returning all enum variants
- **Supports**: Unit variants and single-field variants (recursive expansion)
- **Generated**: `pub fn variants() -> Vec<Self>` or `pub const fn variants() -> &'static [Self]`
- **Usage**: All state enums for iteration and debugging
- **Example**: `Temperature::variants()` returns all temperature sensor variants

#### `#[derive(EnumWithValue)]`

- **Purpose**: Creates companion enum without value types from `*Value` enums
- **Pattern**: `HomeStateValue` → generates `HomeState` enum
- **Generated**:
  - Companion enum with same variants but only ID types
  - `From` implementations between the two enums
  - `From<f64>` and `Into<f64>` conversions
  - `value_to_string()` method
- **Usage**: Only applied to `HomeStateValue`

#### `#[derive(StateTypeInfoDerive)]`

- **Purpose**: Generates `ValueObject` implementations and persistent types
- **Key Features**:
  - Implements `ValueObject` trait for each variant's ID type
  - Handles bool → f64 conversion (true=1.0, false=0.0)
  - Generates `PersistentHomeStateValue` enum for `#[persistent]` variants
  - Creates `HomeState` implementations for data access
- **Usage**: Applied to `HomeStateValue` enum
- **Persistent Variants**: Only variants marked with `#[persistent]` are included in database storage

### Attribute Macros

#### `#[persistent]`

- **Purpose**: Marks enum variants for database persistence
- **Effect**: Included in `PersistentHomeStateValue` generation
- **Usage**: Applied to individual variants in `HomeStateValue`
- **Example**: `#[persistent] Temperature(Temperature, DegreeCelsius)`

### Macro Consequences & Patterns

#### State Type Hierarchy

```rust
// Original definition
#[derive(StateTypeInfoDerive, EnumWithValue)]
pub enum HomeStateValue {
    #[persistent]
    Temperature(Temperature, DegreeCelsius),
    // ...
}

// Generated types:
pub enum HomeState {           // From EnumWithValue
    Temperature(Temperature),
}

pub enum PersistentHomeStateValue {  // From StateTypeInfoDerive
    Temperature(Temperature, DegreeCelsius),
}
```

#### ID System Integration

- All state enums derive `Id` for consistent identification
- Internal IDs use PascalCase: `"Temperature"`, `"Outside"`
- External IDs use snake_case: `"temperature"`, `"outside"`
- Conversion between internal/external formats is automatic

#### Value Object Pattern

- Each state type implements `ValueObject<ValueType = T>`
- Provides `to_f64()` and `from_f64()` for numeric conversion
- Enables time series storage and mathematical operations
- Bool types automatically convert: true=1.0, false=0.0

## Implementation Guidelines

- **New Adapters**: Follow homeassistant pattern with IncomingDataSource + CommandExecutor traits
- **New Actions**: Implement `Action` trait, add to `HomeAction` enum, configure in `home/config.rs`
- **New State Types**:
  - Create enum deriving `Id + EnumVariants` in `home/state/`
  - Add variant to `HomeStateValue` with appropriate value type
  - Mark with `#[persistent]` if data should be stored in database
  - Implement `ValueObject` trait (auto-generated by `StateTypeInfoDerive`)
- **Configuration**: Use `DeviceConfig<T>` for device mappings, settings in `config.toml`
- **Testing**: Use `HomeApi::for_testing()` with mocked data points where applicable
