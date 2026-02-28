---
component: DropZone
category: utility
tier: stateful
foundation_deps: [architecture, accessibility, interactions, forms]
shared_deps: []
related: []
references:
  react-aria: DropZone
---

# DropZone

A standalone drag-and-drop target area that accepts dropped files or data and provides visual
feedback during drag-over. The `DropZone` validates file types, sizes, and counts before accepting
a drop. It can be reused internally by the FileUpload component. Maps to React Aria's `DropZone`.

## 1. State Machine

### 1.1 States

| State          | Description                                                                                |
| -------------- | ------------------------------------------------------------------------------------------ |
| `Idle`         | Default resting state. No active drag interaction.                                         |
| `DragOver`     | A drag operation is hovering over the drop zone.                                           |
| `DropAccepted` | A drop was successfully accepted. Remains until explicitly reset.                          |
| `DropRejected` | A drop was rejected (invalid types, too many files, etc.). Remains until explicitly reset. |

### 1.2 Events

| Event          | Payload             | Description                                                       |
| -------------- | ------------------- | ----------------------------------------------------------------- |
| `DragEnter`    | `DragData`          | A drag operation entered the drop zone. Types are validated.      |
| `DragOver`     | `DragData`          | A drag operation is hovering. Used for continuous feedback.       |
| `DragLeave`    | ---                 | The drag operation left the drop zone.                            |
| `Drop`         | `DragData`          | Items were dropped. Validated against constraints.                |
| `Reset`        | ---                 | Clear the drop state and return to idle.                          |
| `SetProps`     | ---                 | Sync context fields from updated props.                           |
| `DropActivate` | ---                 | Fired after `activate_delay_ms` while hovering in DragOver state. |
| `Focus`        | `is_keyboard: bool` | The drop zone received focus.                                     |
| `Blur`         | ---                 | The drop zone lost focus.                                         |

### 1.3 Domain Types

```rust
/// Data associated with a drag operation. Passed to `DragEnter`, `DragOver`, and `Drop` events.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DragData {
    /// The dragged items. May be empty on `DragEnter`/`DragOver` if the browser
    /// restricts access to item data until drop (security restriction).
    pub items: Vec<DragItem>,
    /// MIME types advertised by the drag source (always available, even before drop).
    pub types: Vec<String>,
}

// `DragItem` — defined in `05-interactions.md`
// `DropOperation` — defined in `05-interactions.md`
```

### 1.4 Context

```rust
/// The context for the `DropZone` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    /// MIME types to accept. Empty means accept all types.
    pub accept: Vec<String>,
    /// The maximum number of files that can be dropped.
    pub max_files: Option<usize>,
    /// The maximum file size in bytes.
    pub max_file_size: Option<u64>,
    /// The disabled state of the drop zone.
    pub disabled: bool,
    /// The focused state of the drop zone.
    pub focused: bool,
    /// The focus visible state of the drop zone.
    pub focus_visible: bool,
    /// The valid drag state of the drop zone.
    pub valid_drag: bool,
    /// The drop target state of the drop zone.
    /// `true` when a drag operation is hovering over the drop zone (state == DragOver).
    /// Adapters use this to apply visual feedback styles (e.g., highlighted border,
    /// background color change) indicating the zone is a valid drop target.
    pub is_drop_target: bool,
    /// The dropped items of the drop zone.
    pub dropped_items: Vec<DragItem>,
    /// The component IDs.
    pub ids: ComponentIds,
    /// The resolved locale for this component instance.
    pub locale: Locale,
    /// Resolved translatable messages.
    pub messages: Messages,
    /// Whether the drop zone is read-only.
    pub read_only: bool,
}
```

### 1.5 Drop Operation Type

On `dragenter`, the adapter sets `event.dataTransfer.effectAllowed` based on the union of `allowed_operations` (e.g., `copyMove` if both `Copy` and `Move` are present). On `dragover`, the adapter sets `event.dataTransfer.dropEffect` to the first matching allowed operation for cursor feedback. When `Props::get_drop_operation` is set, the adapter calls it with the current `DragData` and `allowed_operations` to determine the specific `DropOperation` to use; otherwise, the adapter falls back to static `allowed_operations` matching.

