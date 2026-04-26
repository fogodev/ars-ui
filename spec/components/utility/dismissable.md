---
component: Dismissable
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    react-aria: DismissButton
---

# Dismissable

Dismissable is the canonical specification for the shared dismissable behavior utility. It owns the
behavioral props and the structural dismiss-button helper used by overlays such as Dialog, Popover,
Tooltip, Select, and Menu.

Dismissable is intentionally split into:

- **behavior**: outside-interaction and Escape dismissal configuration
- **structure**: a shared dismiss-button attribute helper

Dismissable does **not** own user-facing wording. Callers resolve an appropriate localized label and
pass the final string to `dismiss_button_attrs`.

## 1. API

### 1.1 Props

```rust
/// Props for the `Dismissable` component.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Props {
    /// Called when the user interacts outside the dismissable element.
    /// The adapter invokes this on `pointerdown` outside, or `focusin` on an element outside.
    pub on_interact_outside: Option<Callback<dyn Fn(InteractOutsideEvent)>>,
    /// Called when the user presses the Escape key while focus is inside.
    pub on_escape_key_down: Option<Callback<dyn Fn()>>,
    /// Called when a dismiss trigger fires (combines outside interaction and Escape).
    pub on_dismiss: Option<Callback<dyn Fn()>>,
    /// When true, outside pointer events are intercepted and prevented from reaching
    /// underlying elements (pointer-events overlay). Default: false.
    pub disable_outside_pointer_events: bool,
    /// DOM IDs of elements that should NOT trigger an outside interaction when clicked.
    /// Typically includes the trigger button that opened the overlay.
    pub exclude_ids: Vec<String>,
}
```

`Props` contains only behavioral configuration. It does not carry locale, messages, or visual
styling.

### 1.2 Connect / Helper API

```rust
#[derive(ComponentPart)]
#[scope = "dismissable"]
pub enum Part {
    Root,
    DismissButton,
}

pub fn dismiss_button_attrs(label: &str) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DismissButton.data_attrs();
    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
    attrs.set(HtmlAttr::Role, "button");
    attrs.set(HtmlAttr::TabIndex, "0");
    attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
    attrs.set_bool(HtmlAttr::Data("ars-visually-hidden"), true);
    attrs
}
```

The helper is shared structure only. Overlay-specific message bundles own phrases such as
`"Dismiss popover"` or `"Close dialog"`.

## 2. Anatomy

```text
Dismissable
├── DismissButton  <button>  (visually hidden, start of region)
├── {content}
└── DismissButton  <button>  (visually hidden, end of region)
```

| Part            | Element    | Key attributes                                                                 |
| --------------- | ---------- | ------------------------------------------------------------------------------ |
| `DismissButton` | `<button>` | `data-ars-scope="dismissable"`, `data-ars-part="dismiss-button"`, `aria-label` |

Adapters should render the element as a native `<button>` whenever possible. The helper still sets
button semantics so the attrs remain usable with alternate render paths.

## 3. Accessibility

DismissButton exists so screen reader and keyboard users can dismiss an overlay without having to
discover Escape handling.

When `disable_outside_pointer_events` is true:

- only pointer interaction is blocked
- keyboard navigation must remain available
- Escape and DismissButton must continue to work

## 4. Behavior

| Trigger                                              | Action                                                                       |
| ---------------------------------------------------- | ---------------------------------------------------------------------------- |
| pointer interaction outside and not in `exclude_ids` | call `on_interact_outside`, then `on_dismiss` when not vetoed by the adapter |
| focus moves outside and not in `exclude_ids`         | call `on_interact_outside`, then `on_dismiss` when not vetoed by the adapter |
| Escape while focus is inside                         | call `on_escape_key_down`, then `on_dismiss`                                 |

Dismissable specifies the normalized behavior surface. Document listeners, node containment checks,
and SSR gating remain adapter responsibilities.

## 5. Integration

Overlay components compose Dismissable internally:

```rust
let dismissable = dismissable::Props {
    on_dismiss: Some(Callback::new_void(move || machine.send(Event::Close))),
    on_escape_key_down: Some(Callback::new_void(move || machine.send(Event::Close))),
    disable_outside_pointer_events: props.modal,
    exclude_ids: vec![trigger_id.clone()],
    ..Default::default()
};

let dismiss_label = (messages.dismiss_label)(locale);
let dismiss_button = dismissable::dismiss_button_attrs(&dismiss_label);
```

## 6. Library Parity

Compared against React Aria:

- ars-ui keeps the DismissButton concept.
- ars-ui also centralizes outside-interaction and Escape configuration in one utility surface.
- ars-ui intentionally does not define a shared message bundle here; wording belongs to the
  consuming overlay or application.
