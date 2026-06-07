# I18n And Accessibility Support

Every adapter component must preserve ars-ui's i18n and accessibility
contracts. Treat these as implementation requirements, not polish.

## I18n Gate

Before coding, identify every user-facing string and locale-sensitive behavior
in the reference implementation and in the planned widgets:

- visible labels, descriptions, placeholders, validation messages, empty states,
  loading text, status text, and error summaries;
- screen-reader-only text, live-region announcements, and generated accessible
  names;
- numbers, dates, times, lists, collation, sorting, parsing, pluralization, and
  message interpolation;
- RTL layout, arrow-key direction, BiDi user input embedded in messages, and
  locale fallback behavior.

Each item must map to one of these sources:

- `MessageFn` or another component message bundle;
- `Translate` in widgets/examples;
- browser-native localized text, with tests asserting semantics rather than
  locale-specific prose;
- explicitly consumer-provided text props or children, documented as consumer
  responsibility;
- `NotApplicable` with a reason in the sketch.

Do not hardcode English user-facing component behavior in adapters or widgets.
Hardcoded demo data is acceptable only when it is not component behavior and the
sketch says so.

Leptos adapters must expose translatable semantic text props as `TextProp` when
the text can appear in the DOM or accessibility tree after a locale switch. Use
`t(MessageKey)` as the normal call site; do not introduce a second translation
helper or wrap translated strings in ad-hoc closures inside examples. This
applies to placeholders, `aria-label`, validation/status text, live-region
announcements, and semantic labels that back custom rendered views. The `t`
helper itself returns `Memo<String>` so rendered text and `TextProp` props can
subscribe without component-owned mirror state.

Leptos consumer styling props must use `TextProp` when exposed by adapter
components: `class` always, and raw `style` when the component intentionally
exposes inline styles as an escape hatch. That keeps styling reactive without
pushing class/style branching logic into examples. Keep relationship
identifiers such as `id`, `form`, `name`, and `aria-*` IDREF props static
unless the component spec explicitly requires a reactive association.

When a component resolves adapter-owned messages, keep the selected messages
and selected locale together. Use the adapter's `use_messages_and_locale(...)`
helper instead of resolving messages and then independently reading locale for
the same render. The return type is framework-native: Leptos returns a reactive
`Signal<(M, Locale)>`; Dioxus returns a render-time `(M, Locale)` tuple because
Dioxus memoization would require a stronger `PartialEq` bound than
`ComponentMessages` provides.

Keep non-translatable DOM wiring out of that bucket. IDs, IDREF relationships,
CSS classes, inline style strings, form association IDs, submit names/values, and
other serialized browser tokens should remain explicit static values unless the
component spec defines a reactive token contract.

This applies to the rendered public examples, not only to adapter APIs. A row
is not `ReferenceOutcomeMatched` if the local widgets page stays English after
the page locale changes, unless the text is documented as consumer-owned demo
data instead of component behavior.

When a message includes user input, preserve meaning in bidirectional contexts.
Use typed message functions or explicit isolates where the relevant spec calls
for them; do not concatenate untrusted user text into localized messages unless
the component contract documents that formatting path.

## Accessibility Gate

Before coding, identify every semantic and interactive accessibility outcome:

- roles, labels, descriptions, required/invalid/disabled/readonly/busy state,
  error relationships, and live-region announcements;
- tab order, focus restoration, roving focus, focus-visible behavior, and
  keyboard shortcuts;
- pointer/touch interactions that affect focus, state, or announcements;
- disabled and readonly behavior, including whether native form controls are
  actually disabled or merely ARIA-disabled;
- axe-clean states after every visible state transition.

Every supported accessibility axis needs:

- agnostic attr/state derivation or an explicit renderer-specific reason;
- adapter tests for semantic output;
- wasm/E2E tests for focus, keyboard, live-region, and browser-only behavior;
- axe coverage after the state is reached, not only at initial render;
- browser evidence in the implementation sketch or PR body.

Do not treat a visual match as an accessibility match. The actual DOM node that
owns a control must receive the required ARIA and native attrs.

The parity audit loop must re-check accessibility after state changes. Initial
render axe coverage is not enough for states such as invalid, open, selected,
dragging, loading, submitting, or reset.

## Sketch Requirements

The reference exploration sketch must include i18n and accessibility rows for
all relevant axes. A row is incomplete when:

- a user-facing message has no localization source;
- an adapter accepts only raw strings where the component owns the message
  semantics;
- a widget demonstrates behavior with hardcoded English instead of `Translate`
  or a message bundle;
- a semantic relationship is implemented only visually;
- axe runs only before state changes;
- keyboard or focus behavior is untested for an interactive state.

Mark these as `ContractGap` and fix the underlying contract in the same PR.
If the gap survives until closeout, the final parity status is `partial`; do not
claim `outcome-complete`.