### 1.6 Props

```rust
/// Props for the `DropZone` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// MIME types to accept. Empty means accept all types.
    /// Matches HTML `<input accept>` naming convention.
    pub accept: Vec<String>,
    /// The maximum number of files that can be dropped.
    pub max_files: Option<usize>,
    /// The maximum file size in bytes.
    pub max_file_size: Option<u64>,
    /// The disabled state of the drop zone.
    pub disabled: bool,
    /// The label of the drop zone.
    pub label: String,
    /// The set of allowed drop operations for this drop zone.
    /// Controls the `effectAllowed` value set on `dragenter` and the cursor feedback via `dropEffect`.
    /// Default: `vec![DropOperation::Move]`.
    pub allowed_operations: Vec<DropOperation>,
    /// The form field name for this drop zone.
    /// When set, the dropped files are available via `Api::form_data()` for form submission.
    /// See section 5 for form integration details.
    pub name: Option<String>,
    /// Whether a file drop is required for form validation.
    pub required: bool,
    /// Whether the current value is invalid (set by form validation).
    pub invalid: bool,
    /// Whether the drop zone is read-only.
    /// Read-only DropZone displays previously dropped files but prevents new drops.
    /// The `form_data()` method still returns items for form submission.
    pub read_only: bool,
    /// Locale override. When `None`, inherits from nearest `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Localizable strings. When `None`, resolved via `resolve_messages()`.
    pub messages: Option<Messages>,
    /// Delay in milliseconds before firing `on_drop_activate` while hovering.
    /// Default: 500.
    pub activate_delay_ms: u32,
    /// Optional callback to determine the drop operation for a given drag.
    /// Receives the current `DragData` and the list of `allowed_operations`.
    /// Returns the `DropOperation` to use. When `None`, falls back to static
    /// `allowed_operations` matching.
    pub get_drop_operation: Option<Callback<(DragData, Vec<DropOperation>), DropOperation>>,
    /// Callback fired when a valid drop is accepted. Receives the accepted items.
    pub on_drop: Option<Callback<Vec<DragItem>>>,
    /// Fired when a drag operation enters the drop zone (drag hover starts).
    /// Receives the `DragData` associated with the entering drag.
    /// Maps to the `DragEnter` machine event.
    pub on_drop_enter: Option<Callback<DragData>>,
    /// Fired when a drag operation leaves the drop zone (drag hover ends without dropping).
    /// Maps to the `DragLeave` machine event.
    pub on_drop_exit: Option<Callback<dyn Fn() + Send + Sync>>,
    /// Fired continuously as the pointer moves over the drop zone during a drag.
    pub on_drop_move: Option<Callback<dyn Fn(DragMoveData) + Send + Sync>>,
    /// Fired when the pointer enters the drop zone (non-drag hover).
    pub on_hover_start: Option<Callback<dyn Fn() + Send + Sync>>,
    /// Fired after `activate_delay_ms` elapses while a drag hovers over the zone.
    pub on_drop_activate: Option<Callback<dyn Fn() + Send + Sync>>,
    /// Fired when the pointer leaves the drop zone (non-drag hover).
    pub on_hover_end: Option<Callback<dyn Fn() + Send + Sync>>,
    // Change callbacks provided by the adapter layer
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            accept: Vec::new(),
            max_files: None,
            max_file_size: None,
            disabled: false,
            label: String::new(),
            allowed_operations: vec![DropOperation::Move],
            name: None,
            required: false,
            invalid: false,
            read_only: false,
            locale: None,
            messages: None,
            activate_delay_ms: 500,
            get_drop_operation: None,
            on_drop: None,
            on_drop_enter: None,
            on_drop_exit: None,
            on_drop_move: None,
            on_drop_activate: None,
            on_hover_start: None,
            on_hover_end: None,
        }
    }
}
```

### 1.7 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, ComponentIds, AttrMap};

// ── States ───────────────────────────────────────────────────────────────────

/// The states for the `DropZone` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The default resting state. No active drag interaction.
    Idle,
    /// A drag operation is hovering over the drop zone.
    DragOver,
    /// A drop was successfully accepted. Remains until explicitly reset.
    DropAccepted,
    /// A drop was rejected (invalid types, too many files, etc.). Remains until explicitly reset.
    DropRejected,
}

// ── Events ───────────────────────────────────────────────────────────────────

/// The events for the `DropZone` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A drag operation entered the drop zone. Types are validated.
    DragEnter(DragData),
    /// A drag operation is hovering. Used for continuous feedback.
    DragOver(DragData),
    /// The drag operation left the drop zone.
    DragLeave,
    /// Items were dropped. Validated against constraints.
    Drop(DragData),
    /// Clear the drop state and return to idle.
    Reset,
    /// Sync context fields from updated props.
    SetProps,
    /// Fired after `activate_delay_ms` while in DragOver state.
    /// Adapter fires `on_drop_activate` callback.
    DropActivate,
    /// The drop zone received focus.
    Focus {
        /// Flag indicates keyboard vs pointer source.
        is_keyboard: bool,
    },
    /// The drop zone lost focus.
    Blur,
}

// ── Machine ──────────────────────────────────────────────────────────────────

/// The machine for the `DropZone` component.
pub struct Machine;

impl Machine {
    /// Checks whether the dragged types match the accepted types.
    /// If `accept` is empty, all types are accepted.
    fn validate_types(accept: &[String], dragged: &[String]) -> bool {
        if accept.is_empty() {
            return true;
        }
        dragged.iter().any(|t| accept.contains(t))
    }

    /// Validates a set of drag items against the component constraints.
    fn validate_drop(ctx: &Context, items: &[DragItem]) -> bool {
        // Check file count.
        if let Some(max) = ctx.max_files {
            if items.len() > max {
                return false;
            }
        }
        // Check individual file sizes.
        if let Some(max_size) = ctx.max_file_size {
            for item in items {
                if let DragItem::File { size, .. } = item {
                    if *size > max_size {
                        return false;
                    }
                }
            }
        }
        // Check MIME types.
        if !ctx.accept.is_empty() {
            for item in items {
                let mime = match item {
                    DragItem::File { mime_type, .. } => Some(mime_type.as_str()),
                    DragItem::Text(_) => Some("text/plain"),
                    DragItem::Html(_) => Some("text/html"),
                    DragItem::Uri(_) => Some("text/uri-list"),
                    DragItem::Custom { mime_type, .. } => Some(mime_type.as_str()),
                    DragItem::Directory { .. } => None,
                };
                if let Some(m) = mime {
                    if !ctx.accept.contains(m) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let ids = ComponentIds::from_id(props.id());
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        let ctx = Context {
            accept: props.accept.clone(),
            max_files: props.max_files,
            max_file_size: props.max_file_size,
            disabled: props.disabled,
            focused: false,
            focus_visible: false,
            valid_drag: false,
            is_drop_target: false,
            dropped_items: Vec::new(),
            ids,
            locale,
            messages,
            read_only: props.read_only,
        };

        (State::Idle, ctx)
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        let mut events = Vec::new();
        if old.disabled != new.disabled
            || old.accept != new.accept
            || old.max_files != new.max_files
            || old.max_file_size != new.max_file_size
            || old.read_only != new.read_only
            || old.locale != new.locale
        {
            events.push(Event::SetProps);
        }
        events
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        // Disabled guard: allow Focus/Blur so AT can still discover the element.
        if ctx.disabled && !matches!(event, Event::Focus { .. } | Event::Blur | Event::SetProps) {
            return None;
        }

        // Read-only guard: blocks DragEnter/Drop but allows Focus/Blur/Reset/SetProps.
        if ctx.read_only && matches!(event, Event::DragEnter(_) | Event::DragOver(_) | Event::Drop(_) | Event::DragLeave | Event::DropActivate) {
            return None;
        }

        match (state, event) {
            // ── SetProps (sync context from props) ────────────────────────────
            (_, Event::SetProps) => {
                let accept = props.accept.clone();
                let max_files = props.max_files;
                let max_file_size = props.max_file_size;
                let disabled = props.disabled;
                let read_only = props.read_only;
                let locale = resolve_locale(props.locale.as_ref());
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.accept = accept;
                    ctx.max_files = max_files;
                    ctx.max_file_size = max_file_size;
                    ctx.disabled = disabled;
                    ctx.read_only = read_only;
                    ctx.locale = locale;
                }))
            }
            // ── Drag enter ──────────────────────────────────────────────────
            // If `get_drop_operation` is set, call it to determine the operation;
            // otherwise fall back to static `allowed_operations` matching.
            // Adapter invokes `on_drop_enter` callback with the DragData.
            (State::Idle, Event::DragEnter(data)) => {
                let valid = Self::validate_types(&ctx.accept, &data.types);
                Some(TransitionPlan::to(State::DragOver).apply(move |ctx| {
                    ctx.valid_drag = valid;
                    ctx.is_drop_target = true;
                }).with_named_effect("drop_activate", |_ctx, props, send| {
                    let platform = use_platform_effects();
                    let delay = Duration::from_millis(props.activate_delay_ms as u64);
                    let handle = platform.set_timeout(delay, Box::new(move || {
                        send.call_if_alive(Event::DropActivate);
                    }));
                    let pc = platform.clone();
                    Box::new(move || pc.clear_timeout(handle))
                }))
            }

            // ── Drag enter from terminal states (auto-reset) ────────────────
            // Adapter invokes `on_drop_enter` callback with the DragData.
            (State::DropAccepted | State::DropRejected, Event::DragEnter(data)) => {
                let valid = Self::validate_types(&ctx.accept, &data.types);
                Some(TransitionPlan::to(State::DragOver).apply(move |ctx| {
                    ctx.dropped_items.clear();
                    ctx.valid_drag = valid;
                    ctx.is_drop_target = true;
                }).with_named_effect("drop_activate", |_ctx, props, send| {
                    let platform = use_platform_effects();
                    let delay = Duration::from_millis(props.activate_delay_ms as u64);
                    let handle = platform.set_timeout(delay, Box::new(move || {
                        send.call_if_alive(Event::DropActivate);
                    }));
                    let pc = platform.clone();
                    Box::new(move || pc.clear_timeout(handle))
                }))
            }

            // ── Drag over (continuous feedback) ─────────────────────────────
            (State::DragOver, Event::DragOver(_)) => {
                // Stay in DragOver — no state or context change needed.
                // The adapter handles `preventDefault()` to allow the drop.
                // The drop_activate timer continues running from DragEnter.
                None
            }

            // ── DropActivate (timer expired while hovering) ─────────────────
            (State::DragOver, Event::DropActivate) => {
                // Adapter fires `on_drop_activate` callback. No state change.
                Some(TransitionPlan::context_only(|_ctx| {
                    // Adapter invokes on_drop_activate callback here.
                }))
            }

            // ── Drag leave ──────────────────────────────────────────────────
            // Adapter invokes `on_drop_exit` callback.
            (State::DragOver, Event::DragLeave) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.valid_drag = false;
                    ctx.is_drop_target = false;
                }).cancel_effect("drop_activate"))
            }

            // ── Drop ────────────────────────────────────────────────────────
            (State::DragOver, Event::Drop(data)) => {
                let items = data.items.clone();
                let valid = Self::validate_drop(ctx, &items);
                if valid {
                    Some(TransitionPlan::to(State::DropAccepted).apply(move |ctx| {
                        ctx.dropped_items = items;
                        ctx.valid_drag = false;
                        ctx.is_drop_target = false;
                        // Adapter invokes on_drop callback with items here.
                    }).with_named_effect("announce", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        platform.announce(&(ctx.messages.drop_accepted_announcement)(&ctx.locale));
                        no_cleanup()
                    }).cancel_effect("drop_activate"))
                } else {
                    // Invalid drop — transition to DropRejected.
                    Some(TransitionPlan::to(State::DropRejected).apply(|ctx| {
                        ctx.valid_drag = false;
                        ctx.is_drop_target = false;
                    }).with_named_effect("announce", |ctx, _props, _send| {
                        let platform = use_platform_effects();
                        platform.announce(&(ctx.messages.drop_rejected_announcement)(&ctx.locale));
                        no_cleanup()
                    }).cancel_effect("drop_activate"))
                }
            }

            // ── Reset from terminal states ─────────────────────────────────
            (State::DropAccepted | State::DropRejected, Event::Reset) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.dropped_items.clear();
                }).cancel_effect("drop_activate"))
            }

            // ── Reset from DragOver (cancel in-progress drag) ──────────────
            (State::DragOver, Event::Reset) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx| {
                    ctx.valid_drag = false;
                    ctx.is_drop_target = false;
                    ctx.dropped_items.clear();
                }).cancel_effect("drop_activate"))
            }

            // ── Focus / Blur (keyboard fallback support) ────────────────────
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
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.8 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "drop-zone"]
pub enum Part {
    Root,
}

/// The API for the `DropZone` component.
pub struct Api<'a> {
    /// The current state of the drop zone.
    state: &'a State,
    /// The context of the drop zone.
    ctx: &'a Context,
    /// The props of the drop zone.
    props: &'a Props,
    /// The send function for the drop zone.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Returns HTML props for the drop zone root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut p = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        p.set(scope_attr, scope_val);
        p.set(part_attr, part_val);
        p.set(HtmlAttr::Data("ars-state"), match self.state {
            State::Idle => "idle",
            State::DragOver => "drag-over",
            State::DropAccepted => "drop-accepted",
            State::DropRejected => "drop-rejected",
        });

        if matches!(self.state, State::DragOver) {
            p.set_bool(HtmlAttr::Data("ars-drag-over"), true);
        }

        // Role is "button" for keyboard interaction (Enter/Space to open file picker).
        p.set(HtmlAttr::Role, "button");
        p.set(HtmlAttr::TabIndex, "0");

        // aria-label: use props.label if provided, otherwise fall back to Messages default.
        if !self.props.label.is_empty() {
            p.set(HtmlAttr::Aria(AriaAttr::Label), &self.props.label);
        } else {
            p.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.label)(&self.ctx.locale));
        }

        // Describe the current drop state for screen readers.
        // Note: aria-dropeffect is deprecated in WAI-ARIA 1.2; use
        // aria-description instead for drop state feedback.
        if matches!(self.state, State::DragOver) && self.ctx.valid_drag {
            p.set(HtmlAttr::Aria(AriaAttr::Description), (self.ctx.messages.drop_ready_description)(&self.ctx.locale));
            p.set_bool(HtmlAttr::Data("ars-drop-ready"), true);
        }

        if self.ctx.disabled {
            p.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            p.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.read_only {
            p.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if self.props.required {
            p.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.props.invalid {
            p.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        // aria-invalid when drop was rejected.
        if matches!(self.state, State::DropRejected) {
            p.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.ctx.focus_visible {
            p.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.ctx.valid_drag {
            p.set_bool(HtmlAttr::Data("ars-drag-valid"), true);
        }

        // Event handlers (drag/drop, focus, blur, keydown for file picker fallback)
        // are typed methods on the Api struct.

        p
    }

    /// Returns the current dropped items for form submission.
    /// When `Props::name` is set, the adapter uses this method in its submit handler
    /// to append files to `FormData`. Returns an empty slice when no items are present
    /// or when the component is disabled.
    pub fn form_data(&self) -> &[DragItem] {
        if self.ctx.disabled || self.props.name.is_none() {
            return &[];
        }
        &self.ctx.dropped_items
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
drop-zone
  Root    <div>    data-ars-scope="drop-zone" data-ars-part="root"
                   role="button" tabindex="0"
                   aria-description="..." (when drag-over)
```

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Attribute          | Element | Value                                                                   |
| ------------------ | ------- | ----------------------------------------------------------------------- |
| `role`             | `Root`  | `button` (for keyboard interaction)                                     |
| `aria-label`       | `Root`  | from `Props::label`, fallback to `Messages::label`                      |
| `aria-description` | `Root`  | from `Messages::drop_ready_description` when drag-over with valid types |
| `aria-disabled`    | `Root`  | `"true"` when disabled (string value, not boolean attribute)            |
| `aria-invalid`     | `Root`  | `"true"` when in `DropRejected` state or `Props::invalid` is true       |
| `aria-required`    | `Root`  | `"true"` when `Props::required` is true                                 |
| `tabindex`         | `Root`  | `0`                                                                     |

