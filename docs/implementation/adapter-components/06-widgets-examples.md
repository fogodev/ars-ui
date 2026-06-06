# Widgets Examples

Widgets examples are not decorative. They are the public visual proof that the
adapter component feels usable and complete.

## Required Crates

Add or update the matching category module in all six widgets crates:

- `examples/widgets-leptos`
- `examples/widgets-dioxus`
- `examples/widgets-leptos-css`
- `examples/widgets-dioxus-css`
- `examples/widgets-leptos-tailwind`
- `examples/widgets-dioxus-tailwind`

Use the matching Rust module file for the spec category. Hyphenated spec
category names become underscore module filenames:

- `data-display` -> `src/categories/data_display.rs`;
- `date-time` -> `src/categories/date_time.rs`;
- all other current categories already match their module filename.

Do not put category-specific example text in root `WidgetsText`. Add it to the
category-local text enum.

Only edit `main.rs`, `text.rs`, or `categories/mod.rs` when the top-level spec
category list itself changes.

## Demo Scope

Each component demo panel should exercise the supported feature surface:

- controlled and uncontrolled state where both are relevant;
- single and multiple selection;
- disabled, readonly, invalid, selected, focused, active, loading, and empty
  states;
- checked or selected, indeterminate, focus-visible, hovered, pressed,
  required, form submit, and form reset states where relevant;
- callbacks with visible readouts;
- links and actions;
- grouping or sections;
- form participation;
- loading and empty affordances;
- drag/drop previews and drop-placement feedback;
- grid/list layout variants;
- composition slots and indicators.

A one-row smoke demo is not enough for a component with real behavior.

## Examples Are Consumers

Widgets examples must showcase the public ars-ui components. They are not a
place to build a second component implementation because the adapter API is
missing a convenient surface.

Allowed example logic:

- choosing sample data and initial prop values;
- storing values that a normal application would own, such as submitted form
  fields or the current locale switch;
- applying public component callbacks to sample data;
- choosing copy that is explicitly consumer-owned;
- arranging layout and styling around public component parts.

Forbidden example logic:

- validation policy or message selection that the component contract owns;
- keyboard navigation, roving focus, typeahead, focus restoration, or open/close
  state machines;
- ARIA relationship construction for component-owned labels, descriptions,
  errors, announcements, or form state;
- selection, disabled-state, section traversal, drag-key resolution,
  drop-target validity, preview ordering, loading suppression, or layout
  policy;
- localized component messages, plural handling, number/date formatting, or
  BiDi-safe interpolation that should come from `Translate`, a component
  message bundle, browser-native localization, or documented consumer-owned
  text.

When the demo needs any forbidden logic to look like the reference, the adapter
or agnostic API is incomplete. Fix that surface first, then make the example
consume it.

Widgets may own local demo data, such as the list of rows displayed on the
page, and may apply component-emitted callbacks to that data so the demo visibly
updates. They must not own or recompute component policy. Selection state,
disabled behavior, drag-key resolution, drop-target validity, drop placement,
live preview order, loading suppression, section traversal, and similar rules
must come from the agnostic component API and adapter render state. If an
example needs to duplicate that behavior to look correct, the component API is
missing a required surface.

The parity loop's consumer reality pass must inspect these demos as rendered
code, not only as screenshots. If a widget uses raw native controls, sibling
error elements, hardcoded English component text, or duplicated policy because
the adapter component is not ergonomic enough, mark the related outcome
`WidgetOnlyWorkaround` and fix the adapter or agnostic contract before claiming
`ReferenceOutcomeMatched`.

## Counterpart Baseline

The first public demo should be visually comparable to the chosen
counterpart's simplest documented example. React Aria / React Spectrum is the
default counterpart when available; otherwise use Ark UI / Chakra UI, then
Radix UI / shadcn/ui, then another mature library only when those do not cover
the component or feature axis. For React Aria GridList, that meant visible
checkboxes, row-wide selected feedback, and clear focus/selection affordances.

Additional sections should cover supported advanced examples from the
counterpart docs. If the counterpart shows drag/drop, empty/loading,
actions/links, grid layout, or another supported state, our widgets page should
make that state visible too.

Ignore counterpart content choices that are not component behavior. For
example, photo cards are not required just because React Aria uses photos in a
GridList drag/drop demo; the required behavior is drag image, drop preview, and
reordering feedback.

## Visual Styling Rules

Demo CSS must make features tangible:

- selected state should affect the full row/card, not only an inner text span;
- disabled items should be visibly muted and expose disabled affordance;
- loading should show visible status copy/spinner, not only an internal
  sentinel;
- drag images should represent the dragged item set, not just the clicked child;
- drop targets should highlight the full placement area;
- popup and overlay positioners should visibly anchor to their trigger;
- controls should not shrink, shift, or overlap after state changes.
- form submit and reset buttons should use ars-ui components when the demo is
  proving ars-ui component integration.

Honour `data-ars-visually-hidden`. Do not override hidden helper styles in demo
CSS.

Use SVG for visual glyphs instead of Unicode marks when geometry matters.

## Widget Smoke Checks

Every visible adapter component must have a public widget verification path
against the real example CSS. Type-checking examples is required, but it is not
widget visual coverage:

```bash
cargo check --manifest-path examples/widgets-leptos/Cargo.toml
cargo check --manifest-path examples/widgets-leptos-css/Cargo.toml
cargo check --manifest-path examples/widgets-leptos-tailwind/Cargo.toml
cargo check --manifest-path examples/widgets-dioxus/Cargo.toml
cargo check --manifest-path examples/widgets-dioxus-css/Cargo.toml
cargo check --manifest-path examples/widgets-dioxus-tailwind/Cargo.toml
```

If the category has an existing browser E2E command that drives public widget
examples, run it for every adapter/style variant the component supports. If no
such command exists, the adapter PR must add one, or add an equivalent checked-in
browser smoke harness, before claiming widget visual coverage. A valid widget
browser smoke path must load the public widgets crate in a browser, use the real
plain/CSS/Tailwind styling, inspect the console, and assert the visible states
listed below.

Do not list a dedicated `cargo xtask e2e widgets` command or focused widget
flags unless the same PR implements them.

At minimum, widget smoke should:

- navigate to the relevant category;
- perform representative pointer and keyboard interactions;
- assert clean browser console;
- assert visible selected or checked, indeterminate, active, open, disabled,
  readonly, invalid, required, focus-visible, hovered, pressed, loading, and
  drop states where supported;
- assert controls maintain stable dimensions;
- assert hidden form inputs serialize values where relevant.
- switch to at least one non-English locale when the page supports locales and
  assert component-owned labels, descriptions, placeholders, validation
  messages, status text, and button text update.

## Browser Review

Before presenting the result, use `playwright-cli` or an equivalent checked-in
browser harness to compare the local widgets page with the counterpart docs used
in the counterpart outcome matrix. Follow
[09-browser-parity-harness.md](09-browser-parity-harness.md) for snapshots,
computed style checks, console review, and artifact paths.

Record in the PR body:

- what counterpart page was inspected;
- which local widget sections map to counterpart features;
- which visual differences are intentional.

Manual browser review is not a substitute for E2E/widget-smoke coverage, but it
is required because poor UX can pass semantic tests.

## Artifacts

Route Playwright or browser-tool artifacts to `.playwright-cli/` or `/tmp/`.
Do not leak screenshots, YAML snapshots, or temporary browser output into the
repo root.
