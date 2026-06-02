# Adapter Component Delivery Workflow

This document defines the end-to-end workflow for implementing adapter-level
components. It exists so agents can deliver a component without rediscovering
the surrounding obligations every time.

Use this workflow for any issue that adds or materially changes a component in
`crates/ars-leptos` or `crates/ars-dioxus`.

## Implementation Discipline

Do not take shortcuts. When a plan, issue, spec, or review finding calls for a
specific implementation shape, implement that shape unless you first prove it is
technically impossible or incorrect. A narrower patch that happens to fix the
currently observed browser, target, example, or test is not equivalent to the
requested design.

In particular:

- Do not replace a renderer-independent adapter design with a renderer-specific
  workaround such as a browser timer, DOM-only scheduler, or platform-only
  branch unless the spec explicitly says the behavior is renderer-specific.
- Do not land a partial implementation and rely on follow-up work to restore the
  intended contract. The adapter task is complete only when the behavior is
  correct across every supported target for that adapter.
- Do not keep known semantic differences between Leptos and Dioxus, or between
  Dioxus web/desktop/mobile, merely because the currently reported reproduction
  is narrower.
- Do not use "works in the example" as evidence that the component contract is
  satisfied. Add the adapter, wasm, e2e, and widget-smoke coverage required by
  this document.
- If the clean implementation has a real blocker, stop and document the blocker
  with code evidence before choosing an alternative. The alternative must
  preserve the public contract, target parity, and test obligations.

## Source Of Truth

Start from the assigned GitHub issue, not from an epic. The issue acceptance
criteria define the delivery scope.

Before moving an adapter task to In Progress, check the component dependency
metadata:

```bash
cargo xtask spec component-deps <component> --adapter leptos
cargo xtask spec component-deps <component> --adapter dioxus
cargo xtask spec issue-deps --adapter leptos --component <component> --dry-run
cargo xtask spec issue-deps --adapter dioxus --component <component> --dry-run
```

Hard blockers come from `spec/manifest.toml` `component_deps` entries with
`kind = "requires"` or `kind = "composes", blocking = true`, plus the adapter
foundation dependencies shown by the issue-dependency report. Those blockers
must exist in both the issue body `Depends on` section and GitHub's native
`blocked_by` graph before the task is considered pickable.

`kind = "boundary"` entries are not blockers. They document feature ownership
limits such as "React Aria grid layout belongs to GridList, not Listbox" and
must appear as issue notes instead of native dependencies. See
`docs/implementation/adapter-component-dependencies.md`.

Before editing code, read:

- the framework-agnostic component spec under `spec/components/<category>/`;
- the matching adapter spec under `spec/leptos-components/<category>/` or
  `spec/dioxus-components/<category>/`;
- the relevant adapter foundation spec:
  `spec/foundation/08-adapter-leptos.md` or
  `spec/foundation/09-adapter-dioxus.md`;
- `docs/implementation/adapter-contract.md`;
- `examples/widgets-ownership.md`.

When touching Leptos or Dioxus code or specs, load the repo skill for that
framework before relying on framework APIs.

## Counterpart Library Parity Review

Adapter components should feel as complete and polished as the strongest
JavaScript/TypeScript counterparts for the same primitive. Before designing or
materially changing an adapter component, perform a parity review against
external component libraries in this preference order:

1. React Aria / React Spectrum (`https://react-aria.adobe.com/`,
   `https://react-spectrum.adobe.com/`);
2. Ark UI (`https://ark-ui.com/`);
3. Radix UI (`https://www.radix-ui.com/`);
4. another mature component library only when the first three do not cover the
   primitive or a specific feature axis.

The review is not a copy exercise and must not override the ars-ui spec without
an explicit spec update. Its purpose is to prevent us from shipping a component
that is technically accessible but visibly or behaviorally behind the ecosystem
baseline.

For each counterpart, inspect both the documented API and the live examples
when available. Record the relevant findings in the issue notes, PR body, or a
small local implementation note used during the task:

- feature surface: controlled/uncontrolled state, disabled/read-only/invalid
  states, grouping, async loading, virtualization, drag/drop, links, empty
  states, form behavior, slots, composition patterns, and callbacks;
- interaction surface: pointer, keyboard, typeahead, focus restoration,
  dismissal, selection behavior, scroll behavior, and mobile/touch affordances;
- visual/UX surface: selected/hover/focus/disabled feedback, popup anchoring,
  control dimensions, trigger/value stability after selection, scroll
  affordances, loading/empty affordances, and icon alignment.

Outcomes are mandatory:

- If the counterpart exposes a feature that belongs in this component's public
  contract, implement it in the agnostic layer first, then wire both adapters.
- If the feature belongs in a different ars-ui component (for example a grid
  layout that should be a GridList instead of Listbox), document that boundary
  in the spec or PR body.
- If the feature is intentionally out of scope, document the reason. Do not
  leave an unexplained gap.
- If the counterpart demonstrates a stronger UX treatment for a state we
  already support, update the widgets examples and widget smoke coverage so our
  examples visually match that standard as closely as our design system allows.

Any feature parity gap that remains after the PR must be explicit. "We did not
notice this counterpart feature" is not an acceptable reason for missing
coverage.

## Required Deliverables

An adapter component task is not complete when only the adapter component
compiles. It must update every surface that consumers, tests, and reviewers use.

For each implemented component, include the applicable items below in the same
PR.

### Adapter Crate

- Add or update the component implementation in
  `crates/ars-leptos/src/<category>/<component>.rs` or
  `crates/ars-dioxus/src/<category>/<component>.rs`.
- Add the module to the category `mod.rs`.
- Re-export **both** the component module _and_ the component's root
  entry-point through `crates/ars-leptos/src/prelude.rs` or
  `crates/ars-dioxus/src/prelude.rs` when the component is user-facing —
  for example, `pub use crate::selection::{listbox, listbox::Listbox};`.
  The bare root name is the compound-component anchor consumers reach
  for first (`<Listbox>…</Listbox>`); the module path stays accessible
  for child slots (`<listbox::Content>`, `<listbox::Item>`, …) and any
  exported types (`listbox::Messages`, `listbox::Part`).
- **Also re-export the framework-agnostic configuration enums** the
  component's props accept. If a consumer must construct a value like
  `Mode::Multiple`, `Behavior::Toggle`, or `Set::Empty` to call a prop,
  that enum belongs in the prelude. Use a stable, non-colliding alias
  when the bare name is too generic: `SelectionMode`, `SelectionBehavior`,
  `SelectionSet` for `ars_collections::selection::{Mode, Behavior, Set}`.
  Slot output types and machine internals stay out of the prelude — the
  rule is "anything you pass into a prop, not anything you read from a
  result".
- Keep both adapter preludes symmetric when both adapters expose the component.
- Update adapter `Cargo.toml` feature wiring if the component needs an
  `ars-components` feature to be reachable.
- Preserve the adapter layering: adapter crates may render framework views and
  browser integration, but component behavior belongs in `ars-components`.

