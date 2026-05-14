---
name: automation-rule
description: Use when a user asks to create or update an automation rule, add a new home automation action, or wire a rule into the goal-based planning system.
---

# Automation Rule Skill

You are adding or updating an automation rule in the rusty-home project. Rules live in `app/src/automation/domain/action/` and are evaluated by the goal-driven planner.

Read `app/src/automation/CLAUDE.md` before starting — it contains the reference architecture and logging conventions you must follow.

## Step 1: Gather Requirements

If not already provided by the user, ask using AskUserQuestion:

- **What should the rule do**: describe the desired behavior in plain language
- **Which devices are involved**: what sensors/state it reads, what devices it controls
- **When should it activate**: conditions, time windows, thresholds, hysteresis
- **Edge cases**: what happens when data is missing, conflicting rules, user overrides

## Step 2: Choose Rule Type

Read the current rule and trait definitions in `app/src/automation/domain/action/mod.rs`.

Choose between:

| Trait | When to use |
|-------|------------|
| `SimpleRule` | Single command output, boolean preconditions. Implement `command()` and `preconditions_fulfilled()`. Automatically gets a `Rule` impl. |
| `Rule` | Multiple commands, conditional command selection, delegation to other rules, or complex return logic. Implement `evaluate()` directly. |

Present the choice to the user with reasoning.

## Step 3: Determine Goal Assignment and Priority

**This is critical and requires explicit user confirmation.**

Read `app/src/automation/domain/goal.rs` to see the current goal ordering and rule lists.

### How priority works

Goals are ordered in `get_active_goals()` from highest to lowest priority. Within each goal, rules are ordered top-to-bottom in the `rules()` vec. The planner evaluates all goals/rules, and the **first rule ready to execute for a given device (CommandTarget) wins** — later rules targeting the same device are skipped due to resource locking.

### What to present to the user

1. **Which goal** the rule belongs to — suggest one based on the rule's purpose, but always confirm
2. **Position within the goal's rule list** — explain what other rules exist in that goal and what being placed before/after them means for priority
3. If the rule targets a device that other goals also control, explain the cross-goal priority implications
4. If no existing goal fits, suggest creating a new one and where it should go in the priority order

**Never auto-assign a goal or position. Always get explicit confirmation.**

## Step 4: Determine Implementation Approach

Assess the rule's complexity:

### Straightforward rules (implement fully)

If the user can specify the exact behavior without ambiguity, use the `implement-rule` skill to design and write the decision logic. That skill provides deeper guidance on the codebase's time-series patterns, state access, hysteresis, and pure function delegation.

### Complex or ambiguous rules (create shell)

If the behavior involves tricky logic, unclear thresholds, or the user needs to experiment:

1. Create the module file with the enum and trait impl structure
2. Add TODO comments explaining what needs to be implemented
3. Do NOT add delegation functions yet — let the user design the data flow when they implement
4. Tell the user explicitly: "The rule logic is left for you to implement"

## Step 5: Create the Rule Module

Create `app/src/automation/domain/action/<snake_case_name>.rs` (or a subdirectory if it groups with related rules like `heating/`).

### Enum definition

```rust
use r#macro::Id;  // add EnumVariants if multiple variants exist

#[derive(Debug, Clone, Id)]
pub enum MyRule {
    Variant1,
    // Add variants as needed
}
```

- Use `#[derive(Debug, Clone, Id)]` always
- Add `EnumVariants` from `r#macro` only if the rule has multiple variants that need iteration
- If only one logical rule with no variants, a unit struct with `#[derive(Debug, Clone, Id)]` works too — but an enum is the convention

### SimpleRule implementation

```rust
impl SimpleRule for MyRule {
    fn command(&self) -> Command {
        // Return the command to execute
    }

    fn preconditions_fulfilled(&self, ctx: &RuleEvaluationContext) -> Result<bool> {
        // Collect data from ctx, delegate to pure function
        let value = ctx.current(SomeState::Variant)?;
        let result = should_activate(value);
        if result {
            tracing::info!("Reason why activating");
        } else {
            tracing::info!("Reason why skipping");
        }
        Ok(result)
    }
}

fn should_activate(value: SomeType) -> bool {
    // Pure decision logic — no ctx access
}
```

### Rule implementation

```rust
impl Rule for MyRule {
    fn evaluate(&self, ctx: &RuleEvaluationContext) -> Result<RuleResult> {
        // Collect data from ctx, delegate to pure function
        let command = match self {
            MyRule::Variant1 => decide(ctx.current(SomeState::X)?),
        };
        Ok(command.map_or(RuleResult::Skip, |c| RuleResult::Execute(vec![c])))
    }
}

fn decide(value: SomeType) -> Option<Command> {
    // Pure decision logic
}
```

### Logging conventions (from CLAUDE.md)

- Every execution or skip must produce an `info!` log explaining the decision
- Log at the point where the decision is made
- Don't mention specific commands in logs
- Use human-readable thresholds (e.g., "more than 3 minutes", not "> 180s")
- Prefer multiple clear decision branches over compound `if` so each branch logs its reason
- `debug!` for intermediate calculations, avoid `trace!`

### Testing (only for fully implemented rules)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        // Test the pure function directly — no RuleEvaluationContext needed
        assert!(should_activate(some_input));
    }
}
```

## Step 6: Wire into HomeAction

In `app/src/automation/domain/action/mod.rs`:

1. Add `mod <snake_case_name>;` at the top (alphabetical order)
2. Add `pub use <snake_case_name>::MyRule;` (alphabetical order)
3. Add variant to `HomeAction` enum: `MyRule(MyRule),`
4. Add match arm to `as_rule()`: `HomeAction::MyRule(r) => (r, r.ext_id()),`

The `#[derive(derive_more::From)]` on `HomeAction` auto-generates `From<MyRule>` — no manual `From` impl needed.

## Step 7: Wire into Goal

In `app/src/automation/domain/goal.rs`:

1. Add `MyRule` to the import block at the top
2. Add rule instance(s) to the confirmed goal's `rules()` match arm at the confirmed position

Example:
```rust
HomeGoal::SomeGoal => vec![
    // ... existing rules above (higher priority)
    MyRule::Variant1.into(),
    // ... existing rules below (lower priority)
],
```

## Step 8: Add Command Variants (if needed)

If the rule needs a `Command` variant that doesn't exist yet:

1. Check existing commands in `app/src/command/domain/mod.rs`
2. If a new variant is needed, add it to both `Command` and `CommandTarget` enums
3. This is a significant change — confirm with the user before proceeding

## Step 9: Verify

Run all three checks:

1. `cargo build` — must compile
2. `cargo test` — all tests must pass
3. `cargo clippy` — no new warnings

Fix any issues before considering the task complete.
