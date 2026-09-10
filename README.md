# Moonlight

Moonlight is a Rust and React behavior comparer plus project-check evaluator for validating new implementations against known-good baselines.

## Landscape role

Moonlight is an evaluator, not an orchestrator or evidence collector:

- **agent-contracts** defines the neutral cross-repository evaluation result and evidence-reference contracts. Moonlight's landscape-facing target is `agent.evaluation-result/v1`; Moonlight-native run records remain internal.
- **coding-tooling** discovers and runs deterministic project checks. Moonlight may evaluate their baseline/candidate outcomes but does not own repository capability discovery.
- **runtime-profiler** produces immutable runtime evidence. Direct profiler-bundle comparison is intentionally a later adapter; the neutral `agent.evidence/v1` and `agent.evaluation-result/v1` boundaries come first.
- **coding-agent-conventions** owns policy about when evaluation is required, acceptable thresholds, and how agents should react to evidence.
- **coding-agent-skills** owns reusable reasoning procedures and flows that may request evaluation through stable interfaces; it does not embed Moonlight semantics.
- **agent-loop-orchestrator** owns candidate identity, durable run state, and the decision to request/store a Moonlight evaluation for workloads that opt into orchestration.
- **agent-loop-setup** owns machine bootstrap and component registration, not reusable worker procedures.

The intended landscape flow is:

```text
candidate + baseline + referenced evidence
                  |
                  v
              Moonlight
                  |
                  +--> Moonlight-native comparison records
                  |
                  +--> agent.evaluation-result/v1
                               |
                               v
                    agent-loop-orchestrator
```

Moonlight must keep producer-specific adapters at its edge. Adding support for a runtime-profiler bundle must not make runtime-profiler's internal schema the shared orchestration contract.

## Rust toolchain

`rust-toolchain.toml` is the canonical development, CI, release, and benchmark toolchain and currently pins Rust **1.98.1**. The workspace `rust-version = "1.87"` remains the MSRV compatibility floor; it is not the repository's development toolchain.

## Command execution

Direct argv execution is the canonical command form for deterministic CLI targets. It avoids shell startup and parsing, is easier to reason about, and is the path Moonlight dogfoods by default.

The legacy shell-string forms (`--primary`, `--candidate`, `--secondary`, batch `primary`/`candidate`/`secondary`, and eval `command`) remain accepted for backward compatibility but are deprecated. Use them only when a target genuinely requires shell semantics such as pipelines, redirects, expansion, or `&&` composition. Moonlight emits a deprecation warning when those shell forms are used.

## Install

Install the Rust CLI:

```sh
cargo install moonlight-cli --locked
moonlight run \
  --primary-argv '["printf","%s\n","{\"value\":42}"]' \
  --candidate-argv '["printf","%s\n","{\"value\":43}"]'
```

Run it through npm:

```sh
npx @moritzbrantner/moonlight run \
  --primary-argv '["printf","%s\n","{\"value\":42}"]' \
  --candidate-argv '["printf","%s\n","{\"value\":43}"]'
```

Run it through Bun:

```sh
bunx @moritzbrantner/moonlight run \
  --primary-argv '["printf","%s\n","{\"value\":42}"]' \
  --candidate-argv '["printf","%s\n","{\"value\":43}"]'
```

Evaluate a coding-agent patch against an existing project:

```sh
git diff --binary main > agent.patch
moonlight eval run --project moonlight.eval.toml --candidate-patch agent.patch --format markdown
```

Inside `moonlight.eval.toml`, prefer `argv = ["program", "arg", ...]` for checks. Retain `command = "..."` only for checks that actually need shell syntax.

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
