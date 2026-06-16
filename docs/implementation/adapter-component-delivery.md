# Adapter Component Delivery Workflow

This file is the mandatory entry point for adapter-level component work.

Use this workflow for any issue that adds or materially changes a component in
`crates/ars-leptos` or `crates/ars-dioxus`.

The detailed workflow is split across:

- [adapter-components/README.md](adapter-components/README.md)
- [adapter-components/01-before-you-code.md](adapter-components/01-before-you-code.md)
- [adapter-components/10-reference-exploration-sketch.md](adapter-components/10-reference-exploration-sketch.md)
- [adapter-components/07-parity-review.md](adapter-components/07-parity-review.md)
- [adapter-components/11-i18n-and-a11y-support.md](adapter-components/11-i18n-and-a11y-support.md)
- [adapter-components/02-adapter-api-and-wiring.md](adapter-components/02-adapter-api-and-wiring.md)
- [adapter-components/03-framework-rules.md](adapter-components/03-framework-rules.md)
- [adapter-components/04-adapter-tests.md](adapter-components/04-adapter-tests.md)
- [adapter-components/13-composition-integration.md](adapter-components/13-composition-integration.md)
- [adapter-components/05-e2e-fixtures-and-harnesses.md](adapter-components/05-e2e-fixtures-and-harnesses.md)
- [adapter-components/06-widgets-examples.md](adapter-components/06-widgets-examples.md)
- [adapter-components/09-browser-parity-harness.md](adapter-components/09-browser-parity-harness.md)
- [adapter-components/12-parity-audit-loop.md](adapter-components/12-parity-audit-loop.md)
- [adapter-components/14-retrofit-audits.md](adapter-components/14-retrofit-audits.md)
- [adapter-components/08-validation-and-pr-closeout.md](adapter-components/08-validation-and-pr-closeout.md)
- [adapter-components/checklists/component-delivery.md](adapter-components/checklists/component-delivery.md)
- [adapter-components/checklists/e2e-feature-matrix.md](adapter-components/checklists/e2e-feature-matrix.md)
- [adapter-components/checklists/widgets-visual-review.md](adapter-components/checklists/widgets-visual-review.md)

## Mandatory Reading Rule

Before planning or implementing an adapter-level component task, fully read:

1. this entry-point file;
2. every workflow file listed above;
3. the checklists listed above.

The checklist files are reminders, not substitutes for the workflow files. Do
not start from a checklist alone.

## Why This Is Split

Adapter delivery spans several distinct concerns:

- issue/spec dependency checks;
- counterpart-driven UX design;
- durable reference-exploration sketches written from live browser evidence;
- full i18n and accessibility contract mapping for every user-facing state;
- adapter API shape and wiring;
- framework-specific Leptos and Dioxus rules;
- adapter SSR/unit tests plus focused wasm browser tests for browser-runtime
  wiring that SSR cannot prove;
- composition integration tests with every foundation context the component
  consumes, especially `Form` and `Fieldset` for form controls;
- E2E fixtures, harnesses, matrix entries, axe, and computed visual assertions
  for complete user-visible workflows and outcome parity;
- public widgets demos in all six example crates;
- repeatable browser evidence with `playwright-cli` comparing the local widgets
  page against the chosen counterpart;
- a bounded parity audit loop that re-checks reference outcomes, consumer demo
  reality, and i18n/a11y/test proof until no outcome is unknown, unverified, or
  implemented only as a widget workaround;
- retrofit-audit rules for older adapter components that predate the current
  workflow or the latest gold-standard component shape;
- an example-consumer audit that proves widgets and fixtures showcase the
  shipped components instead of reimplementing component logic locally;
- validation, audit, PR, CI, and Codex review closeout.

Keeping these directions as separate files makes it easier to review the
relevant layer without losing the end-to-end contract.

## Highest-Risk Rule

Do not implement adapter examples from imagination. Before planning or coding,
inspect the live documentation page for the strongest counterpart, using this
preference order:

1. React Aria / React Spectrum;
2. Ark UI / Chakra UI;
3. Radix UI / shadcn/ui;
4. another mature component library only when the first three do not cover the
   primitive or feature axis.

