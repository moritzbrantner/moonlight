# Moonlight

Moonlight is a Rust and React behavior comparer plus project-check evaluator for validating new implementations against known-good baselines. It is independently usable from the CLI and does not require an orchestrator.

## Landscape role

Moonlight is an evaluator, not an orchestrator or evidence collector:

- **agent-contracts** defines the neutral cross-repository evaluation result and evidence-reference contracts when Moonlight exchanges results with another ecosystem component. Moonlight's landscape-facing target is `agent.evaluation-result/v1`; Moonlight-native run records remain internal.
- **coding-tooling** discovers and runs deterministic project checks. Moonlight may evaluate their baseline/candidate outcomes but does not own repository capability discovery.
- **runtime-profiler** produces immutable runtime evidence. Direct profiler-bundle comparison is intentionally a later adapter; the neutral `agent.evidence/v1` and `agent.evaluation-result/v1` boundaries come first when cross-component interchange is needed.
- **coding-agent-conventions** owns policy about when evaluation is required, acceptable thresholds, and how agents should react to evidence.
- **agent-loop-orchestrator**, when orchestrated mode is selected, owns scheduling and durable run state for that mode, the candidate identity used by the coordinated run, and the decision to request/store a Moonlight evaluation. Outside orchestrated mode these responsibilities remain with the direct caller or other coordinating environment rather than moving into Moonlight.
- **agent-loop-setup** owns reusable worker procedures and installation composition; its procedures may invoke Moonlight directly or through a coordinating runtime without embedding Moonlight semantics.

The core flow is deliberately independent:

```text
candidate + baseline + optional evidence
                  |
                  v
              Moonlight
                  |
                  +--> Moonlight-native comparison records
                  |
                  +--> optional agent.evaluation-result/v1
                               |
                               +--> orchestrator or another consumer
```

A developer, coding agent, CI job, or lightweight loop may invoke Moonlight directly. An orchestrator is one possible consumer of the neutral result, not a prerequisite for producing an evaluation.

Moonlight must keep producer-specific adapters at its edge. Adding support for a runtime-profiler bundle must not make runtime-profiler's internal schema the shared interchange contract.

## Install

Install the Rust CLI:

```sh
cargo install moonlight-cli --locked
moonlight run --primary 'printf "{\"value\":42}\n"' --candidate 'printf "{\"value\":43}\n"'
```

Run it through npm:

```sh
npx @moritzbrantner/moonlight run \
  --primary 'printf "{\"value\":42}\n"' \
  --candidate 'printf "{\"value\":43}\n"'
```

Run it through Bun:

```sh
bunx @moritzbrantner/moonlight run \
  --primary 'printf "{\"value\":42}\n"' \
  --candidate 'printf "{\"value\":43}\n"'
```

Evaluate a coding-agent patch against an existing project:

```sh
git diff --binary main > agent.patch
moonlight eval run --project moonlight.eval.toml --candidate-patch agent.patch --format markdown
```

## Agent Workflow

For coding-agent goals, create an Agent Goal issue and follow
[`AGENTS.md`](AGENTS.md). Before handoff, run:

```sh
bun run agent:check
```

Moonlight can also evaluate an agent branch or patch against the baseline:

```sh
bun run agent:eval -- --candidate-ref <branch-or-sha>
```

The GitHub Pages site explains the repository layout and shows the latest HTTP and CLI benchmark reports:

<https://moritzbrantner.github.io/moonlight/?page=overview>

For detailed local usage, see [docs/moonlight/README.md](docs/moonlight/README.md).

A local self-dogfood regression harness compares the published CLI against the current source build; see [tests/selfdogfood/README.md](tests/selfdogfood/README.md).
