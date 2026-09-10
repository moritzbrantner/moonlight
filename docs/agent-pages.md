# Agent-facing GitHub Pages

Moonlight's Pages build publishes a small read-only machine interface for coding agents alongside the human report UI.

## Discovery

```text
https://moritzbrantner.github.io/moonlight/agent-tool.json
```

## Committed fallback reports

```text
https://moritzbrantner.github.io/moonlight/reports/index.json
https://moritzbrantner.github.io/moonlight/reports/http-latest.json
https://moritzbrantner.github.io/moonlight/reports/cli-latest.json
```

The Pages build copies these committed JSON reports and `test:agent-pages` verifies that the published copies exactly match their source files. They remain stable fallbacks when separately published evidence is unavailable.

## Fresh CLI benchmark evidence

The agent catalog also exposes the persistent exact-head CLI benchmark evidence produced after pushes to `main`:

```text
https://raw.githubusercontent.com/moritzbrantner/moonlight/cli-benchmark-history/latest.json
https://raw.githubusercontent.com/moritzbrantner/moonlight/cli-benchmark-history/history.json
```

`latest.json` is the latest full CLI benchmark. `history.json` retains compact commit-addressed shell-versus-direct-argv performance snapshots. The human Pages overview consumes these resources too, so it does not remain pinned to the older committed CLI benchmark.

This surface is deliberately read-only. GitHub Pages and the evidence branch do not execute a Primary Reference, Candidate, Comparison Run, or project evaluation. Native/local Moonlight operations remain authoritative for producing new comparisons; the published surfaces only expose captured evidence.
