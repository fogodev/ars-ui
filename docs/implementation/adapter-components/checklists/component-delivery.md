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
- [ ] Component-specific usage docs read or created when the component has standalone/composed behavior.
- [ ] Framework skills loaded when touching Leptos/Dioxus.
- [ ] `playwright-cli` skill loaded for reference exploration.
- [ ] Reference implementation tried with `playwright-cli`, not only read.
- [ ] Implementation sketch created under `docs/implementation/sketches/`.
- [ ] Sketch records reference artifact paths and explored states.
- [ ] Sketch maps every reference outcome to ars-ui contract surfaces.
- [ ] For retrofit audits, a fresh audit issue exists, the current gold-standard component comparison is recorded, and old evidence was refreshed.
- [ ] Sketch records API/contract stance for every reference outcome, without treating React/TypeScript API shape as required parity.
- [ ] Sketch maps every user-facing string to an i18n source or consumer-owned text source.
- [ ] Sketch maps every accessible name, description, error, live region, focus path, and keyboard path.
- [ ] Browser-owned behavior is classified by target capability: `TypedWebDom`, `WebViewBridge`, `ServerOrSsr`, or `NoDomNative`.
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
- [ ] Adapter crate exposes unstyled primitives only; reference styled source templates live in `ars-*-components`.
- [ ] Styled crate modules/tests added when the task delivers a ready-made visual component template.
- [ ] Styled templates are organized category-first under `src/<category>/<component>/`.
- [ ] CSS styled templates include a real adjacent `.css` file that can be copied with the component source.
- [ ] CSS styled template files include comments documenting each component part and state selector for user customization.
- [ ] Tailwind styled templates keep class strings inline in the rendered component markup for editor IntelliSense.
- [ ] Styled template Rust files import only `ars_leptos::prelude::*` or `ars_dioxus::prelude::*`; any needed copied-source helper or type is re-exported from that adapter prelude first.
- [ ] Styled templates are documented as future `ars-ui` CLI source-distribution inputs, not as the final user customization boundary.
- [ ] Prop-facing config types are re-exported or aliased.
- [ ] User-facing callback, signal, text, and view props use `#[prop(into)]` / `#[props(into)]` where supported, and examples pass closures/signals/views directly instead of noisy `Callback::new`, `Signal::derive`, `ViewFn::from`, `.into()`, or `EventHandler` wrappers unless the wrapper value is intentionally reused.
- [ ] Semantic attrs come from the agnostic API.
- [ ] Every new private adapter helper is classified as renderer glue, framework-context merge, component semantics, or foundation semantics.
- [ ] Helpers that can be tested without Leptos, Dioxus, DOM refs, browser event types, or renderer APIs were moved to `ars-components` or a shared foundation crate.
- [ ] Duplicated private helpers across Leptos and Dioxus were moved to the agnostic layer, or marked with an `adapter-rendering-glue` / `adapter-framework-glue` justification comment.
- [ ] No adapter-local extension trait adds methods to agnostic `Api`, `Props`, `State`, or `Event`; shared methods live in the agnostic API.
- [ ] User-facing adapter text comes from `MessageFn`, `Translate`, browser-native localized text, or explicit consumer-provided props.
- [ ] Locale, direction, and interpolated user text are handled without hardcoded English or BiDi-unsafe formatting.
- [ ] Consumer styling is forwarded.
- [ ] Dioxus root `class`, `style`, `data-*`, `lang`, `tabindex`, and extra `aria-*` attrs flow through `GlobalAttributes`, not duplicated explicit props, unless a documented semantic or non-root-part reason requires otherwise.
- [ ] Dioxus tests prove `class:` and `style:` forwarding through global attrs for styled components.
- [ ] Multi-part components expose compound part components for stylable anatomy instead of repeated root-level `*_class` / `*_style` props.
- [ ] Dioxus compound parts use `GlobalAttributes`; Leptos compound parts expose reactive `TextProp` class/style until a broader global-attrs surface exists.
- [ ] Machine-backed compound parts use `UseMachineReturn::part_attrs` unless the part needs adapter-specific dynamic attrs, event handlers, refs, or renderer-only behavior.
- [ ] Leptos compound parts with local dynamic attrs use shared `apply_part_attrs` for final `class` / `style` merge and attr conversion.
- [ ] Dynamic Leptos attrs derived from machine part attrs use `UseMachineReturn::attr_*_memo` methods instead of component-local memo copies.
- [ ] `StoredValue` / `CopyValue` used for shared captured values.
- [ ] Changed Dioxus adapter files have stable top-level hook order: no hooks inside `unwrap_or_else`, `map_or_else`, conditionals, loops, iterator adapters, nested closures, or early-return branches.
- [ ] Dioxus fallback-hook search run against the changed Dioxus files and every hit fixed or justified: `rg 'unwrap_or_else\(\|\| use_|map_or_else\([^\n]*use_' <changed-dioxus-files>`.
- [ ] Public primitive renames ran a stale-symbol scan across adapters, tests, widgets, E2E fixtures/harnesses, specs, sketches, and `xtask` snippets.

