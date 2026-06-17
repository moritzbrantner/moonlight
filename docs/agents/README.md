# Agent-Ready Work

This guide explains how to create and review work that a coding agent can take
from a GitHub issue to a green pull request. Agents should follow
`AGENTS.md`; contributors should use this guide when shaping issues and
reviewing agent PRs.

## What Makes An Issue Agent-Ready

An issue is agent-ready when an agent can start without asking what "done"
means. It must include:

- a concrete outcome in user-visible or caller-visible terms
- checkbox acceptance criteria
- the public interface affected by the change
- explicit in-scope and out-of-scope boundaries
- expected behavior tests and likely test locations
- final verification commands
- migration, compatibility, or rollout constraints when relevant

If any of those are missing, keep the issue in `needs-triage` instead of
`agent-ready`.

## Good Goal Examples

Good goals describe observable behavior:

- "When `moonlight run --compact` succeeds, stdout is one JSON line and storage
  still receives the full Comparison Run."
- "The dashboard shows Target Error runs with the same review actions as
  Suspicious Difference runs."
- "Eval reports include a failed check's command name when `check.name` is
  configured."

These goals tell an agent which interface to test and what success looks like.

## Bad Goal Examples

Bad goals describe implementation wishes without a testable outcome:

- "Clean up the CLI code."
- "Make the UI nicer."
- "Refactor eval."
- "Improve performance."

Rewrite these into behavior, constraints, and verification before assigning
them to an agent.

## Acceptance Criteria Rules

Acceptance criteria should be checkboxes and should avoid hidden decisions. Each
criterion should be either verifiable by a test, a documented command, or direct
review of a named file.

Prefer:

```md
- [ ] `moonlight eval run` exits 1 when a candidate check differs from baseline.
- [ ] The JSON summary includes the failed check id and classification.
- [ ] A CLI integration test covers the failure case.
```

Avoid:

```md
- [ ] Eval is better.
- [ ] Tests are updated if needed.
```

## Choosing Verification Commands

Use focused commands while developing and full commands before handoff.

For final handoff, require:

```sh
bun run tdd:check
bun run agent:check
```

For CLI behavior, add the focused command that exercises the new test:

```sh
cargo test -p moonlight-cli --test cli <test_name>
```

For UI behavior, add:

```sh
bun run tdd:ui -- <test_name>
```

## When To Require Moonlight Eval

Use Moonlight eval when comparing an agent branch or patch against `main` adds
confidence beyond local checks. It is most useful for PR-level validation,
regressions in command behavior, and agent handoff reports.

Run:

```sh
bun run agent:eval -- --candidate-ref <branch-or-sha>
```

or:

```sh
bun run agent:eval -- --candidate-patch <patch-file>
```

The root `moonlight.eval.toml` compares exit status only for each check. This
keeps eval stable when tool output changes due to timing, compile ordering, or
test scheduling.

## How To Review Agent PRs

Review agent PRs against the linked issue, not against inferred intent.

Check that:

- the PR links the issue
- every acceptance criterion has evidence
- behavior tests were added or updated at the public interface
- `bun run agent:check` passed
- Moonlight eval was run when applicable
- the diff is scoped to the goal
- residual risk or blockers are stated clearly

If the PR stops short, ask for the blocker report described in `AGENTS.md`.
