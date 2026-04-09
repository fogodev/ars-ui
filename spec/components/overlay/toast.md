---
component: Toast
category: overlay
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: []
references:
    ark-ui: Toast
    radix-ui: Toast
    react-aria: Toast
---

# Toast

A notification system for transient, non-blocking messages with auto-dismiss, swipe-to-dismiss, and announcement coordination.

> **SSR Requirement**: The `toast::Region` container element with `aria-live="polite"`, `role="region"`, and `aria-label={messages.region_label}` MUST exist in the server-rendered HTML. Screen readers only track changes to live regions that were present when the page loaded. Creating the container via client-side JavaScript means toasts announced before hydration completes will be missed.

## 1. State Machine

The individual toast machine manages the lifecycle of a single toast notification.

### 1.1 States

```rust
/// The states of the toast.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// The toast is visible.
    Visible,
    /// The toast is paused.
    Paused,
    /// The toast is dismissing.
    Dismissing,
    /// Terminal state. The adapter/manager observes this and removes the toast
    /// from the visible list. No outgoing transitions.
    Dismissed,
}
```

### 1.2 Events

```rust
/// The events of the toast.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The toast is initialized.
    Init,
    /// The toast is dismissed.
    Dismiss,
    /// Pause countdown (on hover/focus via pointerenter/focusin)
    Pause,
    /// Resume countdown (on leave/blur via pointerleave/focusout)
    Resume,
    /// The toast swipe started.
    SwipeStart(f64),
    /// The toast swipe moved.
    SwipeMove(f64),
    /// The toast swipe ended.
    SwipeEnd {
        /// The velocity of the toast's swipe.
        velocity: f64,
        /// The offset of the toast's swipe.
        offset: f64,
    },
    /// The toast's duration has expired.
    DurationExpired,
    /// The toast's exit animation has completed.
    AnimationComplete,
}
```

### 1.3 Context

```rust
/// The context of the toast.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// The ID of the toast.
    pub id: String,
    /// The title of the toast.
    pub title: Option<String>,
    /// The description of the toast.
    pub description: Option<String>,
    /// The kind of the toast.
    pub kind: Kind,
    /// The duration of the toast (ms; None = indefinite).
    pub duration: Option<u32>,
    /// The remaining time of the toast.
    pub remaining_ms: Option<u64>,
    /// performance_now() timestamp when timer was (re)started.
    pub timer_started_at: Option<f64>,
    /// Whether the toast is paused.
    pub paused: bool,
    /// Whether the toast is being swiped.
    pub swiping: bool,
    /// The offset of the toast's swipe.
    pub swipe_offset: f64,
    /// The threshold for swipe-to-dismiss (default: 50px).
    pub swipe_threshold: f64,
    /// Whether the toast is open (for Presence composition).
    pub open: bool,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

/// The kinds of the toast.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Kind {
    #[default]
    Info,
    Success,
    Warning,
    Error,
    Loading,
}
```

### 1.4 Props

```rust
/// The props of the toast.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the toast.
    pub id: String,
    /// The title of the toast.
    pub title: Option<String>,
    /// The description of the toast.
    pub description: Option<String>,
    /// The kind of the toast.
    pub kind: Kind,
    /// The duration of the toast in ms. None = indefinite.
    pub duration: Option<u32>,
    /// Whether to show a progress bar.
    pub show_progress: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: None,
            description: None,
            kind: Kind::Info,
            duration: Some(5000),
            show_progress: false,
        }
    }
}
```

### 1.5 Pause-on-Hover and Pause-on-Focus

The toast machine supports automatic pause-on-hover and pause-on-focus behavior. When
the user hovers over a toast (`pointerenter`) or focuses into it (`focusin`), the auto-dismiss
timer pauses. When the pointer leaves (`pointerleave`) or focus moves out (`focusout`), the
timer resumes with the remaining duration.

**Timer Lifecycle**:

- On `PauseTimer`: snapshot `remaining_ms = duration - elapsed` and cancel the active timer.
- On `ResumeTimer`: restart the timer with `remaining_ms` as the new duration.

**Adapter Wiring**: The adapter attaches the following event listeners to each toast root element:

- `pointerenter` → `send(Event::Pause)`
- `pointerleave` → `send(Event::Resume)`
- `focusin` → `send(Event::Pause)`
- `focusout` → `send(Event::Resume)`

### 1.6 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, AttrMap};

