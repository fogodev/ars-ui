---
document: component-spec-template
type: foundation
---

# Component Specification Template

This document defines the canonical section structure that every component spec file in `spec/components/` must follow. It ensures consistency across all 111 components, making specs easier to review, navigate, and maintain.

## 1. Component Tiers

Every component belongs to one of three tiers, declared in its YAML frontmatter via `tier:`. The tier determines which sections are required.

### 1.1 Stateless

Components with no state machine — pure prop-to-DOM mappings. Their §1 is titled "API" instead of "State Machine".

**Examples:** VisuallyHidden, Separator, Badge, Skeleton, Meter, Stat, AspectRatio, Heading, Highlight, Landmark, Keyboard, DownloadTrigger, Swap.

### 1.2 Stateful

Standard state machine components with no variant/extension sections. This is the most common tier.

**Examples:** Checkbox, Button, TextField, Switch, NumberInput, Accordion, Popover, Progress, Avatar, Tooltip, ScrollArea, Splitter, Toggle, ToggleGroup, RatingGroup, Clipboard.

### 1.3 Complex

Stateful components that also have variant or extension sections after the core sections. If a stateful component later grows a variant, it upgrades to complex.

**Examples:** Tabs (Closable, Reorderable), Table (Column Resizing, SelectAll), Calendar (Range Selection, Multiple Selection), Select, Combobox, Dialog, Toast, Slider (N-Thumb), ColorPicker, FileUpload, TreeView, DatePicker.

## 2. YAML Frontmatter

Every component file begins with YAML frontmatter:

```yaml
---
component: ComponentName
category: { category }
tier: stateless | stateful | complex
foundation_deps: [architecture, accessibility, ...]
shared_deps: [date-time-types, ...]
related: [sibling-component, ...]
---
```

The `tier` field is required. It must be one of `stateless`, `stateful`, or `complex`.

## 3. Canonical Section Structure

Sections are numbered sequentially with no gaps. If a conditional section is omitted, subsequent sections renumber to fill the gap.

### 3.1 Overview

```text
# ComponentName

## 1. State Machine                          — REQUIRED (stateful, complex)
  ### 1.1 States                             — REQUIRED
  ### 1.2 Events                             — REQUIRED
  ### 1.3 Context                            — REQUIRED
  ### 1.4 Props                              — REQUIRED
  ### 1.5 Guards                             — CONDITIONAL: when transition guards exist
  ### 1.N [component-specific subsections]   — CONDITIONAL
  ### 1.X Full Machine Implementation        — REQUIRED (always second-to-last under §1)
  ### 1.Y Connect / API                      — REQUIRED (always last under §1)

  — OR for stateless tier —

## 1. API                                    — REQUIRED (stateless only)
  ### 1.1 Props                              — REQUIRED
  ### 1.2 Connect / API                      — REQUIRED

## 2. Anatomy                                — REQUIRED (all tiers)

## 3. Accessibility                          — REQUIRED (all tiers)
  ### 3.1 ARIA Roles, States, and Properties — REQUIRED
  ### 3.2 Keyboard Interaction               — CONDITIONAL: interactive components
  ### 3.3 Focus Management                   — CONDITIONAL: when focus is programmatically managed
  ### 3.4 Screen Reader Announcements        — CONDITIONAL: when live regions are used

## N. Internationalization                   — REQUIRED (stateful, complex); CONDITIONAL (stateless)
  ### N.1 Messages                           — CONDITIONAL: when translatable strings exist
  ### N.M [topic subsections]                — RTL, locale formatting, BiDi, etc.

## N. Form Integration                       — CONDITIONAL: when forms is in foundation_deps

## N+ Variant: {Name}                        — CONDITIONAL (complex tier only)
  ### N.1 Additional Props                   — when variant adds props
  ### N.2 Additional Events                  — when variant adds events
  ### N.3 Additional Context                 — when variant adds context fields
  ### N.4 Behavior                           — when variant changes transitions/behavior
  ### N.5 Anatomy Additions                  — when variant adds/modifies parts
  ### N.6 Accessibility                      — when variant changes ARIA/keyboard behavior
  ### N.7 Messages                           — when variant adds translatable strings
```

