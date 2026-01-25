# Agent Guidelines for Rusty Home

## Build/Test Commands

- `cargo build` - Build the project
- `cargo test` - Run all tests
- `cargo test <test_name>` - Run specific test
- `cargo check` - Fast compile check without building
- `cargo clippy` - Run linter
- `cargo fmt` - Format code

## Logging Style Guide (Automation Actions)

- **Purpose**: Logs should let readers infer why an action ran or was skipped without reading code.
- **Always log the decision**: Every execution or skip should produce an info log describing the decision; do not mention specific commands.
- **Levels**:
  - **info**: decision outcomes and skip reasons.
  - **debug**: intermediate calculations only when they add understanding beyond the decision logs.
  - Avoid **trace** for rule decisions; adjust existing trace to the guideline.
- **Scope**: Don’t add common/prefix text; rely on tracing scope/module context.
- **Minimality**: Prefer minimal code changes; small structural adjustments are OK (early-return with logs is fine).
- **Placement**: Log at the point where the decision is made.
- **No input dumps**: Don’t log full input snapshots or large structured objects.
- **Delegation**: In delegating rules, log the workflow decisions there; assume delegated rules already log their own decisions.
- **Threshold phrasing**: Use human-readable wording (e.g., “more than 3 minutes”, “within 3 minutes”).
- **Complex conditions**: Prefer multiple clear decision branches over a single compound `if` so logs don’t read like “this or that”.
