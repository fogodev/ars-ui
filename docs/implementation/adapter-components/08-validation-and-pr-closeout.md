# Validation And PR Closeout

Run the commands named by the issue. Add the focused checks below for the
surfaces touched by the task.

## Adapter Checks

For Leptos:

```bash
cargo check -p ars-leptos
cargo test -p ars-leptos --test <component>
```

For Dioxus:

```bash
cargo check -p ars-dioxus
cargo test -p ars-dioxus --test <component>
```

For browser-backed adapter tests, use the repo browser-test environment
documented in `AGENTS.md`.

## E2E Checks

```bash
cargo check -p ars-e2e
cargo xtask e2e --help
cargo xtask e2e <category> --help
cargo xtask e2e <category> --adapter leptos
cargo xtask e2e <category> --adapter dioxus
```

Use only flags shown by the current `--help` output. If focused component or
test-filter flags are useful, add them to `xtask` and the standalone E2E
harness in the same PR before documenting them as validation.

If the task adds the first E2E-covered component in a category, add the category
subcommand before documenting the validation command as available.

## Widgets Checks

```bash
cargo check --manifest-path examples/widgets-leptos/Cargo.toml
cargo check --manifest-path examples/widgets-leptos-css/Cargo.toml
cargo check --manifest-path examples/widgets-leptos-tailwind/Cargo.toml
cargo check --manifest-path examples/widgets-dioxus/Cargo.toml
cargo check --manifest-path examples/widgets-dioxus-css/Cargo.toml
cargo check --manifest-path examples/widgets-dioxus-tailwind/Cargo.toml
```

Run any existing category E2E command that exercises the widgets page. Do not
list a dedicated `cargo xtask e2e widgets` command unless the PR implements it.
For visible adapter components, the PR must also run or add a browser widget
smoke path that loads the public widgets examples with their real styling; cargo
check alone is only a compile gate. Browser evidence should follow
[09-browser-parity-harness.md](09-browser-parity-harness.md).

## Spec And Workspace Gates

Run when applicable:

```bash
cargo xtask spec validate
cargo xtask lint adapter-parity
cargo +nightly fmt --all --check
```

After user approval and before pushing:

```bash
cargo xci-fast
```

Run full `cargo xci` instead of `cargo xci-fast` when the change is broad,
touches feature-flag interactions, or modifies shared adapter infrastructure.

## Mandatory Audit

Invoke `.agents/skills/post-implementation-audit/SKILL.md` after implementation
and fix every finding in the same PR.

The audit must cover:

- spec/implementation drift;
- at least two "anything else missing?" rounds;
- test coverage across every relevant test type.

## Present Before Commit

Before any commit, present:

- changed surfaces;
- counterpart outcome matrix summary;
- counterpart outcome matrix status: outcome-complete, partial, or
  intentionally scoped;
- `playwright-cli` or browser-harness artifact paths for reference and local
  evidence;
- validation commands and results;
- known N/A axes with reasons;
- remaining risk.

Never commit or push without explicit user approval.

## PR Body

The PR body must include:

- issue auto-close keywords;
- spec references;
- counterpart outcome matrix summary;
- chosen counterpart and fallback counterparts inspected;
- Playwright/browser evidence paths;
- intentional differences from the chosen counterpart;
- parity status: outcome-complete, partial, or intentionally scoped;
- supported parity axes and N/A axes;
- validation commands;
- snapshot review note if `.snap` files changed.

After opening or updating a PR that changes snapshots, add the
`snapshot-reviewed` label.

## Codex Review Loop

After the first push and after every subsequent push, read and follow
`.agents/skills/waiting-for-codex-review/SKILL.md` through to Codex thumbs-up.

Posting `@codex review` alone is not enough. The full poll, thread triage,
fix/push/reply/resolve cycle, and re-trigger loop are required.

Do not treat the PR as merge-ready until CI is green and Codex has left a
thumbs-up.
