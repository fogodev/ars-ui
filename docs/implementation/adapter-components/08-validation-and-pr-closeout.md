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

Adapter wasm tests must prove focused browser-runtime wiring that SSR cannot:
DOM-mounted attributes, generated ids, relationship attrs, callback dispatch,
form event prevention, focus/keyboard/pointer paths, reactive DOM updates, and
mount cleanup where applicable. Do not treat wasm tests as a replacement for
E2E parity coverage.

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

E2E must prove full user-visible outcomes: complete workflows, Leptos/Dioxus
parity, axe-clean reached states, computed visual feedback, styled widget
behavior when routed through the harness, and every supported counterpart axis.
If a supported reference outcome has only wasm proof, the parity row is still
`Unverified` unless that outcome is strictly low-level adapter/browser wiring.

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
cargo xclippy
cargo xtask spec validate
cargo xtask lint adapter-parity
cargo +nightly fmt --all --check
```

Run `cargo xclippy` before presenting results for user review. It is the
workspace warning sweep: fix warnings in changed code at the root cause, and use
`#[expect(..., reason = "...")]` only for deliberate, justified suppressions.
Do not hand off adapter work with known warnings in the changed surface.

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

For adapter component work, the audit must also verify that the parity audit
loop in [12-parity-audit-loop.md](12-parity-audit-loop.md) completed. Do not
hand off while the sketch contains `Unknown`, `Unverified`, `ContractGap`,
`AdapterApiGap`, or `WidgetOnlyWorkaround` rows.
The audit must also verify that widgets and E2E fixtures contain only
consumer-application logic. Any example-owned validation, ARIA relationship
wiring, keyboard/focus behavior, selection policy, drag/drop policy, loading
policy, popup state machine, or component-owned localized message selection is
a component API gap unless the sketch marks the behavior as explicitly
consumer-owned.

## Present Before Commit

Before any commit, present:

- changed surfaces;
- reference-exploration sketch path and current gap status;
- counterpart outcome matrix summary;
- parity audit loop pass summary: reference outcome pass, consumer reality
  pass, and i18n/a11y/test proof pass;
- counterpart outcome matrix status counts for `ReferenceOutcomeMatched`,
  `IntentionallyDifferent`, and `OutOfScopeWithReason`;
- API/contract stance counts for `IdiomaticEquivalent`, `SameNativeBehavior`,
  `HigherLevelComposition`, and `OutOfScopeApiShape`, including any reference
  API shape intentionally not copied because ars-ui exposes the same outcome
  through an idiomatic Rust/Leptos/Dioxus contract;
- confirmation that no row remains `Unknown`, `Unverified`, `ContractGap`,
  `AdapterApiGap`, or `WidgetOnlyWorkaround`, or else a `partial` parity status
  with the remaining rows named;
- confirmation that examples and fixtures only own sample data, controlled
  values, callback sinks, consumer-owned copy, routing, layout, and styling;
- i18n support summary, including message sources and any locale/direction
  evidence;
- accessibility support summary, including roles/names/descriptions, keyboard
  and focus coverage, and axe results;
- `playwright-cli` or browser-harness artifact paths for reference and local
  evidence;
- validation commands and results;
- `cargo xclippy` warning-sweep result and any deliberate warning dispositions;
- known N/A axes with reasons;
- remaining risk.

Never commit or push without explicit user approval.

## PR Body

The PR body must include:

- issue auto-close keywords;
- spec references;
- reference-exploration sketch path;
- counterpart outcome matrix summary;
- parity audit loop pass summary and final row status counts;
- chosen counterpart and fallback counterparts inspected;
- Playwright/browser evidence paths;
- intentional differences from the chosen counterpart;
- parity status: `outcome-complete`, `partial`, or `intentionally-scoped`;
  `outcome-complete` is allowed only when every final matrix row is
  `ReferenceOutcomeMatched`, `IntentionallyDifferent`, or
  `OutOfScopeWithReason`;
- API/contract stance summary, especially any `HigherLevelComposition` or
  `OutOfScopeApiShape` rows;
- supported parity axes and N/A axes;
- every `IntentionallyDifferent` and `OutOfScopeWithReason` row with reasons;
- i18n status and intentional locale/message differences;
- accessibility status and any intentional semantic differences;
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
