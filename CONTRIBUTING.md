# Contributing

## Local Checks

Run these before opening a pull request:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
bun install --frozen-lockfile
bun run typecheck
bun run test:run
bun run build
bun run storybook:build
```

For UI performance and accessibility-oriented review, run:

```sh
scripts/ui-unlighthouse.sh
```

The CI workflow also runs dependency and advisory checks with `cargo audit` and
`cargo deny check`.

## Release Version Sync

Before cutting a release, sync all Rust and npm package versions with:

```sh
scripts/release/sync-release-version.sh <version>
```

The release workflow verifies that the workspace version, npm package versions,
optional native dependency versions, and release tag or workflow input all
match before publishing.
