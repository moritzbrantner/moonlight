# Moonlight

Moonlight compares command behavior against one or two reference commands.

```sh
npx @moritzbrantner/moonlight run \
  --primary 'printf "{\"value\":42}\n"' \
  --candidate 'printf "{\"value\":43}\n"'
```

The npm package installs the `moonlight` and `moonlight-cli` commands. It uses a
small JavaScript launcher plus a platform-specific native package.

If your platform is unsupported, install the Rust CLI directly:

```sh
cargo install moonlight-cli --locked
```

Set `MOONLIGHT_BIN=/path/to/moonlight` to make the npm launcher use a local
binary.