/// The machine for the toast.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let locale = env.locale.clone();
        let messages = messages.clone();
        (State::Visible, Context {
            locale,
            id: props.id.clone(),
            title: props.title.clone(),
            description: props.description.clone(),
            kind: props.kind.clone(),
            duration: props.duration,
            remaining_ms: None,
            timer_started_at: None,
            paused: false,
            swiping: false,
            swipe_offset: 0.0,
            swipe_threshold: 50.0,
            open: true,
            messages,
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // Initial timer setup — adapter sends Init immediately after Service::new()
            (State::Visible, Event::Init) => {
                match ctx.duration {
                    Some(duration_ms) => Some(TransitionPlan::context_only(|ctx| {
                            ctx.timer_started_at = Some(performance_now());
                        })
                        .with_named_effect("duration-timer", move |_ctx, _props, send| {
                            let platform = use_platform_effects();
                            let handle = platform.set_timeout(duration_ms, Box::new(move || (send)(Event::DurationExpired)));
                            let pc = platform.clone();
                            Box::new(move || pc.clear_timeout(handle))
                        })),
                    None => None, // persistent toast, no auto-dismiss
                }
            }

            // Pause — changes state to trigger effect cleanup (cancels timer)
            (State::Visible, Event::Pause) => {
                Some(TransitionPlan::to(State::Paused)
                    .apply(|ctx| {
                        ctx.paused = true;
                        let elapsed = performance_now() - ctx.timer_started_at.unwrap_or(0.0);
                        let duration = ctx.duration.unwrap_or(0) as f64;
                        ctx.remaining_ms = Some((duration - elapsed).max(0.0) as u64);
                        ctx.timer_started_at = None;
                    }))
            }

            // Resume — restarts timer with remaining time
            (State::Paused, Event::Resume) => {
                Some(TransitionPlan::to(State::Visible)
                    .apply(|ctx| {
                        ctx.paused = false;
                        ctx.timer_started_at = Some(performance_now());
                    })
                    .with_named_effect("duration-timer", |ctx, _props, send| {
                        let platform = use_platform_effects();
                        let remaining = ctx.remaining_ms.or(ctx.duration.map(|d| d as u64));
                        let ms = remaining.unwrap_or(5000);
                        let handle = platform.set_timeout(ms, Box::new(move || (send)(Event::DurationExpired)));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    }))
            }

            // Auto-dismiss or manual dismiss → animate out
            (State::Visible, Event::DurationExpired | Event::Dismiss) |
            (State::Paused, Event::Dismiss) => {
                Some(TransitionPlan::to(State::Dismissing)
                    .apply(|ctx| { ctx.open = false; })
                    .with_named_effect("exit-animation", |_ctx, _props, send| {
                        let platform = use_platform_effects();
                        let handle = platform.set_timeout(5000, Box::new(move || {
                            send(Event::AnimationComplete);
                        }));
                        let pc = platform.clone();
                        Box::new(move || pc.clear_timeout(handle))
                    }))
            }

            // Animation complete → final state
            (State::Dismissing, Event::AnimationComplete) => {
                Some(TransitionPlan::to(State::Dismissed))
            }

            // Swipe gestures — also handled in Paused state
            (State::Visible | State::Paused, Event::SwipeStart(offset)) => {
                let offset = *offset;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.swiping = true;
                    ctx.swipe_offset = offset;
                }))
            }
            (State::Visible | State::Paused, Event::SwipeMove(offset)) => {
                let offset = *offset;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.swipe_offset = offset;
                }))
            }
            (State::Visible | State::Paused, Event::SwipeEnd { velocity, offset }) => {
                let velocity = *velocity;
                let offset = *offset;
                let threshold = ctx.swipe_threshold;
                if velocity.abs() > 0.5 || offset.abs() > threshold {
                    Some(TransitionPlan::to(State::Dismissing)
                        .apply(|ctx| {
                            ctx.open = false;
                            ctx.swiping = false;
                            ctx.swipe_offset = 0.0;
                        })
                        .with_named_effect("exit-animation", |_ctx, _props, send| {
                            let platform = use_platform_effects();
                            let handle = platform.set_timeout(5000, Box::new(move || {
                                send(Event::AnimationComplete);
                            }));
                            let pc = platform.clone();
                            Box::new(move || pc.clear_timeout(handle))
                        }))
                } else {
                    Some(TransitionPlan::context_only(move |ctx| {
                        ctx.swiping = false;
                        ctx.swipe_offset = 0.0;
                    }))
                }
            }

            _ => None,
        }
    }
}
```

> **Adapter obligation:** The adapter MUST send `Event::Init` immediately after `Service::new()` to set up the initial auto-dismiss timer. The `init()` function cannot return effects, so this bootstrapping event is required.
>
> **SSR timer safety.** Timer effects (auto-dismiss countdown, open/close delays for Tooltip and HoverCard) MUST only start after hydration completes. During SSR, `platform.set_timeout()` and `performance_now()` are unavailable. Adapters MUST guard timer setup with an `on_mount` lifecycle hook so that `Event::Init` is sent only on the client after the component has mounted.
>
> **`on_close_complete` callback:** Adapters should expose an `on_close_complete: Callback<String>` that fires when the Presence machine transitions to Unmounted after exit animation. This callback receives the toast ID and enables consumers to perform cleanup.

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "toast"]
pub enum Part {
    Root,
    Title,
    Description,
    /// The `alt_text` provides an accessible description of the action for screen readers.
    /// Because the action button label (e.g., "Undo") may not convey enough context on its own,
    /// `alt_text` describes the full effect (e.g., "Undo message deletion"). This mirrors
    /// Radix UI's `Toast.Action` `altText` requirement. The value is set as `aria-label`
    /// on the rendered `<button>`.
    ActionTrigger { alt_text: String },
    CloseTrigger,
    ProgressBar,
}

/// The API of the toast.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

/// Attributes for the toast region container. The adapter renders two regions:
/// one with `aria-live="polite"` (info/success/loading) and one with
/// `aria-live="assertive"` (error/warning). Both use `role="region"`.
pub fn region_attrs(messages: &Messages, locale: &Locale, assertive: bool) -> AttrMap {
    let mut attrs = AttrMap::new();
    attrs.set(HtmlAttr::Role, if assertive { "alert" } else { "status" });
    attrs.set(HtmlAttr::Aria(AriaAttr::Live), if assertive { "assertive" } else { "polite" });
    attrs.set(HtmlAttr::Aria(AriaAttr::Label), (messages.region_label)(locale));
    attrs
}

impl<'a> Api<'a> {
    pub fn is_visible(&self) -> bool { *self.state == State::Visible || *self.state == State::Paused }
    pub fn is_paused(&self) -> bool { *self.state == State::Paused }

    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        let state_str = match self.state {
            State::Visible => "visible",
            State::Paused => "paused",
            State::Dismissing => "dismissing",
            State::Dismissed => "dismissed",
        };
        attrs.set(HtmlAttr::Data("ars-state"), state_str);
        attrs.set(HtmlAttr::Data("ars-kind"), match self.ctx.kind {
            Kind::Info => "info",
            Kind::Success => "success",
            Kind::Warning => "warning",
            Kind::Error => "error",
            Kind::Loading => "loading",
        });
        if self.ctx.swiping {
            attrs.set_bool(HtmlAttr::Data("ars-swiping"), true);
        }
        attrs
    }

    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn action_trigger_attrs(&self, alt_text: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ActionTrigger { alt_text: String::new() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), alt_text);
        attrs
    }

    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.dismiss_label)(&self.ctx.locale));
        attrs
    }

    pub fn on_close_trigger_click(&self) { (self.send)(Event::Dismiss); }

    pub fn progress_bar_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ProgressBar.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Role, "progressbar");
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMin), "0");
        attrs.set(HtmlAttr::Aria(AriaAttr::ValueMax), "100");
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Title => self.title_attrs(),
            Part::Description => self.description_attrs(),
            Part::ActionTrigger { ref alt_text } => self.action_trigger_attrs(alt_text),
            Part::CloseTrigger => self.close_trigger_attrs(),
            Part::ProgressBar => self.progress_bar_attrs(),
        }
    }
}
```