- The `role="button"` is used because the drop zone also acts as a keyboard-accessible control
  that opens a file picker when activated via Enter or Space.
- Screen readers announce the label and the current drop effect status.
- **Live region announcements:** When transitioning to `DropAccepted`, the adapter announces
  `Messages::drop_accepted_announcement` (default: "Files accepted") via a `LiveRegion`. When
  transitioning to `DropRejected`, the adapter announces `Messages::drop_rejected_announcement`
  (default: "Files rejected") via a `LiveRegion`. This ensures screen reader users receive
  immediate feedback about the drop result without needing to inspect the element.
- `aria-invalid="true"` is set on the root element when in the `DropRejected` state or when
  `Props::invalid` is true, signaling to assistive technology that validation failed.
- DropZone MUST meet the minimum 44x44 CSS pixel touch target size when used as a compact drop
  target (see foundation/03-accessibility.md section 7.1.1).

### 3.2 Keyboard Interaction

| Key     | Action                                                |
| ------- | ----------------------------------------------------- |
| `Enter` | Open file picker (fallback for non-drag interaction). |
| `Space` | Open file picker (fallback for non-drag interaction). |

For click-to-browse file selection (in addition to drag-and-drop), compose DropZone with a
`FileTrigger` child component (see `spec/components/input/file-upload.md`). The FileTrigger
provides a native file picker activated via keyboard or click.