### 3.2 Ordering Rules

1. **States, Events, Context, Props** always appear first under §1, in that exact order.
2. **Full Machine Implementation** is always the second-to-last subsection under §1.
3. **Connect / API** is always the last subsection under §1.
4. Component-specific subsections (e.g., RTL handling, locale display, async loading) go between Props (or Guards) and Full Machine Implementation.
5. Variant sections appear after all core sections, each as a top-level `##` heading.
6. Variant subsections only include what the variant actually adds or changes — omit subsections that don't apply.

### 3.3 Numbering Rules

1. Top-level sections use `## N.` with sequential integers starting at 1.
2. Subsections use `### N.M` under their parent.
3. Sub-subsections use `#### N.M.P` when needed.
4. Never skip numbers. If a conditional section is omitted, all following sections renumber.

## 4. Section Content Specifications

### 4.1 State Machine (stateful/complex) or API (stateless)

#### 4.1.1 For stateful and complex components

**§1.1 States** — A Rust enum of all states the machine can inhabit. Include a doc comment for each variant. For simple components, a table format is also acceptable:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Description of state.
    Idle,
    /// Description of state.
    Active,
}
```

**§1.2 Events** — A Rust enum of all events the machine handles. Each variant includes its payload (if any) and a doc comment:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Description.
    Toggle,
    /// Focus received.
    Focus { is_keyboard: bool },
    /// Focus lost.
    Blur,
}
```

**§1.3 Context** — A Rust struct holding all mutable machine state. Use `Bindable<T>` for controlled/uncontrolled values (see `01-architecture.md` §2.6). Include `ComponentIds` for part ID generation (see `03-accessibility.md` §2.6 for the full API — `ids.id()`, `ids.part()`, `ids.item()`):

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    pub value: Bindable<T>,
    pub disabled: bool,
    pub focused: bool,
    pub focus_visible: bool,
    pub ids: ComponentIds,
    // ... component-specific fields
}
```

**§1.4 Props** — A Rust struct of user-provided configuration. Must derive `HasId`, `Clone`, `Debug`, `PartialEq`. Must implement `Default`. Use `Option<T>` for controlled values (present = controlled, absent = uncontrolled):

```rust,no_check
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    pub id: String,
    pub value: Option<T>,        // controlled
    pub default_value: T,        // uncontrolled default
    pub disabled: bool,
    // ... component-specific fields
}

impl Default for Props { ... }
```

**§1.5 Guards** (conditional) — Named boolean functions that gate transitions:

```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
```

**§1.X Full Machine Implementation** — The complete `impl ars_core::Machine for Machine` block showing `init()`, `transition()`, `connect()`, `on_props_changed()` (when applicable), and **`initial_effects()` (when the machine can boot in a non-resting state)**. This is the single most important section of any component spec. It must cover every (state, event) combination the machine handles.

> **`initial_effects` requirement.** When `Machine::init` can return a state that represents an active lifecycle (an open overlay, a mounted presence region, a focused trigger, …) the spec MUST show a `Machine::initial_effects` override that returns the same effect set the equivalent transition would have produced. Without this override, an SSR-rendered or `default_open: true` instance does not get listeners attached, z-index allocated, or focus moved on first mount because no `Closed → Open` transition runs. Adapters drain these via `Service::take_initial_effects` on first mount. See `spec/foundation/01-architecture.md` §2.1.1 for the full contract. Components whose initial state is always the resting state (e.g., a Toggle that always boots `Off`) do not need to override the method — the default returns no effects.

**§1.Y Connect / API** — The Part enum, `Api<'a>` struct, and its per-part methods. Every component MUST define a Part enum with `#[derive(ComponentPart)]`. Each Part variant must have a corresponding `*_attrs()` inherent method returning `AttrMap`. Repeated parts that need instance-identity data (item key, step index) use data-carrying variants; field types must implement `Default` and should match the domain type (e.g., `Key` for collection-based components, `usize` for index-based components). Event handler methods follow the pattern `on_{part}_{event}()`. Must implement `ConnectApi`:

```rust,no_check
#[derive(ComponentPart)]
#[scope = "example"]
pub enum Part {
    Root,
    Control,
    // ... one variant per anatomy part
}

pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    pub fn root_attrs(&self) -> AttrMap { ... }
    pub fn control_attrs(&self) -> AttrMap { ... }
    pub fn on_control_click(&self) { ... }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Control => self.control_attrs(),
        }
    }
}
```

#### 4.1.2 For stateless components

**§1.1 Props** — Same requirements as stateful §1.4 Props.

**§1.2 Connect / API** — A Part enum and stateless `Api` struct created
directly from Props (no state/context). Returns `AttrMap` for each part via
`ConnectApi`.

The `Api` struct stores the full `Props` (`pub struct Api { props: Props }`),
not a hand-picked subset of fields. This keeps every stateless component on
the same shape, lets `Api::new` be a `const fn` (which extraction-style
storage typically cannot when fields like `String id` need to be dropped at
end-of-fn), and makes `Api` self-sufficient — adapters can read every Props
field through `Api` accessors without retaining `Props` separately. Provide
one accessor method per Props field (`id()`, plus one per component-specific
field) so the `Api` is a complete adapter-facing view of the component.

### 4.2 Anatomy

REQUIRED for all tiers. The component's `Part` enum (defined in §1.Y Connect / API) is the single source of truth for the anatomy. The Anatomy section documents the visual nesting structure and element mapping.

Contains:

1. **ASCII diagram** showing the part nesting hierarchy with role annotations. Part names match the Part enum variant names in kebab-case:

    ```text
    ComponentName
    ├── Root                (required)
    ├── Label               (required)
    ├── Control        [A]  (required — role="checkbox")
    ├── Indicator           (optional — aria-hidden)
    └── HiddenInput         (required — native form submission)
    ```

2. **Parts table** listing each part's element and key attributes. The `data-ars-scope` and `data-ars-part` attributes are set automatically via `ComponentPart::data_attrs()`:

| Part    | Element | Key Attributes                                    |
| ------- | ------- | ------------------------------------------------- |
| Root    | `<div>` | `data-ars-state`                                  |
| Control | `<div>` | `role="checkbox"`, `aria-checked`, `tabindex="0"` |

Parts must be marked as required (component broken without it) or optional (enhances UX, degrades gracefully).

### 4.3 Accessibility

REQUIRED for all tiers. Subsections:

**§3.1 ARIA Roles, States, and Properties** — REQUIRED. A table mapping each interactive part to its ARIA role and required `aria-*` attributes. Reference the `AriaRole` and `AriaAttr` enums from `03-accessibility.md` §2.

**§3.2 Keyboard Interaction** — CONDITIONAL (include for any keyboard-interactive component). A table of key bindings following WAI-ARIA Authoring Practices. Must document RTL arrow key reversal where applicable (reference `03-accessibility.md` §4.1 RTL matrix):

| Key   | Action               |
| ----- | -------------------- |
| Space | Toggle checked state |
| Tab   | Move focus to/from   |

**§3.3 Focus Management** — CONDITIONAL (include when the component manages focus programmatically). Document: initial focus placement, focus trapping (if modal), focus restoration on close, roving tabindex strategy if applicable.

**§3.4 Screen Reader Announcements** — CONDITIONAL (include when the component uses live regions for dynamic state changes). Document what is announced, when, and via which live region politeness level.

Additional subsections may be added for component-specific concerns (e.g., forced colors mode, virtual cursor containment, disabled element focus policy).

### 4.4 Internationalization

REQUIRED for stateful/complex. CONDITIONAL for stateless (include only if i18n concerns exist).

**§N.1 Messages** — CONDITIONAL but common. The `Messages` struct with `Cow<'static, str>` or `MessageFn` fields, following the pattern in `04-internationalization.md` §7. Must implement `Default` and `ComponentMessages`:

