# Changelog

## 0.1.1

- Added argv command forms for `run` and `batch`, including benchmark coverage.
- Optimized CLI read commands to use exact-file JSONL access and avoid scanning sibling run files.
- Updated CI, release, Rust, and frontend dependencies for the next release wave.
- Refreshed CLI and HTTP benchmark reports.

## 0.1.0

- Initial public Moonlight CLI packaging for crates.io and npm.
- Added `moonlight` as the primary command alias alongside `moonlight-cli`.
- Added npm launcher package `@moritzbrantner/moonlight` with platform-specific
  native binary packages.
- Added release automation for crate dry-runs, native binary builds, npm package
  dry-runs, crates.io publishing, npm publishing, and GitHub Release assets.
