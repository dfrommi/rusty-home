---
name: implement-rule
description: Use when a user wants to implement or update the decision logic inside an existing automation rule module — the evaluate/preconditions code, thresholds, time-series checks, and pure delegation functions.
---

# Implement Rule Logic Skill

You are implementing or updating the decision logic inside an automation rule in `app/src/automation/domain/action/`. The rule module and wiring (HomeAction enum, goal assignment) should already exist — this skill focuses on the content of the `Rule` or `SimpleRule` trait implementation.

Read `app/src/automation/CLAUDE.md` for the reference architecture and logging conventions.

## Step 1: Understand the Rule

Read the existing rule file. Identify:

- The enum variants and what each represents
- Whether it implements `Rule` or `SimpleRule`
- What `Command` variants it produces
- Any existing logic, TODOs, or empty stubs

Ask the user to describe the desired behavior in plain, high-level language. Don't ask for code-level details yet.

## Step 2: Design Discussion

Have a back-and-forth with the user to refine the logic. Use AskUserQuestion for each design decision. Cover:

1. **What state to read** — which `HomeStateItem` types are needed? Present the relevant options from the toolkit below. Consider whether the value alone suffices (`ctx.current()`) or the timestamp is needed (`ctx.current_dp()`).

2. **Decision conditions** — what thresholds, time windows, or boolean checks determine the outcome? Propose concrete values when you have enough context, but always confirm.

3. **Time behavior** — does the rule depend on how long a state has been active? Does it use daily time ranges? Does it need cooldown or minimum-on durations?

4. **Hysteresis** — if the rule toggles a device on/off based on a measurement, it likely needs hysteresis to prevent rapid switching. Discuss the band width.

5. **Edge cases** — what happens during sleep mode, ventilation, absence, or when conflicting rules run? Does the rule need to check for user overrides?

6. **Testability** — can the core logic be extracted to a pure function? Discuss what parameters it would take and what it returns. If the logic is too intertwined with context access or too exploratory, agree that extraction isn't worth it yet.

It is perfectly fine to leave parts as TODOs if the user isn't sure about specific values or behaviors. Don't force premature decisions.

## Step 3: Implement

Once the design is agreed upon, write the code following these conventions:

### Data-driven delegation pattern

The trait implementation collects data from `RuleEvaluationContext` and delegates to pure function(s) that receive only plain data:

```rust
impl Rule for MyRule {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> Result<RuleResult> {
        // 1. Collect data from context
        let temp = ctx.current(Temperature::Room(Room::Bedroom))?;
        let window = ctx.current_dp(Opened::Room(RoomWithWindow::Bedroom))?;

        // 2. Delegate to pure function
        let command = decide(temp, window);

        // 3. Map to RuleResult
        Ok(command.map_or(RuleResult::Skip, |c| RuleResult::Execute(vec![c])))
    }
}

// Pure function — no ctx, only data
fn decide(temp: DegreeCelsius, window: DataPoint<bool>) -> Option<Command> {
    // Decision logic with logging at each branch
}
```

For `SimpleRule`, the same pattern applies in `preconditions_fulfilled()`:

```rust
impl SimpleRule for MyRule {
    fn preconditions_fulfilled(&self, ctx: &RuleEvaluationContext) -> Result<bool> {
        let value = ctx.current(SomeState::Variant)?;
        Ok(should_activate(value))
    }
}

fn should_activate(value: SomeType) -> bool { /* ... */ }
```

**Skip extraction when**: the rule is a shell for the user to fill in later, or the logic requires multiple ctx calls that can't be cleanly separated (e.g., delegation to other rules like `BlockAutomation` does).

### Logging conventions

Every execution or skip must produce an `info!` log explaining the decision:

```rust
if elapsed < t!(1 minutes) {
    tracing::info!("Window open for less than 1 minute; skipping");
    return None;
} else if elapsed > t!(10 minutes) {
    tracing::info!("Window open for more than 10 minutes; stopping");
    return None;
}
tracing::info!("Window open between 1 and 10 minutes; activating");
```

Rules:
- Log at the point where the decision is made
- Don't mention specific commands in logs — describe the decision
- Use human-readable thresholds ("more than 3 minutes", not "> 180s")
- Prefer multiple clear branches over compound `if` so each branch logs its reason
- Use `debug!` for intermediate calculations, avoid `trace!`
- In delegating rules, log workflow decisions; assume delegated rules log their own

### Tests

