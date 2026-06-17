# Contributing

## TDD Workflow

Moonlight changes should start with one behavior test through the public
interface. Avoid writing a batch of imagined tests up front; use a vertical
red/green slice and let each passing slice teach the next one.

Use the project vocabulary from `CONTEXT.md` in test names and assertions:
Comparison Run, Target, Primary Reference, Secondary Reference, Candidate,
Target Observation, Suspicious Difference, Reference Noise, and Target Error.

Keep tests at the interface callers actually use:

| Behavior                       | Test location                                | Interface                               |
| ------------------------------ | -------------------------------------------- | --------------------------------------- |
| CLI command behavior           | `crates/moonlight-cli/tests/cli/**`          | `moonlight-cli` binary via `assert_cmd` |
| Comparison classification      | `crates/moonlight-core/src/compare/tests.rs` | `compare_targets`                       |
| Storage/review/report behavior | `crates/moonlight-core/src/**/tests.rs`      | public module functions/types           |
| HTTP proxy behavior            | `crates/moonlight-http/src/proxy/tests.rs`   | proxy HTTP routes and target servers    |
| UI behavior                    | `apps/moonlight-ui/src/**/*.test.{ts,tsx}`   | rendered UI and user events             |

Mock only system boundaries. UI tests may mock `../api` at the app boundary.
Rust tests should prefer real temp dirs, real command invocation, and test
support helpers over mocking internal modules.

Use this loop:

1. `RED`: add one failing behavior test.
2. `GREEN`: implement the smallest change that passes that test.
3. `REFACTOR`: clean up only after the test suite is green.

Focused commands for the loop:

```sh
cargo test -p moonlight-cli --test cli <test_name>
cargo test -p moonlight-core <test_name>
bun run tdd:ui -- <test_name>
bun run tdd:check
```

Coding agents should follow `AGENTS.md`. Humans shaping agent work should use
the guide in `docs/agents/README.md`. An `agent-ready` issue must include
acceptance criteria and verification commands before an agent starts.

Final agent handoff requires:

```sh
bun run agent:check
```

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
