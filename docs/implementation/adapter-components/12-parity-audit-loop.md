# Parity Audit Loop

This workflow turns "do we have full parity with the reference?" into a
bounded, repeatable closeout loop. It applies to every visible adapter
component and runs after the first implementation is working, before the user
handoff.

The loop uses the same reference-exploration sketch created before coding. Do
not create a second plan or rely on chat history. Update the sketch in place so
the outcome matrix remains the task's durable source of truth.

## Stop Condition

Run at least three parity passes. Continue past three when any row is still
`Unknown`, `Unverified`, `ContractGap`, `AdapterApiGap`,
`WidgetOnlyWorkaround`, or missing proof.

The loop stops only when every reference outcome row is one of:

- `ReferenceOutcomeMatched`: the local component matches the reference outcome
  through ars-ui contracts, adapter wiring, widgets, tests, and browser
  evidence. The ars-ui API may differ when the sketch records an idiomatic
  Rust/Leptos/Dioxus contract stance.
- `IntentionallyDifferent`: ars-ui deliberately chooses a different
  user-visible outcome, with a spec or PR-body reason and tests for the chosen
  behavior.
- `OutOfScopeWithReason`: the reference outcome does not belong to this
  component or this issue, with a concrete boundary reason.

Do not collapse these statuses into a broad "Supported" claim during closeout.
The final handoff must show which outcomes were matched, which were
intentionally different, and which were out of scope.

## Pass 1: Reference Outcome Pass

Re-read the reference matrix and ask:

- Did the browser exploration include the simplest reference example and the
  advanced examples relevant to this component?
- Is every user-visible reference outcome represented as its own row?
- Are validation outcomes separated by failure type, not collapsed into a
  single "invalid" row when the reference distinguishes them?
- Are submit, reset, serialization, async, empty, loading, keyboard, pointer,
  focus, disabled, readonly, required, invalid, selected, open, and live-region
  outcomes split when they have different user-visible behavior?
- Does each row include reference evidence from `playwright-cli` or a
  checked-in browser harness?
- Does each row separate the reference outcome from the reference API shape?
- Does each row record an API/contract stance such as `IdiomaticEquivalent`,
  `SameNativeBehavior`, `HigherLevelComposition`, or `OutOfScopeApiShape`?

Add rows until the matrix describes outcomes, not implementation areas. For
example, "email missing @ message" and "email incomplete domain message" are
separate outcomes when the reference shows separate messages.

## Pass 2: Consumer Reality Pass

Inspect the actual widgets, fixtures, and adapter call sites:

- Are the public demos using the adapter components for the behavior under
  review, or raw native controls because the adapter API is awkward or missing?
- Are the demos acting like consumer applications, limited to sample data,
  controlled values, callback sinks, copy that is explicitly consumer-owned,
  routing, layout, and styling?
- Is any demo or fixture implementing component-owned validation policy,
  keyboard behavior, focus management, ARIA relationships, localized messages,
  selection, drag/drop, loading, layout, or popup state to make the outcome
  appear complete?
- Does the first demo visually match the simplest reference example's quality
  bar?
- Do all six widgets crates show equivalent supported states?
- Does every visible string in the demo come from `Translate`, a component
  message bundle, browser-native localized text, or documented
  consumer-provided text?
- Does switching locale or direction update labels, descriptions,
  placeholders, validation messages, button text, status text, and
  announcements?
- Do reset and submit paths visibly prove the component contract instead of
  only proving local example state?

If a demo needs raw controls, sibling error rendering, hardcoded English, or
duplicated component policy to look correct, mark the row
`WidgetOnlyWorkaround` and fix the underlying agnostic or adapter API before
claiming parity.

