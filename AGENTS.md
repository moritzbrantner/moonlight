# Agent Operating Contract

## Mission

Implement the assigned goal until the acceptance criteria are met and required
checks are green, or stop only with a concrete blocker report.

## Required Reading

1. `CONTEXT.md`
2. `CONTRIBUTING.md`
3. The assigned GitHub issue
4. Any ADR touching the area being changed
5. Relevant tests near the target behavior

## Work Loop

1. Restate the goal and acceptance criteria.
2. Inspect the current implementation.
3. Add or update one behavior test through the public interface.
4. Run the focused test and confirm RED when practical.
5. Implement the smallest change needed for GREEN.
6. Run the focused test again.
7. Repeat until all acceptance criteria are covered.
8. Refactor only when the suite is green.
9. Run final verification.

## Test Placement

Use the test placement table in `CONTRIBUTING.md`. Tests should verify behavior
through the public interface that callers use, not private implementation
details.

## Verification Ladder

During development:

- CLI slice: `cargo test -p moonlight-cli --test cli <test_name>`
- Core slice: `cargo test -p moonlight-core <test_name>`
- HTTP slice: `cargo test -p moonlight-http <test_name>`
- UI slice: `bun run tdd:ui -- <test_name>`

Before handoff:

- `bun run tdd:check`
- `bun run agent:check`

When branch or patch comparison is useful:

- `bun run agent:eval -- --candidate-ref <branch-or-sha>`
- `bun run agent:eval -- --candidate-patch <patch-file>`

## Definition Of Done

An agent is done only when:

- acceptance criteria are satisfied
- behavior tests exist or were deliberately deemed unnecessary
- `bun run agent:check` passes
- Moonlight eval passes when applicable
- PR description includes verification evidence

## Blocker Report

If blocked, report:

- exact goal and remaining acceptance criteria
- commands run
- failing output summary
- files inspected or changed
- concrete external decision or input needed
- safest next step
