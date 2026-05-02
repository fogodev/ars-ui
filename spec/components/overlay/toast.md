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
    /// The toast is dismissed.
    Dismiss,
    /// Pause countdown (on hover/focus via pointerenter/focusin).
    /// Carries the remaining auto-dismiss time read from the adapter's
    /// clock so the snapshot is recorded atomically with the state flip,
    /// keeping the agnostic core free of `performance.now()` access.
    Pause { remaining: Duration },
    /// Resume countdown (on leave/blur via pointerleave/focusout).
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

/// Typed identifier for every named effect intent the toast machine emits.
///
/// Adapters dispatch on `effect.name` exhaustively, so name typos and
/// unhandled variants surface at compile time — the same convention used
/// by [`dialog::Effect`](crate::components::overlay::dialog), `popover`,
/// and `tooltip`. The variant names themselves are the contract; there is
/// no parallel kebab-case wire form to keep in sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts (or restarts) the auto-dismiss countdown using
    /// `Context::remaining.unwrap_or(Context::duration)` and dispatches
    /// `Event::DurationExpired` when the timer fires.
    DurationTimer,
    /// Adapter waits for the toast's exit animation to complete (or for
    /// the configured remove delay when animations are skipped) and
    /// dispatches `Event::AnimationComplete`.
    ExitAnimation,
    /// Adapter inserts the toast's title/description into the polite
    /// `aria-live` region. Emitted on initial mount for `Kind::Info`,
    /// `Kind::Success`, and `Kind::Loading` toasts.
    AnnouncePolite,
    /// Adapter inserts the toast's title/description into the assertive
    /// `aria-live` region. Emitted on initial mount for `Kind::Warning`
    /// and `Kind::Error` toasts.
    AnnounceAssertive,
    /// Adapter invokes consumer-supplied open-change callbacks (e.g.,
    /// `Provider::on_open_change`) with the post-transition open state.
    OpenChange,
}
```

### 1.3 Context

```rust
/// The context of the toast.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// Hydration-stable component IDs derived from `Props::id` at init.
    /// Adapters read sub-part IDs through `ids.part("title")` etc., keeping
    /// ARIA wiring in sync with the rendered element IDs.
    pub ids: ComponentIds,
    /// The title of the toast.
    pub title: Option<String>,
    /// The description of the toast.
    pub description: Option<String>,
    /// The kind of the toast.
    pub kind: Kind,
    /// The duration of the toast. None = indefinite.
    pub duration: Option<Duration>,
    /// The remaining time of the toast.
    ///
    /// The agnostic core never reads `performance.now()` itself: when the
    /// toast pauses, the adapter computes `duration - elapsed` from its own
    /// clock and the snapshot is recorded atomically through
    /// `Event::Pause { remaining }`. On resume, the adapter restarts its
    /// timer using `remaining.unwrap_or(duration)`.
    pub remaining: Option<Duration>,
    /// Whether the toast is paused.
    pub paused: bool,
    /// Whether the toast is being swiped.
    pub swiping: bool,
    /// The offset of the toast's swipe.
    pub swipe_offset: f64,
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

`Kind` exposes a `kind.announce_priority() -> AnnouncePriority` helper that
returns `Assertive` for `Warning`/`Error` and `Polite` for the rest, so
adapters can route directly to the matching live region without a manual
mapping. `swipe_threshold` lives on `Props` (see §1.4) — it is per-toast
configuration that never changes after init, so it does not belong on
`Context`.

### 1.4 Props

```rust
/// Default swipe-to-dismiss threshold in pixels.
pub const DEFAULT_SWIPE_THRESHOLD: f64 = 50.0;

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
    /// The duration of the toast. None = indefinite.
    pub duration: Option<Duration>,
    /// Whether to show a progress bar.
    pub show_progress: bool,
    /// Distance threshold (px) past which `SwipeEnd` dismisses the toast.
    /// Velocity above 0.5 also dismisses regardless of distance.
    /// Defaults to `DEFAULT_SWIPE_THRESHOLD` (50 px).
    pub swipe_threshold: f64,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: None,
            description: None,
            kind: Kind::Info,
            duration: Some(Duration::from_secs(5)),
            show_progress: false,
            swipe_threshold: DEFAULT_SWIPE_THRESHOLD,
        }
    }
}
```

A fluent builder is provided per the workspace component-spec template
(see `spec/foundation/10-component-spec-template.md`); each public field
has a matching `Props::field(value)` setter.

### 1.5 Pause-on-Hover and Pause-on-Focus

The toast machine supports automatic pause-on-hover and pause-on-focus behavior. When
the user hovers over a toast (`pointerenter`) or focuses into it (`focusin`), the auto-dismiss
timer pauses. When the pointer leaves (`pointerleave`) or focus moves out (`focusout`), the
timer resumes with the remaining duration.

**Timer Lifecycle**:

- On `Event::Pause { remaining }`: the adapter snapshots its clock
  (`duration - elapsed`, clamped to zero) and bundles the value on the
  event itself. The machine writes the value into `Context::remaining`
  and cancels `Effect::DurationTimer` atomically — there is no observable
  `paused == true && remaining == None` intermediate state.
- On `Event::Resume`: machine emits `Effect::DurationTimer` again. The adapter restarts
  its countdown using `Context::remaining.unwrap_or(Context::duration)` so the toast
  finishes the time it had left.

The agnostic core never reads `performance.now()` itself — the adapter owns the clock and
hands the snapshot back through `Event::Pause`.

**Adapter Wiring**: The adapter attaches the following event listeners to each toast root element:

- `pointerenter` → `send(Event::Pause { remaining: snapshot })`
- `pointerleave` → `send(Event::Resume)`
- `focusin` → `send(Event::Pause { remaining: snapshot })`
- `focusout` → `send(Event::Resume)`

### 1.6 Full Machine Implementation