```rust,no_check
#[derive(Clone, Debug)]
pub struct Messages {
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages { ... }
impl ComponentMessages for Messages {}
```

Additional subsections for RTL handling, locale-aware formatting, BiDi isolation, calendar systems, pluralization rules — as needed by the component.

When a component has no translatable strings and no locale-sensitive behavior, a brief statement suffices:

> Label text is consumer-provided. `data-ars-state` values are stable API tokens, not localized. RTL: no special handling needed.

### 4.5 Form Integration

CONDITIONAL — include when `forms` appears in the component's `foundation_deps`. Document:

- **Hidden input pattern**: which hidden `<input>` elements are rendered, their `name` and `value` semantics.
- **Validation states**: how `valid`, `invalid`, `pending` states are reflected in ARIA and data attributes.
- **Error message association**: `aria-describedby` wiring to Description and ErrorMessage parts.
- **Required/optional**: semantic indication via `aria-required`.
- **Reset behavior**: what value the component resets to.
- **Disabled/readonly propagation**: from form context per `07-forms.md` §15.

### 4.6 Variant Sections (complex tier)

Each variant or extension gets its own top-level `##` section. Only include subsections that the variant actually adds or changes:

| Subsection         | Include when                                   |
| ------------------ | ---------------------------------------------- |
| Additional Props   | Variant adds new fields to Props               |
| Additional Events  | Variant adds new events to the Event enum      |
| Additional Context | Variant adds new fields to Context             |
| Behavior           | Variant changes transition logic               |
| Anatomy Additions  | Variant adds or modifies parts                 |
| Accessibility      | Variant changes ARIA roles, keyboard, or focus |
| Messages           | Variant adds translatable strings              |

## 5. Conformance Checklist

Use this checklist when reviewing or writing a component spec.

### All Tiers

- [ ] YAML frontmatter includes `component`, `category`, `tier`, `foundation_deps`, `shared_deps`, `related`
- [ ] Component name in frontmatter matches the `# Heading`
- [ ] Sections are numbered sequentially with no gaps
- [ ] §Anatomy exists with ASCII diagram and parts table
- [ ] §Accessibility exists with §3.1 ARIA table
- [ ] `Part` enum defined with `#[derive(ComponentPart)]` and `#[scope = "..."]`
- [ ] First `Part` variant is `Root` (unit variant)
- [ ] `Part::scope()` matches the component's kebab-case name
- [ ] Every `Part` variant has a corresponding `*_attrs()` inherent method on `Api`
- [ ] Data-carrying Part variants use field types that implement `Default` and match the domain type
- [ ] `ConnectApi` is implemented with `type Part` and `part_attrs()` dispatching to all variants
- [ ] Data-carrying variants are destructured in `part_attrs()` and forwarded to concrete methods

### Code-Block Hygiene (all tiers)

The Rust code blocks under §1 (and elsewhere in the spec) must compile under
the workspace lint policy without ad-hoc suppressions. Specifically:

- [ ] Every public item (struct, enum, trait, fn, const, type alias, method, field, variant) has a `///` doc comment (workspace `missing_docs = "warn"`)
- [ ] Every public type that can derive `Debug` has `#[derive(Debug)]` (workspace `missing_debug_implementations = "warn"`)
- [ ] `Props` derives `Default` where every field is `Default::default()` — use `#[derive(Default)]`, not a manual impl, unless a non-trivial default is required (clippy `derivable_impls`)
- [ ] `Api::new` is a `const fn` when its body is const-feasible (clippy `missing_const_for_fn`)
- [ ] `Api::new` and `Api::*_attrs` carry `#[must_use]` (calls without consumption are bugs)
- [ ] `Api` derives `Clone` and `Debug` so adapters can compose it freely
- [ ] `Api` exposes accessors (`id()`, plus one per Props field) so adapters do not need to retain `Props` separately
- [ ] Trait derive macros (`HasId`, `ComponentPart`) are used unqualified, with a single `use ars_core::{ComponentPart, HasId};` per file

