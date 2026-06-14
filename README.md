# Moonlight

Moonlight is a Rust and React behavior comparer for checking candidate behavior against one or two reference targets.

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

The GitHub Pages site explains the repository layout and shows the latest HTTP and CLI benchmark reports:

<https://moritzbrantner.github.io/moonlight/?page=overview>

For detailed local usage, see [docs/moonlight/README.md](docs/moonlight/README.md).