```rust
use ars_core::{Env, PendingEffect, TransitionPlan};

/// The machine for the toast.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            State::Visible,
            Context {
                locale: env.locale.clone(),
                ids: ComponentIds::from_id(&props.id),
                title: props.title.clone(),
                description: props.description.clone(),
                kind: props.kind,
                duration: props.duration,
                remaining: None,
                paused: false,
                swiping: false,
                swipe_offset: 0.0,
                open: true,
                messages: messages.clone(),
            },
        )
    }

    fn initial_effects(
        _state: &Self::State,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        let mut effects = Vec::new();
        effects.push(PendingEffect::named(announce_intent(ctx.kind)));
        if ctx.duration.is_some() {
            effects.push(PendingEffect::named(Effect::DurationTimer));
        }
        effects
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // Pause — atomically record the remaining-ms snapshot and
            // cancel the running timer.
            (State::Visible, Event::Pause { remaining }) => {
                let remaining = *remaining;
                Some(
                    TransitionPlan::to(State::Paused)
                        .apply(move |ctx: &mut Context| {
                            ctx.paused = true;
                            ctx.remaining = Some(remaining);
                        })
                        .cancel_effect(Effect::DurationTimer),
                )
            }

            // Resume — restart the timer with `remaining`.
            (State::Paused, Event::Resume) => Some(
                TransitionPlan::to(State::Visible)
                    .apply(|ctx: &mut Context| {
                        ctx.paused = false;
                    })
                    .with_effect(PendingEffect::named(Effect::DurationTimer)),
            ),

            // Auto-dismiss or manual dismiss → animate out.
            (State::Visible, Event::DurationExpired | Event::Dismiss)
            | (State::Paused, Event::Dismiss) => Some(dismiss_plan()),

            // Animation complete → final state.
            (State::Dismissing, Event::AnimationComplete) => {
                Some(TransitionPlan::to(State::Dismissed))
            }

            // Swipe gestures — also handled in Paused state.
            (State::Visible | State::Paused, Event::SwipeStart(offset)) => {
                let offset = *offset;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.swiping = true;
                    ctx.swipe_offset = offset;
                }))
            }
            (State::Visible | State::Paused, Event::SwipeMove(offset)) => {
                let offset = *offset;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.swipe_offset = offset;
                }))
            }
            (State::Visible | State::Paused, Event::SwipeEnd { velocity, offset }) => {
                if velocity.abs() > 0.5 || offset.abs() > props.swipe_threshold {
                    Some(
                        dismiss_plan().apply(|ctx: &mut Context| {
                            ctx.swiping = false;
                            ctx.swipe_offset = 0.0;
                        }),
                    )
                } else {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.swiping = false;
                        ctx.swipe_offset = 0.0;
                    }))
                }
            }

            _ => None,
        }
    }
}

fn dismiss_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Dismissing)
        .apply(|ctx: &mut Context| {
            ctx.open = false;
        })
        .cancel_effect(Effect::DurationTimer)
        .with_effect(PendingEffect::named(Effect::ExitAnimation))
        .with_effect(PendingEffect::named(Effect::OpenChange))
}

const fn announce_intent(kind: Kind) -> Effect {
    match kind {
        Kind::Warning | Kind::Error => Effect::AnnounceAssertive,
        Kind::Info | Kind::Success | Kind::Loading => Effect::AnnouncePolite,
    }
}
```

> **Adapter obligation:** The adapter consumes `Service::take_initial_effects()` exactly once after first mount and dispatches each [`Effect`] variant. There is no `Event::Init` ping — initial effects are scheduled by `Machine::initial_effects` instead.
>
> **SSR timer safety.** Timer effects (auto-dismiss countdown, open/close delays for Tooltip and HoverCard) MUST only start after hydration completes. During SSR, `platform.set_timeout()` and `platform.now_ms()` are unavailable. Adapters MUST guard timer setup with an `on_mount` lifecycle hook so the initial-effects buffer is drained only on the client after the component has mounted.
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

impl<'a> Api<'a> {
    pub fn is_visible(&self) -> bool { matches!(self.state, State::Visible | State::Paused) }
    pub fn is_paused(&self) -> bool { matches!(self.state, State::Paused) }
    pub fn is_dismissed(&self) -> bool { matches!(self.state, State::Dismissing | State::Dismissed) }
    pub fn kind(&self) -> Kind { self.ctx.kind }
    pub fn swipe_threshold(&self) -> f64 { self.props.swipe_threshold }

    /// Stamps `id`, `data-ars-scope="toast"`, `data-ars-part="root"`,
    /// `data-ars-state`, `data-ars-kind`, plus `aria-labelledby` /
    /// `aria-describedby` referencing the title / description ids when
    /// they are populated, plus `data-ars-swiping` while a swipe gesture
    /// is in flight.
    ///
    /// The toast root MUST NOT carry its own `role="status"`/`role="alert"`
    /// — that role lives on the surrounding live-region shell (see
    /// §4.1) and stamping it here would duplicate the announcement.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(HtmlAttr::Id, self.ctx.ids.id().to_string());
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), state_token(*self.state));
        attrs.set(HtmlAttr::Data("ars-kind"), self.ctx.kind.as_str());

        if self.ctx.title.is_some() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("title"),
            );
        }
        if self.ctx.description.is_some() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                self.ctx.ids.part("description"),
            );
        }
        if self.ctx.swiping {
            attrs.set_bool(HtmlAttr::Data("ars-swiping"), true);
        }
        attrs
    }

    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("title"));
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    pub fn action_trigger_attrs(&self, alt_text: impl Into<AttrValue>) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::ActionTrigger { alt_text: String::new() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), alt_text);
        attrs
    }

    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.dismiss_label)(&self.ctx.locale));
        attrs
    }

    pub fn on_close_trigger_click(&self) { (self.send)(Event::Dismiss); }
    pub fn on_pointer_enter(&self, remaining: Duration) {
        (self.send)(Event::Pause { remaining });
    }
    pub fn on_pointer_leave(&self) { (self.send)(Event::Resume); }

    /// The progress bar is presentational. `aria-valuenow` is intentionally
    /// **not** emitted because per-frame ARIA updates would defeat the
    /// "screen readers do NOT announce progress" goal in §6.4. Adapters
    /// drive the visual progress through the `--ars-toast-progress` CSS
    /// custom property.
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

The `aria-live` region helper does **not** live on per-toast `Api` —
both the SSR-rendered shells and the manager's `Api::region_attrs(assertive)`
call into a single canonical `manager::region_attrs(messages, locale,
RegionPart)` (see §2.8). The shells stamp `data-ars-scope="toast"` so a
`[data-ars-scope="toast"][data-ars-part="region"]` selector matches both
the polite and assertive containers.