## 4. Internationalization

### 4.1 Messages

```rust
/// Localizable strings for the `DropZone` component.
#[derive(Clone, Debug)]
pub struct Messages {
    /// Default accessible label for the drop zone.
    /// Used when `Props::label` is empty.
    /// Default: "Drop files here".
    pub label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Description announced when a valid drag is hovering over the zone.
    /// Default: "Release to drop files".
    pub drop_ready_description: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announcement when a drop is accepted.
    /// Default: "Files accepted".
    pub drop_accepted_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Announcement when a drop is rejected.
    /// Default: "Files rejected".
    pub drop_rejected_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            label: MessageFn::static_str("Drop files here"),
            drop_ready_description: MessageFn::static_str("Release to drop files"),
            drop_accepted_announcement: MessageFn::static_str("Files accepted"),
            drop_rejected_announcement: MessageFn::static_str("Files rejected"),
        }
    }
}

impl ComponentMessages for Messages {}
```

- The default label `"Drop files here"` should be overridden by the application's i18n system
  via the `label` prop or by providing a custom `Messages` struct through the locale context.
- Callback-provided strings (file names, error messages) are the responsibility of the consumer.
- Drop result announcements (`drop_accepted_announcement`, `drop_rejected_announcement`) are
  localizable via the `Messages` struct.

