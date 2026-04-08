---
applyTo: app/src/trigger/**
---
# Trigger

Manages user-initiated triggers with time-windowed activation. Follows module + client + service pattern.

## Time window logic

- **`active_from`** — optional; when execution should begin (set by planner via `set_triggers_active_from_if_unset`)
- **`active_until`** — optional; when trigger expires (set via `disable_triggers_before_except`)
- Deduplication: `get_all_active_triggers()` returns only the latest trigger per unique `UserTriggerTarget`

## Adding a new trigger

1. Add variant to `UserTrigger` in `domain/mod.rs`
2. Add corresponding `UserTriggerTarget` variant
3. Wire the trigger source (e.g., frontend, remote adapter)