Adapter-level components target maximum practical outcome parity with that
reference, not API-shape parity with the reference framework and not merely
minimum spec completion. The counterpart's simplest example sets the minimum UX
quality bar for the first widgets demo, and every supported counterpart feature
must map to agnostic logic, adapter wiring, adapter tests, E2E assertions,
widgets visual coverage, and repeatable browser evidence in the same PR.
Reference APIs are evidence about intended outcomes, not shapes to copy.
React Aria is TypeScript and React; ars-ui is Rust with framework-agnostic
state machines plus Leptos and Dioxus adapters. Prefer idiomatic Rust types,
state-machine boundaries, adapter callbacks, contexts, and message sources when
they produce the same user-visible, accessible, and localizable outcome. Treat
a reference API as a gap only when the underlying outcome cannot be expressed
ergonomically through ars-ui's public contract.
The same standard applies to i18n and accessibility: no user-facing text,
validation outcome, announcement, accessible name, description, keyboard path,
or locale/direction-sensitive behavior may be hardcoded in examples or adapters
unless the sketch marks it `NotApplicable` or `IntentionallyDifferent` with a
reason.

This review must produce a durable implementation sketch before code changes.
Use the `playwright-cli` skill to drive the reference implementation, capture
snapshots or screenshots for meaningful states, and write the sketch under
`docs/implementation/sketches/`. The sketch is the task's working memory: every
supported reference outcome must be mapped to ars-ui contract surfaces before
implementation starts. If a row can only be demonstrated by widget-local logic
or raw native controls, mark it as a contract gap and fix the underlying
agnostic or adapter API before continuing.

Before handoff, run the parity loop in
[adapter-components/12-parity-audit-loop.md](adapter-components/12-parity-audit-loop.md).
The loop has three required passes: reference outcomes, consumer reality, and
i18n/a11y/test proof. Continue until the final sketch matrix has no `Unknown`,
`Unverified`, `WidgetOnlyWorkaround`, `AdapterApiGap`, or `ContractGap` rows.
Only then may the PR claim `outcome-complete`.

Renderer-independent behavior belongs in `crates/ars-components` or another
shared crate before adapter wiring. Duplicating selection, layout, drag/drop,
load-more, disabled-state, section traversal, or parity rules separately in
Leptos and Dioxus is a workflow violation. Adapters should own framework event
conversion, DOM refs, observers, browser APIs, announcements, and rendering.
Widgets examples and E2E fixtures are not exempt: they may own demo data and
apply emitted callbacks to that data, but they must not recompute component
state, drag keys, drop targets, selection rules, preview order, loading
suppression, or other renderer-independent behavior that the agnostic API can
provide.
Treat every example as a consumer application, not a fallback implementation
site. Examples may choose sample records, remember form values, route callbacks
back into those records, and arrange layout. They must not implement validation
policy, keyboard behavior, roving focus, ARIA relationship construction,
localized component messages, selection algorithms, drag/drop placement, popup
state machines, or other behavior the component contract owns. If a good demo
requires local component-like logic, stop and add the missing core or adapter
surface before continuing.
Adapter-owned browser APIs must still consume core state. For example, native
drag image setup may live in Leptos/Dioxus because it needs `DataTransfer`, but
the dragged key set must come from `crates/ars-components`.

## Adapter Semantic Boundary

Adapter code may own renderer glue:

- converting framework props into agnostic `Props`;
- converting framework events into agnostic `Event`s;
- converting agnostic attrs into Leptos/Dioxus attrs;
- reading framework contexts and merging them into agnostic props;
- wiring children, slots, DOM refs, callbacks, and framework-owned browser
  handles.

Adapter code must not own component semantics. Any helper that can be tested
without Leptos, Dioxus, DOM refs, browser event types, or renderer APIs belongs
in `crates/ars-components` or another shared crate before adapter wiring. This
includes state derivation, event outcomes, `checked`/selected value mapping,
disabled/readonly mutation rules, keyboard meaning, ARIA relationship
construction, validation semantics, ids, form serialization policy, and
component-owned localized messages.

Classify every new private adapter helper before landing it:

- `renderer-glue`: needs framework attrs, views, events, hooks, refs, or DOM
  handles;
- `framework-context-merge`: reads adapter contexts and builds agnostic props;
- `component-semantics`: must move to the agnostic component API;
- `foundation-semantics`: must move to the relevant shared foundation crate.

Duplicated private helpers in both Leptos and Dioxus are presumed to be
renderer-independent until proven otherwise. Move them into the agnostic layer,
or mark the function with a short justification comment such as
`// adapter-rendering-glue: needs Dioxus event types`. Adapter-local extension
traits over agnostic `Api`, `Props`, `State`, or `Event` are not acceptable
escape hatches; add the method to the agnostic API when both adapters need it.

For details, read [adapter-components/07-parity-review.md](adapter-components/07-parity-review.md)
and [adapter-components/09-browser-parity-harness.md](adapter-components/09-browser-parity-harness.md).