### Stateful / Complex

- [ ] §1 is titled "State Machine"
- [ ] §1.1–1.4 are States, Events, Context, Props in that order
- [ ] Props derives `HasId`, `Clone`, `Debug`, `PartialEq` and implements `Default`
- [ ] Context uses `Bindable<T>` for controlled/uncontrolled values
- [ ] Full Machine Implementation shows complete `init()` and `transition()`
- [ ] Every (state, event) pair is handled or explicitly falls through to `_ => None`
- [ ] Connect / API is the last subsection under §1
- [ ] §Internationalization exists (even if brief)

### Complex Only

- [ ] Variant sections appear after all core sections
- [ ] Each variant section only contains subsections for what it changes
- [ ] Variant Messages follow the `MessageFn` pattern from `04-internationalization.md`

### Form Components

- [ ] `forms` is listed in `foundation_deps`
- [ ] §Form Integration section exists
- [ ] Hidden input pattern is documented
- [ ] `aria-describedby` wiring covers Description and ErrorMessage parts

## 6. Skeleton Examples

### 6.1 Stateless Skeleton (based on VisuallyHidden)

````markdown
---
component: ExampleStateless
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
---

# ExampleStateless

Brief description of the component's purpose.

## 1. API

### 1.1 Props

\```rust
/// Props for the `ExampleStateless` component. #[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
/// Component instance ID.
pub id: String,
// component-specific fields...
}

impl Props {
/// Returns fresh props with the documented defaults. #[must_use]
pub fn new() -> Self {
Self::default()
}

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    // Add one `#[must_use] pub const fn <field>(mut self, value: T) -> Self`
    // builder per non-`String` Props field, and a non-const `pub fn` builder
    // for any field whose value is built from `impl Into<...>`.

}
\```

### 1.2 Connect / API

\```rust
/// DOM parts of the `ExampleStateless` component. #[derive(ComponentPart)] #[scope = "example-stateless"]
pub enum Part {
/// The root element.
Root,
}

/// The API for the `ExampleStateless` component. #[derive(Clone, Debug)]
pub struct Api {
props: Props,
}

impl Api {
/// Creates a new `Api` instance from the given props. #[must_use]
pub const fn new(props: Props) -> Self { Self { props } }

    /// Returns the component's instance ID.
    #[must_use]
    pub fn id(&self) -> &str { &self.props.id }

    /// Returns the attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

}

impl ConnectApi for Api {
type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }

}
\```

## 2. Anatomy

\```text
ExampleStateless
└── Root <span> data-ars-scope="example-stateless" data-ars-part="root"
\```

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- No ARIA role required (passive rendering utility).
- Content remains in the accessibility tree.
````

### 6.2 Stateful Skeleton (based on Checkbox)

````markdown
---
component: ExampleStateful
category: input
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: []
---

# ExampleStateful

Brief description of the component's purpose.

## 1. State Machine

### 1.1 States

\```rust #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
/// Description.
Inactive,
/// Description.
Active,
}
\```

### 1.2 Events

\```rust #[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
/// Toggle between Inactive and Active.
Toggle,
/// Focus received.
Focus { is_keyboard: bool },
/// Focus lost.
Blur,
}
\```

### 1.3 Context

\```rust
/// Mutable context the machine threads through transitions.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current value, controlled or uncontrolled per `Props::value`.
    pub value: Bindable<State>,
    /// Whether interactive transitions are blocked.
    pub disabled: bool,
    /// Whether the control currently has focus.
    pub focused: bool,
    /// Whether the focus came from keyboard input (drives `:focus-visible`).
    pub focus_visible: bool,
    /// Component-instance ID set, derived from `Props::id`.
pub ids: ComponentIds,
}
\```

### 1.4 Props

\```rust
/// Props for the `ExampleStateful`component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled value. When`Some`, the component is controlled and
/// reflects the parent-supplied state.
pub value: Option<State>,
/// Initial state for the uncontrolled mode.
pub default_value: State,
/// Whether the component starts disabled.
pub disabled: bool,
}

impl Default for Props {
fn default() -> Self {
// Manual impl required because `default_value: State::Inactive` is
// not `State::default()` (the type has no derived Default), so
// `#[derive(Default)]` is not feasible here.
Self {
id: String::new(),
value: None,
default_value: State::Inactive,
disabled: false,
}
}
}
\```

### 1.5 Guards

\```rust
fn is_disabled(ctx: &Context) -> bool { ctx.disabled }
\```

### 1.6 Full Machine Implementation

\```rust
pub struct Machine;

impl ars_core::Machine for Machine {
type State = State;
type Event = Event;
type Context = Context;
type Props = Props;
type Messages = Messages;
type Effect = Effect; // bespoke per-component enum, or `ars_core::NoEffect` for machines that never emit named effects (see `01-architecture.md` §2.1.2)
type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let (initial, bindable) = match &props.value {
            Some(v) => (*v, Bindable::controlled(*v)),
            None => (props.default_value, Bindable::uncontrolled(props.default_value)),
        };
        let ctx = Context {
            value: bindable,
            disabled: props.disabled,
            focused: false,
            focus_visible: false,
            ids: ComponentIds::from_id(&props.id),
        };
        (initial, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if is_disabled(ctx) {
            match event {
                Event::Toggle => return None,
                _ => {}
            }
        }

        match (state, event) {
            (State::Inactive, Event::Toggle) => {
                Some(TransitionPlan::to(State::Active).apply(|ctx| {
                    ctx.value.set(State::Active);
                }))
            }
            (State::Active, Event::Toggle) => {
                Some(TransitionPlan::to(State::Inactive).apply(|ctx| {
                    ctx.value.set(State::Inactive);
                }))
            }
            (_, Event::Focus { is_keyboard }) => {
                let is_kb = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.focused = true;
                    ctx.focus_visible = is_kb;
                }))
            }
            (_, Event::Blur) => {
                Some(TransitionPlan::context_only(|ctx| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }
            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }

}
\```

### 1.7 Connect / API

\```rust
/// DOM parts of the `ExampleStateful` component. #[derive(ComponentPart)] #[scope = "example-stateful"]
pub enum Part {
/// The root container element.
Root,
/// The label text element.
Label,
/// The interactive control element (the focusable, ARIA-roled node).
Control,
}

/// The API for the `ExampleStateful` component.
///
/// Borrows from a `Service` snapshot. Carries the `send` closure so handler
/// methods can dispatch events without re-acquiring the service. `Debug` is
/// provided via a manual impl below because the `send: &'a dyn Fn(Event)`
/// field is not `Debug`-derivable.
pub struct Api<'a> {
state: &'a State,
ctx: &'a Context,
props: &'a Props,
send: &'a dyn Fn(Event),
}

