# Moonlight self-dogfood harness

This directory contains a local regression harness that uses Moonlight to compare Moonlight's own CLI behavior.

The harness treats a published/stable Moonlight CLI as the behavioral reference and the freshly built source CLI as the candidate. It is intended to catch compatibility regressions in command output, exit status, and compact JSON/JSONL shapes without trusting the candidate binary's own classification as the final oracle.

## Run it

```sh
tests/selfdogfood/run-published-vs-source.sh
```

The runner:

1. Builds the current source CLI with `cargo build --release -p moonlight-cli`.
2. Resolves the reference CLI.
3. Runs every JSONL fixture in `cases.jsonl` against both CLIs.
4. Normalizes expected volatile fields.
5. Independently diffs the normalized published output against the normalized source output.
6. Prints a concise pass/fail summary and exits non-zero if any case differs.

Temporary raw output, normalized output, generated batch input, and per-case diffs are written under `.moonlight/selfdogfood/` by default. This directory is git-ignored.

## Reference binary resolution

Resolution order is deliberately practical and independent of unpublished local state:

1. If `MOONLIGHT_PUBLISHED_BIN` is set, that exact executable is used.
2. Otherwise, the runner uses `npx -y @moritzbrantner/moonlight@latest` when `npx` is available.
3. If neither works, the runner fails with instructions to set `MOONLIGHT_PUBLISHED_BIN`.

Example with an explicit stable binary:

```sh
MOONLIGHT_PUBLISHED_BIN=/path/to/stable/moonlight tests/selfdogfood/run-published-vs-source.sh
```

## Candidate binary resolution

The candidate is always the release build produced from the current checkout:

```sh
cargo build --release -p moonlight-cli
target/release/moonlight
```

## Normalization

`normalize-output.py` normalizes only fields that are expected to vary between two otherwise equivalent runs:

- run IDs and UUID-like strings
- timestamp-like fields
- duration, elapsed, and latency fields
- absolute repository paths
- temporary directory paths
- source binary path strings
- version fields that can legitimately differ between published and source builds

The normalizer keeps meaningful behavior fields such as exit codes, classifications, stdout/stderr bodies, JSON diffs, and batch counts intact.

## Add a case

Add one JSON object per line to `cases.jsonl`:

```json
{"id":"new-case","args":["run","--primary-argv","[\"python3\",\"-c\",\"print('left')\"]","--candidate-argv","[\"python3\",\"-c\",\"print('right')\"]","--compact"]}
```

Each case invokes the same inner Moonlight command on both the published and source CLIs. Prefer `--primary-argv` and `--candidate-argv` with JSON arrays instead of shell strings to avoid quoting surprises.

Batch cases may include a `batch_cases` array and use `{batch_input}` inside `args`; the runner materializes that array as a temporary JSONL input file before invoking `moonlight batch`.

## Scope and limitations

This is a compatibility/regression gate, not a proof of correctness. It checks that the source build behaves like the published CLI for a deterministic fixture matrix. It intentionally avoids broad performance assertions, real HTTP services, or mandatory CI network access in this first version.
