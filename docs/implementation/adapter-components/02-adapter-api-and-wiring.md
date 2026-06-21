# Adapter API And Wiring

Adapter crates render framework views and connect events. Component behavior,
semantic state, ARIA, and `data-ars-*` anatomy belong in the framework-agnostic
component API.

## Required Files

Add or update:

- `crates/ars-leptos/src/<category>/<component>/`
- `crates/ars-dioxus/src/<category>/<component>/`
- category `mod.rs` files;
- `crates/ars-leptos/src/lib.rs` and `crates/ars-dioxus/src/lib.rs` when a new
  category module is introduced;
- adapter `Cargo.toml` feature wiring when the component needs an
  `ars-components` feature.

Use directory-backed modules with `mod.rs` when a component module owns child
modules.

## Prelude Exports

For unstyled adapter primitives, re-export:

- the component module; and
- the primitive part entry points that consumers compose directly.

Example:

```rust
pub use crate::input::checkbox;
```

Also re-export configuration types that consumers pass into props. If the
framework-agnostic name is too generic, use stable aliases such as:

```rust
pub use ars_collections::selection::{
    Behavior as SelectionBehavior,
    Mode as SelectionMode,
    Set as SelectionSet,
};
```

Do not put machine internals, slot output internals, adapter hooks, or
component-author-only helpers in the end-user prelude.

Do not flatten component parts into the end-user prelude. Re-export the
component module and require namespace-qualified parts such as `tabs::Root`,
`tabs::List`, `checkbox::Root`, or `field::Label`. This keeps part names from
different component families from colliding and makes examples show the same
compound-component boundary that users should keep in application code.
Tests must follow the same rule. Import the component module (`navigation::tabs`
or `input::checkbox`) and refer to every adapter part, row helper, renderer
type, and collection source through that module (`tabs::Root`, `tabs::Tab`,
`tabs::TabRenderItem`, `tabs::TabsSource`). Do not import part names directly in
test files, even when the local file only covers one component; direct imports
hide future name collisions.

Low-level adapter primitive roots are named `Root` inside the component module,
matching the Checkbox standard. Prefer `field::Root`, `fieldset::Root`,
`form::Root`, and `checkbox::Root` over semantic root names such as `Field`,
`Fieldset`, or `Form`. The module name identifies the component family; `Root`
identifies the part. Reserve semantic names for future higher-level wrappers or
styled source templates that orchestrate a closed anatomy.

Ready-made styled components do not belong in `ars-leptos` or `ars-dioxus`.
Unstyled does not mean unstylable: adapter primitives should expose the public
parts and attribute hooks consumers need to style the component without
reimplementing component-owned semantics.
The checked-in styled crates (`ars-leptos-components` and
`ars-dioxus-components`) are the reference/source-template layer used by
widgets, examples, tests, and the future source-distribution workflow. They may
re-export their component modules from their own preludes. Both adapter
preludes must stay symmetric.

## Public API Fidelity

Before implementing any public type, function, prop, slot, or event handler:

1. read the corresponding spec example;
2. implement the exact public shape unless it cannot compile;
3. cross-check after implementation;
4. update the spec in the same PR if the spec is wrong or incomplete.

Do not invent equivalent-looking APIs. Spec drift invalidates downstream code.

## Attribute Ownership

Render semantic and anatomy attributes from the agnostic API only:

- roles;
- ARIA attributes;
- `data-ars-*` flags;
- ids and relationships;
- disabled, readonly, invalid, selected, focused, active, loading, and hidden
  state attrs.

Adapter code may add framework-specific event wiring, consumer styling
forwarding, and view composition. It must not fork the component's semantic
contract.

Render slots that need item state must receive it from the agnostic API. Do not
make Leptos and Dioxus render callbacks recompute selected, disabled, hovered,
pressed, dragging, drop-target, section, or layout-preview state from local
adapter data. If a renderer needs that state, add an agnostic render-state
helper first, then pass that state through both adapters.

Browser-only APIs still belong to adapters, but their inputs must come from the
agnostic API. For example, a drag-preview element id is adapter-owned DOM
configuration; the dragged key set, drop target, and preview order are
core-owned state.

## Consumer Styling