## 2. Toast Manager

The `ToastManager` coordinates multiple toast instances, handling queuing, stacking, deduplication, pause-on-hover, and announcement-coordination for the toast region.

```rust
/// Lifecycle stage of a tracked toast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryStage {
    /// Counts against `max_visible` — currently rendered.
    Visible,
    /// The per-toast machine reached `Dismissing`. The manager retains
    /// the entry for `remove_delay` ms so the exit animation can finish
    /// before the row is removed via `Event::HideQueueAdvance`.
    Dismissing,
}

/// One toast tracked by the manager.
#[derive(Clone, Debug, PartialEq)]
pub struct ToastEntry {
    pub id: String,
    pub config: Config,
    pub stage: EntryStage,
}

/// Live-region urgency for a queued announcement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnnouncePriority { Polite, Assertive }

/// The context of the toast manager.
#[derive(Clone, Debug, PartialEq)]
pub struct ManagerContext {
    /// Toasts currently tracked (visible + dismissing).
    pub toasts: Vec<ToastEntry>,
    /// Configs awaiting admission because the visible count is at
    /// `max_visible`. Promoted in FIFO order on `Remove` /
    /// `HideQueueAdvance`.
    pub queued: VecDeque<Config>,
    /// Pending announcements (toast id + priority). Drained by
    /// `Event::DrainAnnouncement` in priority + FIFO order.
    pub announcement_queue: VecDeque<(String, AnnouncePriority)>,
    /// Adapter clock timestamp (ms) of the most recent announcement drain.
    /// Updated through `Event::DrainAnnouncement` so the next drain can
    /// enforce the §4.2 500 ms gap.
    pub last_announcement_at: Option<u64>,
    /// Maximum number of simultaneously visible toasts. Default: 5.
    pub max_visible: usize,
    /// Where toasts appear on screen.
    pub placement: Placement,
    /// Pixel gap between visible toasts.
    pub gap: f64,
    /// Delay between per-toast `Dismissing` and full removal.
    pub remove_delay: Duration,
    /// Default auto-dismiss durations per kind.
    pub default_durations: DefaultDurations,
    /// When `true`, every `Add` defaults to deduplicate.
    pub deduplicate_all: bool,
    /// Safe-area insets passed through to adapters.
    pub offsets: EdgeOffsets,
    /// Whether toasts visually overlap (stacked-card mode).
    pub overlap: bool,
    /// Whether all timers are currently paused (mirrors `State::Paused`).
    pub paused_all: bool,
    /// Resolved locale.
    pub locale: Locale,
    /// Resolved manager-level messages (region label).
    pub messages: Messages,
}
```

The `next_id` auto-id counter is intentionally **private** — adapters
MUST NOT depend on the format of generated ids. Callers receive the id
as the return value of `Toaster::create` (or equivalent adapter handle).

`Vec<toast::State>` would be the wrong type because the manager needs
each toast's full config (kind, title, description, callbacks) to drive
deduplication and announcement routing. `ToastEntry` carries that data
plus the lifecycle stage.

```rust
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

/// Axis along which the per-toast swipe gesture is measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwipeAxis { Horizontal, Vertical }

impl Placement {
    /// Returns the swipe axis the placement implies. Center placements
    /// swipe vertically; edge placements swipe horizontally per §7.3.
    pub const fn swipe_axis(self) -> SwipeAxis { /* … */ }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DefaultDurations {
    pub info: Duration,
    pub success: Duration,
    pub warning: Duration,
    pub error: Duration,
    pub loading: Option<Duration>,
}

impl Default for DefaultDurations {
    fn default() -> Self {
        Self {
            info: Duration::from_secs(5),
            success: Duration::from_secs(5),
            warning: Duration::from_secs(5),
            error: Duration::from_secs(8),
            loading: None, // persistent by default
        }
    }
}
```

The `manager` module exposes the agnostic-core `Props`, `Event`, and
`Config` types. Adapters render their own `Provider` component (Leptos
/ Dioxus) on top — the framework wrapper, not a separate Rust module.

````rust
pub mod manager {
    /// Manager `Props` — the immutable configuration handed to
    /// `Service::<manager::Machine>::new`. Note that `messages` is **not**
    /// here; like every other `ars_core::Machine`, the manager takes its
    /// `Messages` as a separate constructor argument so adapters can
    /// thread localized labels through `ArsContext`.
    #[derive(Clone, Debug, PartialEq, HasId)]
    pub struct Props {
        /// Component instance id (hydration-stable).
        pub id: String,
        /// Where toasts appear on screen. Default: `BottomEnd`.
        pub placement: Placement,
        /// Maximum number of simultaneously visible toasts. Default: 5.
        /// Clamped to a minimum of 1 by the builder.
        pub max_visible: usize,
        /// Pixel gap between visible toasts. Default: 16.0.
        pub gap: f64,
        /// Delay before removing a dismissed toast from the DOM. Allows
        /// exit animations to complete. Default: 200 ms.
        pub remove_delay: Duration,
        /// Default auto-dismiss duration per toast kind (milliseconds).
        /// Used when a per-toast `Config::duration` is `None`.
        pub default_durations: DefaultDurations,
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
        /// Keyboard shortcut that focuses the toast region when pressed.
        /// Adapters install a global `keydown` listener and call
        /// [`Hotkey::matches`](ars_interactions::Hotkey::matches) from it;
        /// on a match the adapter moves focus to the rendered region
        /// container. Default: `None` (no hotkey).
        ///
        /// Construct with the typed builders from
        /// [`ars_interactions::Hotkey`]:
        /// ```rust,no_check
        /// use ars_interactions::{Hotkey, KeyboardKey};
        /// // Alt+T
        /// Hotkey::char('t').with_alt();
        /// // F8 alone
        /// Hotkey::named(KeyboardKey::F8);
        /// // Cmd+Shift+K (mac)
        /// Hotkey::char('k').with_meta().with_shift();
        /// ```
        pub hotkey: Option<Hotkey>,
    }

    /// Manager `Messages` — owns the `aria-live` region label only.
    /// Per-toast labels (e.g. `dismiss_label`) live on
    /// [`single::Messages`](super::super::single::Messages).
    #[derive(Clone, Debug, PartialEq)]
    pub struct Messages {
        pub region_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    }

