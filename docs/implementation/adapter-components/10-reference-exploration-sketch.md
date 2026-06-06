# Reference Exploration Sketch

This workflow creates durable working memory before adapter implementation
starts. It prevents counterpart details from living only in the context window
or in ad hoc browser notes.

## Required Output

Before coding, create a markdown sketch at:

```text
docs/implementation/sketches/<issue-or-task>-<component>-counterpart-sketch.md
```

Use [templates/reference-exploration-sketch.md](templates/reference-exploration-sketch.md)
as the starting point.

The sketch is not a retrospective. It is the implementation guide for the task.
Keep it open while coding and update it when new reference axes, contract gaps,
or intentional differences are discovered.

The same sketch becomes the closeout artifact. After implementation, update its
matrix through the parity audit loop in
[12-parity-audit-loop.md](12-parity-audit-loop.md) instead of writing a second
summary from memory.

## Exploration Rules

1. Load the `playwright-cli` skill before driving the reference page.
2. Use `playwright-cli`, not only screenshots in the in-app browser, for the
   repeatable exploration pass.
3. Open the strongest counterpart first, normally React Aria / React Spectrum:

    ```bash
    playwright-cli -s=reference open <counterpart-url>
    playwright-cli -s=reference snapshot --filename=.playwright-cli/reference-<component>-initial.yml
    ```

4. Try the component, do not only read the page. Exercise every visible state
   and outcome that could belong to the ars-ui component contract.
5. Capture snapshots for semantic state and screenshots only when visual
   layout, spacing, or alignment is the evidence.
6. Store artifacts under `.playwright-cli/` or `/tmp/`; do not place generated
   browser artifacts in the repo root.

## What To Explore

Review these axes when relevant:

- basic rendering and anatomy;
- user-facing text, validation messages, status messages, and announcements;
- locale-sensitive formatting, pluralization, collation, parsing, and
  bidirectional or RTL behavior;
- labels, descriptions, groups, slots, and form ownership;
- controlled and uncontrolled state;
- disabled, readonly, required, invalid, focused, focus-visible, hovered,
  pressed, selected, checked, indeterminate, loading, and empty states;
- keyboard, pointer, touch, and focus transitions;
- submit, reset, serialization, validation, and server-error flows for form
  components;
- async loading, virtualization, drag/drop, overlays, portals, and dismissal
  where the counterpart demonstrates them;
- visible feedback, stable dimensions, alignment, and accessibility attrs after
  every meaningful state transition.
- accessible names, descriptions, error relationships, live regions, focus
  order, keyboard operation, and axe-clean states after every meaningful state
  transition.

## Contract Mapping Gate

Every observed reference outcome must be mapped before implementation starts.
Use these statuses:

- `Supported`: ars-ui will expose the outcome through agnostic logic,
  renderer-specific adapter wiring, tests, widgets, and browser evidence in the
  same PR.
- `RendererSpecific`: the behavior genuinely depends on framework or browser
  APIs; both adapters still need tests or an explicit adapter-scope reason.
- `ContractGap`: the current ars-ui core, spec, or adapter API cannot express
  the outcome. Stop adapter-only coding and fix the contract first.
- `NotApplicable`: the counterpart feature does not belong to this component;
  record why.
- `IntentionallyDifferent`: ars-ui supports the axis but chooses a different
  user-visible treatment; record the reason and test the chosen outcome.

A reference API shape is not itself a required outcome. For each row, record
the API/contract stance:

- `IdiomaticEquivalent`: ars-ui exposes the same outcome through an idiomatic
  Rust, Leptos, Dioxus, or state-machine API.
- `SameNativeBehavior`: ars-ui depends on the same browser/native behavior.
- `HigherLevelComposition`: the reference API belongs above this primitive; the
  outcome is consumer-owned or belongs to a higher-level ars-ui component.
- `OutOfScopeApiShape`: the exact reference API shape is not an ars-ui goal.

Do not mark `ContractGap` merely because React Aria, Ark UI, Radix, or another
counterpart names a prop differently or exposes a React-specific hook. Mark a
gap only when the user-visible, accessible, localizable outcome cannot be
expressed through ars-ui's public contract.

A widget demo is not contract support by itself. If the behavior only works by
reimplementing component logic in the example, using raw native controls because
adapter props are missing, or rendering a sibling error message because the
component error part cannot be driven, mark the row `ContractGap`.

Hardcoded English strings in adapters or widgets are also not contract support
when the text is user-facing component behavior. Map those rows to a message
bundle, `Translate` implementation, browser-native localized message, or
documented consumer-provided string source before coding continues.

## Parity Loop Gate

Before handoff, run at least three passes over the same matrix:

1. Reference outcome pass: confirm every reference outcome is represented as a
   row and split by user-visible behavior.
2. Consumer reality pass: inspect the actual adapter usage, fixtures, and all
   six widgets for raw-control workarounds, duplicated component policy, and
   hardcoded user-facing text.
3. I18n/a11y/test proof pass: attach locale, direction, accessibility, test,
   E2E, widget-smoke, and browser evidence to every supported row.

Continue looping until no row remains `Unknown`, `Unverified`, `ContractGap`,
`AdapterApiGap`, or `WidgetOnlyWorkaround`.

The final status of each row must be one of:

- `ReferenceOutcomeMatched`;
- `IntentionallyDifferent`;
- `OutOfScopeWithReason`.

If the final matrix still has a gap or unverified row, set final parity status
to `partial` and name the rows.

## Handoff Gate

Before presenting work, update the same sketch with:

- local widget and E2E evidence paths;
- i18n evidence for user-facing text and locale/direction-sensitive states;
- accessibility evidence for semantic relationships, keyboard/focus paths, and
  axe-clean reached states;
- test names for every supported axis;
- final parity status: `outcome-complete`, `partial`, or
  `intentionally-scoped`;
- all `NotApplicable` and `IntentionallyDifferent` reasons;
- any remaining risks.

Do not present a supported axis as complete unless the sketch shows reference
evidence, local evidence, implementation surface, and tests.
