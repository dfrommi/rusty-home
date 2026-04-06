# Automation

Resource-centric action planning system. Reacts to `HomeStateEvent` changes and a 30-second timer.

## Planner Pipeline

`resource_plans()` defines all devices and their prioritized rules. `plan_and_execute()` evaluates each resource's rules sequentially — first non-Skip wins, its single command is executed. `should_execute()` applies cooldown (30s minimum wait, per-type cooldown) and state-reflection checks before firing via `CommandClient`.

Each rule returns a single `Command` (not a vec). Rules are independent — they don't delegate to each other. Lower-priority rules win naturally when higher-priority ones return Skip.

## Adding or updating a rule

Use the `automation-rule` skill (structure/wiring) and `implement-rule` skill (decision logic).
