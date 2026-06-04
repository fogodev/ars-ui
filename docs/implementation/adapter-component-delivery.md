# Adapter Component Delivery Workflow

This file is the mandatory entry point for adapter-level component work.

Use this workflow for any issue that adds or materially changes a component in
`crates/ars-leptos` or `crates/ars-dioxus`.

The detailed workflow is split across:

- [adapter-components/README.md](adapter-components/README.md)
- [adapter-components/01-before-you-code.md](adapter-components/01-before-you-code.md)
- [adapter-components/07-parity-review.md](adapter-components/07-parity-review.md)
- [adapter-components/02-adapter-api-and-wiring.md](adapter-components/02-adapter-api-and-wiring.md)
- [adapter-components/03-framework-rules.md](adapter-components/03-framework-rules.md)
- [adapter-components/04-adapter-tests.md](adapter-components/04-adapter-tests.md)
- [adapter-components/05-e2e-fixtures-and-harnesses.md](adapter-components/05-e2e-fixtures-and-harnesses.md)
- [adapter-components/06-widgets-examples.md](adapter-components/06-widgets-examples.md)
- [adapter-components/09-browser-parity-harness.md](adapter-components/09-browser-parity-harness.md)
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
- adapter API shape and wiring;
- framework-specific Leptos and Dioxus rules;
- adapter SSR/unit and wasm browser tests;
- E2E fixtures, harnesses, matrix entries, axe, and computed visual assertions;
- public widgets demos in all six example crates;
- repeatable browser evidence with `playwright-cli` comparing the local widgets
  page against the chosen counterpart;
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
Adapter-owned browser APIs must still consume core state. For example, native
drag image setup may live in Leptos/Dioxus because it needs `DataTransfer`, but
the dragged key set must come from `crates/ars-components`.

For details, read [adapter-components/07-parity-review.md](adapter-components/07-parity-review.md)
and [adapter-components/09-browser-parity-harness.md](adapter-components/09-browser-parity-harness.md).
