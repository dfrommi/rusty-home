---
applyTo: app/src/command/**
---
# Command

Executes commands against external smart-home systems. Follows module + client + service pattern.

Executors are tried sequentially: Tasmota → Z2M → HomeAssistant. Each returns `Ok(true)` (handled), `Ok(false)` (not mine), or `Err` (failed).

## State Reflection

Before re-executing a command, the planner checks two things in `command_state.rs`:

- **`is_reflected_in_state()`** — is the desired effect already visible in home state?
- **`min_wait_duration_between_executions()`** — per-command-type cooldown

## Adding a new command

Use the `command` skill.