Also inspect the adapter API shape chosen during implementation against the
original plan. If the final shape intentionally differs, the sketch and specs
must name the better final contract and why it preserves the reference outcome.
This is especially important for public anatomy: a planned standalone part may
be the wrong boundary when it would force consumers to rebuild focus, keyboard,
ARIA, close, drag/drop, or localized-message policy. In that case, document
the behavior-critical subpart as private, expose the nearest safe public part
or typed renderer, and attach tests proving the supported customization path.
Do not leave the sketch claiming a public part exists when the implementation
correctly chose a different boundary.

## Pass 3: A11y, I18n, And Test Proof Pass

For every row that remains supported, attach proof:

- adapter SSR/unit test for semantic output;
- adapter/core test proving public part and state attrs come from the
  agnostic `Api`, not adapter-local `AttrMap` construction or row-data
  recomputation;
- wasm/browser test for focused adapter wiring that SSR cannot prove, such as
  DOM-mounted attrs, generated ids, callback dispatch, reactive DOM updates,
  focus, keyboard, form, live-region, or other DOM-only behavior;
- E2E harness assertion for the full user-visible workflow, cross-adapter
  parity, computed visual feedback, and axe-clean reached states;
- computed cursor proof for draggable or clickable compound parts when cursor
  state communicates pointer or drag affordance, covering the public shell and
  every visible interactive child before and during drag;
- widget smoke or browser evidence for the styled public demo;
- locale evidence for every user-facing message and locale-sensitive output;
- direction or BiDi evidence when layout, arrow keys, or interpolated user text
  can change meaning;
- `playwright-cli` artifact path or checked-in harness assertion for reference
  and local states.

A row without proof is `Unverified`, even if the code appears to support it.
Run the missing test or capture the missing evidence before handoff.

Do not count a wasm test as E2E proof unless the supported outcome is only the
low-level adapter/browser wiring itself. Most reference outcomes need both:
focused wasm coverage for adapter runtime integration and E2E/browser evidence
for the public outcome users can see and operate.

## Required Matrix Shape

The final matrix must include proof columns, not only prose:

| Reference outcome                     | Final status            | API/contract stance    | Reference proof                             | Local proof                             | Adapter tests                         | E2E/browser proof                 | I18n proof                       | A11y proof                     | Notes                                                |
| ------------------------------------- | ----------------------- | ---------------------- | ------------------------------------------- | --------------------------------------- | ------------------------------------- | --------------------------------- | -------------------------------- | ------------------------------ | ---------------------------------------------------- |
| Basic labelled field                  | ReferenceOutcomeMatched | IdiomaticEquivalent    | `.playwright-cli/reference-field-basic.yml` | `.playwright-cli/local-field-basic.yml` | `field_renders_label_and_description` | `field_form_semantics_are_linked` | `pt-BR widget screenshot`        | `axe field basic`              |                                                      |
| Native invalid email message variants | ReferenceOutcomeMatched | SameNativeBehavior     | `/tmp/reference-email-missing-at.png`       | `/tmp/local-email-missing-at.png`       | `form_errors_drive_named_field`       | `field_form_validation_variants`  | browser-native localized message | `aria-errormessage` assertion  |                                                      |
| Form-level custom summary             | OutOfScopeWithReason    | HigherLevelComposition | reference docs note                         | NotApplicable                           | NotApplicable                         | NotApplicable                     | consumer-provided text           | consumer-owned alert semantics | Composed by consumers, not part of `Form` primitive. |

Allowed API/contract stances: `IdiomaticEquivalent`, `SameNativeBehavior`,
`HigherLevelComposition`, `OutOfScopeApiShape`.

## Handoff Rule

Before presenting results to the user, include:

- the sketch path;
- the final matrix status counts;
- the API/contract stance counts and any rows where the reference API shape is
  intentionally not copied;
- every `IntentionallyDifferent` and `OutOfScopeWithReason` row;
- every command or artifact path used as proof;
- confirmation that no row remains `Unknown`, `Unverified`,
  `WidgetOnlyWorkaround`, `AdapterApiGap`, or `ContractGap`.

If any gap remains, say the parity status is `partial` and name the remaining
rows. Do not use `outcome-complete`.
