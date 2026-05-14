---
name: home-state
description: Use when a user asks to create or update a derived home state item, add a new calculation to the home-state module, or wire a new HomeStateValue variant.
---

# Home State Skill

You are adding or updating a derived home state item in the rusty-home project. Home state items derive higher-level state (occupancy, felt temperature, risk of mould, etc.) from raw device state or other home state items.

Read `app/src/home_state/CLAUDE.md` before starting.

## Step 1: Gather Requirements

If not already provided by the user, ask using AskUserQuestion:

- **What to derive**: what higher-level state should be computed? (e.g., air quality index, sleep quality, room brightness)
- **Input sources**: which raw device state values or other home state items does it depend on?
- **Output type**: what type does it produce? Check existing units in `app/src/core/unit/` before creating new ones
- **Variants**: does it have multiple instances? (e.g., per-room, per-zone, per-device)

## Step 2: Classify the Work

Read `app/src/home_state/items/mod.rs` to inspect the current `HomeStateValue` enum.

- If a variant already exists for this item → follow the **Update** path (modify existing `DerivedStateProvider` impl)
- If no variant exists → follow the **New Item** path

## Step 3: Confirm Names

**Never guess or auto-pick enum variant names.** Always:

1. Suggest an identifier enum name (e.g., `AirQuality`) and its variants (e.g., `LivingRoom`, `Bedroom`)
2. Suggest a `HomeStateValue` variant name (typically matches the identifier enum name)
3. Present suggestions to the user using AskUserQuestion
4. Wait for confirmation before writing any code

## Step 4: Create the Item File

Create `app/src/home_state/items/<snake_case_name>.rs`.

### Identifier enum

```rust
use r#macro::{Id, EnumVariants};

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Id, EnumVariants)]
pub enum MyItem {
    Variant1,
    Variant2(SomeParameter),  // for parameterized variants like Room, HeatingZone
}
```

- Always derive `Id` and `EnumVariants`
- Use `Copy` when all variant data is `Copy` (enums, primitives)
- For parameterized variants, wrap existing domain enums (Room, HeatingZone, Radiator, etc.)

### State provider

```rust
pub struct MyItemStateProvider;

impl DerivedStateProvider<MyItem, OutputType> for MyItemStateProvider {
    fn calculate_current(&self, id: MyItem, ctx: &StateCalculationContext) -> Option<OutputType> {
        // Pull dependencies via ctx methods
        // Delegate to pure function for testable logic
        // Return None if dependencies are missing — never panic
    }
}
```

### StateCalculationContext API

| Method | Returns | When to use |
|--------|---------|-------------|
| `ctx.get(id)` | `Option<DataPoint<T>>` | Get current value of another home state item (triggers lazy calculation) |
| `ctx.all_since(id, since)` | `Option<DataFrame<T>>` | Get historical time-series of a home state |
| `ctx.device_state(id)` | `Option<DataPoint<T>>` | Get raw device state value |
| `ctx.user_trigger(target)` | `Option<UserTriggerExecution>` | Check for user trigger |
| `ctx.trace(id, name, value)` | `()` | Emit metric for monitoring |

**Important**: Always return `None` if any required dependency is missing. Never unwrap or panic.

### Implementation approach

For trivial logic (passthrough, simple boolean composition), implement directly.

For non-trivial calculation logic (time-series analysis, mathematical models, state machines), use the `implement-home-state` skill which provides deeper guidance on the codebase's mathematical toolkit and design patterns.

## Step 5: Wire into the Dispatch

In `app/src/home_state/items/mod.rs`:

1. Add `mod <snake_case_name>;` (alphabetical order)
2. Add `pub use <snake_case_name>::MyItem;` (alphabetical order)
3. Add variant to `HomeStateValue` enum:
   ```rust
   MyItem(MyItem, OutputType),
   ```
   The `StateEnumDerive` macro automatically generates:
   - `HomeStateId::MyItem(MyItem)` variant
   - `HomeStateItem` impl for `MyItem` with `try_downcast`
   - Type conversions

4. Add dispatch arm in `HomeStateDerivedStateProvider::calculate_current`:
   ```rust
   HomeStateId::MyItem(id) => my_item::MyItemStateProvider
       .calculate_current(id, ctx)
       .map(|value| HomeStateValue::MyItem(id, value)),
   ```

## Step 6: Create Unit Type (if needed)

Check if a suitable unit already exists in `app/src/core/unit/`:

| Unit | Type | File |
|------|------|------|
| Temperature | `DegreeCelsius` | `degree_celsius.rs` |
| Percentage | `Percent` | `percent.rs` |
| Power | `Watt` | `watt.rs` |
| Energy | `KiloWattHours` | `kwh.rs` |
| Light | `Lux` | `light.rs` |
| Air density | `GramPerCubicMeter` | (in unit module) |
| Probability | `Probability` | `probability.rs` |
| Rate of change | `RateOfChange<T>` | `derivative.rs` |
| Fan | `FanAirflow`, `FanSpeed` | `fan.rs` |
| Range | `Range<T>` | `core/range.rs` |
| Boolean | `bool` | (built-in) |

If no existing unit fits, create a new one in `app/src/core/unit/<name>.rs` following existing patterns, and export from `app/src/core/unit/mod.rs`.

## Step 7: Verify

Run all three checks:

1. `cargo build` — must compile
2. `cargo test` — all tests must pass
3. `cargo clippy` — no new warnings

Fix any issues before considering the task complete.