## 2. Toast Manager

The `ToastManager` coordinates multiple toast instances, handling queuing, stacking, deduplication, and pause-on-hover for the toast region.

```rust
/// The context of the toast manager.
#[derive(Clone, Debug, PartialEq)]
pub struct ManagerContext {
    /// The toasts in the manager.
    pub toasts: Vec<toast::State>,
    /// Maximum number of simultaneously visible toasts. Default: 5.
    /// Excess toasts are queued and shown as visible toasts are dismissed.
    pub max_visible: usize,
    /// The placement of the toast manager.
    pub placement: Placement,
    /// The gap between toasts.
    pub gap: f64,
}

/// The placement of the toast manager.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Placement {
    TopStart, TopCenter, TopEnd,
    #[default]
    BottomEnd,
    BottomStart, BottomCenter,
    // Physical variants (non-RTL-aware)
    TopLeft, TopRight, BottomLeft, BottomRight,
}
```

```rust
/// Safe area insets from viewport edges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeOffsets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Default for EdgeOffsets {
    fn default() -> Self {
        Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 }
    }
}

/// Default auto-dismiss durations per toast kind.
#[derive(Clone, Debug, PartialEq)]
pub struct DefaultDurations {
    pub info: u32,
    pub success: u32,
    pub warning: u32,
    pub error: u32,
    pub loading: Option<u32>,
}

impl Default for DefaultDurations {
    fn default() -> Self {
        Self {
            info: 5000,
            success: 5000,
            warning: 5000,
            error: 8000,
            loading: None, // persistent by default
        }
    }
}
```

The `toast::Provider` (adapter-level wrapper) accepts `placement` as a prop and passes it to the `ToastManager`:

```rust
pub mod provider {
    #[derive(Clone, Debug, PartialEq)]
    pub struct Props {
        /// Where toasts appear on screen. Default: `BottomEnd`.
        pub placement: super::Placement,
        /// The maximum number of simultaneously visible toasts.
        pub max_visible: Option<usize>,
        /// The messages of the toast provider.
        pub messages: super::Messages,
        /// Enable deduplication globally. Default: false.
        pub deduplicate_all: bool,
        /// Whether hovering over the toast region pauses all auto-dismiss timers. Default: true.
        pub pause_on_hover: bool,
        /// Whether to pause all toast timers when the browser tab loses focus.
        /// Uses the Page Visibility API (`visibilitychange` event). Default: true.
        pub pause_on_page_idle: bool,
        /// Safe area insets from viewport edges in pixels (top, right, bottom, left).
        /// Prevents toasts from overlapping browser chrome or system UI.
        /// Default: `EdgeOffsets { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 }`.
        pub offsets: EdgeOffsets,
        /// Whether toasts visually overlap (stacked cards) instead of spreading vertically.
        /// When true, only the frontmost toast is fully visible; others peek from behind.
        /// Default: false.
        pub overlap: bool,
        /// Keyboard shortcut to move focus to the toast region (e.g., `"altKey+KeyT"`).
        /// Format: modifier keys joined with `+`, ending with a `KeyboardEvent.code` value.
        /// Default: `None` (no hotkey).
        pub hotkey: Option<String>,
        /// Delay in milliseconds before removing a dismissed toast from the DOM.
        /// Allows exit animations to complete. Default: 200ms.
        pub remove_delay: u32,
        /// Default auto-dismiss duration per toast kind (milliseconds).
        /// Overrides the individual toast's `duration` when not explicitly set.
        /// Example: errors display for 8000ms, info for 5000ms.
        pub default_durations: DefaultDurations,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum Event {
        Add(Config),
        Update(String, Config),
        Remove(String),
        PauseAll,
        ResumeAll,
        DismissAll,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub struct Config {
        pub title: Option<String>,
        pub description: Option<String>,
        pub kind: super::Kind,
        pub duration: Option<u32>,
        pub dismissible: bool,
        /// When true, a new toast with identical kind + title + description
        /// resets the existing toast's timer instead of creating a duplicate.
        pub deduplicate: bool,
        /// Callback invoked when the toast pause state changes.
        pub on_pause_change: Option<Callback<bool>>,
    }
}
```

### 2.1 Toast Queuing

When the visible toast count exceeds `max_visible`, new toasts are queued:

- `queued_toasts: Vec<toast::provider::Config>` is added to `ManagerContext`
- On `Add` when `toasts.len() >= max_visible`, push to `queued_toasts` instead of displaying
- On `Remove`, dequeue the oldest queued toast and display it
- Queued toasts do not start their auto-dismiss timer until they become visible

### 2.2 Stacking Order

Visible toasts are rendered in insertion order within the `toast::Region`. The most recent toast appears at the edge closest to the placement anchor (e.g., for `BottomEnd`, the newest toast is at the bottom). Each toast is offset by the `gap` value from its neighbor. The adapter applies CSS transforms or flexbox ordering to achieve the stacking layout.

### 2.3 Deduplication

When `Add(config)` is received and a visible or queued toast already has the same `kind` and identical `title` + `description`, the existing toast's timer is reset instead of creating a duplicate. This prevents notification spam when the same event fires repeatedly (e.g., network errors). Deduplication is opt-in via the `deduplicate` field on `Config`.

### 2.4 Pause-on-Hover

When the pointer enters the `toast::Region` container, ALL visible toasts pause their auto-dismiss timers (`Event::PauseAll`). When the pointer leaves, all timers resume (`Event::ResumeAll`). This ensures users have time to read or interact with toasts without them disappearing. The adapter attaches `pointerenter`/`pointerleave` listeners on the `toast::Region` element. Focus within a toast also pauses all timers (for keyboard and screen reader users), resuming on `focusout` when focus leaves the region entirely.

### 2.5 Toaster Imperative API

The `Toaster` provides an imperative handle for creating, updating, and dismissing toasts from anywhere in the application. The adapter exposes this handle via a context provider or a standalone function.