impl<'a> fmt::Debug for Api<'a> {
fn fmt(&self, f: &mut fmt::Formatter<'\_>) -> fmt::Result {
f.debug_struct("example_stateful::Api")
.field("state", self.state)
.field("ctx", self.ctx)
.field("props", self.props)
.finish_non_exhaustive()
}
}

impl<'a> Clone for Api<'a> {
fn clone(&self) -> Self {
Self { state: self.state, ctx: self.ctx, props: self.props, send: self.send }
}
}

impl<'a> Api<'a> {
/// Returns the attributes for the root element. #[must_use]
pub fn root_attrs(&self) -> AttrMap {
let mut attrs = AttrMap::new();
let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
attrs.set(scope_attr, scope_val);
attrs.set(part_attr, part_val);
attrs.set(HtmlAttr::Data("ars-state"), match self.state {
State::Inactive => "inactive",
State::Active => "active",
});
if self.ctx.disabled {
attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
}
if self.ctx.focus_visible {
attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
}
attrs
}

    /// Returns the attributes for the label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// Returns the attributes for the focusable control element (per §3.1).
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Control.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::TabIndex, "0");
        attrs
    }

    /// Dispatches a `Toggle` event in response to a control activation.
    pub fn on_control_click(&self) { (self.send)(Event::Toggle); }
    /// Dispatches a `Focus` event with the input modality.
    pub fn on_control_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }
    /// Dispatches a `Blur` event when focus leaves the control.
    pub fn on_control_blur(&self) { (self.send)(Event::Blur); }

}

