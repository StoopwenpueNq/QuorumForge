# Contributing to QuorumForge

Thanks for considering a contribution. QuorumForge is an offline engine: it
reads deliberation records and never calls a model or a network service.

## Development setup

- Rust stable for the engine: `cargo build`, `cargo test`,
  `cargo clippy --all-targets -- -D warnings`.
- Node 20+ for the viewer: `cd viewer && npm install && npm run build && npm test`.

## Before you open a pull request

1. `cargo fmt --check`, clippy with warnings denied, and `cargo test` must pass.
2. The viewer build and its test suite must pass when `viewer/` was touched.
3. Keep the engine dependency-free so verdicts reproduce anywhere.