Leptos components must forward consumer styling through a `class` prop where
the adapter convention expects one. Leptos `class` props must be `TextProp`;
there is no non-reactive class-prop path. Use `TextProp` for `style` too when
the component exposes raw inline style. This lets application state and
locale-driven styling flow through the same prop without examples rebuilding
component logic. Merge consumer class props with `merge_consumer_class_prop_into`
so component-owned classes and reactive consumer classes end up in one final
`class` attribute; do not split classes across separate attributes or hardcode
example-only class branching.

Dioxus props must extend `GlobalAttributes` when the component exposes global
HTML attributes to consumers. Do not add explicit `class`, `style`, `data-*`,
`lang`, `tabindex`, or extra `aria-*` props to Dioxus components when
`GlobalAttributes` already captures them. Those attrs should flow through the
`attrs: Vec<Attribute>` field and be merged with component-owned attrs at the
root. Add an explicit Dioxus prop only when it is semantic component data, maps
to a non-root part, or has a documented precedence/validation rule that cannot
be expressed as a global attr.

Avoid swallowing consumer classes or replacing them with fixed demo styling.

### Multi-Part Components

For components with internal anatomy or adapter-rendered structural nodes, do
not add a long series of `*_class` / `*_style` props for every part. That API
scales poorly, makes the root component harder to read, and forces every
framework to carry repetitive styling escape hatches.

Prefer a compound-part API when consumers need to style internal anatomy:

- keep adapter crates focused on unstyled primitives;
- put checked-in closed-anatomy styled source templates in `ars-*-components`;
- expose public part components for the rendered anatomy, with the low-level
  root named `Root` and other parts such as
  `Control`, `Indicator`, `Description`, `ErrorMessage`, and status or live
  region parts;
- make each part consume the same agnostic machine/context as the convenience
  component;
- render roles, ARIA, ids, and `data-ars-*` attrs from the agnostic API, not
  from the example;
- let consumers pass normal framework attributes to each part.

For every core `Part` enum variant and every adapter-rendered structural node,
make an explicit public styling decision:

- expose a public stylable part when consumers may reasonably need to style,
  position, hide, or target that node;
- keep it private only when exposing it would break semantics or duplicate
  required browser behavior;
- record private exceptions in the adapter spec with the reason and the
  supported styling alternative, such as stable `data-ars-*` selectors.

Core anatomy does not automatically imply a standalone public adapter
component. A node may be a required `Part` / anatomy node, rendered by the
adapter with stable attrs, and still remain private when exposing it would make
callers pass adapter-owned refs, event wiring, measurement styles, or raw
converted attrs. Adapter specs should name these as "anatomy nodes" or
"internal nodes" rather than listing them in the public component API.

Hidden or infrastructure-oriented nodes are not automatically exempt. Status
regions, live regions, hidden inputs, portals, anchors, overlay layers, and
measurement wrappers still need the same decision. Some of them should remain
private, but the decision must be documented instead of assumed.

Visible nodes are not automatically safe public parts either. If splitting a
node out would make consumers responsible for behavior that belongs to the
adapter or agnostic machine, keep that behavior-critical subpart inside a
larger public part and document the boundary. Examples include triggers that
own roving focus, keyboard dispatch, DOM refs, ARIA relationships, drag/drop
wiring, or close affordance event handling. In those cases, expose the nearest
safe public part, provide stable `data-ars-part` / state selectors, and offer a
semantic customization prop or typed renderer for purely visual content. Do
not publish a standalone part that looks composable but silently requires
consumers to rebuild component-owned policy.

When the supported styling path for a private behavior-critical subpart is
selector-based, document that path concretely in the adapter spec. Include the
exact `data-ars-part` names and at least one Tailwind arbitrary-variant example
showing where classes should be attached, for example on `List`, `Root`, or a
larger public part. Also name any visual-content hook, such as a
`close_trigger` glyph/content prop, and state that it customizes only content,
not the wrapper semantics or event policy.

Raw adapter attr props are a public-API smell. A Leptos component that accepts
`Vec<LeptosAttribute>`, or a Dioxus component/helper that accepts raw
`Vec<Attribute>` without `#[props(extends = GlobalAttributes)]`, is usually an
internal renderer helper. Keep it private, or redesign it to read context and
expose normal consumer styling (`class` / `style` in Leptos,
`GlobalAttributes` in Dioxus).

### Typed Collection Renderers

Collection-backed components must keep one semantic source of item order. The
root primitive should receive the typed collection source, and collection
parts should iterate that source instead of asking consumers to repeat keys in
child markup.

