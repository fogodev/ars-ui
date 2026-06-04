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
cargo xtask e2e <category> --adapter leptos --component <component>
cargo xtask e2e <category> --adapter dioxus --component <component>
```

Run focused filters while debugging, then run the full relevant category before
presenting the result.

If the task adds the first E2E-covered component in a category, add the category
subcommand before documenting the validation command as available.

## Widgets Checks

```bash
cargo xtask e2e widgets --adapter leptos --style plain --category <category>
cargo xtask e2e widgets --adapter leptos --style css --category <category>
cargo xtask e2e widgets --adapter leptos --style tailwind --category <category>
cargo xtask e2e widgets --adapter dioxus --style plain --category <category>
cargo xtask e2e widgets --adapter dioxus --style css --category <category>
cargo xtask e2e widgets --adapter dioxus --style tailwind --category <category>
```

Also check the example crates:

```bash
cd examples
cargo check -p widgets-leptos -p widgets-dioxus \
  -p widgets-leptos-css -p widgets-dioxus-css \
  -p widgets-leptos-tailwind -p widgets-dioxus-tailwind
```

## Spec And Workspace Gates

Run when applicable:

```bash
cargo xtask spec validate
cargo xtask lint adapter-parity
cargo +nightly fmt --check
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
- counterpart UX brief summary;
- validation commands and results;
- known N/A axes with reasons;
- remaining risk.

Never commit or push without explicit user approval.

## PR Body

The PR body must include:

- issue auto-close keywords;
- spec references;
- counterpart UX brief;
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