    #[derive(Clone, Debug, PartialEq)]
    pub enum Event {
        /// Admit a new toast, or queue it if `toasts.len() == max_visible`.
        Add(Config),
        /// Update an existing toast (live entry by id, or queued slot by id).
        Update(String, Config),
        /// Mark a toast `EntryStage::Dismissing`. Adapter completes the
        /// exit animation and dispatches `HideQueueAdvance(id)` to free
        /// the slot.
        Remove(String),
        /// Pause every visible toast's auto-dismiss timer.
        PauseAll,
        /// Resume every visible toast's auto-dismiss timer.
        ResumeAll,
        /// Mark every visible toast `Dismissing` and clear the queue.
        DismissAll,
        /// Adapter heartbeat — drains the next announcement entry if at
        /// least 500 ms have elapsed since the previous drain. Carries
        /// the current adapter-clock timestamp (ms) so the gate is
        /// enforced atomically; the agnostic core holds no clock.
        DrainAnnouncement { now_ms: u64 },
        /// Per-toast machine reported `State::Dismissed` (or its
        /// `remove_delay` elapsed). The manager removes the entry and
        /// promotes the next queued config if any.
        HideQueueAdvance(String),
        /// Page Visibility API report. `false` pauses all timers; `true`
        /// resumes them when the manager was previously paused.
        SetVisibility(bool),
    }

    #[derive(Clone, Debug)]
    pub struct Config {
        /// Optional explicit id. When `None`, the manager mints an opaque
        /// monotonic id of the form `toast-<n>`. Adapters MUST NOT depend
        /// on the format — treat it as opaque.
        pub id: Option<String>,
        pub title: Option<String>,
        pub description: Option<String>,
        pub kind: Kind,
        pub duration: Option<Duration>,
        pub dismissible: bool,
        /// When true, a new toast with identical kind + title + description
        /// resets the existing toast (visible match → `Update`; queued
        /// match → in-place replacement that preserves the queued id).
        pub deduplicate: bool,
        /// Callback invoked when the toast pause state changes.
        pub on_pause_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
    }
}
````

### 2.1 Toast Queuing

When the visible toast count would exceed `max_visible`, new toasts are queued:

- The manager carries `queued: VecDeque<manager::Config>` on `ManagerContext`.
- On `Add`, when the count of `EntryStage::Visible` entries equals `max_visible`,
  the config is pushed onto `queued` instead of admitting it.
- On `HideQueueAdvance(id)` (the per-toast `Dismissed` propagation), the
  manager removes the entry and pops the front of `queued`, admitting it
  with full announcement scheduling.
- Queued toasts do not start their auto-dismiss timer until they become visible.
- Queue length is implementation-capped at `max_visible * 32` to avoid
  runaway growth in pathological loops; producers SHOULD throttle
  add-rates rather than rely on the cap.

### 2.2 Stacking Order

Visible toasts are rendered in insertion order within the `toast::Region`. The most recent toast appears at the edge closest to the placement anchor (e.g., for `BottomEnd`, the newest toast is at the bottom). Each toast is offset by the `gap` value from its neighbor. The adapter applies CSS transforms or flexbox ordering to achieve the stacking layout.

### 2.3 Deduplication

When `Add(config)` is received and a **visible** toast already has the
same `kind`, `title`, and `description`, the existing toast is reset by
emitting an internal `Update` (re-running its admission flow and
re-announcing). When the same triple matches a **queued** entry instead,
the queued slot is replaced in place — the queued id is preserved so
adapter-side bookkeeping (callbacks already wired against that id) keeps
working. Either way, no duplicate toast is created.

This prevents notification spam when the same event fires repeatedly
(e.g., network errors). Deduplication is opt-in via the `deduplicate`
field on `Config`, or the global `Props::deduplicate_all` flag.

### 2.4 Pause-on-Hover

When the pointer enters the `toast::Region` container, ALL visible toasts pause their auto-dismiss timers (`Event::PauseAll`). When the pointer leaves, all timers resume (`Event::ResumeAll`). This ensures users have time to read or interact with toasts without them disappearing. The adapter attaches `pointerenter`/`pointerleave` listeners on the `toast::Region` element. Focus within a toast also pauses all timers (for keyboard and screen reader users), resuming on `focusout` when focus leaves the region entirely.

### 2.5 Toaster Imperative API — layering

The imperative API is split between **agnostic core** (config-builder
factories) and **adapters** (event-dispatch glue). The split keeps the
agnostic core free of any `Box<dyn Fn(Event)>` send closure, while still
giving consumers a one-call ergonomic surface.

#### Agnostic core — `Toaster` ZST

```rust
/// Zero-sized handle for building [`manager::Config`] values without an
/// active manager `Api`. Adapters wrap this in their own dispatching
/// `ToasterHandle` (Leptos / Dioxus).
#[derive(Clone, Copy, Debug, Default)]
pub struct Toaster;

impl Toaster {
    pub fn info(title: impl Into<String>, description: impl Into<String>) -> manager::Config { /* … */ }
    pub fn success(title: impl Into<String>, description: impl Into<String>) -> manager::Config { /* … */ }
    pub fn warning(title: impl Into<String>, description: impl Into<String>) -> manager::Config { /* … */ }
    pub fn error(title: impl Into<String>, description: impl Into<String>) -> manager::Config { /* … */ }
    /// Persistent (`duration: None`) toast for promise-style flows.
    pub fn loading(title: impl Into<String>, description: impl Into<String>) -> manager::Config { /* … */ }
}

/// Body for a toast message — used by `Promise` and adapters that
/// transform success/error values into toast bodies on resolution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToastContent {
    pub title: Option<String>,
    pub description: Option<String>,
}

/// Configuration for a promise toast: a loading body shown immediately,
/// plus mappers that turn the future's `Ok(T)` / `Err(E)` into the final
/// success / error body. Spawning the future is adapter-owned.
pub struct Promise<T, E> {
    pub loading: ToastContent,
    pub success: Callback<dyn Fn(T) -> ToastContent + Send + Sync>,
    pub error: Callback<dyn Fn(E) -> ToastContent + Send + Sync>,
}
```

#### Adapter — `ToasterHandle`