```rust
/// Imperative toast API. Obtained from the adapter's toast provider context.
pub struct Toaster {
    send: Box<dyn Fn(provider::Event)>,
}

impl Toaster {
    /// Create a toast with full configuration. Returns the toast ID.
    pub fn create(&self, config: provider::Config) -> String {
        let id = generate_unique_id();
        (self.send)(provider::Event::Add(provider::Config { id: Some(id.clone()), ..config }));
        id
    }

    /// Convenience: create an info toast.
    pub fn info(&self, title: &str, description: &str) -> String {
        self.create(provider::Config {
            title: Some(title.to_string()),
            description: Some(description.to_string()),
            kind: Kind::Info,
            ..Default::default()
        })
    }

    /// Convenience: create a success toast.
    pub fn success(&self, title: &str, description: &str) -> String {
        self.create(provider::Config {
            kind: Kind::Success,
            title: Some(title.to_string()),
            description: Some(description.to_string()),
            ..Default::default()
        })
    }

    /// Convenience: create a warning toast.
    pub fn warning(&self, title: &str, description: &str) -> String {
        self.create(provider::Config {
            kind: Kind::Warning,
            title: Some(title.to_string()),
            description: Some(description.to_string()),
            ..Default::default()
        })
    }

    /// Convenience: create an error toast.
    pub fn error(&self, title: &str, description: &str) -> String {
        self.create(provider::Config {
            kind: Kind::Error,
            title: Some(title.to_string()),
            description: Some(description.to_string()),
            ..Default::default()
        })
    }

    /// Update an existing toast's content and/or kind.
    pub fn update(&self, id: &str, config: provider::Config) {
        (self.send)(provider::Event::Update(id.to_string(), config));
    }

    /// Dismiss a specific toast by ID.
    pub fn dismiss(&self, id: &str) {
        (self.send)(provider::Event::Remove(id.to_string()));
    }

    /// Dismiss all visible toasts.
    pub fn dismiss_all(&self) {
        (self.send)(provider::Event::DismissAll);
    }

    /// Track an async operation with loading → success/error states.
    ///
    /// Creates a loading toast immediately, then transitions to success or error
    /// based on the future's result.
    pub fn promise<T, E>(
        &self,
        options: PromiseToastOptions<T, E>,
    ) -> String
    where
        T: 'static,
        E: 'static,
    {
        let id = self.create(provider::Config {
            title: options.loading.title,
            description: options.loading.description,
            kind: Kind::Loading,
            duration: None, // persistent until resolved
            ..Default::default()
        });
        // The adapter spawns the future and calls update() on resolution.
        // See adapter obligation below.
        id
    }
}

/// Options for a promise-backed toast.
#[derive(Clone, Debug)]
pub struct PromiseToastOptions<T, E> {
    /// Configuration shown while the future is pending.
    pub loading: ToastContent,
    /// Configuration shown when the future resolves successfully.
    /// Receives the success value for dynamic message formatting.
    pub success: Box<dyn Fn(T) -> ToastContent>,
    /// Configuration shown when the future rejects.
    /// Receives the error value for dynamic message formatting.
    pub error: Box<dyn Fn(E) -> ToastContent>,
}

/// Content for a toast message (used in promise options).
#[derive(Clone, Debug, Default)]
pub struct ToastContent {
    pub title: Option<String>,
    pub description: Option<String>,
}
```

**Adapter obligation for promise toasts:** The adapter MUST:

1. Spawn the user-provided future on the async runtime (Leptos: `spawn_local`, Dioxus: `spawn`)
2. On success: call `toaster.update(id, success_config)` with `Kind::Success` and reset duration to the default
3. On error: call `toaster.update(id, error_config)` with `Kind::Error` and reset duration to the error default
4. The loading toast remains visible and persistent until the future resolves

## 3. Anatomy

```text
toast::Region  (viewport — aria-live region)
└── Toast (per notification)
    ├── Root             (required)
    ├── Title            (optional)
    ├── Description      (optional)
    ├── ProgressBar      (optional — when show_progress=true)
    ├── ActionTrigger    (optional — CTA button, aria-label from alt_text)
    └── CloseTrigger     (optional)
```

| Part          | Element    | Key Attributes                                                                           |
| ------------- | ---------- | ---------------------------------------------------------------------------------------- |
| Region        | `<div>`    | `aria-live="polite"` or `"assertive"`, `role="region"`, `aria-label`                     |
| Root          | `<div>`    | `data-ars-scope="toast"`, `data-ars-state`, `data-ars-kind`                              |
| Title         | `<div>`    | `data-ars-scope="toast"`, `data-ars-part="title"`                                        |
| Description   | `<div>`    | `data-ars-scope="toast"`, `data-ars-part="description"`                                  |
| ProgressBar   | `<div>`    | `role="progressbar"`, `aria-valuenow`, `aria-valuemin="0"`, `aria-valuemax="100"`        |
| ActionTrigger | `<button>` | `data-ars-scope="toast"`, `data-ars-part="action-trigger"`, `aria-label` from `alt_text` |
| CloseTrigger  | `<button>` | `aria-label` from Messages                                                               |

## 4. Accessibility

The Toast system renders **two** `toast::Region` containers in server HTML:

1. `<div aria-live="polite" role="status" aria-label={messages.region_label}>` — for info, success, and loading toasts
2. `<div aria-live="assertive" role="alert" aria-label={messages.region_label}>` — for error and warning toasts

