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
2. [07-parity-review.md](07-parity-review.md)
3. [02-adapter-api-and-wiring.md](02-adapter-api-and-wiring.md)
4. [03-framework-rules.md](03-framework-rules.md)
5. [04-adapter-tests.md](04-adapter-tests.md)
6. [05-e2e-fixtures-and-harnesses.md](05-e2e-fixtures-and-harnesses.md)
7. [06-widgets-examples.md](06-widgets-examples.md)
8. [09-browser-parity-harness.md](09-browser-parity-harness.md)
9. [08-validation-and-pr-closeout.md](08-validation-and-pr-closeout.md)

Then keep the checklists open while implementing:

- [checklists/component-delivery.md](checklists/component-delivery.md)
- [checklists/e2e-feature-matrix.md](checklists/e2e-feature-matrix.md)
- [checklists/widgets-visual-review.md](checklists/widgets-visual-review.md)

The checklist files are reminders, not substitutes for the workflow files.

## Non-Negotiable Contract

An adapter component task is complete only when the same PR includes:

- adapter crate code, module wiring, feature wiring, and symmetric prelude
  exports;
- adapter SSR/unit tests and wasm browser tests for interactive behavior;
- E2E fixtures and harness coverage for both adapters;
- matrix entries, axe coverage across visible states, and computed visual
  assertions;
- widgets examples in all six widgets crates;
- counterpart-driven visual UX review, starting with React Aria / React
  Spectrum when available, then Ark UI / Chakra UI, then Radix UI / shadcn/ui;
- repeatable browser comparison evidence collected with `playwright-cli` or an
  equivalent checked-in browser harness;
- spec synchronization for any drift surfaced during implementation;
- focused validation, `post-implementation-audit`, user review before commit,
  `cargo xci-fast`, and the Codex review loop after push.

Do not defer any of these to follow-up issues unless the issue explicitly
excludes the surface and the PR body records the exception with a reason.
