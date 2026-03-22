# Core

Shared domain types, time abstractions, time-series data structures, and unit types.

## `t!` macro (time DSL)

```rust
t!(now)                    // DateTime::now()
t!(10:30)                  // Time::at(10, 30)
t!(10:00 - 14:00)         // DailyTimeRange
t!(5 minutes)             // Duration
t!(10 minutes ago)        // DateTime
t!(in 5 hours)            // DateTime
```

## Non-obvious behaviors

- `DateTime::now()` uses a task-local override — set it in tests for deterministic time.
- `DataFrame` deduplicates on insert: consecutive identical values are silently dropped.
- Unit types (`DegreeCelsius`, `Percent`, etc.) divided by `Duration` produce `RateOfChange<T>`.
