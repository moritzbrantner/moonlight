# Moonlight

Moonlight is a Rust and React behavior comparer plus project-check evaluator for validating new implementations against known-good baselines.

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

The GitHub Pages site explains the repository layout and shows the latest HTTP and CLI benchmark reports:

<https://moritzbrantner.github.io/moonlight/?page=overview>

For detailed local usage, see [docs/moonlight/README.md](docs/moonlight/README.md).

A local self-dogfood regression harness compares the published CLI against the current source build; see [tests/selfdogfood/README.md](tests/selfdogfood/README.md).