## 5. Form Integration

DropZone uses a custom `form_data()` pattern instead of `HiddenInputConfig` because browser
security prevents setting file input values programmatically. The adapter registers a submit
handler that reads `form_data()` from the DropZone API and appends the file data to the
`FormData` object. Form-level validation can check `api.form_data().is_empty()` for
required-file scenarios.

### 5.1 How It Works

When `Props::name` is set:

1. The `Api::form_data()` method returns the current `ctx.dropped_items` as `&[DragItem]`.
2. In the adapter's form submit handler, the adapter appends these items to `FormData` using
   the `FormData.append()` API, keyed by `Props::name`.
3. When the component is disabled, `form_data()` returns an empty slice, excluding it from
   form submission.

### 5.2 Adapter Submit Handler Pattern

```rust
// In the adapter's form submit handler:
if let Some(ref name) = props.name {
    let items = api.form_data();
    for item in items {
        match item {
            DragItem::File { handle, name: file_name, .. } => {
                // handle.to_file() returns a web_sys::File (or platform equivalent)
                form_data.append_with_blob_and_filename(name, &handle.to_file(), file_name);
            }
            DragItem::Text(text) => {
                form_data.append_with_str(name, text);
            }
            _ => { /* Other item types appended as appropriate */ }
        }
    }
}
```

