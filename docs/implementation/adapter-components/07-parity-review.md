# Counterpart UX And Parity Review

Counterpart review is a design input, not a late checklist. The initial
adapter, widgets examples, and E2E matrix should be shaped by the best
available counterpart before implementation starts. The durable output of that
review is the implementation sketch described in
[10-reference-exploration-sketch.md](10-reference-exploration-sketch.md).

The goal is maximum practical outcome parity with the chosen reference
implementation, not framework API parity and not minimum spec completion. A PR
must not say "full parity" unless the sketch, spec updates, tests, browser
evidence, and PR body include a real parity matrix that proves it.

## Outcome Parity, Not API Parity

Reference APIs explain how that framework exposes an outcome; they are not the
contract ars-ui must copy. React Aria is TypeScript and React. ars-ui should
use idiomatic Rust, framework-agnostic state machines, and Leptos/Dioxus
adapter surfaces even when that means different prop names, value types,
callbacks, contexts, or composition boundaries.

A row is a gap only when the reference outcome cannot be expressed through
ars-ui's public contract. If our API is different but reaches the same
user-visible behavior, accessibility relationships, i18n behavior, keyboard
paths, focus behavior, and state transitions, mark the row
`ReferenceOutcomeMatched` with an API stance note. If the reference API is a
higher-level composition pattern, mark the exact API shape out of scope and
decide separately whether the outcome belongs to this component.

Use these API stance values in sketches and PR bodies:

- `IdiomaticEquivalent`: ars-ui exposes the same outcome through a Rust,
  Leptos, Dioxus, or state-machine-shaped API.
- `SameNativeBehavior`: ars-ui relies on the same browser/native behavior, even
  if the framework API differs.
- `HigherLevelComposition`: the reference API describes composition above this
  primitive; the outcome is owned by consumers or a future higher-level
  component.
- `OutOfScopeApiShape`: the exact reference API shape is not an ars-ui goal.

## Source Order

Inspect external component libraries in this order:

1. React Aria / React Spectrum;
2. Ark UI / Chakra UI;
3. Radix UI / shadcn/ui;
4. another mature component library only when the first three do not cover the
   primitive or a needed feature axis.

React Aria / React Spectrum is the primary counterpart when it has the
component.

## Browser Requirement

Use `playwright-cli` to inspect and interact with the live counterpart
documentation page, not only the API text. Load the `playwright-cli` skill
before relying on command syntax, use named sessions for repeatability, and
record the commands or artifact paths in the implementation sketch.

For each counterpart, review:

- the simplest documented example;
- advanced examples;
- API notes;
- accessibility notes;
- internationalization/localization notes;
- locale, direction, translated-message, and browser-native validation behavior;
- visible state treatments.

The simplest counterpart example sets the minimum UX quality bar for the first
widgets demo.

## Counterpart Outcome Matrix

Before coding, write the outcome matrix in the implementation sketch:

```md
## Counterpart Outcome Matrix

Primary counterpart: React Aria <Component>
URL: <docs URL>
Fallback counterparts inspected:

- Ark UI / Chakra UI: <URL or NotApplicable with reason>
- Radix UI / shadcn/ui: <URL or NotApplicable with reason>

Observed examples:

- Basic: ...
- Advanced: ...
- Empty/loading: ...
- Drag/drop: ...
- Forms: ...

| Axis          | Reference evidence | Local evidence     | Tests/E2E     | Status    | API/contract stance    | Notes                             |
| ------------- | ------------------ | ------------------ | ------------- | --------- | ---------------------- | --------------------------------- |
| Basic         | <snapshot or note> | <snapshot or note> | <test name>   | Supported | IdiomaticEquivalent    | <intentional differences>         |
| Invalid/error | <snapshot or note> | <snapshot or note> | <test name>   | Supported | SameNativeBehavior     | <native/custom validation policy> |
| I18n          | <snapshot or note> | <snapshot or note> | <test name>   | Supported | IdiomaticEquivalent    | <message and locale source>       |
| Accessibility | <snapshot or note> | <snapshot or note> | <test name>   | Supported | IdiomaticEquivalent    | <name, description, focus, axe>   |
| Loading       | NotApplicable      | NotApplicable      | NotApplicable | N/A       | HigherLevelComposition | <reason>                          |
```

Do not leave a feature gap unexplained.

Each row must be classified before implementation starts:

- `Supported`: implement or expose the outcome through ars-ui contract
  surfaces, with tests and browser evidence in the same PR.
- `RendererSpecific`: explain why the behavior belongs in the adapter, then
  test both adapters.
- `ContractGap`: stop adapter-only work and update the agnostic API, adapter
  API, or spec before continuing.
- `NotApplicable`: explain why the counterpart feature does not belong to this
  component.

Demo-only behavior is not a supported parity outcome. If a widget uses
component-external state, raw native controls, or a sibling alert because the
component API cannot express the reference behavior, mark the row
`ContractGap`.

After implementation, refine the matrix with the final closeout statuses from
[12-parity-audit-loop.md](12-parity-audit-loop.md):

- `ReferenceOutcomeMatched`: local behavior matches the reference outcome
  through ars-ui contract surfaces, adapter wiring, widgets, tests, and browser
  evidence.
