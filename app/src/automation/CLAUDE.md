# Automation

Goal-driven action planning system. Reacts to `HomeStateEvent` changes and a 30-second timer.

## Planner Pipeline

`plan_and_execute()` processes all actions in three phases:

1. **Evaluate** — actions evaluate in parallel (tokio::spawn)
2. **Lock** — sequential resource lock passed action-to-action via oneshot channels; if any `CommandTarget` is already locked, the action is skipped
3. **Execute** — for each command: check `should_execute()` (30s minimum wait, per-type cooldown, state reflection), then fire via `CommandClient`. All commands in an action must acquire locks — partial execution is prevented.

## Adding or updating a rule

Use the `automation-rule` skill (structure/wiring) and `implement-rule` skill (decision logic).