### 5.3 Read-Only Behavior

When `read_only` is true, `form_data()` still returns the current items for inclusion in form submissions. The read-only state only prevents new file additions, not form data serialization.

### 5.4 Form Reset

When the parent form dispatches a reset event, the adapter MUST send `Event::Reset` to clear all
dropped items and return to Idle state. Unlike other form components, there is no "initial value"
to restore — reset always clears.

### 5.5 Platform Note

> **Platform Note:** The drag-and-drop API (`DataTransfer`, `dragenter`/`dragover`/`drop` events) is web-specific. Dioxus Desktop provides native OS drag-and-drop that does not use `DataTransfer`. The Dioxus adapter must abstract drag events: on web, map from DOM drag events; on Desktop, map from native file drop events. The `DragData` type should work with both sources.

## 6. Library Parity

> Compared against: React Aria (`DropZone`).

### 6.1 Props

| Feature             | ars-ui                        | React Aria         | Notes                                                         |
| ------------------- | ----------------------------- | ------------------ | ------------------------------------------------------------- |
| Disabled            | `disabled`                    | `isDisabled`       | Both libraries                                                |
| Get drop operation  | `get_drop_operation`          | `getDropOperation` | Both libraries                                                |
| Accept (MIME types) | `accept`                      | --                 | ars-ui adds type filtering; RA uses getDropOperation for this |
| Max files           | `max_files`                   | --                 | ars-ui addition for built-in validation                       |
| Max file size       | `max_file_size`               | --                 | ars-ui addition for built-in validation                       |
| Read-only           | `read_only`                   | --                 | ars-ui addition                                               |
| Label               | `label`                       | --                 | ars-ui addition for accessible name                           |
| Form integration    | `name`, `required`, `invalid` | --                 | ars-ui addition                                               |
| Activate delay      | `activate_delay_ms`           | --                 | ars-ui addition                                               |

