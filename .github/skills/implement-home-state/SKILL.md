---
name: implement-home-state
description: Use when a user wants to implement or update the calculation logic inside an existing home state item — the DerivedStateProvider body, mathematical models, time-series analysis, feature engineering, and pure computation functions.
---

# Implement Home State Calculation Skill

You are implementing or updating the calculation logic inside a home state item in `app/src/home_state/items/`. The item's structural wiring (enum, `HomeStateValue` variant, dispatch) should already exist — this skill focuses on the `DerivedStateProvider::calculate_current` body and its supporting pure functions.

This is the most critical code in the smart home. The quality of these calculations determines whether the home behaves intelligently or not. Take the time to get it right.

## Step 1: Understand the Item

Read the existing item file. Identify:

- The identifier enum and its variants
- The output type
- Any existing logic, TODOs, or stubs
- What consumes this state (check automation rules in `app/src/automation/domain/action/`, other home state items, or frontends)

Ask the user to describe the desired behavior in plain, high-level language. Don't ask for code-level details yet.

## Step 2: Design Discussion

Have a back-and-forth with the user to refine the calculation. Use AskUserQuestion for each design decision. Cover:

1. **Inputs**: What data does this need? Raw device state (`ctx.device_state()`) or other derived home state (`ctx.get()`, `ctx.all_since()`)? Does it need historical data or just the current value?

2. **The relationship**: Is it a direct mapping, a physics formula, a statistical model, or a state machine? See the approach spectrum below.

3. **Model parameters**: If using a mathematical approach — what should the center, width, thresholds, or decay constants be? Propose values based on domain knowledge and physical reasoning, but always confirm.

4. **Edge cases**: What happens when inputs are missing? During transitions? At boundary conditions?

5. **Testability**: What makes a good test? Known physics values? Labeled scenarios? Boundary cases?

It is perfectly fine to leave model parameters as TODOs if the user isn't sure yet. Don't force premature decisions — this code needs iteration to get right.

## Step 3: Choose the Right Approach

The codebase uses a spectrum of approaches. Choose the simplest one that correctly models the phenomenon. Don't over-simplify continuous relationships into if-chains, but also don't force math where simple logic suffices.

### Simple passthrough / boolean composition

For items that directly map device state or compose booleans.

```rust
// opened.rs — OR of multiple window sensors
fn any_of(values: Vec<DataPoint<bool>>) -> bool {
    values.into_iter().any(|dp| dp.value)
}
```

When to use: Direct sensor readings, logical combinations, trivial mappings.

### Physics formulas

For well-known physical relationships with established formulas.

```rust
// absolute_humidity.rs — Magnus formula
fn calculate_abs_humidity(temp: DegreeCelsius, rh: Percent) -> GramPerCubicMeter {
    let mw = 18.016;
    let gk = 8214.3;
    let t0 = 273.15;
    let temp_c = temp.0;
    let sdd = 6.1078 * 10_f64.powf((7.5 * temp_c) / (237.3 + temp_c));
    let dd = rh.0 / 100.0 * sdd;
    GramPerCubicMeter(100000.0 * mw / gk * dd / (temp_c + t0))
}
```

When to use: Temperature conversions, humidity calculations, heat transfer. Validate against known reference values in tests.

### Mathematical models (sigmoid, tanh, exponential decay)

**This is the preferred approach for continuous relationships.** Instead of hardcoded thresholds with if-chains, use smooth mathematical functions that naturally model gradual transitions.

Available tools in `app/src/core/math.rs`:

**Sigmoid** — maps a continuous value to probability [0, 1]:
```rust
// Center at 10 g/m³, ~80% of transition happens within ±4 g/m³
let sigmoid = Sigmoid::around(GramPerCubicMeter(10.0), GramPerCubicMeter(4.0));
let effect: Probability = sigmoid.eval(abs_humidity);

// Or fit from two known points
let sigmoid = Sigmoid::from_example((p(0.9), 80.0), (p(0.1), 20.0));
```

**Tanh** — maps to [-1, 1], useful for bidirectional effects:
```rust
// Negative below 21°C, positive above
let tanh = Tanh::new(DegreeCelsius(21.0), 0.3);
let direction: f64 = tanh.eval(temperature);  // ∈ [-1, 1]
```

**Combining models** — multiply sigmoids/tanh for multi-factor effects:
```rust
// felt_temperature.rs — humidity correction depends on BOTH humidity AND temperature
let humidity_factor: Probability = sigmoid_humidity.eval(abs_humidity);  // [0, 1]
let temp_direction: f64 = tanh_temp.eval(temperature);                  // [-1, 1]
let correction = temp_direction * humidity_factor.factor() * max_gain;
```