Both regions MUST have `aria-label` to be exposed as navigable landmark regions (ARIA 1.2 §5.3.7).

The `ToastManager` routes each toast to the appropriate region based on `Kind`:

- `Kind::Info | Kind::Success | Kind::Loading` → polite region
- `Kind::Error | Kind::Warning` → assertive region

### 4.1 ARIA Roles, States, and Properties

| Part          | Property      | Value                                        |
| ------------- | ------------- | -------------------------------------------- |
| Region        | `aria-live`   | `"polite"` or `"assertive"` by Kind          |
| Region        | `role`        | `"status"` (polite) or `"alert"` (assertive) |
| Region        | `aria-label`  | From Messages (landmark identification)      |
| Region        | `aria-atomic` | `"false"` — announce individual toasts       |
| ActionTrigger | `aria-label`  | From `alt_text` (consumer-provided)          |
| CloseTrigger  | `aria-label`  | From Messages (`dismiss_label`)              |

- Both regions MUST be present in the server-rendered HTML
- Auto-dismiss **pauses** when hover or keyboard focus is within the region
- Minimum display duration of **5 seconds** per WCAG 2.2.1 (Timing Adjustable)

> **Toast live region placement.** The `aria-live` region(s) MUST be rendered at the application root level and persist for the lifetime of the app. Never place them inside components that may unmount.

### 4.2 Announcement Coordination

When multiple toasts are visible simultaneously, the `ToastManager` coordinates their screen reader announcements.

#### 4.2.1 Ordering

Announcements follow **priority-first, then FIFO** ordering:

1. **Error/Warning toasts** (assertive region) are announced before **Info/Success/Loading toasts** (polite region) because the browser's native `aria-live="assertive"` behavior interrupts polite announcements.
2. Within the same priority level, toasts are announced in **creation order** (FIFO).

#### 4.2.2 Announcement Timing

When multiple toasts arrive in rapid succession, each toast is announced individually with a minimum **500ms gap** between insertions into the live region:

1. Toast A arrives → content inserted into the appropriate live region immediately.
2. Toast B arrives 100ms later → content insertion is **delayed** until 500ms after Toast A's insertion.
3. Toast C arrives 200ms later → queued behind Toast B, inserted 500ms after Toast B.

**Implementation:** The `ToastManager` maintains an `announcement_queue: VecDeque<(String, AnnouncePriority)>` and a `last_announcement_at: Option<Instant>`. A timer drains the queue at 500ms intervals, inserting each message via the two-step pattern (clear → wait 100ms → insert) defined in `LiveRegion`.

#### 4.2.3 No Batching

Toasts are **never batched** into summary announcements. Each toast is announced individually with its full title and description.

> **Edge case:** If the announcement queue exceeds 10 pending items, the `ToastManager` SHOULD log a development-mode warning suggesting the application reduce toast frequency. The queue is not capped — all toasts are eventually announced.

## 5. Internationalization

### 5.1 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Accessible label for the toast region landmark (default: "Notifications").
    pub region_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    /// Dismiss label (default: "Dismiss notification").
    pub dismiss_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}
// Pause/resume is handled implicitly via hover/focus — no explicit buttons.

impl Default for Messages {
    fn default() -> Self {
        Self {
            region_label: MessageFn::static_str("Notifications"),
            dismiss_label: MessageFn::static_str("Dismiss notification"),
        }
    }
}

impl ComponentMessages for Messages {}
```

- `Placement::TopStart/TopEnd/BottomStart/BottomEnd` resolve correctly in RTL.

## 6. Variant: Progress Indicator

Toasts with a `duration` can display a visual progress bar showing time remaining before auto-dismiss.

### 6.1 Additional Props

```rust
/// Added to toast::Props.
/// When true, a progress bar is rendered inside the toast showing elapsed/remaining time.
pub show_progress: bool,
```

### 6.2 Behavior

- Progress advances from 0 → 1 over the toast's `duration`.
- When `pause_on_hover` is true, progress animation pauses while the pointer is over the toast.
- When `pause_on_focus_loss` is true, progress pauses when the document loses focus.
- On resume, progress continues from where it paused (not reset).

### 6.3 CSS Custom Property

The progress value is exposed as a CSS custom property for styling:

```css
[data-ars-part="progress-bar"] {
    --ars-toast-progress: 0; /* 0.0 to 1.0, updated by the adapter */
}
```

### 6.4 Accessibility

- `role="progressbar"` with `aria-valuenow` (0-100) for screen readers.
- The progress bar is a visual enhancement — the auto-dismiss timer functions identically with or without it.
- Screen readers do NOT need to announce progress updates (no `aria-live` on the progress bar itself).

## 7. Adapter-Level Imperative API

Framework adapters must expose an imperative API for creating toasts programmatically.

### 7.1 Leptos Adapter

```rust
/// Create a toast queue that can be used to imperatively add/remove toasts.
pub fn create_toast_queue() -> toast::Queue { /* ... */ }

