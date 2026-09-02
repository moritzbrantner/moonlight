# Agent-facing GitHub Pages

Moonlight's existing Pages build publishes a small read-only machine interface for coding agents alongside the human report UI.

## Discovery

```text
https://moritzbrantner.github.io/moonlight/agent-tool.json
```

## Latest committed reports

```text
https://moritzbrantner.github.io/moonlight/reports/index.json
https://moritzbrantner.github.io/moonlight/reports/http-latest.json
https://moritzbrantner.github.io/moonlight/reports/cli-latest.json
```

The Pages build copies the same committed JSON reports that power the Moonlight UI. The generator parses the source files before publishing them, and `test:agent-pages` verifies that the published JSON is structurally discoverable and exactly matches the committed source reports.

This surface is deliberately read-only. GitHub Pages does not execute a Primary Reference, Candidate, Comparison Run, or project evaluation. New comparisons remain authoritative local Moonlight CLI operations; the Pages contract only helps agents discover Moonlight and consume already committed evidence.
