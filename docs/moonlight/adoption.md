# Consumer Adoption

Moonlight evaluates baseline/candidate behavior. Availability of the evaluator does not make every repository comparison merge-blocking.

## Default: advisory

A new consumer starts with Moonlight as advisory evidence. The first objective is to determine whether the selected checks, normalization, environment assumptions, and baseline identity produce stable classifications on real changes.

Use this sequence:

```text
stable repository-owned check/workload
        |
        v
baseline + candidate
        |
        v
Moonlight evaluation
        |
        v
advisory result + retained evidence
        |
        `--> explicit repository promotion, if earned
```

Do not create a generic `moonlight.eval.toml` merely to say the repository has Moonlight. A comparison needs a meaningful stable baseline/candidate seam.

## Initial canaries

Prefer repositories that already have deterministic comparison inputs:

- Moonlight self-dogfood;
- `scenedetect-rs` project evaluation;
- `rect` after its deterministic benchmark/check baseline is stable;
- `dirbase` after its JSON Server parity workload is stable.

Add further consumers by workload shape rather than by repository count.

## Result states

Consumer automation must distinguish at least:

- **comparable / equivalent** — the configured comparison ran and found no policy-relevant difference;
- **comparable / changed** — the comparison ran and found a difference;
- **baseline failed** — the known-good side could not establish the expected behavior;
- **candidate failed** — the candidate could not establish the comparison input;
- **incompatible evidence/environment** — inputs cannot safely be compared;
- **unavailable** — required evaluator/check/runtime capability is absent.

Environment or evidence incompatibility is not a regression verdict. Baseline failure is not candidate success.

## Promotion to a blocking gate

Promote one specific Moonlight evaluation to merge-blocking only when all of these hold:

1. The underlying workload/check represents a real behavior or compatibility contract.
2. Baseline and candidate identities are exact and immutable for each evaluation.
3. Inputs are deterministic enough that repeated unchanged-source runs have understood variance.
4. Normalization removes only irrelevant differences and has explicit tests.
5. Environment incompatibility is reported separately from behavioral change.
6. Real canary changes have demonstrated expected positive and negative classifications.
7. False-positive regressions are effectively absent in the known canary corpus.
8. The consumer repository explicitly opts that evaluation into blocking authority.

Promotion applies to that evaluation/configuration, not to Moonlight globally. A repository may have one blocking compatibility oracle and several advisory exploratory comparisons.

## Demotion

If a blocking evaluation begins producing unexplained noise, environment-dependent verdicts, or repeated false positives, demote it to advisory until the cause is characterized and covered by regression tests. Do not train consumers to ignore a noisy red gate.

## Relationship to runtime-profiler

`runtime-profiler` captures immutable runtime facts. Moonlight may evaluate compatible evidence, but profiler capture alone has no pass/fail meaning. Before comparing profiler bundles, ensure scenario identity, metric semantics, and environment fingerprints are compatible.

Performance thresholds and acceptable regressions belong to the consumer/evaluation policy. They do not belong in runtime-profiler and should not become implicit Moonlight defaults.

## Relationship to coding-tooling

`coding-tooling` discovers and runs repository-declared deterministic capabilities. Moonlight may compare their baseline/candidate outcomes; it does not redefine those capabilities or infer missing repository commands.

The repository remains valid without Moonlight when deterministic checks alone establish completion.

## Relationship to orchestration

Agent Loop or another coordinator may request/store an evaluation and use its explicit authority. The evaluator itself does not schedule work, approve candidates, merge branches, or publish changes.