When to use: Any continuous input-to-output mapping. Probabilities, compensations, risk scores, comfort indices. Prefer this over fixed thresholds.

### Time-series analysis with exponential decay

For features that should emphasize recent behavior over old history.

```rust
// occupancy.rs — presence weighted by recency
let presence_df = ctx.all_since(Presence::LivingRoomCouch, t!(1 hours ago))?;
let feature = presence_df.weighted_aged_sum(t!(30 minutes), LastSeenInterpolator);
// Higher feature value = more recent/sustained presence
```

**`weighted_aged_sum(tau, interpolator)`**: Integral of values weighted by exponential decay `e^(-t/tau)`. Recent values contribute more. The `tau` parameter controls how fast old data decays — `t!(30 minutes)` means data from 30 minutes ago has ~37% weight.

**`weighted_aged_mean(tau, interpolator)`**: Same but normalized — gives a smoothed average that emphasizes recent values. Good for comparing measurements across different time windows.

When to use: Any state that should reflect "what's been happening recently" rather than "what is true right now". Occupancy, activity levels, trend detection.

### Logistic regression (for probability outputs)

When you need a probability based on engineered features. The codebase pattern:

1. **Engineer features** from raw data (e.g., `weighted_aged_sum` of presence)
2. **Label training data** in tests — create synthetic DataFrames with known expected probabilities
3. **Fit coefficients** using `linfa` linear regression on logit-transformed labels
4. **Hardcode the fitted coefficients** in the production function

```rust
// occupancy.rs — production code uses hardcoded coefficients
let prior: f64 = -1.797;       // log-odds baseline (~14% idle probability)
let w_presence: f64 = 8.635;   // weight for presence feature
let sigmoid = Sigmoid::default();
Some(sigmoid.eval(prior + w_presence * feature))
```

```rust
// occupancy.rs — test trains the model to verify/update coefficients
#[test]
fn training() {
    let sigmoid = Sigmoid::default();
    // Training data: different presence patterns with expected probabilities
    let features = array![[feature_always_present], [feature_never_present], ...];
    let targets = array![sigmoid.inverse(p(0.9)), sigmoid.inverse(p(0.1)), ...];
    let model = LinearRegression::default().fit(&Dataset::new(features, targets)).unwrap();
    // Coefficients should match hardcoded values
}
```

This approach is **not expected to be perfect on the first try**. It requires iteration:
- Define initial training scenarios
- Fit and test
- Add more scenarios if the model behaves unexpectedly
- Adjust feature engineering or add features
- Re-fit

The skill should guide this iterative process, not try to get it right in one shot.

### Time-series comparison and rate-of-change

For items that compare values over time or detect trends.

```rust
// risk_of_mould.rs — compare bathroom dewpoint against reference rooms
let range = DateTimeRange::new(t!(3 hours ago), t!(now));
let ref_df = ctx.all_since(DewPoint::Room(Room::LivingRoom), *range.start())?;
let smoothed_reference = ref_df.weighted_aged_mean(t!(2 hours), LinearInterpolator);

// temperature_change.rs — rate of change over minimum 5 minutes
let df = ctx.all_since(Temperature::Room(room), t!(2 hours ago))?;
let rate = df.last_change(t!(5 minutes))?;  // RateOfChange<DegreeCelsius>
```

**`by_reducing2`** — combine two DataFrames point-by-point:
```rust
// heating_demand.rs — radiator temp minus room temp over time
let delta_df = DataFrame::by_reducing2(
    (&radiator_df, LinearInterpolator),
    (&room_df, LinearInterpolator),
    |rad, room| rad.value - room.value,
);
```

When to use: Trend detection, comparative analysis, derivative-based decisions.

### State machine logic

For genuinely sequential/stateful behavior.

```rust
// resident.rs — sleep detection with time-phased logic
fn sleeping(tv_on: DataPoint<bool>, ventilation: DataPoint<bool>) -> Option<bool> {
    let bed_range = t!(22:30 - 13:00).active_or_previous_at(t!(now));
    if !bed_range.is_active() { return Some(false); }
    // Phase 1: Check TV, Phase 2: Check ventilation ...
}
```

When to use: Multi-phase behavior, sequential conditions, mode transitions. Acceptable but should not be the default for continuous phenomena.

## Step 4: Implementation Conventions