## Tests And Examples

- [ ] Adapter SSR/unit tests added.
- [ ] Composition integration tests added for every consumed context.
- [ ] Form controls have `Form` and `Fieldset` integration tests covering submit, reset, inherited disabled/readonly/invalid, matching validation errors by `name`, and unmatched errors ignored.
- [ ] Wasm tests added for focused adapter/browser wiring that SSR cannot prove: DOM-mounted attrs, generated ids, relationship attrs, callback dispatch, and reactive DOM updates where applicable.
- [ ] Wasm tests are not being used as the only proof for full counterpart UX parity, styled visual states, axe across reached states, or cross-adapter workflows.
- [ ] E2E category aggregators updated for both adapters.
- [ ] E2E component fixture modules added for both adapters.
- [ ] E2E harness has one test per feature axis.
- [ ] E2E matrix entry covers every axis or records N/A.
- [ ] E2E/browser proof covers full user-visible workflows, computed visual states, axe-clean reached states, and Leptos/Dioxus parity for every supported counterpart outcome.
- [ ] Any E2E failure is classified as `ComponentAssertionFailed`, `HarnessSetupFailed`, or `EvidenceMissing`; only reached component assertions are treated as component behavior evidence.
- [ ] Every browser/UX review complaint from the session has a matching E2E regression assertion, or an explicit N/A / intentionally-different matrix row with the reason.
- [ ] Axe runs across visible states.
- [ ] Keyboard and focus behavior tested for every interactive state.
- [ ] Accessible names, descriptions, error relationships, and live-region behavior tested.
- [ ] I18n tests cover translated messages, locale-sensitive formatting/parsing, pluralization, and RTL/BiDi behavior when applicable.
- [ ] Forms and validation surfaces cover valid state, each invalid reason, field-error placement below the input, status-region isolation, computed invalid styling, localized visible messages, and adapter parity.
- [ ] Computed visual assertions cover visible states.
- [ ] Widgets examples updated in all six crates.
- [ ] Widgets import ready-made visual components from `ars-leptos-components` / `ars-dioxus-components` when styled templates exist.
- [ ] Plain widgets compose adapter primitives directly; CSS/Tailwind widgets import the matching styled source-template variants.
- [ ] Widget examples import adapter/framework APIs through `ars_leptos::prelude::*` or `ars_dioxus::prelude::*` as much as possible; deep adapter/framework imports are used only when an item is not intentionally prelude-exported.
- [ ] Component docs state whether consumers should use adapter primitives, styled crate templates, or future `ars-ui` copied source for customization.
- [ ] Tailwind widget examples style component anatomy with Tailwind classes, public compound parts, and/or Tailwind arbitrary variants, not Rust string CSS or private widget-only selectors.
- [ ] Component-specific usage docs updated for public standalone, form, fieldset, group, provider, or overlay composition behavior.
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
- [ ] Parity audit loop pass 2 explicitly reviewed adapter-private helpers and found no renderer-independent logic left in Leptos/Dioxus.
- [ ] `cargo xtask lint adapter-parity` semantic-boundary failures are fixed, not bypassed with unreasoned marker comments.
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