impl ConnectApi for Api<'\_> {
type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
        }
    }

}
\```

## 2. Anatomy

\```text
ExampleStateful
├── Root (required)
├── Label (required)
└── Control [A] (required — role, aria-checked, tabindex)
\```

| Part    | Element   | Key Attributes                                                          |
| ------- | --------- | ----------------------------------------------------------------------- |
| Root    | `<div>`   | `data-ars-scope="example-stateful"`, `data-ars-part="root"`             |
| Label   | `<label>` | `data-ars-scope="example-stateful"`, `data-ars-part="label"`            |
| Control | `<div>`   | `role="..."`, `aria-checked`, `tabindex="0"`, `data-ars-part="control"` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Property        | Value                        |
| --------------- | ---------------------------- |
| Role            | `...` on Control             |
| `aria-checked`  | `"true"` / `"false"`         |
| `aria-disabled` | Present when `disabled=true` |

### 3.2 Keyboard Interaction

| Key   | Action       |
| ----- | ------------ |
| Space | Toggle state |
| Tab   | Move focus   |

## 4. Internationalization

- Label text is consumer-provided.
- `data-ars-state` values are stable API tokens, not localized.

## 5. Form Integration

- Hidden `<input>` submits the current value.
- Reset restores `default_value`.
````

### 6.3 Complex Skeleton (based on Tabs)

````markdown
---
component: ExampleComplex
category: navigation
tier: complex
foundation_deps: [architecture, accessibility, interactions, collections]
shared_deps: []
related: []
---

# ExampleComplex

Brief description of the component's purpose.

## 1. State Machine

### 1.1 States

| State                   | Description        |
| ----------------------- | ------------------ |
| `Idle`                  | No item has focus. |
| `Focused { item: Key }` | An item has focus. |

### 1.2 Events

| Event                         | Payload       | Description                  |
| ----------------------------- | ------------- | ---------------------------- |
| `Select(Key)`                 | item key      | Activate an item.            |
| `Focus { item, is_keyboard }` | `Key`, `bool` | An item received focus.      |
| `Blur`                        | —             | Focus left.                  |
| `FocusNext`                   | —             | Move focus to next item.     |
| `FocusPrev`                   | —             | Move focus to previous item. |

### 1.3 Context

\```rust
/// Mutable context the machine threads through transitions.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current selected item, controlled or uncontrolled per `Props::value`.
    pub value: Bindable<Key>,
    /// Item currently holding focus, if any.
    pub focused_item: Option<Key>,
    /// Whether focus came from keyboard input (drives `:focus-visible`).
pub focus_visible: bool,
/// Layout orientation; affects arrow-key navigation semantics.
pub orientation: Orientation,
/// Document text direction; affects logical arrow-key mapping.
pub dir: Direction,
/// Ordered item set the component navigates over.
pub items: Vec<Key>,
// ...
}
\```

### 1.4 Props

\```rust
/// Props for the `ExampleComplex`component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Controlled value. When`Some`, the component is controlled and
/// reflects the parent-supplied selection.
pub value: Option<Key>,
/// Initial selected item for the uncontrolled mode.
pub default_value: Key,
/// Layout orientation; affects arrow-key navigation semantics.
pub orientation: Orientation,
/// Document text direction; affects logical arrow-key mapping.
pub dir: Direction,
/// Item keys for which interactive transitions are blocked.
pub disabled_keys: BTreeSet<Key>,
// ...
}

impl Default for Props { ... }
\```

### 1.5 Full Machine Implementation

\```rust
pub struct Machine;

impl ars_core::Machine for Machine {
// ... init() and transition() covering all (state, event) pairs
}
\```

### 1.6 Connect / API

\```rust #[derive(ComponentPart)] #[scope = "example-complex"]
pub enum Part {
Root,
List,
Item(Key), // (item_key)
Content(Key, Key), // (content_key, item_key)
}

pub struct Api<'a> { ... }

impl<'a> Api<'a> {
pub fn root_attrs(&self) -> AttrMap { ... }
pub fn list_attrs(&self) -> AttrMap { ... }
pub fn item_attrs(&self, item_key: &Key) -> AttrMap { ... }
pub fn content_attrs(&self, content_key: &Key, item_key: &Key) -> AttrMap { ... }

    pub fn on_item_click(&self, item_key: &Key) { ... }
    pub fn on_item_keydown(&self, item_key: &Key, data: &KeyboardEventData) { ... }
    pub fn on_item_focus(&self, item_key: &Key) { ... }
    pub fn on_item_blur(&self) { ... }

}

impl ConnectApi for Api<'\_> {
type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            Part::Item(ref item_key) => self.item_attrs(item_key),
            Part::Content(ref content_key, ref item_key) => self.content_attrs(content_key, item_key),
        }
    }

}
\```

## 2. Anatomy

\```text
ExampleComplex
├── Root
├── List role="tablist"
│ └── Item (×N) role="tab"
└── Content (×N) role="tabpanel"
\```

| Part    | Element    | Key Attributes                                       |
| ------- | ---------- | ---------------------------------------------------- |
| Root    | `<div>`    | `data-ars-scope`, `data-ars-part="root"`             |
| List    | `<div>`    | `role="tablist"`, `aria-orientation`                 |
| Item    | `<button>` | `role="tab"`, `aria-selected`, `aria-controls`       |
| Content | `<div>`    | `role="tabpanel"`, `aria-labelledby`, `tabindex="0"` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part    | Role       | Properties                        |
| ------- | ---------- | --------------------------------- |
| List    | `tablist`  | `aria-orientation`                |
| Item    | `tab`      | `aria-selected`, `aria-controls`  |
| Content | `tabpanel` | `aria-labelledby`, `tabindex="0"` |

### 3.2 Keyboard Interaction

| Key                           | Behavior                          |
| ----------------------------- | --------------------------------- |
| `ArrowRight` (horizontal LTR) | Focus next item                   |
| `ArrowLeft` (horizontal LTR)  | Focus previous item               |
| `Home`                        | Focus first item                  |
| `End`                         | Focus last item                   |
| `Enter` / `Space`             | Select focused item (manual mode) |

> RTL: Arrow keys reverse per `03-accessibility.md` §4.1.

### 3.3 Focus Management

- Roving tabindex: only the selected item has `tabindex="0"`.
- Arrow keys cycle focus; wraps if `loop_focus` is enabled.

## 4. Internationalization

- RTL: `dir="rtl"` reverses arrow key semantics for horizontal orientation.
- Labels are consumer-provided.

## 5. Closable Items

Items may be individually closable by the user.

### 5.1 Additional Props

\```rust
pub struct ItemDef {
pub key: Key,
pub label: String,
pub closable: bool,
}
\```

### 5.2 Additional Events

\```rust
CloseItem(Key), // item key
\```

### 5.3 Behavior

- When the active item is closed, selection moves to the next item.
- The event fires before removal — the consumer decides whether to actually remove.

### 5.4 Anatomy Additions

\```text
Item
├── Label
└── CloseTrigger (<button>; data-ars-part="close-trigger")
\```

### 5.5 Messages

\```rust #[derive(Clone, Debug)]
pub struct Messages {
pub close_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
fn default() -> Self {
Self { close_label: MessageFn::new(|label, \_locale| format!("Close {}", label)) }
}
}

impl ComponentMessages for Messages {}
\```

### 5.6 Accessibility

| Key      | Behavior                |
| -------- | ----------------------- |
| `Delete` | Close the focused item. |
````
