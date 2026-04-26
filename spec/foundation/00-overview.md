# ars-ui: Project Overview

## 1. Vision Statement

**ars-ui** is a Rust-native, framework-agnostic UI component library that brings production-grade, accessible, and internationalized components to the Rust web ecosystem. It combines the architectural approach of [Ark UI](https://ark-ui.com/) (state machine-driven components via Zag.js) with the deep accessibility and internationalization rigor of [React Aria](https://react-aria.adobe.com/) by Adobe.

The core component logic lives in pure Rust state machines with zero framework dependency. Thin adapter layers bridge these machines into **Leptos** and **Dioxus**, making ars-ui the first framework-agnostic Rust UI component library spanning multiple frameworks.

### 1.1 Why ars-ui?

- **No existing solution**: There is no framework-agnostic Rust UI component library today. Radix Leptos is Leptos-only; Dioxus has no mature headless library.
- **Type safety**: Rust's type system catches state machine errors, invalid ARIA usage, and prop misconfigurations at compile time.
- **Performance**: Zero-cost abstractions, no virtual DOM overhead in core, direct DOM manipulation where possible.
- **WASM-native**: Designed from the ground up for WebAssembly, not ported from JavaScript.
- **Write once, adapt twice**: Component logic written once powers both Leptos and Dioxus (and future frameworks).

## 2. Design Principles

### 2.1 Accessibility First

Every component meets **WCAG 2.1 Level AA** minimum. WAI-ARIA Authoring Practices are the specification, not a nice-to-have. Components are tested against screen reader behavior across VoiceOver, NVDA, JAWS, and TalkBack.

### 2.2 Framework-Agnostic Core

State machines in `ars-core` have **zero framework dependency**. They operate on abstract state transitions and produce DOM attribute maps. They never import Leptos, Dioxus, web-sys, or any rendering library. Every aspect and behavior MUST have proper documentation and tests.

### 2.3 Type Safety Over Runtime Checks

Prefer compile-time guarantees:

- State enums make invalid states unrepresentable
- Typed ARIA attributes prevent typos
- Props structs with builder patterns enforce required fields
- Phantom types for state machine lifecycle (Created, Running, Stopped)
- Avoid strings when you can use enums, strings are more error-prone and harder to maintain
- Use enums for error handling instead of strings

### 2.4 Incremental Adoption

Each component is usable independently. Users can adopt a single `Checkbox` without pulling in the entire library. Feature flags control what gets compiled.

### 2.5 Safe Rust

Avoid `unsafe` blocks unless absolutely necessary and no safe alternative exists. Never leak memory (e.g., `Box::leak`, `mem::forget`) to work around lifetime constraints — restructure the code instead. Prefer safe abstractions that leverage Rust's ownership and borrowing system.

### 2.6 Performance

- State machines are allocation-free where possible
- No unnecessary cloning — leverage Rust ownership
- Lazy evaluation of computed properties
- Zero-cost ARIA attribute generation (compile-time string interning)

### 2.7 Headless by Default

Zero styling opinions in core. Components emit data attributes (`data-ars-state`, `data-ars-part`) for CSS targeting but never impose CSS. Users bring their own design system.

### 2.8 Internationalization Built-In

Every component supports RTL layouts, locale-aware formatting, and translatable accessibility strings out of the box, not as an afterthought.

## 3. Naming Conventions

### 3.1 Crate Names

| Crate              | Purpose                                                                                                                                                             |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ars-core`         | State machine engine, `Machine` trait, `Service`, `ConnectApi` trait, `ComponentPart` trait, `AttrMap`, `Callback<T>`, `MessageFn<T>`, `WeakSend<T>`, `Bindable<T>` |
| `ars-a11y`         | ARIA types, focus management, keyboard navigation, screen reader utilities                                                                                          |
| `ars-i18n`         | Locale system, RTL, date/number/string formatting, calendar systems                                                                                                 |
| `ars-interactions` | Press, hover, focus, long press, move, drag-and-drop abstractions                                                                                                   |
| `ars-collections`  | Collection trait, selection model, virtualization, async loading                                                                                                    |
| `ars-forms`        | Validation framework, form context, field association, hidden-input helpers                                                                                         |
| `ars-components`   | Framework-agnostic component machines and connect APIs                                                                                                              |
| `ars-derive`       | Internal proc-macro crate for `#[derive(HasId)]` and `#[derive(ComponentPart)]`                                                                                     |
| `ars-dom`          | web-sys/wasm-bindgen DOM utilities, positioning, portal, focus, scroll, URL sanitization, background inert, modality tracking, media queries                        |
| `ars-leptos`       | Leptos adapter — component wrappers, signal integration                                                                                                             |
| `ars-dioxus`       | Dioxus adapter — component wrappers, signal integration                                                                                                             |

> **Canonical definition:** The full crate dependency graph with feature flags is in `01-architecture.md` §1 (Crate Structure). This table is a summary.

### 3.2 Module Naming

Within each crate, modules follow `snake_case`:

```text
ars-core/
  src/
    machine.rs          // Machine trait
    service.rs          // Service<M> runtime
    connect.rs          // ConnectApi trait, ComponentPart trait, AttrMap type
    bindable.rs         // Controlled/uncontrolled value abstraction
    transition.rs       // State transition types
    callback.rs         // Callback, MessageFn, WeakSend
```

### 3.3 Type Naming

Types use **module namespacing** to avoid name stuttering. Each component lives in its own module and uses short, generic names:

```rust
// In mod checkbox:
pub struct Machine;      // Machine struct -- used as checkbox::Machine externally
pub enum State { ... }    // States — used as checkbox::State externally
pub enum Event { ... }    // Events — used as checkbox::Event externally
pub struct Context { ... } // Internal context — checkbox::Context
pub struct Props { ... }   // Configuration — checkbox::Props
pub struct Api<'a> { ... } // Connect output — checkbox::Api
```

External usage: `checkbox::State`, `checkbox::Props`, `Service::<checkbox::Machine>::new(...)`.

**Domain-specific value types** keep their descriptive names (not shortened):

- `checkbox::State`, `InputType`, `selection::Mode`, `Orientation` (`Horizontal`/`Vertical`), `Direction` (`Ltr`/`Rtl`, defined in `ars-i18n`)
- `color::Value`, `color::Format`, `SortDirection`, `FilterMode`
- `select::Item`, `radio_group::Radio`, `file_upload::Item`, `slider::Mark`

### 3.4 Data Attributes

Components emit standardized data attributes for CSS targeting:

| Attribute                | Purpose                | Example                             |
| ------------------------ | ---------------------- | ----------------------------------- |
| `data-ars-scope`         | Component scope        | `data-ars-scope="accordion"`        |
| `data-ars-part`          | Part within component  | `data-ars-part="item-trigger"`      |
| `data-ars-state`         | Current state          | `data-ars-state="open"`             |
| `data-ars-disabled`      | Disabled state         | `data-ars-disabled` (presence)      |
| `data-ars-focus-visible` | Keyboard focus         | `data-ars-focus-visible` (presence) |
| `data-ars-highlighted`   | Highlighted item       | `data-ars-highlighted` (presence)   |
| `data-ars-selected`      | Selected item          | `data-ars-selected` (presence)      |
| `data-ars-orientation`   | Orientation            | `data-ars-orientation="horizontal"` |
| `data-ars-placement`     | Overlay placement      | `data-ars-placement="bottom-start"` |
| `data-ars-loading`       | Loading state          | `data-ars-loading` (presence)       |
| `data-ars-readonly`      | Read-only state        | `data-ars-readonly` (presence)      |
| `data-ars-draggable`     | Draggable element      | `data-ars-draggable` (presence)     |
| `data-ars-busy`          | Busy/animating state   | `data-ars-busy` (presence)          |
| `data-ars-expanded`      | Expanded state         | `data-ars-expanded` (presence)      |
| `data-ars-checked`       | Checked state          | `data-ars-checked="true"`           |
| `data-ars-pressed`       | Pressed state          | `data-ars-pressed` (presence)       |
| `data-ars-invalid`       | Validation error       | `data-ars-invalid` (presence)       |
| `data-ars-required`      | Required field         | `data-ars-required` (presence)      |
| `data-ars-indeterminate` | Indeterminate state    | `data-ars-indeterminate` (presence) |
| `data-ars-current`       | Current item marker    | `data-ars-current="date"`           |
| `data-ars-active`        | Active item            | `data-ars-active` (presence)        |
| `data-ars-open`          | Open state             | `data-ars-open` (presence)          |
| `data-ars-value`         | Current value          | `data-ars-value="50"`               |
| `data-ars-complete`      | Completed/filled state | `data-ars-complete` (presence)      |
| `data-ars-dragging`      | Active drag gesture    | `data-ars-dragging` (presence)      |
| `data-ars-half`          | Half-filled state      | `data-ars-half` (presence)          |

CSS example:

```css
[data-ars-scope="accordion"][data-ars-part="item-trigger"] {
    cursor: pointer;
}
[data-ars-scope="accordion"][data-ars-part="item-trigger"][data-ars-state="open"] {
    font-weight: bold;
}
```

> **CSS Specificity Note.** Combining multiple attribute selectors produces high specificity:
> `[data-ars-scope][data-ars-part][data-ars-state]` = **0-3-0**. A single class selector
> (0-1-0) will NOT override these rules without `!important`.
>
> **Recommendations for user style overrides:**
>
> - Use **CSS custom properties** (e.g., `--ars-accordion-trigger-font-weight`) exposed by
>   the component stylesheet. Custom properties cascade normally and avoid specificity battles.
> - Use **single attribute selectors** where possible (e.g., `[data-ars-state="open"]`) for
>   lower specificity (0-1-0).
> - **Performance:** Attribute selectors are slightly slower than class selectors for CSS
>   matching. For most applications this is negligible. In performance-critical scenarios
>   with thousands of styled elements, prefer single attribute selectors over multi-attribute
>   compound selectors.

## 4. Glossary of Terms

| Term               | Definition                                                                                                                                                                                                                                      |
| ------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Machine**        | A finite state machine definition encapsulating component logic — states, events, transitions, guards, actions                                                                                                                                  |
| **Service**        | A running instance of a machine. Created from a Machine type + props, can receive events and be queried for state                                                                                                                               |
| **connect**        | `Machine::connect()` — the method that transforms machine state into a typed `Api` struct with attribute maps and event handler methods                                                                                                         |
| **ConnectApi**     | A trait bound on `Machine::Api<'a>` providing `part_attrs(part) -> AttrMap` — returns data-only attribute maps (no event handlers; those are typed methods on the `Api` struct directly)                                                        |
| **Anatomy**        | The complete set of named DOM parts that compose a component                                                                                                                                                                                    |
| **Part**           | A single named DOM element within an anatomy (e.g., Root, Trigger, Content, Thumb)                                                                                                                                                              |
| **Adapter**        | A framework-specific binding layer that bridges state machines to framework components                                                                                                                                                          |
| **Bindable**       | A value abstraction supporting both controlled (externally managed) and uncontrolled (internally managed) modes                                                                                                                                 |
| **AttrMap**        | The output of an `Api` method (e.g., `root_attrs()`, `trigger_attrs()`) — a data-only map of HTML attributes, ARIA attributes, CSS classes, and inline styles for a specific part (no event handlers; those are typed methods on `Api` structs) |
| **Guard**          | A boolean condition that determines whether a state transition should occur                                                                                                                                                                     |
| **Action**         | A side-effect function executed during a state transition                                                                                                                                                                                       |
| **Effect**         | A long-running side effect active while a state is entered, cleaned up on exit                                                                                                                                                                  |
| **TransitionPlan** | A declarative description of what should happen in response to an event — target state, context mutation, follow-up events, and side effects                                                                                                    |
| **PendingEffect**  | A named side effect (with setup and cleanup) that the adapter must manage after a state transition                                                                                                                                              |
| **Presence**       | A utility component that manages mount/unmount with animation support — controls whether content is in the DOM                                                                                                                                  |

## 5. Relationship to Prior Art

### 5.1 From Zag.js / Ark UI

- **State machine architecture**: Component logic as finite state machines
- **Connect pattern**: Machine state → DOM props transformation
- **Anatomy system**: Named parts with data attributes
- **Compound component pattern**: Root/Trigger/Content composition
- **Component catalog**: 40+ production-ready component designs
- **Controlled/uncontrolled**: Bindable value pattern

### 5.2 From React Aria

- **Deep accessibility**: Full WAI-ARIA Authoring Practices implementation with screen reader testing
- **Focus management**: FocusScope, FocusRing, roving tabindex, aria-activedescendant
- **Internationalization**: 16 calendar systems, locale-aware formatting, RTL, 30+ languages
- **Interaction abstractions**: Press, hover, focus, move, drag-and-drop normalization
- **Collection model**: Unified Collection trait for lists, tables, trees with selection and virtualization
- **Component catalog**: More production-ready components that don't exist in Ark UI
- **Form validation**: Constraint validation, custom validation, server-side errors
- **Screen reader support**: LiveAnnouncer, VisuallyHidden, dynamic content announcements

### 5.3 Novel to ars-ui

- **Rust type safety**: Compile-time state machine validation, typed ARIA enums
- **Framework-agnostic Rust core**: First library spanning Leptos and Dioxus
- **Unified library**: Single source combining Ark UI breadth + React Aria depth (these are separate in JS)
- **WASM-optimized**: Designed for WebAssembly binary size and performance
- **ICU4X integration**: Rust-native internationalization (no JavaScript `Intl` dependency)

## 6. Non-Goals

- **Not a CSS framework or design system**: No visual styles, themes, or design tokens
- **Not targeting native mobile** (initially): Web-first, with potential Dioxus Desktop support
- **Not reimplementing browser primitives**: Use native `<button>`, `<input>` where appropriate
- **Not a complete React Aria port**: We take the patterns and requirements, not the React-specific implementation
- **Not backwards-compatible with any existing API**: Clean Rust-idiomatic design from scratch

## 7. Component Anatomy Developer Guide

Every ars-ui component is composed of named **parts** — discrete DOM elements that together form the component's structure. Understanding anatomy is essential for rendering, styling, and accessibility.

### 7.1 What Is a Part?

A **part** is a single named DOM element within a component's anatomy. Each part:

- Has a unique name within its component (e.g., `Root`, `Trigger`, `Content`, `Thumb`)
- Emits a `data-ars-part` attribute for CSS targeting (e.g., `data-ars-part="trigger"`)
- Receives ARIA attributes and event handlers from the `connect` function's `Api`
- May be **required** (component is broken without it) or **optional** (enhances functionality)

### 7.2 Required vs Optional Parts

| Marker       | Meaning                                                                              |
| ------------ | ------------------------------------------------------------------------------------ |
| **Required** | Must be rendered for the component to function. Omitting it breaks a11y or behavior. |
| **Optional** | Enhances UX but can be omitted. The component degrades gracefully.                   |

Example for **Dialog**:

```text
Dialog
├── Backdrop        (optional — visual overlay behind dialog)
├── Positioner      (required — positions dialog in viewport)
│   └── Content     (required — the dialog surface)
│       ├── Title       (required — aria-labelledby target)
│       ├── Description (optional — aria-describedby target)
│       └── CloseTrigger (optional — explicit close button)
└── Trigger         (optional — button that opens the dialog)
```

### 7.3 Anatomy Diagram Convention

Each component spec includes an anatomy section using this format:

```text
ComponentName
├── Root          (container element; always required)
├── PartA     [A] (required — role and purpose)
│   ├── SubPartA1 (optional — description)
│   └── SubPartA2 (required — description)
└── PartB         (optional — description)
```

**Legend — ARIA markers:**

| Marker        | Meaning                                                                                                                                                                                                                                |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `[A]`         | **ARIA part** — carries one or more ARIA attributes (`role`, `aria-*`) emitted by the `connect` function. These parts are accessibility-critical; omitting them or failing to spread their `AttrMap` will break screen reader support. |
| _(no marker)_ | **Structural part** — purely presentational or layout-oriented. Receives `data-ars-*` attributes for styling but no ARIA attributes. Safe to restyle or wrap without affecting accessibility semantics.                                |

> **Guideline for component spec authors:** When documenting a component's anatomy, annotate every part that receives ARIA attributes with the `[A]` marker. A part qualifies as `[A]` if its `*_attrs()` method sets any `HtmlAttr::Aria(...)` value or an HTML `role` attribute. Parts that only emit `data-ars-*` attributes, `id`, `style`, or event handlers are structural and should remain unmarked.

**Example — Dialog with ARIA markers:**

```text
Dialog
├── Backdrop              (optional — visual overlay behind dialog)
├── Positioner            (required — positions dialog in viewport)
│   └── Content       [A] (required — role="dialog", aria-labelledby, aria-describedby, aria-modal)
│       ├── Title     [A] (required — aria-labelledby target, referenced by Content)
│       ├── Description[A](optional — aria-describedby target, referenced by Content)
│       └── CloseTrigger  (optional — explicit close button)
└── Trigger           [A] (optional — aria-haspopup="dialog", aria-expanded, aria-controls)
```

The tree structure shows nesting relationships. Each component defines a `Part` enum (via `#[derive(ComponentPart)]`) whose variants correspond 1:1 to these anatomy parts. The `connect` function returns an `Api` struct with methods like `root_attrs()`, `trigger_attrs()`, `content_attrs()` that produce `AttrMap` values for each part. The `ConnectApi` trait's `part_attrs(part)` method dispatches to these concrete methods by matching on the `Part` enum.

### 7.4 Rendering Parts in Adapters

Adapters render each part as a framework element. The `Api` method returns an `AttrMap`, which is converted to framework-native attributes via `attr_map_to_leptos()` or `attr_map_to_dioxus()`, then spread onto the element:

```rust
// Leptos adapter example for Accordion
#[component]
pub fn Accordion(children: Children) -> impl IntoView {
    let api = use_context::<accordion::Api>().expect("must be inside Accordion");
    let strategy = use_style_strategy();
    let attrs = api.root_attrs();  // AttrMap for the Root part
    let result = attr_map_to_leptos(attrs, &strategy, None);
    view! {
        <div {..result.attrs}>
            {children()}
        </div>
    }
}
```

Developers MUST render all **required** parts. Optional parts can be omitted — the `Api` still returns valid attributes for them, but no DOM element is created.

### 7.5 Common Part Patterns

Many components share recurring part patterns:

| Pattern               | Parts                                                   | Used By                                     |
| --------------------- | ------------------------------------------------------- | ------------------------------------------- |
| **Trigger → Content** | `Trigger`, `Content`                                    | Dialog, Popover, Tooltip, Collapsible, Menu |
| **Root → Item[]**     | `Root`, `Item`                                          | Accordion, Tabs, RadioGroup, Menu           |
| **Field wrapper**     | `Root`, `Label`, `Input`, `Description`, `ErrorMessage` | TextField, NumberInput, Select              |
| **Thumb control**     | `Track`, `Thumb`, `Range`                               | Slider, Switch, ColorSlider                 |

---

## 8. Migration from Zag.js / Ark UI

ars-ui draws architectural inspiration from Zag.js (state machines) and Ark UI (component API design). This section maps key concepts for developers familiar with those libraries.

### 8.1 API Naming Mappings

| Zag.js / Ark UI     | ars-ui                           | Notes                           |
| ------------------- | -------------------------------- | ------------------------------- |
| `machine.connect()` | `use_<component>()` hook         | Returns context + props spread  |
| `api.isOpen`        | `ctx.is_open()`                  | Reactive getter                 |
| `collection` prop   | `items: Collection<T>`           | Trait-based, not JS array       |
| `onValueChange`     | `on_value_change: Callback<T>`   | Single generic callback         |
| `onOpenChange`      | `on_open_change: Callback<bool>` | Explicit open/close             |
| `highlightedValue`  | `highlighted_key`                | Uses `Key` type, not string     |
| `asChild`           | Component anatomy parts          | Explicit slot-based composition |

### 8.2 Key Behavioral Differences

1. **State Ownership**: Zag.js uses external state stores; ars-ui uses `Bindable<T>` for controlled/uncontrolled patterns with automatic two-way sync.
2. **Collection Model**: Ark UI passes plain arrays; ars-ui uses a trait-based `Collection<T>` with built-in virtualization, sections, and drag-reorder support.
3. **Type Safety**: Props are fully typed Rust structs with compile-time guarantees. No runtime prop validation needed.
4. **Peer Equivalents**: All Zag.js/Ark UI components have ars-ui counterparts. Zag.js `Editable` → ars-ui `Editable`; Zag.js `Clipboard` → ars-ui `Clipboard`; Ark UI `Environment` → not needed (adapter handles environment context).

---

## 9. Leptos ↔ Dioxus Quick Reference

This section provides a side-by-side comparison for developers porting component usage between the two supported adapters.

### 9.1 Signal / State Creation

```rust
// Leptos                                               // Dioxus
let (count, set_count) = signal(0);                     let mut count = use_signal(|| 0);
let derived = Memo::new(move |_prev| count() * 2);      let derived = use_memo(move || count() * 2);
```

### 9.2 Component Instantiation

```rust
// Leptos                                    // Dioxus
view! {                                      rsx! {
    <Checkbox                                    Checkbox {
        checked                                      checked: checked,
        on_checked_change=set_checked                on_checked_change: move |v| set_checked(v),
    />                                           }
}                                            }
```

### 9.3 Event Handling

```rust
// Leptos                                    // Dioxus
view! {                                      rsx! {
    <Button on_press=move |_| {                  Button {
        set_count.update(|n| *n += 1);               on_press: move |_| {
    } />                                                 count += 1;
}                                                    }
                                                 }
                                             }
```

### 9.4 Equivalent API Table

| Concept         | Leptos (`ars-leptos`)              | Dioxus (`ars-dioxus`)             |
| --------------- | ---------------------------------- | --------------------------------- |
| Reactive value  | `ReadSignal<T>` / `WriteSignal<T>` | `Signal<T>` (combined read/write) |
| Derived value   | `Memo::new(move \|_\| ...)`        | `use_memo(move \|\| ...)`         |
| Machine hook    | `use_machine(props)`               | `use_machine(props)`              |
| Context provide | `provide_context(val)`             | `use_context_provider(\|\| val)`  |
| Context consume | `use_context::<T>() → Option<T>`   | `use_context::<T>() → T` (panics) |
| Cleanup         | `on_cleanup(move \|\| ...)`        | `use_drop(move \|\| ...)`         |
| Children        | `Children` (boxed fn)              | `Element`                         |
| Named slots     | `#[slot]` macro                    | Explicit `Element` props          |
| Callback type   | `Callback<T>`                      | `Callback<T>`                     |
| Attr spreading  | `<div {..result.attrs}>`           | `div { ..result.attrs }`          |
| SSR feature     | `leptos/ssr`                       | `dioxus/server` feature           |

> **Note on Callback\<T\>:** `Callback<T>` is the ars-ui core callback type. It is NOT used in core `Machine::Props` (which contain only data). Adapter-level component props may use `Callback<T>` for event handlers (`on_change`, `on_submit`, etc.). Adapters convert to framework-native types at the boundary.