#### Component-owned attributes must come from the agnostic API

Adapters must not hand-author component anatomy or semantic attributes such as
`data-ars-scope`, `data-ars-part`, `data-ars-state`, `data-ars-selected`,
`role`, `aria-*`, `tabindex`, `hidden`, `disabled`, `draggable`, or form-participation
attributes. Those attributes belong in the framework-agnostic component API
(`Api::<part>_attrs()` / `ConnectApi::part_attrs`) so every adapter receives the
same contract and future adapters do not need to rediscover component-specific
rules.

This includes conditional and reactive attributes. If a prop or state toggles a
DOM contract value such as Tabs'
`aria-roledescription="draggable tab"`, `draggable="true|false"`,
`aria-owns`, `aria-controls`, `hidden`, or `aria-selected`, put that condition
in the agnostic `Api::<part>_attrs()` method and make each adapter render the
returned attrs reactively. Do not patch the same condition into Leptos, Dioxus,
and future adapters one by one.

When an adapter needs a wrapper or structural node, add a real anatomy part and
attrs method to the agnostic component first. Example: Tabs closable triggers
need a presentational `TabShell` wrapper so the tab trigger and close trigger
are siblings rather than nested interactive controls. The adapter should render
`api.tab_shell_attrs()`, not manually write
`data-ars-part="tab-shell" role="presentation"`.

Adapter-authored attributes are limited to framework mechanics and consumer
pass-through merging: event handlers, node refs, framework keys, Dioxus
`onmounted`, Leptos `node_ref`, consumer `class` / arbitrary attrs after
conflict resolution, and truly host-framework-only details. If a value affects
accessibility, styling selectors, forms, state exposure, or the public DOM
contract, move it into `ars-components` and add agnostic tests before wiring the
adapter.

Adapters should not directly mutate a component `AttrMap` to add or override a
missing semantic value. Direct `AttrMap` mutation in adapter code is acceptable
only inside generic conversion/merge helpers or framework reactivity bridges
such as `memo_to_reactive_attrs`, where the adapter is preserving an
already-authored agnostic contract for the renderer. If component code needs
`let mut attrs = api.some_part_attrs(); attrs.set(HtmlAttr::Aria(...), ...)`,
that is a signal the attr belongs in the agnostic API first.

#### Prefer platform abstractions before target-specific code

Adapter components must try to express renderer effects through the most
platform-agnostic abstraction available before reaching for target-specific
APIs. For shared framework-agnostic effects, use
`ars_core::PlatformEffects`. For adapter-local operations that need framework
handles, use the adapter platform layer first, such as
`ars_dioxus::platform::DioxusPlatform` for `MountedData` / Dioxus event-backed
services.

Do not inline `web_sys`, browser timers, DOM queries, or desktop-only APIs in a
component event handler just because the current reproduction happens in that
target. Add a platform method with no-op/null fallbacks and a targeted
implementation for the host that supports the behavior, then have the
component call the platform method. Keep direct target APIs inside the platform
implementation, not mixed into component rendering logic.

Dioxus platform methods should prefer `MountedData` over DOM ids whenever the
component owns an `onmounted` handle. Pass `Rc<MountedData>` (or
`Option<Rc<MountedData>>` when mount timing is legitimately uncertain) into
`DioxusPlatform`, and let the platform implementation downcast or otherwise
resolve the renderer handle. Id-based lookup is an explicit fallback only: use
it for agnostic-core effects that are specified by id, hydration/pre-mounted
timing gaps, or legacy APIs that cannot yet expose a mounted handle. Do not add
new id-first Dioxus platform methods when a mounted-handle shape can express
the behavior.

Examples:

- Focus, geometry, scroll, timers, announcements, inert/scroll lock, and active
  element queries belong behind `PlatformEffects` when the agnostic machine can
  describe the effect by ID or intent.
- Dioxus renderer operations that need `MountedData` or a live Dioxus event,
  such as setting a custom drag image during `dragstart`, observing mounted
  elements, or mutating a child style under a mounted container, belong behind
  `DioxusPlatform` with `NullPlatform` / unsupported targets returning a no-op
  result.
- If no existing platform trait can express the behavior, extend the relevant
  platform trait first and cover its fallback behavior with focused tests
  before wiring the component.

#### Avoid unnecessary clones: prefer `StoredValue` (Leptos) and `CopyValue` (Dioxus)

A value captured into many closures will be cloned once per capture if you
write `let foo_for_X = foo.clone();` for every closure. The arena-backed
`Copy` handles built into each framework solve this once-for-all:

- **Leptos** — wrap non-`Copy` per-instance values in
  [`StoredValue<T>`](https://docs.rs/leptos/latest/leptos/reactive/owner/struct.StoredValue.html).
  The handle itself is `Copy`, so every closure (`on:click`, `on_cleanup`,
  `machine.derive`, etc.) can capture it freely. Read with
  `value.with_value(|v: &T| ...)` to borrow, or `value.get_value()` to clone
  out only at the moment the owned value is actually needed.
- **Dioxus** — wrap non-`Copy` per-instance values in
  [`CopyValue<T>`](https://docs.rs/dioxus/latest/dioxus/prelude/struct.CopyValue.html)
  (NOT `Signal<T>` — `CopyValue` is the non-reactive primitive). Construct
  once inside `use_hook(|| CopyValue::new(value))` so the handle survives
  every render, then read with `value.read()` / `value.cloned()` only where
  an owned value is required.

Apply this pattern whenever the same per-mount value is needed in **two or
more** closures — for example, an `Item` slot's `key` flowing into
`register_item`, the cleanup `deregister_item`, the per-item attrs derive,
and the click/hover event handlers. Single-closure captures don't need the
indirection — one move (or one clone) is fine and the arena allocation
isn't free.

When a slot prop is consumed by exactly one `Signal::derive` /
`use_hook(|| ...)` closure and not used after, **don't** create an
intermediate `let foo_for_signal = foo.clone();` — move the original prop
into the closure directly. Those `_for_signal` shims are dead clones
left over from refactors and clippy's `needless_pass_by_value` will
catch them in the workspace lint pass.

Inside event handlers, `with_api_ephemeral(|api| ...)` (no `move`)
borrows captured locals from the outer click closure — pull the value out
once with `let k = key.get_value();` / `key.cloned()`, let the inner
closure borrow `k`, then move `k` into the downstream `send.run(...)` /
`send.call(...)` dispatch. Adding `move` to the inner closure forces an
extra clone for every call.

#### Dioxus Rules of Hooks: never short-circuit a hook into an `unwrap_or_else` closure

[`use_stable_id`](https://dioxuslabs.com/learn/0.7/reference/use_stable_id),
`use_hook`, `use_signal`, `use_effect`, `use_drop`, and every other
`use_*` helper is a **hook** — Dioxus identifies its hook-slot by
_call-site ordinal_ in the component body. The runtime requires hooks
to run _the same number of times, in the same order, on every render_.
A subsequent render that skips or adds a hook call shifts the hook-slot
indices and corrupts state.

The pitfall:

```rust
// WRONG — `use_stable_id` only runs when `props.id` is None.
// First render with `id = None` allocates slot N for the id; a later
// render with `id = Some(...)` skips that allocation and every later
// hook reads from the wrong slot.
let id = props.id.unwrap_or_else(|| use_stable_id("listbox"));
```

The fix is to call the hook _unconditionally_ and only use its return
value when the prop is `None`:

```rust
// RIGHT — `use_stable_id` runs every render, slot index stays stable.
let stable_id = use_stable_id("listbox");
let id = props.id.unwrap_or(stable_id);
```

The same applies to every other `use_*` helper. If you find yourself
writing `unwrap_or_else(|| use_*())`, `if cond { use_*() }`, or
`some_iter.map(|_| use_*())`, stop — the hook MUST move to the top of
the component body where it always runs.

#### Forward consumer styling: `class` (Leptos) and `extends = GlobalAttributes` (Dioxus)

Every DOM-rendering slot must accept consumer-supplied class tokens (or
arbitrary HTML attributes on Dioxus) so users can style the rendered
elements with Tailwind utility chains, BEM-style class lists, or any
other CSS framework without escape hatches. The merge concatenates the
component's own class with the consumer's, keeping accessibility-critical
attributes (role, aria-_, data-ars-_) intact.

- **Leptos** — declare `#[prop(optional, into)] class: Option<Oco<'static, str>>`,
  then call `merge_consumer_class_into(&mut attrs, class.as_deref())`
  before converting the `AttrMap` to inline attrs. When `attrs` comes
  from `machine.derive(|api| api.<part>_attrs())`, move `class` into the
  same derive closure so the merge re-runs reactively with the rest of
  the part's attrs.

    ```rust
    #[component]
    pub fn Label(
        #[prop(optional, into)] class: Option<Oco<'static, str>>,
        children: Children,
    ) -> impl IntoView {

        let attrs = machine.derive(move |api| {
            let mut attrs = api.label_attrs();
            merge_consumer_class_into(&mut attrs, class.as_deref());
            attrs
        });
        view! { <div {..attr_map_to_leptos_inline_attrs(attrs.get())}>{children()}</div> }
    }
    ```

- **Dioxus** — add `#[props(extends = GlobalAttributes)] pub attrs: Vec<Attribute>`
  to the slot's props struct, then call `merge_dioxus_attrs(props.attrs,
component_attrs)` to combine consumer attrs with the
  `attr_map_to_dioxus_inline_attrs(...)` output. The merge concatenates
  tokenized attrs (`class`, `style`, relationship token lists) and lets
  the component's own value win for ordinary attrs on conflict.

```rust
#[derive(Props, Clone, Debug, PartialEq)]
pub struct LabelProps {
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,
    #[props(default)]
    pub children: Element,
}

#[component]
pub fn Label(props: LabelProps) -> Element {
    let attrs = ctx.machine.derive(|api| api.label_attrs());
    let component_attrs = attr_map_to_dioxus_inline_attrs(attrs.cloned());
    let attrs = merge_dioxus_attrs(props.attrs, component_attrs);
    rsx! { div { ..attrs, {props.children} } }
}
```

Apply this to every DOM-rendering slot, including the root, content,
items, labels, descriptions, error messages, hidden inputs, and any
adapter-owned structural nodes. Provider-only slots that emit no DOM
(e.g. `HeadingLevelProvider`, `Section`) do not need it. Add an SSR
test asserting the consumer class appears on each slot's rendered
element so a future refactor cannot silently drop the merge.

#### Annotate `String` / `Option<String>` props with `into`

Every `String` and `Option<String>` prop must be declared with
`#[prop(into)]` (Leptos) or `#[props(into)]` (Dioxus), combined with
`optional` where applicable. The annotation lets consumers pass either
an owned `String` or a borrowed `&'static str` literal, so call sites
read as `name="fruit"` / `name: "fruit"` instead of forcing
`.to_owned()` at every use.

- **Leptos** — `#[prop(into)] text_value: String,` and
  `#[prop(optional, into)] name: Option<String>,`. The macro generates
  `impl Into<String>` / `impl Into<Option<String>>` bounds so `&str`
  literals coerce automatically via `String::from`.
- **Dioxus** — `#[props(into)] pub text_value: String,` and
  `#[props(optional, into)] pub name: Option<String>,`. Same coercion
  through the `Into<T>` impl on the prop's stored type.

This pairs naturally with the `class` / `attrs` pass-through pattern:
adapter slots accept short literal call sites in both styling tokens
and identifier strings without losing the option to pass owned `String`
values from upstream signals or computations.

#### Leptos `children`: prefer `TypedChildren<T>` over `Children`

Every Leptos component that accepts **required** children should declare
the prop as `TypedChildren<T>` rather than the type-erased `Children`
alias. Keeping the view type a generic parameter avoids the runtime
boxing that `Children` introduces and preserves the static type
information Leptos uses to drive its DOM updates.

The required-children migration shape:

```rust
#[component]
pub fn Label<T>(
    #[prop(optional, into)] class: Option<Oco<'static, str>>,
    children: TypedChildren<T>,
) -> impl IntoView
where
    View<T>: IntoView,
{
    // ...

    // `TypedChildren<T>` is not directly callable. Pull the inner
    // closure out once before the `view!` macro so the body can call
    // it with the familiar `children()` syntax.
    let children = children.into_inner();

    view! { <div ..>{children()}</div> }
}
```

Required: import via `use leptos::children::TypedChildren;` and add the
`where View<T>: IntoView,` bound. Inside the body call
`children.into_inner()` once, then invoke the resulting closure
normally.

**Optional children stay on `Option<Children>`** — `Option<TypedChildren<T>>`
forces consumers to spell out `::<T>` at every call site that omits the
prop because the compiler cannot infer `T` without a usage. Components
like Select's `ValueText` and `EmptyState` that fall back to a localized
default when children are omitted keep the type-erased
`Option<Children>` signature and document the tradeoff inline:

```rust
#[component]
pub fn ValueText(
    #[prop(optional, into)] placeholder: Option<String>,
    /// Optional explicit children. When omitted, the agnostic core's
    /// computed selected text (or placeholder) is rendered. Uses the
    /// type-erased [`Children`] alias so consumers can omit the prop
    /// without specifying a turbofish for an unused generic.
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    // ...
}
```

The Dioxus equivalent is `Element` (`#[props(default)] pub children: Element`),
which already has the same property as `TypedChildren<T>`: no
type-erasure, no runtime boxing. No migration is needed on the Dioxus
side.

#### Reactive context: read provider signals untracked at component construction

The setup body of an adapter component runs once per mount, _outside_
any reactive scope. Reading a provider Signal via `.get()` from that
scope triggers Leptos's "accessed signal outside reactive tracking
context" warning, which fires once per mount and clutters the dev
console (60+ messages on a busy page is realistic — every
`use_machine`-backed component contributes one).

The rule: any one-shot snapshot read of a provider Signal at
construction time MUST use `.get_untracked()` (Leptos) or `.peek()` /
`.cloned()` (Dioxus, which is non-reactive by default for these
primitives). Reactive locale/color-mode/etc. updates flow through:

- the machine's `Props` signal, re-synced by
  `use_machine_with_reactive_props` on every prop change;
- the `t()` helper for translated text, which already wraps the locale
  read in its own `Signal::derive` closure;
- `machine.derive(|api| ...)` memos inside `view!` / `rsx!`, which run
  inside a reactive scope.

Adapter helpers that already follow this rule:

- `resolve_locale(adapter_props_locale: Option<&Locale>) -> Locale`
  reads `use_locale().get_untracked()` and is documented as a one-shot
  snapshot for component construction. Callers that need reactive
  locale must read the locale signal directly inside their own
  reactive closure.
- `use_messages::<M>(props, Some(&locale))` accepts the pre-resolved
  locale so it never re-reads the signal.

#### Reactive attribute spread: use `memo_to_reactive_attrs` for dynamic attrs

A bare `{..attr_map_to_leptos_inline_attrs(memo.get())}` spread reads
the memo once at `view!`-expansion time and renders static
attributes — it does NOT re-react when the memo's attrs change. Each
dynamic attribute must be re-emitted as an `AttrValue::Reactive`
(string) or `AttrValue::ReactiveBool` closure so Leptos updates that
exact attribute fine-grained when the memo invalidates.

The repo ships a helper for this:

```rust
let attrs = machine.derive(move |api| {
    let mut attrs = api.label_attrs();
    merge_consumer_class_into(&mut attrs, class.as_deref());
    attrs
});

view! {
    <div {..memo_to_reactive_attrs(
        attrs,
        LABEL_STRING_KEYS,
        LABEL_BOOL_KEYS,
    )}>{children()}</div>
}
```

`LABEL_STRING_KEYS` and `LABEL_BOOL_KEYS` are part-specific static
slices listing which attribute names are dynamic strings vs. dynamic
booleans for that slot. Static attrs (`role`, `data-ars-scope`,
`data-ars-part`) can be omitted from both — they're emitted once at
construction.

If a slot's attrs are entirely static (no state changes ever produce
different keys/values), `attr_map_to_leptos_inline_attrs(attrs.get())`
is fine — but lean on `memo_to_reactive_attrs` by default. The Dioxus
adapter uses `attr_map_to_dioxus_inline_attrs(attrs.cloned())` inside
`memo.with(...)` closures, which natively re-runs because the closure
is what Dioxus's signal system tracks.

#### Effect::new over Effect::watch in component bodies

In Leptos 0.8, `Effect::watch(deps, callback, immediate=false)`
empirically failed to fire reliably when its tracked signal updated
from a non-render code path (e.g. a registry-revision `RwSignal`
incremented inside a child slot's mount block). The reliable pattern
is `Effect::new`, which runs the closure once on creation and again
on every dependency read inside the closure:

```rust
Effect::new(move |_| {
    let _rev = registry_revision.get();
    let coll = registry.with_value(|r| r.build_collection());
    machine.send.run(Event::UpdateItems(coll));
});
```

Use `Effect::new` whenever you need to:

- propagate a child-slot registration to the machine
  (`registry_revision` → `Event::UpdateItems`);
- relay a derived presence flag through to the agnostic core
  (`description_count` → `Event::SetDescriptionPresent`);
- drive any "when this signal changes, dispatch an event" pattern.

Reserve `Effect::watch` for cases where you explicitly want
skip-initial behaviour AND you've verified the dependency tracking
works in your specific call site.

#### Deferred event dispatch in `with_api_ephemeral`

`UseMachineReturn::with_api_ephemeral(|api| ...)` (Leptos) borrows the
service from a `StoredValue<Service<M>>` for the duration of the
closure. If an `api.on_X_click()` method internally dispatches an
event via `service.update_value(|s| s.send(...))`, the dispatch
re-enters the same `StoredValue` and panics on re-entrant borrow.

The adapter's `with_api_ephemeral` solves this with a deferred-drain
buffer: events emitted via the API during the borrow are queued into a
`RefCell<Vec<M::Event>>` and drained back into
`service.update_value(...)` after the borrow releases. Adapter event
handlers must:

- use `machine.with_api_ephemeral(|api| { api.get().on_X_click(); })`
  when the API method itself queues events (it goes through the
  buffer correctly);
- use `machine.send.run(Event::X)` / `send.call(Event::X)` directly
  for the simple "dispatch one event, no api borrow needed" path.

What you must not do: call `machine.send.run(...)` from inside a
closure that already holds a `with_api_ephemeral` borrow — even if it
appears to work, you've bypassed the deferred-drain and you're one
state mutation away from a re-entrant borrow panic. The same pattern
holds on Dioxus via `Signal<Service>::peek()` borrow scopes.

#### `mousedown` `prevent_default` on popup-anchored Item slots

Select-style popups are usually anchored to a trigger via focus: the
trigger's `on_blur` dispatches `Event::Blur` which closes the popup.
Clicking a popup Item then races:

1. browser default `mousedown` shifts focus to the Item element;
2. `blur` fires on the trigger → popup closes;
3. the Item's `click` handler never lands (the popup is gone).

Fix: attach `on:mousedown` (Leptos) / `onmousedown:` (Dioxus) to the
Item slot that calls `event.prevent_default()`. The trigger keeps
focus, the popup stays open, the `click` reaches the Item, and
`Event::SelectItem` lands.

```rust
let on_mousedown = move |ev: ev::MouseEvent| {
    ev.prevent_default();
};

view! {
    <div
        {..attrs}
        on:mousedown=on_mousedown
        on:click=on_click
        on:mouseenter=on_mouseenter
        on:mouseleave=on_mouseleave
    >
        {children()}
    </div>
}
```

Apply to Item slots inside every popup-anchored component (Select,
Combobox, future Menu / Autocomplete) where the popup closes on
trigger blur. Listbox does NOT need it — its content shares focus
with its host so there's no blur race.

#### Aggregate trigger text on multi-select API methods

When a selection-family component's trigger reflects the current
selection (e.g. Select's `ValueText`), the agnostic core's
`Api::selected_text()` must return the joined display text of ALL
selected items, not just the first one. The contract on the helper is
"display text suitable for the trigger's ValueText slot" — for empty
selection return `None`, for single return the item's `text_value`,
for multi return all `text_value`s joined with `", "` (allocate
once).

Adapters consume this with a single call:

```rust
let text = machine.derive(move |api| {
    api.selected_text().unwrap_or_else(|| {
        placeholder
            .clone()
            .unwrap_or_else(|| api.placeholder_text())
    })
});
```

A first-key-only implementation looks correct in single-select tests
but silently breaks multi-select demos (the trigger shows only the
last clicked option). The spec doc string and the unit test that
asserts `Some("Bravo")` need to reflect the multi-select behaviour
explicitly. If you find yourself writing
`selection.first().and_then(|k| items.text_value_of(k))`, stop and
join the full iterator instead.

### Adapter Tests

- Add or update framework-specific adapter tests under:
  - `crates/ars-leptos/tests/`
  - `crates/ars-dioxus/tests/`
- Follow the file naming and parity expectations enforced by
  `cargo xtask lint adapter-parity`.
- Cover the component's required props, attributes, ARIA output, state
  synchronization, callbacks, disabled/read-only states, and as-child behavior
  where applicable.
- Add browser-backed wasm tests for behavior that requires a real DOM: focus,
  keyboard navigation, pointer events, layout, clipboard, file upload,
  drag/drop, media queries, observers, portal behavior, or cleanup of browser
  resources.
- Add snapshot tests only when a stable rendered structure is part of the review
  surface. Inspect snapshots before marking the work ready; PRs that touch
  `.snap` files must receive the `snapshot-reviewed` label.

### E2E Fixtures And Harnesses

**The fixture + harness layer must exhaustively cover every public
feature of every adapter-level component.** Smoke-only coverage is a
workflow violation — the issue acceptance criteria are a floor for
how much testing must exist, not a ceiling.

"Exhaustive" means: for every prop, slot, callback, ARIA wiring,
keyboard path, pointer interaction, controlled-vs-uncontrolled mode,
and discrete state branch the component exposes, the E2E layer must
drive it from a real browser and assert observable outcomes (DOM
state, ARIA, focus, hidden-input value, form submission payload). The
guardrail is: if you can list a feature in the adapter spec but
cannot point to an E2E test fn that exercises it, the coverage is
incomplete.

The E2E matrix must also cover the counterpart-library parity findings from
the review above. If React Aria / React Spectrum, Ark UI, Radix UI, or the
chosen fallback library demonstrates an interaction that we support, there
must be a browser test for our version of that interaction. If the counterpart
exposes a feature we intentionally do not support, the matrix entry should mark
the axis `NotApplicable` with the same reason documented in the spec or PR
body.

#### Required files

Update both fixtures when both adapters exist:

- `crates/ars-e2e/fixtures/leptos/src/categories/<category>.rs`
- `crates/ars-e2e/fixtures/dioxus/src/categories/<category>.rs`

Then add or update the matching E2E harness module:

- `crates/ars-e2e/src/<category>/<component>.rs`
- `crates/ars-e2e/src/<category>/mod.rs`
- `crates/ars-e2e/src/lib.rs` only if this is the first component in a new
  category.
- `xtask/src/e2e.rs`, `xtask/src/main.rs`, and `crates/ars-e2e/src/main.rs`
  when this is the first E2E-covered component in a category that does not yet
  have an E2E subcommand.

#### Fixture design

The fixture page must expose AT LEAST ONE id'd instance per feature
combination the harness drives. Examples for a Select fixture:

- `#select-single-basic` — single-select, no groups, no description;
- `#select-multi-grouped` — `multiple=true` with two `ItemGroup`s and
  one disabled item;
- `#select-with-clear-and-indicator` — `ClearTrigger` + `Indicator`
  visible, used to assert clear-empties-selection;
- `#select-named-form` — `name="…"` + `HiddenInput` for serialization
  assertions and form-reset coverage;
- `#select-invalid-with-error` — `invalid=true` + `ErrorMessage`
  rendered;
- `#select-disabled-root` — disabled whole select;
- `#select-readonly-root` — read-only whole select;
- `#select-empty-state` — zero items, exercises `EmptyState`;
- `#select-multi-line-trigger` — `multi_line_trigger=true` chip
  layout, multi-select.

The harness queries each instance by its id, so the fixture's
duplication is intentional and acceptable. Don't share state between
instances.

#### Harness design

Add ONE async test fn per feature axis, named after what it asserts.
Examples for Select (this is illustrative, not exhaustive):

- `single_select_click_updates_aria_selected_and_closes_popup`
- `multi_select_aggregates_value_text_and_hidden_input`
- `multi_select_keeps_popup_open_on_item_click`
- `clear_trigger_empties_selection_and_hidden_input`
- `indicator_rotates_on_open`
- `escape_key_closes_popup_and_restores_trigger_focus`
- `arrow_keys_move_active_descendant_in_open_popup`
- `home_end_keys_jump_to_first_last_item`
- `space_enter_select_highlighted_item`
- `typeahead_from_closed_trigger_opens_and_highlights`
- `typeahead_from_open_popup_highlights_match`
- `disabled_item_skipped_by_keyboard_and_not_clickable`
- `disabled_keys_prop_skips_listed_keys`
- `disabled_root_blocks_trigger_open`
- `readonly_root_keeps_trigger_open_but_blocks_mutation`
- `invalid_root_wires_error_message_via_aria_describedby`
- `label_wires_trigger_via_aria_labelledby`
- `item_group_label_wires_group_via_aria_labelledby`
- `placeholder_visible_when_selection_empty`
- `hidden_input_serializes_single_value`
- `hidden_input_serializes_multi_value_comma_separated`
- `form_reset_restores_default_value`
- `outside_pointer_dismiss_closes_popup`
- `controlled_value_signal_round_trips_selection`
- `empty_state_renders_when_no_items`
- `positioner_emits_data_ars_placement`

For Listbox the matrix is similar but excludes popup-related items
and adds `selection_behavior` (Toggle vs Replace), `orientation`
(Vertical vs Horizontal), `loop_focus`, `disallow_empty_selection`,
`on_action` callback, `LoadingSentinel` + `on_load_more` (when the
intersection observer wiring lands).

#### Axe-core re-runs across visible states (REQUIRED — exhaustive)

Axe-core must run after **every distinct visible state** the
component can reach — not just the initial mount. Most accessibility
regressions surface only after a state transition (popup opens,
selection toggles, validation flips to invalid, a disabled flag
flips), and a baseline-only scan misses 100 % of them. A PR that
ships one axe scan per harness is **not** exhaustive coverage and is
not mergeable under this contract.

##### Helpers

Two helpers live in `crates/ars-e2e/src/axe.rs`:

```rust
// Whole-document scan. Use when every scenario on the fixture page
// is intended to pass axe — i.e. nothing is deliberately
// "bare-minimum / unlabelled" for raw-behaviour testing.
pub(crate) async fn run_axe(driver: &WebDriver) -> Result<(), Error>;

// Scoped scan. Use when the fixture page co-hosts scenarios with
// intentionally varied a11y completeness (e.g. an
// "unlabelled bare-minimum" scenario alongside a fully labelled
// one) and you only want to enforce a11y on the scenarios that
// promise complete wiring.
pub(crate) async fn run_axe_on(driver: &WebDriver, css: &str) -> Result<(), Error>;
```

The multi-instance fixture layout described above co-hosts
bare-minimum and fully-labelled scenarios on the same page, so the
default is `run_axe_on(driver, "#<scenario-id>")` scoped to the
specific scenario being asserted. Whole-page `run_axe` is the
exception, reserved for components whose entire fixture page is
fully labelled.

##### Required state matrix

The harness must include one axe scan (scoped to the relevant
scenario) for **each row** of the state matrix below that the
component supports. If the component does not support a row
(e.g. Listbox has no closed/open state), state that explicitly in
the PR body rather than silently dropping the row.

| State                                                   | Why audit here                                                 |
| ------------------------------------------------------- | -------------------------------------------------------------- |
| Closed / initial labelled scenario                      | catches baseline ARIA wiring                                   |
| Initial scenario with `<Label>` + `<Description>`       | aria-labelledby + aria-describedby chain                       |
| Initial scenario with `invalid=true` + `<ErrorMessage>` | aria-invalid + aria-describedby + role="alert" / aria-live     |
| Initial scenario with `disabled=true`                   | aria-disabled propagation, focusability                        |
| Initial scenario with `readonly=true` (if supported)    | aria-readonly + interaction blocking                           |
| Open popup, no selection (selection-family)             | role="listbox" semantics inside hidden-becomes-visible content |
| Open popup, single selection (selection-family)         | aria-selected on the active option                             |
| Open popup, multi-selection (selection-family)          | aria-multiselectable + multiple aria-selected="true"           |
| Closed trigger after selection (selection-family)       | trigger value-text accessible name update                      |
| With `<ItemGroup>` + `<ItemGroupLabel>`                 | group role + aria-labelledby into the group label              |
| Empty state visible (selection-family with zero items)  | role="status" / accessible empty-state messaging               |
| Multi-line / chip trigger layout (if supported)         | accessible name aggregation when value-text wraps              |

For Listbox the "closed trigger" and "open popup" rows collapse into
"populated listbox" since the listbox is always visible — but the
populated/empty/invalid/disabled/group axes still apply.

For Tooltip / Toast / Popover / Dialog: substitute the open vs
dismissed state with placement / inert-background / focus-trap
states (e.g. modal open + a11y backdrop, modal open + focus
trapped, dialog after `escape`-dismiss flush).

##### Naming convention for axe-only tests

When a state has no behavioural assertion of its own, add a dedicated
test fn:

```text
axe_clean_<state>
```

Examples that already ship in the selection harnesses:

- `axe_clean_after_multi_selected_state`
- `axe_clean_across_multiple_visible_states`

When the state IS already exercised by a behavioural test, append the
axe call at the end of that test fn instead of duplicating the setup,
e.g.:

```rust
async fn error_message_links_to_content_when_invalid_and_axe_clean(
    driver: &WebDriver,
) -> Result<(), Error> {
    // ... aria-describedby assertions ...
    run_axe_on(driver, "#listbox-invalid-with-error").await?;
    Ok(())
}
```

Name the fn `<feature>_and_axe_clean` when bundling so the suffix
makes the audit visible at a glance.

##### What "axe-clean" means in practice

The axe call returns `Ok(())` only when there are no violations at
the `serious` or `critical` impact level after the
`color-contrast` and `link-in-text-block` rules are disabled (those
two would require visual styling decisions the bare fixture cannot
make and would create false positives). All other rules — including
`aria-required-attr`, `aria-valid-attr-value`,
`aria-input-field-name`, `aria-allowed-attr`, `landmark-*`,
`region`, `nested-interactive`, and the full WCAG 2.1 AA rule set —
must pass.

The baseline-only pattern (one axe scan per harness) is explicitly
**not** sufficient.

#### Adapter parity is mandatory

The Leptos and Dioxus harnesses must run the same set of assertions
against their respective fixtures. If a feature axis has an async
test fn in `selection/select.rs` (Leptos run), it must have a
matching fn in the Dioxus run, and vice versa. Cross-adapter parity
is the primary value of this layer — when only one adapter has a
given test, that's a gap that creates spec drift.

When the cargo-xtask invocation grows, prefer one harness fn per
feature axis over inline blocks inside a single mega-test — failures
should pinpoint the broken feature, not "select harness failed".

#### Interaction matrix entries are mandatory

Every implemented adapter component must be represented in
`crates/ars-e2e/src/matrix.rs`. The matrix is the authoritative checklist for
browser-observable adapter coverage. A component entry must account for every
axis:

- `Pointer` — click, pointerdown/up/click sequences, outside click, and disabled
  click suppression.
- `Keyboard` — Tab / Shift+Tab, Enter, Space, Escape, arrow keys, Home / End,
  typeahead, and modifier chords where supported.
- `Focus` — active element ownership, roving focus, focus restoration,
  focus-visible state, focus trap, and skipped disabled targets.
- `State` — selected, active, focused, disabled, readonly, invalid, open,
  closed, hidden, visible, loading, and similar `data-ars-*` / ARIA states.
- `Forms` — hidden input value, native form serialization, form reset,
  submit/reset button behavior, and disabled form participation.
- `Visual` — computed style and layout assertions for visible feedback,
  disabled distinction, popup anchoring, stable control dimensions,
  scrollability, and nonzero rendered boxes.
- `A11y` — axe-clean scenarios, roles, names, descriptions, errors, and ARIA
  linkages.
- `Lifecycle` — controlled prop sync, mount/unmount cleanup, listener cleanup,
  portal cleanup, and one-shot callback ordering.

If an axis is not meaningful for a component, record that with
`NotApplicable { axis, reason }` in the matrix. Do not leave an axis implicit. A
matrix entry that covers only behavior but omits visual coverage is incomplete
unless the component is genuinely invisible and the reason is documented.

#### Computed-visual assertions are mandatory

Any component with visible states must have E2E assertions that inspect the
rendered result, not just ARIA/data attributes. Use shared helpers from
`crates/ars-e2e/src/visual.rs` or category-local wrapper helpers for:

- `getComputedStyle(...)` checks such as `background-color`, `font-weight`,
  `color`, `cursor`, `opacity`, `display`, and `visibility`;
- `getBoundingClientRect()` checks for popup anchoring, stable trigger width,
  nonzero rendered controls, and scroll container dimensions;
- hidden-vs-visible assertions for popup positioners, closed content, and
  visually hidden helpers;
- browser console cleanliness when a user interaction might surface a runtime
  panic or framework warning.

Visual tests should assert intent, not exact pixels. Prefer thresholds (`left
edge within 2px`, `width unchanged within 1px`, `popup remains anchored near the
trigger`) and semantic computed-style checks (`selected state has
non-transparent background or stronger font weight`) over full screenshot
baselines. Screenshot baselines are optional and should only be added when
rendering is deterministic enough to avoid churn.

#### Filtered E2E runs are part of the workflow

Every category E2E command must support focused execution so a single
interaction regression can be debugged without running the entire browser
matrix:

```bash
cargo xtask e2e <category> --adapter leptos --component <component>
cargo xtask e2e <category> --adapter dioxus --component <component>
cargo xtask e2e <category> --adapter dioxus --component <component> --test-filter <substring>
cargo xtask e2e <category> --adapter dioxus --component <component> --visual-only
cargo xtask e2e <category> --adapter dioxus --component <component> --behavior-only
```

The full category command must keep working, but new harness code should be
written so component-local visual and behavior subsets are runnable. This is
required for cold-build, ChromeDriver, and flake isolation: if a full category
run times out, the focused component command still needs to produce a useful
pass/fail signal.

#### Exceptions

Smoke-only coverage is acceptable ONLY when:

- the component is purely static (no state, no interaction) AND
  adapter tests already cover every output branch, OR
- the issue acceptance criteria explicitly limit the E2E surface AND
  the PR body lists which feature axes remain unexercised with a
  followup-issue reference for each.

In both cases the PR body must explain the exception. The default —
no special exemption noted — is full exhaustive coverage.

#### Reviewer signal

A PR adding or materially changing an adapter component should show
new test-fn entries in both the fixture and harness files
proportional to the new feature surface. A diff with 6+ new features
in the adapter but only 1–2 new E2E tests is incomplete; reviewers
should request the missing coverage before approving.

### Widgets Examples

Add a visual/demo entry to every applicable widgets example crate:

- `examples/widgets-leptos`
- `examples/widgets-dioxus`
- `examples/widgets-leptos-css`
- `examples/widgets-dioxus-css`
- `examples/widgets-leptos-tailwind`
- `examples/widgets-dioxus-tailwind`

Use the category module that matches the component:

- `src/categories/input.rs`
- `src/categories/selection.rs`
- `src/categories/overlay.rs`
- `src/categories/navigation.rs`
- `src/categories/date_time.rs`
- `src/categories/data_display.rs`
- `src/categories/layout.rs`
- `src/categories/specialized.rs`
- `src/categories/utility.rs`

Do not put category-specific example text in root `WidgetsText`. Add it to the
category-local text enum such as `UtilityText`, `NavigationText`, or
`InputText`.

Only edit `main.rs`, `text.rs`, or `categories/mod.rs` when the top-level spec
category list itself changes.

#### Showcase as many features per demo panel as practical

The widgets examples are the visual proving ground for each
component. Each category panel should exercise the full feature
surface of every component it hosts — multi-select alongside
single-select, grouped items alongside ungrouped, disabled items,
labels + descriptions + error messages, indicators + clear triggers,
controlled-callback readouts ("Latest action: …"), and so on. A
single-item smoke demo is not enough — features that ship without a
visible demo tend to ship with hidden bugs (the multi-select Select
trigger-text aggregation regression was invisible until a demo with
multiple selected items existed).

When adding a new component to a category panel, audit the existing
panel for missing feature coverage at the same time and extend it.
Each example crate (`widgets-leptos`, `widgets-leptos-css`,
`widgets-leptos-tailwind`, and the three Dioxus equivalents) must
stay symmetric — features demonstrated in one belong in all six.

#### Demo CSS must demonstrate features visually

Adapter components are headless; the demo stylesheets in
`examples/widgets-*-css/` and `examples/widgets-*-tailwind/` are what
make the features tangible to a sighted reviewer. Follow these rules
to avoid demos that look broken even when the underlying behaviour
works:

- **Honour the `data-ars-visually-hidden` data attribute.** The
  framework-agnostic core emits `data-ars-visually-hidden="true"` on
  screen-reader-only elements (e.g. Dismissable's hidden dismiss
  buttons). The canonical `crates/ars-core/ars-base.css` includes a
  `[data-ars-visually-hidden]` rule alongside `.ars-visually-hidden`.
  Don't override or shadow it — without that rule the hidden buttons
  render as visible empty pills.

- **Match variant override specificity to the base rule.** A
  base rule like `[data-ars-scope="button"][data-ars-part="root"]`
  (specificity 0,2,0) silently overrides variant rules like
  `[data-ars-variant="link"]` (specificity 0,1,0). When a variant
  needs to clear an inherited property (`box-shadow: none`,
  `padding-inline: 0`, `background: transparent`), prefix its
  selector with the scope + part so the specificity matches:
  `[data-ars-scope="button"][data-ars-part="root"][data-ars-variant="link"]`.

- **Make multi-child buttons / triggers inline-flex with gap.** Any
  component root that mounts multiple inline children at runtime
  (loading indicator + label, value text + indicator + clear) needs
  `display: inline-flex; align-items: center; justify-content: center;
gap: 0.5rem` so children stay separated when the runtime adds the
  second child. Without `gap` the loading spinner mounts flush against
  the "L" of "Loading"; with `align-items: center` glyphs and text
  share a vertical baseline.

- **Show scroll affordances on Mac.** macOS auto-hides scrollbars
  by default, so a `max-height` + `overflow-y: auto` content block
  looks truncated to sighted users. Demo stylesheets must include
  explicit `::-webkit-scrollbar`, `::-webkit-scrollbar-track`,
  `::-webkit-scrollbar-thumb` rules on scrollable slot parts (Listbox
  Content, Select Content) and `scrollbar-gutter: stable` to reserve
  the gutter so layout doesn't shift when scroll becomes necessary.

- **Use SVG, not Unicode, for visual glyphs.** Indicator chevrons,
  close marks, and similar UI icons rendered inline must use inline
  SVG with `stroke="currentColor"` rather than Unicode glyphs like
  `⌄`, `▾`, `▼`. Unicode characters have font-dependent metrics — the
  visible mark can sit visibly off-center within its character box on
  different fonts/OSes, and no `line-height` / `padding` / `transform`
  tweak corrects it reliably across the matrix. SVGs have predictable
  geometry (their viewBox sits centered in the layout box) and inherit
  colour via `currentColor`. Pattern:

    ```html
    <svg
        xmlns="http://www.w3.org/2000/svg"
        width="12"
        height="12"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2.5"
        stroke-linecap="round"
        stroke-linejoin="round"
    >
        <polyline points="6 9 12 15 18 9" />
    </svg>
    ```

- **Reactive-context warnings count as user-visible regressions.**
  If a Mac + Chrome reviewer opens the dev console on a freshly
  loaded demo and sees a flood of "accessed signal outside reactive
  context" warnings (one per mounted component), treat that as a
  blocker — it points at construction-time reads of a provider
  signal that should be `.get_untracked()` (see the "Reactive
  context" rule in the Adapter Crate section). Demos must boot
  warning-free in dev for the example to be considered shippable.

#### Widget smoke checks are mandatory for visual components

Adapter fixtures prove adapter behavior with controlled markup. They do not
prove that the public example stylesheets still render the component correctly.
Every adapter component with visible interaction states must also have a public
widgets smoke path that runs against the real example CSS:

```bash
cargo xtask e2e widgets --adapter leptos --style plain --category <category>
cargo xtask e2e widgets --adapter leptos --style css --category <category>
cargo xtask e2e widgets --adapter leptos --style tailwind --category <category>
cargo xtask e2e widgets --adapter dioxus --style plain --category <category>
cargo xtask e2e widgets --adapter dioxus --style css --category <category>
cargo xtask e2e widgets --adapter dioxus --style tailwind --category <category>
```

At minimum, the widget smoke must:

- navigate to the relevant category tab;
- perform one representative pointer interaction;
- perform one representative keyboard interaction when the component has a
  keyboard contract;
- assert the browser console is clean;
- assert selected / active / open states have visible computed-style feedback;
- assert disabled examples have a visible distinction such as muted text, lower
  opacity, or `cursor: not-allowed`;
- assert popup / overlay positioners are hidden when closed and anchored to the
  trigger when open;
- assert controls do not shrink or shift after user-visible state changes;
- assert hidden form inputs serialize selected values when the component
  participates in native forms.

The widget smoke must include at least one explicit counterpart-UX assertion
for every visible state copied from the parity review. Examples:

- Listbox selected and hovered rows fill the row width, not only the text span;
- disabled options are visibly muted and expose a disabled cursor;
- Select popups anchor to the trigger rather than the page edge;
- Select trigger width remains stable after selection;
- close buttons, indicators, drag previews, and loading affordances are
  visually centered and do not disappear in selected states.

The smoke should use computed style and bounding-box assertions first. Do not
rely on "I clicked it manually and it looked fine" as the only evidence; a bug
in a checked-in Tailwind output or CSS cascade must fail an automated command.

#### Don't leak playwright / dev artifacts into the repo

When running `playwright-cli` (or any other browser-automation tool)
during development, route all artifact paths to either the
`.playwright-cli/` directory (already in the repo `.gitignore`) or
to `/tmp/`. Never use bare filenames or paths inside the repo root
— `playwright-cli screenshot --filename=page.png` writes to cwd,
which is usually the repo root, and creates untracked artifacts that
leak into `git status`. Prefer
`playwright-cli screenshot --filename=.playwright-cli/page.png` or
`/tmp/page.png` instead.

The same applies to `*.yml` / `*.yaml` snapshot files written by
playwright-cli's snapshot output: pass the directory prefix
explicitly or let the tool default to `.playwright-cli/`.

### Specification Updates

Update the relevant spec in the same PR whenever implementation reveals drift
or incomplete spec coverage. Follow
[README.md § Spec synchronization](../README.md#spec-synchronization).

- Shared behavior belongs in `spec/foundation/` or `spec/shared/`.
- Dependency-machine changes belong in that machine's component spec (for
  example new `popover::Api` accessors used by a composition layer).
- Adapter-specific behavior belongs in
  `spec/foundation/08-adapter-leptos.md`,
  `spec/foundation/09-adapter-dioxus.md`, or the per-component adapter spec.
- Do not leave spec drift as a follow-up — port back missing helpers, incomplete
  code sketches, anatomy/ARIA table gaps, and default-source clarity in the
  same PR.
- Run `cargo xtask spec validate` after spec edits.

## Implementation Order

Use this order unless the issue explicitly says otherwise:

1. Move the issue to **In Progress** on the GitHub Project board.
2. Read the issue acceptance criteria and the specs listed above.
3. Run the counterpart library parity review. Start with React Aria / React
   Spectrum, then Ark UI, then Radix UI, then another mature library only when
   needed. Record feature, interaction, and UX gaps before choosing the
   implementation shape.
4. Add or update focused adapter tests first.
5. Implement the adapter component code.
6. Wire modules, prelude exports, and feature flags.
7. Add or update E2E fixtures and E2E harness modules, including parity-review
   feature and interaction coverage.
8. Add or update widgets examples in the matching category modules, including
   visual UX parity coverage for supported states.
9. Update specs when the intended contract changed or when implementation
   surfaced incomplete spec coverage (see
   [README.md § Spec synchronization](../README.md#spec-synchronization)).
10. Run focused tests and checks for the edited component.
11. Invoke `.agents/skills/post-implementation-audit/SKILL.md`.
12. Fix every audit finding in the same PR.
13. Present the result for user review before committing.

## Validation Checklist

Run the exact commands named by the issue. In addition, use the applicable
checks below.

For Leptos adapter code:

```bash
cargo check -p ars-leptos
cargo test -p ars-leptos --test <component>
```

For Dioxus adapter code:

```bash
cargo check -p ars-dioxus
cargo test -p ars-dioxus --test <component>
```

For browser-backed adapter tests, use the repo browser-test environment
documented in `AGENTS.md` when running wasm tests locally.

For E2E changes:

```bash
cargo check -p ars-e2e
cargo xtask e2e navigation --adapter leptos
cargo xtask e2e navigation --adapter dioxus
cargo xtask e2e selection --adapter leptos
cargo xtask e2e selection --adapter dioxus
cargo xtask e2e utility --adapter leptos
cargo xtask e2e utility --adapter dioxus
```

Today the supported E2E categories are `navigation`, `selection`, and
`utility`. If a task adds the first E2E-covered component for another category,
add the matching category subcommand before documenting the validation command
as available.

For widget smoke changes:

```bash
cargo xtask e2e widgets --adapter leptos --style plain --category <category>
cargo xtask e2e widgets --adapter leptos --style css --category <category>
cargo xtask e2e widgets --adapter leptos --style tailwind --category <category>
cargo xtask e2e widgets --adapter dioxus --style plain --category <category>
cargo xtask e2e widgets --adapter dioxus --style css --category <category>
cargo xtask e2e widgets --adapter dioxus --style tailwind --category <category>
```

For widgets examples:

```bash
cd examples
cargo check -p widgets-leptos -p widgets-dioxus \
  -p widgets-leptos-css -p widgets-dioxus-css \
  -p widgets-leptos-tailwind -p widgets-dioxus-tailwind
```

For workspace gates before publishing:

```bash
cargo fmt --all --check
cargo xtask lint adapter-parity
cargo xci-fast
```

Run full `cargo xci` instead of `cargo xci-fast` when the change is broad,
touches feature-flag interactions, or modifies shared adapter infrastructure.

## PR Closeout

Before asking for review:

- confirm all required deliverables above are either implemented or explicitly
  marked N/A with a reason;
- include the issue auto-close keyword in the PR body;
- list spec refs and validation commands in the PR body;
- attach `snapshot-reviewed` if `.snap` files changed;

After the first push (and after every subsequent push to the PR branch), **read
and follow** `.agents/skills/waiting-for-codex-review/SKILL.md` through to Codex
👍. Posting `@codex review` alone does not satisfy this step — the full poll
loop, thread triage, fix/push/reply/resolve cycle, and re-trigger are all
required. Do not treat the PR as merge-ready until the skill completes.

- keep the issue, PR, and Project board state aligned with the actual work.

Do not close the issue until the PR is merged, CI is green, and Codex review has
left a thumbs-up.