Put typed collection render callbacks on the primitive that owns the typed
collection source unless there is a stronger ergonomic reason not to. When
styling requires public per-row anatomy, it is acceptable for collection parts
to be generic and own the renderer props, as with `tabs::List<K> { tab_row: … }`
and `tabs::Panels<K> { panel: … }`. That trades default-call inference for
direct composition of public parts; every default call site must then spell the
key type explicitly. Document that tradeoff in the adapter spec.

When consumers need custom per-item anatomy, prefer typed render callbacks
that receive an item render context selected from the root collection:

```rust,no_check
pub struct ItemRenderItem<K: ItemKey> {
    pub item: Item<K>,
}
```

The callback may wrap or arrange public adapter parts, but it must not
recompute selection, disabled state, keyboard policy, ARIA relationships,
drag/drop placement, or localized component messages. If the renderer needs
extra state, add an agnostic render-state helper first and pass that state
through both adapters.

When a typed row shell owns a row item and renders smaller child parts, prefer
having the shell publish the row item through typed framework context and make
the child parts read that context. This keeps examples and source templates
focused on anatomy instead of repeated `item.clone()` plumbing. It is fine for
the child parts to require an explicit key generic, such as
`Trigger<SettingsTab>` or `Trigger::<SettingsTab>`, when Rust cannot infer the
type after the item prop is removed. Keep a test proving those child parts
resolve the row from the shell context and still receive the correct labels,
ids, state attributes, and event wiring.

For Dioxus, the usual shape is:

```rust,no_check
#[props(optional)]
pub render_item: Option<Callback<ItemRenderItem<K>, Element>>;
```

For Leptos, prefer a small public wrapper around a cloneable callback when the
`view!` macro or `leptosfmt` makes inline typed closure props brittle:

```rust,no_check
#[derive(Clone)]
pub struct ItemRenderer<K: ItemKey>(/* private */);

impl<K, F, V> From<F> for ItemRenderer<K>
where
    K: ItemKey,
    F: Fn(ItemRenderItem<K>) -> V + Send + Sync + 'static,
    V: IntoView + 'static,
{
    /* ... */
}
```

Use local renderer values in examples and tests when the closure has an
explicit item type:

```rust,no_check
let render_item = ItemRenderer::from(|item: ItemRenderItem<MyKey>| {
    view! { <items::ItemShell item=item /> }
});

view! {
    <items::Root items render_item=render_item>
        <items::List />
    </items::Root>
}
```

Put renderer props on the root when that preserves key-type inference for the
plain collection parts (`<items::List/>`, `<items::Panels/>`, `items::List {}`,
`items::Panels {}`). Use generic collection parts only when the framework can
infer the key type ergonomically and formatter-safe call sites remain clear.

### Required Structural Parts And Fallbacks

Some structural nodes are required for accessibility or native behavior even
when a consumer does not render the corresponding compound part. Use this
pattern:

- expose a public part for styling or placement;
- render an unstyled adapter fallback when the public part is omitted;
- suppress the fallback when the explicit part is present, so only one semantic
  node exists;
- keep required roles, ARIA, ids, relationships, event behavior, and generated
  text owned by the machine or adapter;
- do not let consumer children replace required live-region, hidden-input, or
  relationship content unless the component spec explicitly makes that content
  consumer-owned.

For example, a form status live region can be a public `StatusRegion` part for
styling and placement, while `Form` still auto-renders an unstyled fallback when
the part is omitted. The status message source remains `status_message` and the
core form machine, not arbitrary consumer children.

For Dioxus part components, use
`#[props(extends = GlobalAttributes)] attrs: Vec<Attribute>` for each stylable
part unless the part has a documented semantic reason to restrict attributes.
For Leptos part components, expose `class: Option<TextProp>` and
`style: Option<TextProp>` on stylable parts until the adapter has a broader
global-attributes surface. Merge those attrs with the agnostic part attrs.

Tailwind widget examples for multi-part components should style the public part
components directly, or use Tailwind arbitrary variants over `data-ars-*`
anatomy. Do not inject raw component CSS strings into Rust examples to style
internal parts. If a demo cannot be written without raw CSS or private
anatomy, promote the needed primitive part or add the styled wrapper to
`ars-*-components` first.