Write tests for pure functions. No `RuleEvaluationContext` needed:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activates_when_threshold_exceeded() {
        assert!(should_activate(DegreeCelsius(25.0)));
    }

    #[test]
    fn skips_below_threshold() {
        assert!(!should_activate(DegreeCelsius(18.0)));
    }
}
```

## Decision-Logic Toolkit

Reference for the patterns and types available in this codebase.

### State access

| Method | Returns | When to use |
|--------|---------|-------------|
| `ctx.current(id)` | `S::Type` (value only) | Simple value checks, comparisons |
| `ctx.current_dp(id)` | `DataPoint<S::Type>` | Need timestamp: elapsed time, temporal ordering, time-range membership |
| `ctx.latest_trigger(target)` | `Option<&UserTriggerExecution>` | Check for user overrides |

`DataPoint<V>` has `.value: V` and `.timestamp: DateTime`.

**Critical: timestamp semantics.** Data is deduplicated — the timestamp is NOT when the last message was received. It marks when the current value became active. Repeated updates with the same value do not produce new timestamps. This means:
- `dp.timestamp.elapsed()` = how long the current value has been in effect
- A fan showing `is_on()` with `timestamp.elapsed() > t!(30 minutes)` means it has been continuously on for 30+ minutes
- A window showing `value: true` with `timestamp.elapsed() > t!(3 minutes)` means it has been open for 3+ minutes
- This is what makes elapsed-time checks meaningful for decision logic — they reflect real state duration, not polling artifacts

### The `t!()` macro

| Form | Returns | Example |
|------|---------|---------|
| `t!(now)` | `DateTime` | Current moment |
| `t!(N minutes)` | `Duration` | `t!(30 minutes)`, `t!(1 hours)`, `t!(45 seconds)` |
| `t!(N minutes ago)` | `DateTime` | `t!(10 minutes ago)` — point in past |
| `t!(HH:MM)` | `Time` | `t!(22:30)` — time of day |
| `t!(HH:MM - HH:MM)` | `DailyTimeRange` | `t!(20:00 - 11:00)` — recurring range, handles midnight crossing |

`DailyTimeRange` methods:
- `.is_now()` — is current time in range?
- `.contains(time)` — is specific time in range?
- `.active()` → `Option<DateTimeRange>` — current active period if in range
- `.active_or_previous_at(ref)` — active or most recent occurrence relative to reference

`DateTime` methods:
- `.elapsed()` → `Duration` since now
- `.elapsed_since(other)` → `Duration` between two DateTimes

`Duration` comparisons: `>`, `<`, `>=`, `<=` with other Durations (from `t!()` or `.elapsed()`).

### Hysteresis

When toggling devices based on measurements, use hysteresis to prevent rapid on/off switching:

```rust
fn hysteresis_above<T: PartialOrd>(is_active: bool, current: T, range: (T, T)) -> bool
```

- Above high threshold → enable
- Below low threshold → disable
- In between → maintain current state (`is_active`)

Example from `dehumidify.rs`:
```rust
hysteresis_above(
    current_fan_state.value.is_on(),
    current_dewpoint,
    (DegreeCelsius(10.0), DegreeCelsius(10.5)),
)
```

Note: `hysteresis_above` is defined locally in `dehumidify.rs`. If your rule needs it, either reuse it (move to a shared location) or implement a similar pattern inline.

### Domain types

| Type | Values | Operations | Used for |
|------|--------|------------|----------|
| `DegreeCelsius(f64)` | -10 to +50 typical | `+`, `-`, comparisons | Temperature, dew point |
| `Percent(f64)` | 0.0–100.0 | Comparisons | Humidity, heating demand |
| `Probability(f64)` | 0.0–1.0 | Comparisons | Occupancy |
| `FanAirflow` | `Off`, `Forward(speed)`, `Reverse(speed)` | `.is_on()`, `.is_off()` | Fan state |
| `FanSpeed` | `Silent`, `Low`, `Medium`, `High`, `Turbo` | Clone, compare | Fan control |
| `HeatingMode` | `EnergySaving`, `Comfort`, `Sleep`, `Ventilation`, `Away`, `Manual(temp, id)` | Pattern match, `==` | Heating state |
| `Range<T>` | e.g. `Range<DegreeCelsius>` | `.contains()` | Setpoints, demand limits |

### Available HomeStateItem types

These are the identifiers used with `ctx.current()` / `ctx.current_dp()`. Check `app/src/home_state/items/` for the full current list. Common ones:

| State item | Type returned | Typical variants |
|-----------|---------------|-----------------|
| `Temperature::Room(Room)` | `DegreeCelsius` | Bedroom, LivingRoom, Kitchen, etc. |
| `Temperature::Radiator(Radiator)` | `DegreeCelsius` | Bedroom, LivingRoomSmall, etc. |
| `DewPoint::Room(Room)` | `DegreeCelsius` | Bedroom, LivingRoom |
| `RiskOfMould::*` | `bool` | Bathroom |
| `FanActivity::*` | `FanAirflow` | BedroomCeilingFan, LivingRoomCeilingFan, BedroomDehumidifier |
| `Opened::Room(RoomWithWindow)` | `bool` | LivingRoom, Bedroom |
| `ColdAirComingIn::Room(RoomWithWindow)` | `bool` | LivingRoom, Bedroom |
| `Presence::*` | `bool` | AtHomeDennis, AtHomeSabine |
| `Resident::*` | `bool` | AnyoneSleeping |
| `Ventilation::Room(RoomWithWindow)` | `bool` | Bedroom, LivingRoom |
| `HeatingMode` / `TargetHeatingMode` | `HeatingMode` enum | Per HeatingZone |
| `PowerAvailable::*` | `bool` | InfraredHeater |
| `SetPoint::HeatingZone(zone)` | `Range<DegreeCelsius>` | Per zone |

### Good reference implementations

| File | Pattern demonstrated |
|------|---------------------|
| `support_with_fan.rs` | Multiple pure functions (`heating`, `ventilation`, `dehumidify`), `DataPoint` timestamp checks, temperature arithmetic |
| `inform_window_open.rs` | Collecting state across variants with `EnumVariants`, `should_send_push_notification` with temporal filtering |
| `dehumidify.rs` | `SimpleRule` with hysteresis, multiple early-return branches, elapsed-time guards |
| `block_automation.rs` | Delegation to other rules, user override checks, combining time ranges with state |

## Step 4: Verify

1. `cargo build` — must compile
2. `cargo test` — all tests pass (especially the new pure function tests)
3. `cargo clippy` — no new warnings