**Gaps:** None. ars-ui is a superset.

### 6.2 Anatomy

| Part | ars-ui | React Aria | Notes          |
| ---- | ------ | ---------- | -------------- |
| Root | `Root` | `DropZone` | Both libraries |

**Gaps:** None.

### 6.3 Events

| Callback           | ars-ui               | React Aria                | Notes          |
| ------------------ | -------------------- | ------------------------- | -------------- |
| on_drop            | `on_drop`            | `onDrop`                  | Both libraries |
| on_drop_enter      | `on_drop_enter`      | `onDropEnter`             | Both libraries |
| on_drop_exit       | `on_drop_exit`       | `onDropExit`              | Both libraries |
| on_drop_move       | `on_drop_move`       | `onDropMove`              | Both libraries |
| on_drop_activate   | `on_drop_activate`   | `onDropActivate`          | Both libraries |
| on_hover_start/end | `on_hover_start/end` | `onHoverStart/End/Change` | Both libraries |

**Gaps:** None.

### 6.4 Features

| Feature                       | ars-ui                         | React Aria                       |
| ----------------------------- | ------------------------------ | -------------------------------- |
| Drop validation               | Yes (built-in type/size/count) | Yes (via getDropOperation)       |
| Drop accepted/rejected states | Yes                            | --                               |
| Focus tracking                | Yes                            | Yes (isFocusVisible render prop) |
| Form integration              | Yes                            | --                               |
| Activate delay                | Yes                            | --                               |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity -- ars-ui is a superset.
- **Divergences:** ars-ui provides built-in validation (accept, max_files, max_file_size) while React Aria delegates to `getDropOperation`. ars-ui adds explicit DropAccepted/DropRejected states.
- **Recommended additions:** None.
