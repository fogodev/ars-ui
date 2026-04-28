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

Dismissable owns the shared generic `Messages` bundle for dismiss-button fallback wording, but it
does **not** resolve user-facing wording inside `Props` or the connect API. Callers resolve an
appropriate localized label and pass the final string to the connect API or to
`dismiss_button_attrs`.

## 1. API

### 1.1 Props

```rust
/// Why a dismissable surface was dismissed.
///
/// Passed to `on_dismiss` after the dismiss decision is finalized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DismissReason {
    /// A pointer event landed outside the dismissable surface and outside
    /// every registered inside-boundary or portal-owner.
    OutsidePointer,

    /// Focus moved to an element outside the dismissable surface.
    OutsideFocus,

    /// The user pressed `Escape` while the dismissable was the topmost overlay.
    Escape,

    /// One of the visually-hidden dismiss buttons (or the adapter handle's
    /// `dismiss`, e.g. `dismissable::Handle::dismiss`) was activated.
    DismissButton,
}

/// Veto-capable wrapper passed to `on_interact_outside` and `on_escape_key_down`.
///
/// Calling `prevent_dismiss()` sets a shared atomic flag the adapter checks
/// before dispatching `on_dismiss`. `Clone` shares the veto identity so
/// observation from any clone is visible to the original.
pub struct DismissAttempt<E> {
    pub event: E,
    veto: Arc<AtomicBool>,
}

impl<E> DismissAttempt<E> {
    pub fn new(event: E) -> Self { /* … */ }
    pub fn prevent_dismiss(&self) { /* … */ }
    pub fn is_prevented(&self) -> bool { /* … */ }
}

/// Localizable strings for the Dismissable structural helper.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the visually-hidden dismiss buttons.
    pub dismiss_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            dismiss_label: MessageFn::static_str("Dismiss"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Props for the `Dismissable` component.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Props {
    /// Called **before** the final dismiss decision. The callback receives a
    /// `DismissAttempt<InteractOutsideEvent>` and may call
    /// `prevent_dismiss()` on it to veto the upcoming `on_dismiss` invocation.
    /// The adapter fires this on `pointerdown` outside or `focusin` on an
    /// element outside the registered boundary.
    pub on_interact_outside:
        Option<Callback<dyn Fn(DismissAttempt<InteractOutsideEvent>) + Send + Sync>>,

    /// Called **before** the final dismiss decision when the user presses
    /// `Escape` while the dismissable is the topmost overlay. The callback
    /// receives a `DismissAttempt<()>` and may call `prevent_dismiss()` on
    /// it to veto the upcoming `on_dismiss` invocation.
    pub on_escape_key_down: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,

    /// Called **after** the dismiss decision is finalized — observational only,
    /// not cancelable. The callback receives a `DismissReason` identifying
    /// which path triggered the dismissal.
    pub on_dismiss: Option<Callback<dyn Fn(DismissReason) + Send + Sync>>,

    /// When true, outside pointer events are intercepted and prevented from
    /// reaching underlying elements (pointer-events overlay). Default: false.
    pub disable_outside_pointer_events: bool,

    /// DOM IDs of elements that should NOT trigger an outside interaction when
    /// clicked. Typically includes the trigger button that opened the overlay.
    ///
    /// **IDs are mandatory for participation.** Adapter containment walks
    /// the DOM ancestor chain comparing each node's `id` attribute (and
    /// `data-ars-portal-owner` for portaled subtrees). Elements without
    /// an `id` cannot be matched against `exclude_ids` or against the
    /// adapter's reactive `inside_boundaries` set — wrappers that need to
    /// register a node as inside-boundary must ensure it has an `id`.
    pub exclude_ids: Vec<String>,
}
```

`Props` contains only behavioral configuration. It does not carry locale, messages, or visual
styling.

The struct fields are public so adapter destructure patterns (and proptest fuzzers that map
generated values 1:1 to fields) keep working, but the documented construction path is the inherent
builder:

```rust
impl Props {
    pub fn new() -> Self;

    pub fn on_interact_outside<F>(self, f: F) -> Self
    where F: Fn(DismissAttempt<InteractOutsideEvent>) + Send + Sync + 'static;

    pub fn on_escape_key_down<F>(self, f: F) -> Self
    where F: Fn(DismissAttempt<()>) + Send + Sync + 'static;

    pub fn on_dismiss<F>(self, f: F) -> Self
    where F: Fn(DismissReason) + Send + Sync + 'static;

    pub fn disable_outside_pointer_events(self, value: bool) -> Self;
    pub fn exclude_ids<I, S>(self, ids: I) -> Self
    where I: IntoIterator<Item = S>, S: Into<String>;
}
```

Each callback setter accepts the closure directly (no `Some(Callback::new(_))` wrapping at the call
site) and `exclude_ids` accepts any `IntoIterator<Item: Into<String>>`. See §5 Integration for the
canonical chain.

### 1.2 Connect / Helper API

```rust
#[derive(ComponentPart)]
#[scope = "dismissable"]
pub enum Part {
    Root,
    DismissButton,
}

/// Stateless connect API for deriving Dismissable DOM attributes.
pub struct Api {
    props: Props,
    dismiss_button_label: AttrValue,
}

impl Api {
    /// Creates a new Dismissable attribute API.
    ///
    /// `dismiss_button_label` is the final accessible label for both visually-hidden
    /// dismiss buttons. It accepts static strings and reactive `AttrValue` inputs so
    /// adapters can pass provider-resolved localized labels without adding wording
    /// to `Props`.
    pub fn new(props: Props, dismiss_button_label: impl Into<AttrValue>) -> Self;

    /// Returns root container attributes for the dismissable boundary.
    ///
    /// The root is structural only. Document listeners, containment checks, and
    /// platform fallbacks remain adapter-owned.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.props.disable_outside_pointer_events {
            attrs.set_bool(HtmlAttr::Data("ars-disable-outside-pointer-events"), true);
        }
        attrs
    }

    /// Returns attributes for either visually-hidden dismiss button.
    pub fn dismiss_button_attrs(&self) -> AttrMap {
        dismiss_button_attrs(self.dismiss_button_label.clone())
    }

    pub fn disable_outside_pointer_events(&self) -> bool;
    pub fn exclude_ids(&self) -> &[String];
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::DismissButton => self.dismiss_button_attrs(),
        }
    }
}

pub fn dismiss_button_attrs(label: impl Into<AttrValue>) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = Part::DismissButton.data_attrs();
    attrs.set(scope_attr, scope_val);
    attrs.set(part_attr, part_val);
    attrs.set(HtmlAttr::Role, "button");
    // Force `type="button"` so dismiss controls inside a `<form>` never
    // double as the implicit submit button.
    attrs.set(HtmlAttr::Type, "button");
    attrs.set(HtmlAttr::TabIndex, "0");
    attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
    attrs.set_bool(HtmlAttr::Data("ars-visually-hidden"), true);
    attrs
}
```

The helper and connect API are shared structure only. The shared `Messages` bundle provides the
generic `"Dismiss"` fallback for adapter-level regions. Overlay-specific message bundles may own
more precise phrases such as `"Dismiss popover"` or `"Close dialog"`, resolve them before
constructing the API, and pass the final string or reactive `AttrValue` into Dismissable.

## 2. Anatomy

```text
Dismissable
├── DismissButton  <button>  (visually hidden, start of region)
├── {content}
└── DismissButton  <button>  (visually hidden, end of region)
```

| Part            | Element    | Key attributes                                                                                  |
| --------------- | ---------- | ----------------------------------------------------------------------------------------------- |
| `Root`          | container  | `data-ars-scope="dismissable"`, `data-ars-part="root"`                                          |
| `DismissButton` | `<button>` | `data-ars-scope="dismissable"`, `data-ars-part="dismiss-button"`, `aria-label`, `type="button"` |