```rust
// In ars-leptos / ars-dioxus.
pub struct ToasterHandle {
    send: Box<dyn Fn(manager::Event)>,
}

impl ToasterHandle {
    /// Dispatch a pre-built `Config`. Returns the toast id (auto-generated
    /// when the supplied config has none).
    pub fn add(&self, config: manager::Config) -> String { /* … */ }

    /// Update an existing toast's content and/or kind.
    pub fn update(&self, id: &str, config: manager::Config) { /* … */ }

    /// Dismiss a specific toast by ID.
    pub fn dismiss(&self, id: &str) { /* … */ }

    /// Dismiss all visible toasts.
    pub fn dismiss_all(&self) { /* … */ }

    /// Track an async operation with loading → success/error states.
    pub fn promise<T, E, F>(&self, future: F, promise: Promise<T, E>) -> String
    where
        T: 'static, E: 'static,
        F: Future<Output = Result<T, E>> + 'static,
    { /* spawn_local / spawn + update(id, …) on resolution */ }
}
```

**Adapter obligation for promise toasts:** The adapter MUST:

1. Spawn the user-provided future on the async runtime (Leptos: `spawn_local`, Dioxus: `spawn`)
2. On success: call `update(id, success_config)` with `Kind::Success` and reset duration to the default
3. On error: call `update(id, error_config)` with `Kind::Error` and reset duration to the error default
4. The loading toast remains visible and persistent until the future resolves
5. If the toast was dismissed before the future completed, silently discard the result.

### 2.6 Runtime Property Changes

`Machine::on_props_changed` synthesizes `Event::SyncProps` whenever any
context-backed prop differs (`placement`, `max_visible`, `gap`,
`remove_delay`, `default_durations`, `deduplicate_all`, `offsets`,
`overlap`). Currently-tracked toasts and queue contents are preserved;
the new values take effect on the next `Add` (for fields like
`default_durations` that are read at admission time) or immediately (for
visual fields like `placement` that the adapter re-renders).

**`max_visible` shrink semantics:** when `max_visible` drops at runtime
below the current visible count, **already-admitted toasts are
preserved**. `max_visible` is an admission cap, not a retroactive cull —
yanking toasts a user just saw because a config knob moved would be
worse UX than briefly exceeding the cap. The cap re-applies on the next
`Add` (which queues if `visible_count >= max_visible`), and naturally
catches up as existing toasts dismiss.

`Props::id` is asserted unchangeable — like every other component in
this workspace, mutating an id at runtime would silently break ARIA
wiring.

### 2.7 Named Effects

The manager dispatches the following named effects through
`PendingEffect::named`. Adapters MUST `match effect.name` exhaustively
so unhandled variants surface at compile time.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter inserts the head-of-queue announcement into the polite
    /// `aria-live` region.
    AnnouncePolite,
    /// Adapter inserts the head-of-queue announcement into the assertive
    /// `aria-live` region.
    AnnounceAssertive,
    /// Adapter (re-)starts its 500 ms heartbeat that re-emits
    /// `Event::DrainAnnouncement` until `announcement_queue` is empty.
    /// Emitted whenever a new entry is pushed onto an empty queue.
    ScheduleAnnouncement,
    /// Adapter forwards `Event::Pause` to every visible per-toast machine.
    PauseAllTimers,
    /// Adapter forwards `Event::Resume` to every visible per-toast machine.
    ResumeAllTimers,
    /// Adapter forwards `Event::Dismiss` to every visible per-toast machine.
    DismissAllToasts,
}
```

### 2.8 Manager Connect / API

The manager exposes a single `Part::Root` (scope `toast-provider`) plus
helper accessors for the visible-id list, queue length, and announcement
backlog. The `aria-live` region shells live one layer down on a shared
scope (`toast`) so styling selectors target both the regions and the
per-toast elements uniformly.

```rust
pub struct Api<'a> { /* state, ctx, props, send */ }

impl Api<'_> {
    pub fn is_paused(&self) -> bool { /* … */ }
    pub fn placement(&self) -> Placement { /* … */ }
    pub fn swipe_axis(&self) -> SwipeAxis { /* … */ }
    pub fn visible_ids(&self) -> Vec<&str> { /* … */ }
    pub fn queued_len(&self) -> usize { /* … */ }
    pub fn announcement_backlog(&self) -> usize { /* … */ }

    /// Outer container (`data-ars-scope="toast-provider"`,
    /// `data-ars-part="root"`, `data-ars-placement`, `data-ars-paused`,
    /// `data-ars-overlap`).
    pub fn root_attrs(&self) -> AttrMap { /* … */ }

    /// `aria-live` region shell (`data-ars-scope="toast"`,
    /// `data-ars-part="region"`, `aria-live`, `role`, `aria-atomic="false"`,
    /// `aria-label`, `data-ars-live`).
    pub fn region_attrs(&self, assertive: bool) -> AttrMap { /* … */ }

    pub fn add(&self, config: manager::Config) { /* … */ }
    pub fn update(&self, id: impl Into<String>, config: manager::Config) { /* … */ }
    pub fn dismiss(&self, id: impl Into<String>) { /* … */ }
    pub fn dismiss_all(&self) { /* … */ }
    pub fn pause_all(&self) { /* … */ }
    pub fn resume_all(&self) { /* … */ }
    /// Emits `DrainAnnouncement { now_ms }` with the adapter's clock.
    pub fn drain_announcement(&self, now_ms: u64) { /* … */ }
}

/// Region-part selector for the canonical `region_attrs` helper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionPart { Polite, Assertive }