- `IntentionallyDifferent`: ars-ui deliberately chooses a different
  user-visible outcome, with a reason and proof for the chosen outcome.
- `OutOfScopeWithReason`: the reference outcome does not belong to this
  component or this issue, with a concrete boundary reason.

Any remaining `Unknown`, `Unverified`, `ContractGap`, `AdapterApiGap`, or
`WidgetOnlyWorkaround` row means the parity status is `partial`, not
`outcome-complete`.

## Required Audit Loop

Before handoff, run the parity loop:

1. Reference outcome pass: split and verify every observed reference outcome.
2. Consumer reality pass: inspect actual adapter usage, all six widgets, local
   examples, hardcoded text, and raw-control workarounds.
3. I18n/a11y/test proof pass: attach tests, locale evidence, accessibility
   evidence, E2E/browser assertions, and artifact paths to every supported row.

Run at least these three passes and continue until the final matrix has no
unknown, unverified, gap, or workaround-backed rows. The loop is defined in
[12-parity-audit-loop.md](12-parity-audit-loop.md).

## Axes To Review

Feature surface:

- controlled and uncontrolled state;
- disabled, readonly, invalid, required, selected or checked, indeterminate,
  active, focused, focus-visible, hovered, pressed, loading, and empty states;
- grouping, sections, slots, composition, links, actions, and forms;
- form submit and form reset behavior;
- async loading, virtualization, drag/drop, overlays, and portals where
  relevant.

Interaction surface:

- pointer and touch behavior;
- keyboard navigation;
- typeahead;
- focus restoration;
- dismissal;
- selection behavior;
- scroll behavior;
- drag/drop affordances.

Visual/UX surface:

- selected or checked, indeterminate, hovered, pressed, focused,
  focus-visible, disabled, readonly, invalid, required, and loading feedback;
- icon and control alignment;
- full-row or full-card feedback when the state applies to the whole item;
- popup anchoring;
- drag image and drop placement;
- empty/loading affordances;
- stable dimensions after state changes.

I18n surface:

- all user-facing strings, labels, descriptions, validation messages, status
  messages, and announcements;
- typed message sources (`MessageFn`, `Translate`, browser-native validation
  text, or explicitly consumer-provided text);
- pluralization, number/date/time formatting, collation, parsing, locale
  fallback, and user input interpolation;
- RTL and bidirectional text behavior when layout, arrow keys, or interpolated
  user text can change meaning.

Accessibility surface:

- accessible names, descriptions, error relationships, required/invalid/busy
  state, live-region behavior, and landmark/group semantics;
- keyboard and focus order for every interactive state;
- pointer/touch parity when it affects focus or announcements;
- axe-clean coverage after every reachable visible state.

## Outcomes

If the counterpart exposes a feature that belongs in this component's public
contract, implement it in the agnostic layer first, then wire both adapters.
Renderer-independent state and rules belong in `crates/ars-components` or
another shared crate: selection, layout metadata, section/header traversal,
disabled behavior, press policy, drag-key resolution, DnD validity, reorder
payload construction, non-DOM reorder math, load-more suppression, and
hover/press/drop-target state.

Leptos and Dioxus should own only framework event translation, live DOM refs,
element-rectangle hit testing, observers, native browser APIs such as
`DataTransfer` drag images, announcements, and rendering the attrs emitted by
the agnostic API.

Duplicating those shared rules separately in both adapters is a workflow
violation, even when the duplicate code happens to pass the local example.
The same rule applies to widgets and E2E fixtures. They may hold example data
and apply emitted events, but they must not become a second implementation of
selection, drag/drop, layout preview, section traversal, or loading policy.
When an example needs those values for styling, expose renderer-independent
render state from the agnostic API and pass it through the adapters.

Use the "consumer application" test for every demo line: would an application
developer reasonably write this code after choosing ars-ui, or is the demo
rebuilding the component to hide a missing adapter surface? Reasonable consumer
code includes sample data, controlled values, callback sinks, routing, and
layout. Rebuilt component code includes validation subreason selection, ARIA
relationship wiring, roving focus, keyboard/typeahead behavior, popup state
machines, selection algorithms, drag/drop placement, loading policy, and
component-owned localized messages. Rebuilt component code is a parity gap, not
demo glue.

If the feature belongs in a different ars-ui component, document that boundary
in the spec or PR body.

If the feature is intentionally out of scope, document the reason.

If the counterpart demonstrates a stronger UX treatment for a state we already
support, update the widgets examples and widget smoke coverage so our examples
match that standard as closely as our design system allows.

Every supported parity axis must have:

- agnostic logic or an explicit reason the behavior is renderer-specific;
- adapter wiring in both in-scope adapters;
- adapter tests for API/semantic output;
- E2E assertions for browser behavior;
- i18n assertions or documented native/consumer-provided message sources for
  every user-facing string;
- accessibility assertions for names, descriptions, roles, state attrs, focus,
  keyboard behavior, and axe-clean reached states;
- widgets visual coverage and widget-smoke/computed-visual assertions;
- browser comparison evidence following
  [09-browser-parity-harness.md](09-browser-parity-harness.md).

Unsupported axes must be listed as N/A with reasons in the PR body.
