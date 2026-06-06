# Adapter Component Delivery

This directory is the end-to-end workflow for implementing adapter-level
components in `crates/ars-leptos` and `crates/ars-dioxus`.

The old monolithic workflow now lives as this doc set so agents can read the
full contract in a stable order and then return to the specific layer they are
editing.

## Mandatory Reading Order

Before planning or implementing any adapter-level component task, read every
file below:

1. [01-before-you-code.md](01-before-you-code.md)
2. [10-reference-exploration-sketch.md](10-reference-exploration-sketch.md)
3. [07-parity-review.md](07-parity-review.md)
4. [11-i18n-and-a11y-support.md](11-i18n-and-a11y-support.md)
5. [02-adapter-api-and-wiring.md](02-adapter-api-and-wiring.md)
6. [03-framework-rules.md](03-framework-rules.md)
7. [04-adapter-tests.md](04-adapter-tests.md)
8. [05-e2e-fixtures-and-harnesses.md](05-e2e-fixtures-and-harnesses.md)
9. [06-widgets-examples.md](06-widgets-examples.md)
10. [09-browser-parity-harness.md](09-browser-parity-harness.md)
11. [12-parity-audit-loop.md](12-parity-audit-loop.md)
12. [08-validation-and-pr-closeout.md](08-validation-and-pr-closeout.md)

Then keep the checklists open while implementing:

- [checklists/component-delivery.md](checklists/component-delivery.md)
- [checklists/e2e-feature-matrix.md](checklists/e2e-feature-matrix.md)
- [checklists/widgets-visual-review.md](checklists/widgets-visual-review.md)

The checklist files are reminders, not substitutes for the workflow files.

## Non-Negotiable Contract

An adapter component task is complete only when the same PR includes:

- adapter crate code, module wiring, feature wiring, and symmetric prelude
  exports;
- adapter SSR/unit tests and focused wasm browser tests for adapter/browser
  wiring that SSR cannot prove;
- E2E fixtures and harness coverage for both adapters, covering complete
  user-visible workflows and outcome parity;
- matrix entries, axe coverage across visible states, and computed visual
  assertions;
- widgets examples in all six widgets crates;
- proof that widgets and fixtures behave as consumer applications: they may
  provide sample data and apply callbacks, but they must not duplicate
  component-owned validation, accessibility, selection, keyboard, focus,
  drag/drop, layout, or message policy;
- counterpart-driven visual UX review, starting with React Aria / React
  Spectrum when available, then Ark UI / Chakra UI, then Radix UI / shadcn/ui;
- a checked-in reference-exploration sketch under
  `docs/implementation/sketches/` that records `playwright-cli` reference
  evidence, outcome axes, ars-ui contract mapping, and any pre-code gaps;
- i18n and accessibility mappings for every user-facing string, validation
  message, announcement, accessible relationship, focus path, keyboard path,
  locale-sensitive output, and direction-sensitive layout behavior;
- repeatable browser comparison evidence collected with `playwright-cli` or an
  equivalent checked-in browser harness;
- a completed parity audit loop with at least three passes over reference
  outcomes, consumer reality, and i18n/a11y/test proof, leaving no unverified
  or workaround-backed supported outcomes;
- spec synchronization for any drift surfaced during implementation;
- focused validation, `post-implementation-audit`, user review before commit,
  `cargo xci-fast`, and the Codex review loop after push.

Do not defer any of these to follow-up issues unless the issue explicitly
excludes the surface and the PR body records the exception with a reason.
