# Counterpart UX And Parity Review

Counterpart review is a design input, not a late checklist. The initial
adapter, widgets examples, and E2E matrix should be shaped by the best
available counterpart before implementation starts.

The goal is maximum practical outcome parity with the chosen reference
implementation, not framework API parity and not minimum spec completion. A PR
must not say "full parity" unless the spec and PR body include a real parity
matrix that proves it.

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

Use the browser to inspect the live counterpart documentation page, not only the
API text.

For each counterpart, review:

- the simplest documented example;
- advanced examples;
- API notes;
- accessibility notes;
- visible state treatments.

The simplest counterpart example sets the minimum UX quality bar for the first
widgets demo.

## Counterpart Outcome Matrix

Before coding, write a short brief in the plan, issue note, local task note, or
PR draft:

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

| Axis          | Reference evidence | Local evidence     | Tests/E2E     | Status    | Notes                             |
| ------------- | ------------------ | ------------------ | ------------- | --------- | --------------------------------- |
| Basic         | <snapshot or note> | <snapshot or note> | <test name>   | Supported | <intentional differences>         |
| Invalid/error | <snapshot or note> | <snapshot or note> | <test name>   | Supported | <native/custom validation policy> |
| Loading       | NotApplicable      | NotApplicable      | NotApplicable | N/A       | <reason>                          |
```

Do not leave a feature gap unexplained.

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
- widgets visual coverage and widget-smoke/computed-visual assertions;
- browser comparison evidence following
  [09-browser-parity-harness.md](09-browser-parity-harness.md).

Unsupported axes must be listed as N/A with reasons in the PR body.