### Trait impl reads context, delegates to pure function(s)

```rust
impl DerivedStateProvider<MyItem, OutputType> for MyItemStateProvider {
    fn calculate_current(&self, id: MyItem, ctx: &StateCalculationContext) -> Option<OutputType> {
        // Read data from context
        let input1 = ctx.get(SomeState::Variant)?;
        let history = ctx.all_since(OtherState::Variant, t!(1 hours ago))?;

        // Delegate to pure function (no ctx)
        let result = calculate(input1.value, history);

        // Optionally emit metric
        if let Some(ref value) = result {
            ctx.trace(id, "my_item", *value);
        }

        result
    }
}

fn calculate(input: InputType, history: DataFrame<OtherType>) -> Option<OutputType> {
    // Pure computation — testable without StateCalculationContext
}
```

### Timestamp semantics

Data is deduplicated. The timestamp marks when the current value **became active**, not when the last message was received. Repeated updates with the same value do not produce new timestamps.

- `dp.timestamp.elapsed()` = how long the current value has been in effect
- This makes elapsed-time checks meaningful — they reflect real state duration

### Return None, never panic

If any required dependency is missing, return `None`. The framework handles it gracefully. Never unwrap context calls.

### Prefer mathematical models over if-chains

When the relationship between input and output is continuous, use sigmoid, tanh, or exponential decay instead of fixed thresholds. Fixed thresholds create sharp discontinuities that cause the smart home to oscillate or behave unnaturally at boundaries.

**Instead of:**
```rust
if humidity > 70.0 { high_risk }
else if humidity > 60.0 { medium_risk }
else { low_risk }
```

**Prefer:**
```rust
let risk = Sigmoid::around(Percent(65.0), Percent(15.0)).eval(humidity);
```

That said, some decisions are genuinely discrete (is someone sleeping? is a window open?) — use boolean logic there.

### It's OK to iterate

Getting mathematical models right requires experimentation. The skill should encourage:
- Starting with a simple model and refining
- Creating training/validation data in tests
- Adjusting parameters based on observed behavior
- Adding features incrementally

### It's OK to leave TODOs

If the user isn't sure about model parameters, threshold values, or whether a feature is useful:
```rust
let prior: f64 = -1.5; // TODO: fit with real data once available
let w_feature: f64 = 5.0; // TODO: tune based on observed behavior
```

## Step 5: Testing

Test pure functions directly. Never test through `StateCalculationContext`.

### Physics validation
```rust
#[test]
fn test_known_physics_value() {
    let result = calculate_dew_point(DegreeCelsius(20.0), Percent(50.0));
    assert!((result.0 - 9.26).abs() < 0.1);
}
```

### Scenario-based testing
```rust
#[test]
fn test_high_presence_gives_high_occupancy() {
    let df = DataFrame::new(vec![DataPoint::new(true, t!(1 hours ago))]);
    let result = Occupancy::calculate(prior, w_presence, df).unwrap();
    assert!(result > p(0.8));
}
```

### Model training (for logistic regression)
```rust
#[test]
fn training() {
    // Create labeled training data
    let features = array![[feature1], [feature2], ...];
    let targets = array![sigmoid.inverse(p(0.9)), sigmoid.inverse(p(0.1)), ...];

    let model = LinearRegression::default()
        .fit(&Dataset::new(features, targets))
        .unwrap();

    // Verify or update hardcoded coefficients
    println!("Coefficients: {:?}", model.params());
    println!("Prior: {:?}", model.intercept());
}
```

### Time-series testing
```rust
#[test]
fn test_with_synthetic_timeseries() {
    let df = DataFrame::new(vec![
        DataPoint::new(DegreeCelsius(20.0), t!(30 minutes ago)),
        DataPoint::new(DegreeCelsius(22.0), t!(15 minutes ago)),
        DataPoint::new(DegreeCelsius(21.0), t!(5 minutes ago)),
    ]);
    let result = calculate_trend(df);
    // Assert expected behavior
}
```

## Toolkit Reference

### StateCalculationContext API

| Method | Returns | Use |
|--------|---------|-----|
| `ctx.get(id)` | `Option<DataPoint<T>>` | Current value of another home state (triggers lazy calculation) |
| `ctx.all_since(id, since)` | `Option<DataFrame<T>>` | Historical time-series of a home state |
| `ctx.device_state(id)` | `Option<DataPoint<T>>` | Raw device state value |
| `ctx.user_trigger(target)` | `Option<UserTriggerExecution>` | User trigger |
| `ctx.trace(id, name, value)` | `()` | Emit metric for monitoring |