For private subparts that deliberately remain internal, Tailwind examples may
target their stable `data-ars-part` attrs from a safe public ancestor. Keep the
selector local to the component root/list/panel and do not use this as an
excuse to rebuild component behavior in example code.

### Styled Source Templates

Use `ars-leptos-components` and `ars-dioxus-components` for checked-in
reference implementations of ready-made visual components inspired by React
Spectrum outcomes. Each styled crate may expose CSS and Tailwind variants, for
example `input::checkbox::css::Checkbox` and
`input::checkbox::tailwind::Checkbox`, built by composing the adapter
primitives.

These crates are not the final customization boundary. They are the source of
truth for the component source that the future `ars-ui` binary will copy into a
user application, shadcn/ui style. Installed components should become
user-owned files such as `src/components/ars/checkbox.rs` plus
`src/components/ars/checkbox.css` for CSS variants, or a Rust file with static
Tailwind classes for Tailwind variants. After installation, users customize the
copied source directly.

Organize styled templates by component category, not by styling system. The
canonical crate layout is `src/<category>/<component>/`, for example
`src/input/checkbox/css.rs`, `src/input/checkbox/tailwind.rs`, and
`src/input/checkbox/checkbox.css`. CSS variants must have a real adjacent CSS
file that can be copied with the component source. That CSS file must include
plain comments documenting what each component part and state selector styles,
so users who receive copied source can safely tweak the visual design. Do not
add top-level variant-first module trees such as `css::checkbox` or
`tailwind::checkbox`. Those crates are source-template staging areas for the
future installer, so category-first paths should be the only public shape.

Tailwind source templates must keep class strings inline on the rendered
elements instead of hiding them behind `const` identifiers. Inline strings make
the copied component source easier to edit and allow Tailwind-aware editor
extensions to provide completion, hover, and class validation at the exact
markup location. Use helper functions only for semantic logic; do not move
ordinary Tailwind class lists out of the component markup.

Tailwind variants must be fully styleable by editing the copied component
source. Do not require users to add component-specific rules to a separate
`tailwind.css` file to reach supported visual states. If a Tailwind template
needs to style an outer public part based on inner component state, promote
that renderer-independent state to agnostic `data-ars-*` attrs on the public
part first, then consume those attrs with direct Tailwind variants such as
`data-ars-selected:...`, `data-ars-focus-visible:...`, or
`group-data-ars-selected:...`. Reserve bracketed data variants such as
`data-[size=large]:...` for value checks, and reserve scoped arbitrary variants
for custom selectors that the named variants cannot express. External CSS is
acceptable only for the CSS variant, global app chrome, or truly
application-owned styling. When editing Tailwind component templates or
widgets, follow `.agents/skills/tailwind-v4/SKILL.md` and run its warning
sweeps before handoff.

Styled widget crates (`examples/widgets-*-css` and
`examples/widgets-*-tailwind`) must consume the high-level styled components
from `ars-*-components`, not manually compose the low-level adapter parts.
Those galleries prove the ready-made CSS/Tailwind source-template experience.
Only the unstyled widget crates (`examples/widgets-leptos` and
`examples/widgets-dioxus`) should demonstrate direct primitive composition with
adapter parts such as `Root`, `List`, `TabShell`, `Trigger`, `CloseTrigger`,
`Panels`, and `Panel`. Adapter tests and E2E fixtures still cover the
low-level primitives directly.

For draggable compound parts, cursor state is part of the visual contract.
The same rest cursor should apply to the drag source and every visible
interactive child inside it, and the active drag cursor must override child
cursors. In Tailwind templates, put `cursor-pointer` on clickable shells and
children, `data-ars-dragging:cursor-grabbing` on the state-owning shell, and
`group-data-ars-dragging:cursor-grabbing` on child parts whose own cursor
rules would otherwise win. Use `data-ars-disabled:cursor-not-allowed` /
`aria-disabled:cursor-not-allowed` for disabled states rather than leaving a
clickable cursor on disabled controls.

Styled source templates may expose semantic props and root-level customization
(`class`/`style` for Leptos, `GlobalAttributes` for Dioxus), but they should
not grow per-part prop families. Consumers who need deep styling before the
CLI exists should compose adapter primitives directly. Once the CLI exists,
deep customization should happen by editing copied component source instead of
wrapping a closed package component.

Because styled templates are copied into user applications, each template Rust
file must import framework and ars-ui APIs only through the matching adapter
prelude:

```rust
use ars_leptos::prelude::*;
```

or:

```rust
use ars_dioxus::prelude::*;
```

Do not import directly from `leptos`, `dioxus`, `ars_forms`, deep adapter
modules such as `ars_leptos::input::checkbox`, or other foundation crates in a
styled template. If copied component source needs a type or helper, re-export
the user-facing item from the adapter prelude first and then consume it from
there. This keeps installed components easy to paste into ordinary
applications with one adapter dependency and one predictable import.

Plain widgets (`examples/widgets-leptos` and `examples/widgets-dioxus`) must
demonstrate the unstyled adapter primitives directly. They are the reference
for component anatomy and primitive composition, not visual polish. Never import
`ars-leptos-components` or `ars-dioxus-components` from a plain widget just
because a styled template exists. CSS widgets must import the CSS styled source
templates from `ars-*-components`, and Tailwind widgets must import the Tailwind
styled source templates.

When a merge helper needs to inspect an existing `AttrMap` value and then
overwrite that same key, use `AttrMap::take()` instead of `get()` plus a clone.
This is especially relevant for class merging in adapters:
`merge_consumer_class_prop_into` consumes the existing component-owned class,
builds the merged reactive value, and sets the key once.

## Prop Ergonomics

Annotate `String` and `Option<String>` props with `#[prop(into)]` when the
framework supports it and the local adapter convention uses it.

Prefer `#[prop(into)]` / `#[props(into)]` for user-facing callback, signal,
memo, text, and view props whenever the framework macro supports the
conversion. Call sites should pass closures, signals, translated memos, and
view closures directly instead of wrapping them with `Callback::new(...)`,
`Signal::derive(...)`, `ViewFn::from(...)`, `.into()`, or `EventHandler`
constructors. Keep explicit wrappers only when a value is intentionally shared
across multiple props, stored as a reusable local, or the macro cannot infer
the conversion clearly. Wrapper/styled components should also forward optional
callback props as `Option<Callback<_>>` / `Option<EventHandler<_>>` instead of
creating no-op defaults just to satisfy a child component.

Keep semantic data separate from rendered views. Rich framework views are useful
for customization, but components still need semantic sources for accessible
names, ids, announcements, form serialization, and state-machine wiring.

For Leptos adapter APIs, use `TextProp` for user-facing semantic text props that
must accept static strings, provider-localized text, or application state:
placeholders, accessible labels, status text, validation messages, live
announcements, empty-state text, and semantic labels behind custom views. The
public `t(...)` helper returns `Memo<String>`, which converts into
`TextProp`, so examples and widgets should pass `t(MessageKey)` without a
parallel helper.

For Dioxus adapter APIs and widgets, use the hookless `t(MessageKey)` helper as
the default inline translation path. It reads the current `ArsProvider` context
without consuming a hook slot, so it is safe inside conditional `rsx!` branches,
iterator closures, and small render expressions while still subscribing to
locale and signal-backed message reads during render. Use `use_t(MessageKey)`
only when the component needs a reusable `Memo<String>` handle, such as a value
stored for repeated use in the same render path or passed into an API that
expects a memo. Because `use_t` is a hook, call it unconditionally at the top
level of the component before conditional rendering.

Resolve component message bundles together with the locale used to select them.
Leptos uses `use_messages_and_locale(...) -> Signal<(M, Locale)>`; Dioxus uses
`use_messages_and_locale(...) -> (M, Locale)` because `use_memo` would require
an extra `PartialEq` bound not guaranteed by `ComponentMessages`. Do not resolve
messages and then separately read locale for the same render path.

Do not use reactive text types for DOM tokens and relationships that the browser
serializes as identifiers: `id`, `form`, `name`, `aria-labelledby`,
`aria-describedby`, `aria-controls`, and similar IDREF props should remain
static strings unless the component spec names a reactive association. Leptos
consumer styling props are the exception: use `TextProp` for `class` and raw
`style` escape hatches when exposed, and merge them into the adapter attr map.
Native form ownership is `form="form-id"`; a `NodeRef` helper may only be an
ergonomic addition if it resolves to a stable element ID.

Use typed semantic enums for well-known HTML vocabularies when the component
owns the helper element. For example, Field's native input helper exposes
`InputType` instead of raw `type` strings, with `#[non_exhaustive]` on the enum
so future HTML input types can be added without breaking consumers.