Adapters should render the element as a native `<button>` whenever possible. The helper still sets
button semantics so the attrs remain usable with alternate render paths.

## 3. Accessibility

DismissButton exists so screen reader and keyboard users can dismiss an overlay without having to
discover Escape handling.

The anatomy in §2 specifies **two** visually-hidden DismissButtons — one at the start of the
region, one at the end. Both fire `on_dismiss(DismissButton)` identically; the duplication is
deliberate and serves three assistive-technology paths:

1. **Forward and backward tab exits.** When focus is trapped inside the overlay, `Tab` from the
   last interactive element wraps to the first and `Shift+Tab` from the first wraps to the last. A
   dismiss target at each boundary keeps the overlay one keystroke from dismissed regardless of
   direction.
2. **Reading-order proximity for screen readers.** SR users typically traverse overlays linearly.
   The start button is announced immediately when focus enters the overlay so users learn the exit
   up front; the end button is the next interactive stop after the user has read through the
   content top-to-bottom so they do not have to navigate back through the body to find a dismiss
   control.
3. **Rotor / element-list discovery.** Buttons-list rotors (VoiceOver, NVDA, JAWS) surface both
   instances, letting users pick whichever is closest to their current focus point.

Sighted users see neither button — `dismiss_button_attrs` sets `data-ars-visually-hidden`. The
duplication is strictly an assistive-technology concern; it has no visual cost.

When `disable_outside_pointer_events` is true:

- only pointer interaction is blocked
- keyboard navigation must remain available
- Escape and DismissButton must continue to work
- `Api::root_attrs()` emits `data-ars-disable-outside-pointer-events`

## 4. Behavior

| Trigger                                              | Action                                                                                                                                       |
| ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| pointer interaction outside and not in `exclude_ids` | call `on_interact_outside(DismissAttempt::new(InteractOutsideEvent::PointerOutside { … }))`, then `on_dismiss(OutsidePointer)` if not vetoed |
| focus moves outside and not in `exclude_ids`         | call `on_interact_outside(DismissAttempt::new(InteractOutsideEvent::FocusOutside))`, then `on_dismiss(OutsideFocus)` if not vetoed           |
| Escape while topmost                                 | call `on_escape_key_down(DismissAttempt::new(()))`, then `on_dismiss(Escape)` if not vetoed                                                  |
| visually-hidden DismissButton clicked / programmatic | call `on_dismiss(DismissButton)` directly (no veto-capable callbacks run first)                                                              |

Dismissable specifies the normalized behavior surface. Document listeners, node containment checks,
the **node-boundary registration helper** (`ars_dom::outside_interaction::target_is_inside_boundary`),
the **platform capability helper**
(`ars_dom::outside_interaction::install_outside_interaction_listeners`), and SSR gating remain
adapter responsibilities.

## 5. Integration

Overlay components compose Dismissable internally:

```rust
let dismissable = dismissable::Props::new()
    .on_dismiss(move |_reason: DismissReason| {
        machine.send(Event::Close);
    })
    // Wrappers that want to refuse dismissal — e.g. unsaved-form guards —
    // call `attempt.prevent_dismiss()` here. `on_dismiss` won't fire.
    .on_escape_key_down(move |_attempt: DismissAttempt<()>| {})
    .disable_outside_pointer_events(props.modal)
    .exclude_ids([trigger_id.clone()]);

let dismiss_label = overlay_messages.dismiss_label(locale);
let dismissable_api = dismissable::Api::new(dismissable, &dismiss_label);
let root = dismissable_api.root_attrs();
let dismiss_button = dismissable_api.dismiss_button_attrs();
```

## 6. Library Parity

Compared against React Aria:

- ars-ui keeps the DismissButton concept.
- ars-ui also centralizes outside-interaction and Escape configuration in one utility surface.
- ars-ui intentionally does not define a shared message bundle here; wording belongs to the
  consuming overlay or application, while the final resolved label is passed into the agnostic API.