#[derive(Clone)]
pub struct Queue { /* ... */ }

impl Queue {
    /// Add a new toast. Returns the toast's unique ID for later removal.
    pub fn add(&self, config: toast::provider::Config) -> toast::Id { /* ... */ }
    /// Remove a specific toast by ID.
    pub fn remove(&self, id: toast::Id) { /* ... */ }
    /// Remove all toasts.
    pub fn clear(&self) { /* ... */ }
}
```

### 7.2 Dioxus Adapter

```rust
/// Hook to create a toast queue in Dioxus.
pub fn use_toast_queue() -> toast::Queue { /* ... */ }
```

Both adapters internally wire the `Queue` to the `ToastManager` state machine, translating `add()` calls into `toast::provider::Event::Add(Config)` events.

### 7.3 Swipe Direction Configuration

The toast swipe-to-dismiss direction is determined by the `placement` of the toast region:

- **Left/right placements** (e.g., `BottomEnd`, `TopStart`): swipe horizontally to dismiss.
- **Center placements** (e.g., `BottomCenter`, `TopCenter`): swipe vertically (down for bottom, up for top).
- The swipe threshold is `50px` by default (configurable via `Context::swipe_threshold`).

The adapter MUST:

1. Detect `pointerdown` → track pointer movement along the swipe axis.
2. Apply CSS transform to visually track the swipe (`--ars-toast-swipe-offset`).
3. On release: if offset exceeds threshold or velocity exceeds `0.5`, dismiss; otherwise animate back.

### 7.4 Promise Toast API

```rust
/// A toast that tracks an async operation through loading -> success/error states.
pub struct Promise<T, E> {
    pub loading: toast::Data,
    pub success: Callback<dyn Fn(T) -> toast::Data>,
    pub error: Callback<dyn Fn(E) -> toast::Data>,
}