### Math toolkit (`app/src/core/math.rs`)

| Tool | Purpose | Key methods |
|------|---------|-------------|
| `Sigmoid<T>` | Map continuous value to probability [0,1] | `around(center, width_p80)`, `from_example((p,v), (p,v))`, `eval(x)`, `inverse(p)` |
| `Tanh<T>` | Map to [-1, 1] range | `new(center, scale)`, `eval(x)` |
| `sigmoid(x)` | Raw sigmoid function | Returns `Probability` |
| `logit(p)` | Inverse sigmoid | Returns `f64` |
| `exp_decay_since(ts, tau)` | Exponential decay by age | Returns `f64` |
| `DataFrameStatsExt` | Time-series statistics | `weighted_aged_sum(tau, interp)`, `weighted_aged_mean(tau, interp)`, `average()` |
| `round_to_one_decimal(x)` | Round to 1 decimal | Avoids floating-point display artifacts |

### DataFrame<T> (`app/src/core/timeseries/dataframe.rs`)

| Method | Returns | Use |
|--------|---------|-----|
| `retain_range(range, start_interp, end_interp)` | `DataFrame<T>` | Window to time range with boundary interpolation |
| `last_change(min_duration)` | `Option<RateOfChange<T>>` | Rate of change to present |
| `change_at(at, min_duration)` | `Option<RateOfChange<T>>` | Rate of change at specific time |
| `fulfilled_since(predicate)` | `Option<DateTime>` | When condition last became continuously true |
| `latest_where(predicate)` | `Option<&DataPoint<T>>` | Last matching point |
| `by_reducing2(df1, df2, f)` | `DataFrame<U>` | Combine two DataFrames point-by-point |
| `last()` | `Option<&DataPoint<T>>` | Most recent point |
| `at(time, interpolator)` | `Option<DataPoint<T>>` | Value at specific time via interpolation |

### Interpolators (`app/src/core/timeseries/interpolate.rs`)

| Type | Behavior | Use for |
|------|----------|---------|
| `LastSeenInterpolator` | Step function (holds previous value) | Booleans, categories, discrete states |
| `LinearInterpolator` | Linear interpolation between points | Continuous measurements (temperature, humidity) |

### Domain types (`app/src/core/unit/`)

| Type | Range | Key traits |
|------|-------|-----------|
| `DegreeCelsius(f64)` | Typical -10 to +50 | Arithmetic (+, -, *), Ord |
| `Percent(f64)` | 0.0–100.0 | Arithmetic, clamp |
| `Probability(f64)` | 0.0–1.0 | `factor()`, `inv()`, Mul |
| `GramPerCubicMeter(f64)` | 0+ | Arithmetic |
| `RateOfChange<T>` | Any | `per_hour()`, `per_minute()`, `per(duration)` |
| `FanAirflow` | Off/Forward/Reverse | `is_on()`, `is_off()` |
| `HeatingMode` | Enum | Pattern match |

### t!() macro (`app/src/core/time/builder.rs`)

| Form | Returns | Example |
|------|---------|---------|
| `t!(now)` | `DateTime` | Current moment |
| `t!(N minutes)` | `Duration` | `t!(30 minutes)`, `t!(2 hours)` |
| `t!(N minutes ago)` | `DateTime` | `t!(1 hours ago)` |
| `t!(HH:MM - HH:MM)` | `DailyTimeRange` | `t!(22:00 - 9:00)` — handles midnight crossing |

`DailyTimeRange` methods: `.is_now()`, `.active()`, `.contains(time)`, `.active_or_previous_at(ref)`

### Reference implementations

| File | Approach | Complexity |
|------|----------|-----------|
| `items/opened.rs` | Boolean aggregation | Simple |
| `items/absolute_humidity.rs` | Magnus formula (physics) | Medium |
| `items/risk_of_mould.rs` | Time-series comparison with `weighted_aged_mean` | Medium |
| `items/felt_temperature.rs` | Multi-sigmoid/tanh compensation model | High |
| `items/occupancy.rs` | Logistic regression with `linfa` training | High |
| `items/temperature_change.rs` | Rate-of-change from DataFrame | Medium |
| `items/resident.rs` | Time-phased state machine | High |
| `items/heating/target_heating_adjustment.rs` | Multi-level state machine with projections | Very High |

## Step 6: Verify

1. `cargo build` — must compile
2. `cargo test` — all tests pass
3. `cargo clippy` — no new warnings