/// Single canonical region helper. Stamps `data-ars-scope="toast"` so
/// styling selectors match both region shells uniformly with the per-
/// toast elements.
pub fn region_attrs(messages: &Messages, locale: &Locale, part: RegionPart) -> AttrMap { /* … */ }
```

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

| Part          | Element    | Key Attributes                                                                                                                                                                                                 |
| ------------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ProviderRoot  | `<div>`    | `data-ars-scope="toast-provider"`, `data-ars-part="root"`, `data-ars-placement`, `data-ars-paused`, `data-ars-overlap`                                                                                         |
| Region        | `<div>`    | `data-ars-scope="toast"`, `data-ars-part="region"`, `aria-live="polite"`/`"assertive"`, `role="status"`/`"alert"`, `aria-atomic="false"`, `aria-label`, `data-ars-live`                                        |
| Root          | `<div>`    | `id`, `data-ars-scope="toast"`, `data-ars-part="root"`, `data-ars-state`, `data-ars-kind`, `aria-labelledby` (if title), `aria-describedby` (if description), `data-ars-swiping` (presence-only while swiping) |
| Title         | `<div>`    | `id={root-id}-title`, `data-ars-scope="toast"`, `data-ars-part="title"`                                                                                                                                        |
| Description   | `<div>`    | `id={root-id}-description`, `data-ars-scope="toast"`, `data-ars-part="description"`                                                                                                                            |
| ProgressBar   | `<div>`    | `role="progressbar"`, `aria-valuemin="0"`, `aria-valuemax="100"` (no `aria-valuenow` — see §6.4)                                                                                                               |
| ActionTrigger | `<button>` | `data-ars-scope="toast"`, `data-ars-part="action-trigger"`, `type="button"`, `aria-label` from `alt_text`                                                                                                      |
| CloseTrigger  | `<button>` | `data-ars-scope="toast"`, `data-ars-part="close-trigger"`, `type="button"`, `aria-label` from Messages                                                                                                         |

## 4. Accessibility

The Toast system renders **two** `toast::Region` containers in server HTML:

1. `<div aria-live="polite" role="status" aria-label={messages.region_label}>` — for info, success, and loading toasts
2. `<div aria-live="assertive" role="alert" aria-label={messages.region_label}>` — for error and warning toasts

Both regions MUST have `aria-label` to be exposed as navigable landmark regions (ARIA 1.2 §5.3.7).

The `ToastManager` routes each toast to the appropriate region based on `Kind`:

- `Kind::Info | Kind::Success | Kind::Loading` → polite region
- `Kind::Error | Kind::Warning` → assertive region

### 4.1 ARIA Roles, States, and Properties

| Part          | Property           | Value                                                               |
| ------------- | ------------------ | ------------------------------------------------------------------- |
| Region        | `aria-live`        | `"polite"` or `"assertive"` by Kind                                 |
| Region        | `role`             | `"status"` (polite) or `"alert"` (assertive)                        |
| Region        | `aria-label`       | From manager `Messages::region_label` (landmark identification)     |
| Region        | `aria-atomic`      | `"false"` — announce individual toasts                              |
| Root          | `aria-labelledby`  | Title element id (only when `Context::title` is `Some`)             |
| Root          | `aria-describedby` | Description element id (only when `Context::description` is `Some`) |
| ActionTrigger | `aria-label`       | From `alt_text` (consumer-provided)                                 |
| CloseTrigger  | `aria-label`       | From per-toast `Messages::dismiss_label`                            |

The toast Root deliberately omits its own `role` — the surrounding live
region already supplies `role="status"` or `role="alert"` and stamping
the role twice would cause double-announcement on insertion.

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

> **Edge case:** If the adapter observes `Api::announcement_backlog() > 10`,
> it SHOULD log a development-mode warning suggesting the application
> reduce toast frequency. The agnostic core never logs (it has no logging
> dependency); the warning belongs in adapter code where `log` /
> framework-specific tracing is already available. The queue itself is
> not capped — all toasts are eventually announced.

## 5. Layering

The Toast surface deliberately splits between agnostic core
(`ars-components`) and framework adapters (`ars-leptos`, `ars-dioxus`).
The split is **the** contract that lets one machine drive both
frameworks.

| Layer            | Type / function                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Agnostic core    | `single::{Machine, Props, Context, Event, Effect, Api, Part, Messages, Kind, region_attrs}`, `manager::{Machine, Props, ManagerContext, Event, Effect, Api, Part, Messages, Placement, SwipeAxis, AnnouncePriority, EntryStage, ToastEntry, Config, EdgeOffsets, DefaultDurations, RegionPart, region_attrs, Toaster, ToastContent, Promise<T, E>}`. The hotkey type is reused from `ars_interactions::{Hotkey, HotkeyTrigger}`. |
| Adapter (Leptos) | `ars_leptos::toast::ToasterHandle`, `ars_leptos::toast::Provider`, `pointer_capture` for swipe, `set_timeout` for `Effect::DurationTimer`/`Effect::ExitAnimation`, `aria-live` insertion via `NodeRef`                                                                                                                                                                                                                           |
| Adapter (Dioxus) | `ars_dioxus::toast::ToasterHandle`, `ars_dioxus::toast::Provider`, `pointer_capture` for swipe, `set_timeout`-equivalent on the Dioxus async runtime, `aria-live` insertion via `MountedData`                                                                                                                                                                                                                                    |

### 5.1 Adapter responsibilities

The agnostic core deliberately **does not** implement the following
behaviors. Each entry lists why the agnostic core cannot handle it.

- **Real timers** (`Effect::DurationTimer`, `Effect::ExitAnimation`) —
  agnostic core is `no_std`-friendly and has no clock; adapters call
  `PlatformEffects::set_timeout` and dispatch the corresponding
  follow-up event when the timer fires.
- **Per-toast pause/resume forwarding** — when the manager emits
  `Effect::PauseAllTimers`, the adapter snapshots its clock for **each**
  visible toast (so `remaining_ms` reflects each toast's true elapsed
  time) and dispatches `single::Event::Pause { remaining_ms }` to every
  per-toast `Service`. `Effect::ResumeAllTimers` mirrors this with
  `single::Event::Resume`. `Effect::DismissAllToasts` forwards
  `single::Event::Dismiss`.
- **`performance.now()` snapshots** — agnostic core takes the
  remaining-ms snapshot via `Event::Pause { remaining_ms }`; adapters
  read the clock immediately before sending the event.
- **Swipe pointer capture** (`pointerdown` / `pointermove` /
  `pointerup`, plus CSS transform application) — requires DOM event
  listeners and `setPointerCapture`; adapters translate gestures into
  `Event::SwipeStart` / `SwipeMove` / `SwipeEnd`.
- **`aria-live` insertion timing** (the §4.2.2 "clear → wait 100ms →
  insert" two-step pattern) — adapters own the live-region DOM nodes
  and the heartbeat that drives `Event::DrainAnnouncement`.
- **Page Visibility API** wiring — adapters subscribe to
  `visibilitychange` and dispatch `Event::SetVisibility(bool)`.
- **Promise spawning** — `Promise<T, E>` is data-only in the agnostic
  core; the adapter's `ToasterHandle::promise(future, promise)` is what
  calls `spawn_local` / `spawn` and `update(id, …)` on resolution.
- **Hotkey** (`Props::hotkey: Option<Hotkey>`) — adapters install a
  global `keydown` listener, filter `event.repeat` and
  `event.is_composing`, translate the DOM event into a
  `KeyboardEventData`, then call
  [`Hotkey::matches(&event)`](ars_interactions::Hotkey::matches). On
  a match, the adapter moves focus to the rendered region container.
  The agnostic core never installs the listener and never converts DOM
  events.
- **WCAG 2.2.1 (Timing Adjustable)** — the agnostic core does **not**
  enforce a minimum auto-dismiss duration. Pause-on-hover/focus
  satisfies the WCAG "Adjust" criterion (the user can extend the timer
  by hovering), but adapters and consumers SHOULD still avoid pathologically
  short durations (e.g. `Some(500)` for a notification with a long
  description). Adapters MAY clamp at admission time if their UX
  guidelines require it.

See §8 for the full adapter API surface.

## 6. Internationalization

### 6.1 Messages

The toast surface ships **two** message bundles, each tied to its
`ars_core::Machine` impl. Per-toast labels live with the per-toast
machine; the manager owns the region landmark label.

```rust
// Per-toast machine — one bundle per toast `Service`.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Dismiss button label (default: "Dismiss notification").
    pub dismiss_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            dismiss_label: MessageFn::static_str("Dismiss notification"),
        }
    }
}
impl ComponentMessages for Messages {}