impl Queue {
    /// Creates a promise toast that automatically updates based on the async result.
    pub fn create_promise<T, E, F: Future<Output = Result<T, E>>>(
        &self,
        promise: F,
        config: Promise<T, E>,
    ) -> toast::Id {
        let id = self.add(config.loading);
        // Adapter spawns the future; on completion, calls update(id, success/error data)
        id
    }
}
```

The adapter spawns the future on the framework's async runtime. When the future resolves, the adapter calls `update(id, new_data)` to replace the loading toast's content with the success or error toast data. If the toast was dismissed before the future completes, the result is silently discarded.

## 8. Library Parity

> Compared against: Ark UI (`Toast`/`Toaster`), Radix UI (`Toast`), React Aria (`Toast`/`ToastQueue`).

### 8.1 Props

| Feature               | ars-ui                          | Ark UI            | Radix UI                    | React Aria | Notes                                                             |
| --------------------- | ------------------------------- | ----------------- | --------------------------- | ---------- | ----------------------------------------------------------------- |
| Auto-dismiss duration | `duration`                      | `duration`        | `duration`                  | `timeout`  | All libraries                                                     |
| Toast kind/type       | `kind`                          | (create method)   | `type`                      | --         | Ark UI uses method variants; Radix has foreground/background type |
| Placement             | `placement` (Provider)          | `placement`       | --                          | --         | Ark UI parity; Radix uses manual viewport positioning             |
| Max visible           | `max_visible` (Provider)        | `max`             | --                          | --         | Ark UI parity                                                     |
| Gap                   | `gap` (Provider)                | `gap`             | --                          | --         | Ark UI parity                                                     |
| Offsets               | `offsets` (Provider)            | `offsets`         | --                          | --         | Ark UI parity                                                     |
| Overlap mode          | `overlap` (Provider)            | `overlap`         | --                          | --         | Ark UI parity                                                     |
| Pause on hover        | `pause_on_hover` (Provider)     | (implicit)        | (onPause/onResume)          | --         | All libraries support; different APIs                             |
| Pause on page idle    | `pause_on_page_idle` (Provider) | `pauseOnPageIdle` | --                          | --         | Ark UI parity                                                     |
| Hotkey                | `hotkey` (Provider)             | `hotkey`          | `hotkey` (Viewport)         | --         | Ark UI/Radix                                                      |
| Remove delay          | `remove_delay` (Provider)       | `removeDelay`     | --                          | --         | Ark UI parity                                                     |
| Default durations     | `default_durations` (Provider)  | --                | --                          | --         | ars-ui addition for per-kind defaults                             |
| Swipe direction       | (implicit from placement)       | --                | `swipeDirection`            | --         | Radix explicit; ars-ui derives from placement                     |
| Swipe threshold       | `swipe_threshold` (Context)     | --                | `swipeThreshold`            | --         | Radix parity                                                      |
| Region label          | `messages.region_label`         | --                | `label` (Provider/Viewport) | --         | Radix parity                                                      |
| Deduplicate           | `deduplicate` (Config)          | --                | --                          | --         | ars-ui addition                                                   |
| Alt text (Action)     | `alt_text` (ActionTrigger)      | --                | `altText` (Action)          | --         | Radix parity                                                      |
| Show progress         | `show_progress`                 | --                | --                          | --         | ars-ui addition                                                   |
| Open change callback  | --                              | --                | `onOpenChange`              | --         | Radix per-toast callback                                          |

**Gaps:** None.

### 8.2 Anatomy

| Part          | ars-ui        | Ark UI              | Radix UI    | React Aria                | Notes                    |
| ------------- | ------------- | ------------------- | ----------- | ------------------------- | ------------------------ |
| Region        | Region        | Toaster             | Viewport    | ToastRegion               | Container for all toasts |
| Root          | Root          | Toast.Root          | Root        | Toast                     | Individual toast         |
| Title         | Title         | Toast.Title         | Title       | Text (slot="title")       | Toast heading            |
| Description   | Description   | Toast.Description   | Description | Text (slot="description") | Toast body               |
| ActionTrigger | ActionTrigger | Toast.ActionTrigger | Action      | --                        | CTA button               |
| CloseTrigger  | CloseTrigger  | Toast.CloseTrigger  | Close       | Button (slot="close")     | Dismiss button           |
| ProgressBar   | ProgressBar   | --                  | --          | --                        | ars-ui addition          |
| Provider      | Provider      | --                  | Provider    | --                        | Global configuration     |

**Gaps:** None.

### 8.3 Events

| Callback         | ars-ui                 | Ark UI     | Radix UI          | React Aria | Notes                       |
| ---------------- | ---------------------- | ---------- | ----------------- | ---------- | --------------------------- |
| Pause            | Event::Pause           | (implicit) | `onPause`         | --         | Timer pause                 |
| Resume           | Event::Resume          | (implicit) | `onResume`        | --         | Timer resume                |
| Swipe start      | Event::SwipeStart      | --         | `onSwipeStart`    | --         | Radix parity                |
| Swipe move       | Event::SwipeMove       | --         | `onSwipeMove`     | --         | Radix parity                |
| Swipe end        | Event::SwipeEnd        | --         | `onSwipeEnd`      | --         | Radix parity                |
| Escape key       | (Event::Dismiss)       | --         | `onEscapeKeyDown` | --         | Radix has separate callback |
| Duration expired | Event::DurationExpired | --         | --                | --         | Internal event              |
| Open change      | (via Manager)          | --         | `onOpenChange`    | --         | Radix per-toast             |
| On close         | (via Toaster)          | --         | --                | `onClose`  | React Aria per-toast        |

**Gaps:** None.

### 8.4 Features

| Feature                   | ars-ui                                      | Ark UI        | Radix UI                          | React Aria |
| ------------------------- | ------------------------------------------- | ------------- | --------------------------------- | ---------- |
| Auto-dismiss timer        | Yes                                         | Yes           | Yes                               | Yes        |
| Pause on hover/focus      | Yes                                         | Yes           | Yes                               | --         |
| Swipe to dismiss          | Yes                                         | --            | Yes                               | --         |
| Toast kinds               | Yes (5: info/success/warning/error/loading) | Yes (methods) | Yes (type: foreground/background) | --         |
| Stacking/queuing          | Yes                                         | Yes           | Yes (manual)                      | Yes        |
| Promise toast             | Yes                                         | Yes           | --                                | --         |
| Update existing           | Yes                                         | Yes           | --                                | --         |
| Deduplication             | Yes                                         | --            | --                                | --         |
| Progress bar              | Yes                                         | --            | --                                | --         |
| Dual live regions         | Yes (polite + assertive)                    | --            | --                                | --         |
| Announcement coordination | Yes                                         | --            | --                                | --         |
| Pause on page idle        | Yes                                         | Yes           | --                                | --         |
| Keyboard hotkey           | Yes                                         | Yes           | Yes                               | --         |
| SSR region requirement    | Yes                                         | --            | --                                | --         |

**Gaps:** None.

### 8.5 Summary

- **Overall:** Full parity.
- **Divergences:** (1) ars-ui uses a state machine per toast + a `ToastManager` for coordination, while Ark UI uses `createToaster()` and Radix uses a Provider/Viewport pattern. (2) ars-ui renders dual live regions (polite/assertive) based on toast kind for correct screen reader urgency, whereas other libraries use a single region. (3) Swipe direction is automatically derived from placement rather than configured separately. (4) Toast kinds include `Loading` for promise-toast patterns.
- **Recommended additions:** None.
