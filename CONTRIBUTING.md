# Contributing to ars-ui

Thank you for your interest in contributing to ars-ui! This document covers the development workflow, conventions, and expectations for contributions.

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain, edition 2024)
- Rust nightly (for formatting only): `rustup toolchain install nightly`

## Getting Started

```bash
git clone https://github.com/fogodev/ars-ui.git
cd ars-ui
cargo build
```

## Development Workflow

### Building

```bash
cargo build                    # Build all workspace crates
cargo build -p spec-tool       # Build a specific crate
```

### Testing

```bash
cargo test                     # Run all tests
cargo test -p <crate>          # Run tests for a specific crate
```

### Linting

The workspace has strict clippy lints configured. All code must pass without warnings:

```bash
cargo clippy --workspace       # Run clippy on all crates
```

### Formatting

Formatting requires the nightly toolchain for import merging and grouping:

```bash
cargo +nightly fmt             # Format all code
cargo +nightly fmt -- --check  # Check formatting without modifying files
```

## Code Conventions

### Rust Style

- **No `unsafe`** -- `unsafe_code` is forbidden at the workspace level. If a crate genuinely needs it, document the justification.
- **No `#[allow]`** -- use `#[expect]` instead, so the suppression warns you when the underlying issue is fixed.
- **No `.unwrap()`** -- use `.expect("reason")` to document why the call cannot fail.
- **Explicit clones on smart pointers** -- write `Rc::clone(&x)` or `Arc::clone(&x)` instead of `x.clone()`.
- **Safe casts** -- prefer `i32::from(x)` over `x as i32` when the conversion is lossless.

### Imports

Imports are formatted by `rustfmt` with crate-level granularity and grouped by origin:

```rust
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use serde::Deserialize;
```

### Commit Messages

- Use imperative mood ("Add feature", not "Added feature")
- Keep the first line under 72 characters
- Reference issues where relevant

## Project Structure

```text
ars-ui/
  crates/          Workspace crates (ars-core, ars-a11y, ars-i18n, etc.)
  tools/           Development tools (spec-tool)
  spec/            Component specification documents
```

## Submitting Changes

1. Fork the repository and create a branch from `main`
2. Make your changes, ensuring `cargo clippy` and `cargo +nightly fmt --check` pass
3. Write tests for new functionality
4. Open a pull request with a clear description of what changed and why

## Spec Synchronization

If your change affects the specification or adapter layers, review the
[Adapter contract reference](docs/implementation/adapter-contract.md) before
opening a PR. The PR template includes an adapter sync checklist that must be
completed for any adapter or framework-specific work.

## License

By contributing to ars-ui, you agree that your contributions will be licensed under the same terms as the project: MIT OR Apache-2.0, at the user's choice.