// Manager machine — one bundle per provider.
pub mod manager {
    #[derive(Clone, Debug, PartialEq)]
    pub struct Messages {
        /// Accessible label for the toast region landmark (default: "Notifications").
        pub region_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
    }

    impl Default for Messages {
        fn default() -> Self {
            Self {
                region_label: MessageFn::static_str("Notifications"),
            }
        }
    }
    impl ComponentMessages for Messages {}
}
```

Pause/resume is handled implicitly via hover/focus — no explicit buttons.

- `Placement::TopStart/TopEnd/BottomStart/BottomEnd` resolve correctly in RTL.

## 7. Variant: Progress Indicator

Toasts with a `duration` can display a visual progress bar showing time remaining before auto-dismiss.

### 7.1 Additional Props

```rust,no_check
/// Added to toast::Props.
/// When true, a progress bar is rendered inside the toast showing elapsed/remaining time.
pub show_progress: bool,
```

### 7.2 Behavior

- Progress advances from 0 → 1 over the toast's `duration`.
- When `pause_on_hover` is true, progress animation pauses while the pointer is over the toast.
- When `pause_on_page_idle` is true, progress pauses when the browser tab loses focus.
- On resume, progress continues from where it paused (not reset).

### 7.3 CSS Custom Property

The progress value is exposed as a CSS custom property for styling:

```css
[data-ars-part="progress-bar"] {
    --ars-toast-progress: 0; /* 0.0 to 1.0, updated by the adapter */
}
```

### 7.4 Accessibility

- `role="progressbar"` with static `aria-valuemin="0"` and
  `aria-valuemax="100"`. **`aria-valuenow` is intentionally not emitted**
  — per-frame ARIA updates would defeat the next bullet, and adapters
  drive the visual progress through the `--ars-toast-progress` CSS
  custom property instead.
- The progress bar is a visual enhancement — the auto-dismiss timer functions identically with or without it.
- Screen readers do NOT need to announce progress updates (no `aria-live` on the progress bar itself).

## 8. Adapter-Level Imperative API

Framework adapters expose an imperative `ToasterHandle` for creating,
updating, and dismissing toasts programmatically. The handle wraps the
agnostic-core [`Toaster`](#agnostic-core--toaster-zst) config-builder
factories with the framework-specific event-dispatch glue (a
`Box<dyn Fn(manager::Event)>` ultimately routed into the manager
`Service::send`).

### 8.1 Leptos Adapter

```rust,no_check
/// Imperative handle obtained from the toast `Provider` context. The
/// handle is `Clone` and can be moved across signals / async tasks.
#[derive(Clone)]
pub struct ToasterHandle { /* … */ }

impl ToasterHandle {
    /// Dispatch a pre-built `Config`. Returns the toast id (auto-generated
    /// when the supplied config has none).
    pub fn add(&self, config: toast::manager::Config) -> String { /* … */ }

    /// Update an existing toast's content and/or kind.
    pub fn update(&self, id: &str, config: toast::manager::Config) { /* … */ }

    /// Dismiss a specific toast by id.
    pub fn dismiss(&self, id: &str) { /* … */ }

    /// Dismiss every visible toast and clear the queue.
    pub fn dismiss_all(&self) { /* … */ }

    /// Track an async operation with loading → success / error states.
    /// See §8.4.
    pub fn promise<T, E, F>(&self, future: F, promise: toast::manager::Promise<T, E>) -> String
    where
        T: 'static, E: 'static,
        F: Future<Output = Result<T, E>> + 'static,
    { /* … */ }
}

