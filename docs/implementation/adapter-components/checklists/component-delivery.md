# Adapter Component Delivery Checklist

Use this checklist while implementing. It is not a substitute for reading the
workflow docs.

## Before Code

- [ ] Assigned issue read.
- [ ] Issue moved to In Progress.
- [ ] Dependency checks run for Leptos and Dioxus.
- [ ] Agnostic and adapter specs read.
- [ ] Adapter foundation specs read.
- [ ] `adapter-contract.md` read.
- [ ] `examples/widgets-ownership.md` read.
- [ ] Framework skills loaded when touching Leptos/Dioxus.
- [ ] `playwright-cli` skill loaded for reference exploration.
- [ ] Reference implementation tried with `playwright-cli`, not only read.
- [ ] Implementation sketch created under `docs/implementation/sketches/`.
- [ ] Sketch records reference artifact paths and explored states.
- [ ] Sketch maps every reference outcome to ars-ui contract surfaces.
- [ ] Sketch records API/contract stance for every reference outcome, without treating React/TypeScript API shape as required parity.
- [ ] Sketch maps every user-facing string to an i18n source or consumer-owned text source.
- [ ] Sketch maps every accessible name, description, error, live region, focus path, and keyboard path.
- [ ] Sketch has no unresolved `ContractGap` rows before adapter-only coding.
- [ ] Counterpart outcome matrix written from live browser review.
- [ ] Counterpart outcome matrix records primary and fallback sources.
- [ ] `playwright-cli` reference/local browser evidence plan written.

## Adapter Code

- [ ] Component module added or updated.
- [ ] Category module wired.
- [ ] `lib.rs` category export updated if needed.
- [ ] Feature wiring updated if needed.
- [ ] Prelude exports are symmetric.
- [ ] Prop-facing config types are re-exported or aliased.
- [ ] Semantic attrs come from the agnostic API.
- [ ] User-facing adapter text comes from `MessageFn`, `Translate`, browser-native localized text, or explicit consumer-provided props.
- [ ] Locale, direction, and interpolated user text are handled without hardcoded English or BiDi-unsafe formatting.
- [ ] Consumer styling is forwarded.
- [ ] `StoredValue` / `CopyValue` used for shared captured values.
- [ ] Changed Dioxus adapter files have stable top-level hook order: no hooks inside `unwrap_or_else`, `map_or_else`, conditionals, loops, iterator adapters, nested closures, or early-return branches.
- [ ] Dioxus fallback-hook search run against the changed Dioxus files and every hit fixed or justified: `rg 'unwrap_or_else\(\|\| use_|map_or_else\([^\n]*use_' <changed-dioxus-files>`.

## Tests And Examples

- [ ] Adapter SSR/unit tests added.
- [ ] Wasm tests added for focused adapter/browser wiring that SSR cannot prove: DOM-mounted attrs, generated ids, relationship attrs, callback dispatch, and reactive DOM updates where applicable.
- [ ] Wasm tests are not being used as the only proof for full counterpart UX parity, styled visual states, axe across reached states, or cross-adapter workflows.
- [ ] E2E category aggregators updated for both adapters.
- [ ] E2E component fixture modules added for both adapters.
- [ ] E2E harness has one test per feature axis.
- [ ] E2E matrix entry covers every axis or records N/A.
- [ ] E2E/browser proof covers full user-visible workflows, computed visual states, axe-clean reached states, and Leptos/Dioxus parity for every supported counterpart outcome.
- [ ] Every browser/UX review complaint from the session has a matching E2E regression assertion, or an explicit N/A / intentionally-different matrix row with the reason.
- [ ] Axe runs across visible states.
- [ ] Keyboard and focus behavior tested for every interactive state.
- [ ] Accessible names, descriptions, error relationships, and live-region behavior tested.
- [ ] I18n tests cover translated messages, locale-sensitive formatting/parsing, pluralization, and RTL/BiDi behavior when applicable.
- [ ] Forms and validation surfaces cover valid state, each invalid reason, field-error placement below the input, status-region isolation, computed invalid styling, localized visible messages, and adapter parity.
- [ ] Computed visual assertions cover visible states.
- [ ] Widgets examples updated in all six crates.
- [ ] Widget smoke covers counterpart UX states.
- [ ] Browser evidence compares counterpart and local widgets pages.
- [ ] Widget smoke switches locale when locale controls exist and verifies component-owned visible text updates.
- [ ] No widget uses raw native controls, sibling error UI, hardcoded component text, or duplicated component policy to cover a supported parity outcome.
- [ ] Widgets and fixtures contain only consumer-application logic: sample data, controlled values, callback sinks, consumer-owned copy, routing, layout, and styling.
- [ ] No widget or fixture implements component-owned validation, ARIA relationships, keyboard/focus behavior, selection, drag/drop, loading, layout, popup state, or localized-message policy.

## Closeout

- [ ] Spec drift fixed in same PR.
- [ ] Focused checks pass.
- [ ] `cargo xclippy` run before user handoff, with changed-code warnings fixed or explicitly justified.
- [ ] `cargo xtask lint adapter-parity` passes without component skips.
- [ ] Parity audit loop pass 1 completed: reference outcomes reviewed and split by user-visible behavior.
- [ ] Parity audit loop pass 2 completed: consumer demos, all six widgets, raw-control workarounds, and hardcoded text reviewed.
- [ ] Parity audit loop pass 2 explicitly reviewed example-owned logic and found no component reimplementation in widgets or fixtures.
- [ ] Parity audit loop pass 3 completed: i18n, a11y, tests, E2E/browser proof attached to every supported row.
- [ ] Final outcome matrix has no `Unknown`, `Unverified`, `ContractGap`, `AdapterApiGap`, or `WidgetOnlyWorkaround` rows.
- [ ] Every final row is `ReferenceOutcomeMatched`, `IntentionallyDifferent`, or `OutOfScopeWithReason`.
- [ ] Every final row records `IdiomaticEquivalent`, `SameNativeBehavior`, `HigherLevelComposition`, or `OutOfScopeApiShape`.
- [ ] `post-implementation-audit` completed and findings fixed.
- [ ] Results presented before commit.
- [ ] Sketch updated with local evidence, final parity status, i18n/a11y status, and remaining N/A axes.
- [ ] User approval received before commit/push.
- [ ] `cargo xci-fast` passes before push.
- [ ] PR opened with auto-close keyword and counterpart outcome matrix.
- [ ] PR body includes browser evidence paths and parity status.
- [ ] `waiting-for-codex-review` loop completed after every push.