/// Hook returning the active provider's [`ToasterHandle`].
pub fn use_toaster() -> ToasterHandle { /* … */ }
```

Convenience constructors live on the agnostic-core
[`Toaster`](#agnostic-core--toaster-zst) ZST and are reused across
adapters: `Toaster::info(title, description)`, `::success`, `::warning`,
`::error`, `::loading`. Adapters compose them with their own dispatch:

```rust,no_check
let toaster: ToasterHandle = use_toaster();
let id = toaster.add(toast::manager::Toaster::error("Save failed", "Network error"));
```

### 8.2 Dioxus Adapter

The Dioxus surface mirrors §8.1 verbatim — same `ToasterHandle` shape,
same method names, same `promise` signature. The hook is named
`use_toaster()` for symmetry. Internally the Dioxus adapter spawns
futures via the Dioxus async runtime instead of `spawn_local`.

### 8.3 Swipe Direction Configuration

The toast swipe-to-dismiss direction is determined by the `placement` of the toast region:

- **Left/right placements** (e.g., `BottomEnd`, `TopStart`): swipe horizontally to dismiss.
- **Center placements** (e.g., `BottomCenter`, `TopCenter`): swipe vertically (down for bottom, up for top).
- The swipe threshold defaults to `50px` ([`single::DEFAULT_SWIPE_THRESHOLD`]) and is configured per-toast via [`single::Props::swipe_threshold`]. Adapters can resolve the placement-derived axis through [`manager::Placement::swipe_axis`] / [`manager::Api::swipe_axis`].

The adapter MUST:

1. Detect `pointerdown` → track pointer movement along the swipe axis.
2. Apply CSS transform to visually track the swipe (`--ars-toast-swipe-offset`).
3. On release: if offset exceeds threshold or velocity exceeds `0.5`, dismiss; otherwise animate back.

### 8.4 Promise Toast API

The agnostic-core type lives in §2.5:

```rust,no_check
pub struct Promise<T, E> {
    pub loading: ToastContent,
    pub success: Callback<dyn Fn(T) -> ToastContent + Send + Sync>,
    pub error: Callback<dyn Fn(E) -> ToastContent + Send + Sync>,
}
```

Adapters consume it through `ToasterHandle::promise(future, promise)`:

```rust,no_check
let toaster = use_toaster();
let promise = toast::manager::Promise::new(
    toast::manager::ToastContent::new("Saving"),
    |saved: SaveOk| toast::manager::ToastContent::new(format!("Saved {}", saved.name)),
    |err: SaveError| toast::manager::ToastContent::new(format!("Failed: {err}")),
);

toaster.promise(save_to_server(), promise);
```

The adapter:

1. Constructs a `manager::Config` with `kind: Loading`, `duration: None`, and the `loading: ToastContent` body, then dispatches `Event::Add(config)` and remembers the resulting toast id.
2. Spawns the user-provided future on the framework's async runtime (Leptos: `spawn_local`; Dioxus: `spawn`).
3. On `Ok(value)`: builds a Success-kind `Config` from `(promise.success)(value)` and dispatches `Event::Update(id, config)`. The adapter MUST also reset `duration` to `default_durations.success` so the success toast auto-dismisses.
4. On `Err(value)`: builds an Error-kind `Config` from `(promise.error)(value)` and dispatches `Event::Update(id, config)`, resetting `duration` to `default_durations.error`.
5. If the toast was dismissed (or the manager unmounted) before the future completes, the result is silently discarded.

## 9. Library Parity

> Compared against: Ark UI (`Toast`/`Toaster`), Radix UI (`Toast`), React Aria (`Toast`/`ToastQueue`).

### 9.1 Props

| Feature               | ars-ui                              | Ark UI            | Radix UI                    | React Aria | Notes                                                             |
| --------------------- | ----------------------------------- | ----------------- | --------------------------- | ---------- | ----------------------------------------------------------------- |
| Auto-dismiss duration | `duration`                          | `duration`        | `duration`                  | `timeout`  | All libraries                                                     |
| Toast kind/type       | `kind`                              | (create method)   | `type`                      | --         | Ark UI uses method variants; Radix has foreground/background type |
| Placement             | `placement` (Provider)              | `placement`       | --                          | --         | Ark UI parity; Radix uses manual viewport positioning             |
| Max visible           | `max_visible` (Provider)            | `max`             | --                          | --         | Ark UI parity                                                     |
| Gap                   | `gap` (Provider)                    | `gap`             | --                          | --         | Ark UI parity                                                     |
| Offsets               | `offsets` (Provider)                | `offsets`         | --                          | --         | Ark UI parity                                                     |
| Overlap mode          | `overlap` (Provider)                | `overlap`         | --                          | --         | Ark UI parity                                                     |
| Pause on hover        | `pause_on_hover` (Provider)         | (implicit)        | (onPause/onResume)          | --         | All libraries support; different APIs                             |
| Pause on page idle    | `pause_on_page_idle` (Provider)     | `pauseOnPageIdle` | --                          | --         | Ark UI parity                                                     |
| Hotkey                | `hotkey: Option<Hotkey>` (Provider) | `hotkey` (string) | `hotkey` (Viewport, string) | --         | Typed `Hotkey` builder vs Ark UI/Radix `"altKey+KeyT"` strings    |
| Remove delay          | `remove_delay` (Provider)           | `removeDelay`     | --                          | --         | Ark UI parity                                                     |
| Default durations     | `default_durations` (Provider)      | --                | --                          | --         | ars-ui addition for per-kind defaults                             |
| Swipe direction       | (implicit from placement)           | --                | `swipeDirection`            | --         | Radix explicit; ars-ui derives from placement                     |
| Swipe threshold       | `swipe_threshold` (per-toast Props) | --                | `swipeThreshold`            | --         | Radix parity                                                      |
| Region label          | `messages.region_label`             | --                | `label` (Provider/Viewport) | --         | Radix parity                                                      |
| Deduplicate           | `deduplicate` (Config)              | --                | --                          | --         | ars-ui addition                                                   |
| Alt text (Action)     | `alt_text` (ActionTrigger)          | --                | `altText` (Action)          | --         | Radix parity                                                      |
| Show progress         | `show_progress`                     | --                | --                          | --         | ars-ui addition                                                   |
| Open change callback  | --                                  | --                | `onOpenChange`              | --         | Radix per-toast callback                                          |

**Gaps:** None.

### 9.2 Anatomy

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

### 9.3 Events

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

### 9.4 Features

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

### 9.5 Summary

- **Overall:** Full parity.
- **Divergences:** (1) ars-ui uses a state machine per toast + a `ToastManager` for coordination, while Ark UI uses `createToaster()` and Radix uses a Provider/Viewport pattern. (2) ars-ui renders dual live regions (polite/assertive) based on toast kind for correct screen reader urgency, whereas other libraries use a single region. (3) Swipe direction is automatically derived from placement rather than configured separately. (4) Toast kinds include `Loading` for promise-toast patterns. (5) Hotkeys are a typed [`Hotkey`](ars_interactions::Hotkey) builder rather than the JS-flavored `"altKey+KeyT"` string Ark UI / Radix use — the typed enum prevents unparseable chords at compile time.
- **Recommended additions:** None.
