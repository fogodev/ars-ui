---
component: Popover
category: overlay
tier: stateful
foundation_deps: [architecture, accessibility, interactions]
shared_deps: [z-index-stacking]
related: [dialog]
references:
    ark-ui: Popover
    radix-ui: Popover
    react-aria: Popover
---

# Popover

A non-modal overlay anchored to a trigger element for rich content.

Non-modal popovers use `role="group"` to avoid confusing screen readers (JAWS announces 'dialog' and users expect Tab trapping). Reserve `role="dialog"` with `aria-modal="true"` for truly modal popovers that trap focus.

## 1. State Machine

### 1.1 States

```rust
/// The states of the popover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The popover is closed and not visible.
    #[default]
    Closed,
    /// The popover is open and visible.
    Open,
}
```

The `Copy` / `Eq` / `Default` derives match the workspace convention for unit-variant
state enums (Dialog, Tooltip, Presence): they cost nothing for unit variants and let
adapters write `State::default()` instead of `State::Closed`.

### 1.2 Events

```rust
/// The events of the popover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// The popover is opened.
    Open,
    /// The popover is closed.
    Close,
    /// The popover is toggled.
    Toggle,
    /// User pressed Escape. The state machine guards on
    /// `Props::close_on_escape` before transitioning. Adapters MUST invoke
    /// `Props::on_escape_key_down` with a `DismissAttempt<()>` (the shared
    /// veto-capable event from
    /// `crates/ars-components/src/utility/dismissable.rs`) before sending
    /// this event, and MUST NOT send it when `DismissAttempt::is_prevented`
    /// returns `true`.
    CloseOnEscape,
    /// An outside interaction occurred. The state machine guards on
    /// `Props::close_on_interact_outside` before transitioning. Adapters
    /// MUST invoke `Props::on_interact_outside` with a `DismissAttempt<()>`
    /// before sending this event, and MUST NOT send it when
    /// `DismissAttempt::is_prevented` returns `true`.
    CloseOnInteractOutside,
    /// Adapter reported a positioning measurement (placement and optional
    /// arrow offset) for the open popover. The payload is the DOM-free
    /// `PositioningSnapshot` defined in `crates/ars-components/src/overlay/positioning.rs`
    /// — the agnostic core never sees raw rects or element references.
    PositioningUpdate(PositioningSnapshot),
    /// Adapter reported the z-index allocated for this popover instance
    /// (response to the `popover-allocate-z-index` effect).
    SetZIndex(u32),
    /// A title element was rendered; sets `Context::title_id` so the content
    /// `aria-labelledby` attribute is emitted.
    RegisterTitle,
    /// A description element was rendered; sets `Context::description_id`
    /// so the content `aria-describedby` attribute is emitted.
    RegisterDescription,
    /// Re-apply context-backed `Props` fields after a prop change. Emitted
    /// by `Machine::on_props_changed` when any non-`open` field that drives
    /// `Context` differs between old and new props (`modal`, `positioning`,
    /// `offset`, `cross_offset`).
    SyncProps,
}
```

### 1.3 Context

```rust
/// The context of the popover.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved locale for message formatting.
    pub locale: Locale,
    /// Whether the popover is open.
    pub open: bool,
    /// Whether the popover is modal.
    pub modal: bool,
    /// Hydration-stable ID of the trigger element (derived from `Props::id`).
    pub trigger_id: String,
    /// Hydration-stable ID of the content element (derived from `Props::id`).
    pub content_id: String,
    /// ID of the title element. `None` until `Event::RegisterTitle` fires.
    pub title_id: Option<String>,
    /// ID of the description element. `None` until `Event::RegisterDescription` fires.
    pub description_id: Option<String>,
    /// Adapter-supplied positioning configuration (mirror of `Props::positioning`
    /// after applying the `offset` / `cross_offset` convenience aliases).
    pub positioning: PositioningOptions,
    /// Current resolved placement of the floating element. Initialized from
    /// `positioning.placement` and updated by `Event::PositioningUpdate` when
    /// the adapter flips placement after measurement.
    pub current_placement: Placement,
    /// Latest arrow offset reported by the adapter. `None` until measurement.
    pub arrow_offset: Option<ArrowOffset>,
    /// Adapter-allocated z-index for the positioner. `None` until
    /// `Event::SetZIndex` fires (typically right after the
    /// `popover-allocate-z-index` effect resolves).
    pub z_index: Option<u32>,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}
```

`PositioningSnapshot` and `ArrowOffset` are DOM-free types defined alongside
`PositioningOptions` in `crates/ars-components/src/overlay/positioning.rs`. The
agnostic core never imports `ars_dom::positioning::PositioningResult`; that
type stays in the adapter layer where the actual measurement happens.

### 1.4 Props

```rust
/// The props of the popover.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// The ID of the popover. Used as the base for all derived part IDs and
    /// must be hydration-stable across SSR/CSR.
    pub id: String,
    /// Controlled open state. When `Some`, the consumer owns the open state.
    pub open: Option<bool>,
    /// Default open state for uncontrolled mode. Default: `false`.
    pub default_open: bool,
    /// Whether the popover is rendered in modal mode. Default `false` —
    /// popovers default to non-modal `role="group"` per §3.1.
    pub modal: bool,
    /// Whether the popover is closed on escape. Default `true`.
    pub close_on_escape: bool,
    /// Whether the popover is closed on outside pointer/focus interaction.
    /// Default `true`.
    pub close_on_interact_outside: bool,
    /// Positioning options forwarded to the adapter's measurement engine.
    pub positioning: PositioningOptions,
    /// Convenience alias that populates `positioning.offset.main_axis`.
    /// Distance (in CSS pixels) between the trigger and the popover along
    /// the placement direction. Default: `0.0`. When non-zero this
    /// overrides the corresponding axis in `positioning.offset`.
    pub offset: f64,
    /// Convenience alias that populates `positioning.offset.cross_axis`.
    /// Distance (in CSS pixels) between the trigger and the popover along
    /// the cross axis. Default: `0.0`. When non-zero this overrides the
    /// corresponding axis in `positioning.offset`.
    pub cross_offset: f64,
    /// **Adapter-only hint.** When `true`, adapters MUST set
    /// `min-width: <trigger-width>px` on the positioner element after
    /// measuring the trigger. The agnostic core never reads `offsetWidth`
    /// or sets `min-width`; it only forwards the boolean via
    /// `Api::same_width()`. Useful for dropdown-style popovers. Default
    /// `false`.
    pub same_width: bool,
    /// **Adapter-only hint.** When `true`, the popover content is
    /// rendered into the portal root by the adapter. The agnostic core
    /// only forwards the boolean via `Api::portal()`. Default `true`.
    pub portal: bool,
    /// **Adapter-only hint.** When `true`, popover content is not
    /// mounted until first opened. The agnostic core only forwards the
    /// boolean via `Api::lazy_mount()`. Default `false`.
    pub lazy_mount: bool,
    /// **Adapter-only hint.** When `true`, popover content is removed
    /// from the DOM after closing. Works with Presence for exit animations.
    /// The agnostic core only forwards the boolean via
    /// `Api::unmount_on_exit()`. Default `false`.
    pub unmount_on_exit: bool,
    /// Callback invoked when the popover open state changes. Fires after
    /// the transition with the new open state value (`true` for open,
    /// `false` for close).
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,
    /// Callback invoked before `Event::CloseOnEscape` is dispatched. The
    /// adapter passes a clone of the `DismissAttempt<()>` it constructed;
    /// if the consumer calls `DismissAttempt::prevent_dismiss` the close
    /// is cancelled (the veto flag is shared between clones).
    pub on_escape_key_down: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,
    /// Callback invoked before `Event::CloseOnInteractOutside` is
    /// dispatched. The adapter passes a clone of the `DismissAttempt<()>`
    /// it constructed; if the consumer calls
    /// `DismissAttempt::prevent_dismiss` the close is cancelled.
    pub on_interact_outside: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            modal: false,
            close_on_escape: true,
            close_on_interact_outside: true,
            positioning: PositioningOptions::default(),
            offset: 0.0,
            cross_offset: 0.0,
            same_width: false,
            portal: true,
            lazy_mount: false,
            unmount_on_exit: false,
            on_open_change: None,
            on_escape_key_down: None,
            on_interact_outside: None,
        }
    }
}
```

`DismissAttempt<E>` is the shared veto-capable wrapper defined in
`crates/ars-components/src/utility/dismissable.rs` and reused by Dialog,
Popover, HoverCard, and any future overlay that needs preventable dismissal.
Its veto flag is backed by `Arc<AtomicBool>` so the adapter that constructs
the attempt observes any `prevent_dismiss()` the consumer's callback
performed on a clone. Popover wraps `DismissAttempt<()>` because the
underlying event payload (the keyboard / pointer event) is consumed by the
adapter and is not forwarded to the consumer.

### 1.5 Click-Outside Race Prevention

When a popover opens, attaching a click-outside listener synchronously creates a race
condition: the same click event that triggered the popover bubbles up to `document` and
immediately closes it. Adapters MUST guard against this.

**Strategy 1 — rAF Delay (recommended):**
Defer the click-outside listener attachment by one `requestAnimationFrame` after the state
machine transitions to `Open`. This ensures the originating click has fully propagated
before the listener becomes active.

**Strategy 2 — Timestamp Comparison (fallback):**
Record the `event.timeStamp` of the triggering click. The click-outside handler ignores
any event whose `timeStamp` is less than or equal to the recorded value.

**Cleanup ordering:** Adapters MUST remove existing click-outside listeners BEFORE
attaching new ones during state transitions. This prevents duplicate listeners from
accumulating during rapid interactions.

**Rapid open/close guard:** If the state transitions to `Closed` before the deferred rAF
callback fires, the pending listener attachment MUST be cancelled. Otherwise a stale
listener attaches to an already-closed popover.

The two strategies are illustrated below. The blocks are marked `rust,ignore`
because they reference adapter-resolved types (`RafHandle`, `ListenerHandle`,
`ElementRef`, browser `PointerEvent`) that live in framework-specific code
(`ars-leptos`, `ars-dioxus`) — they are not types the agnostic core publishes.

```rust,ignore
/// Strategy 1: rAF-based deferral
struct ClickOutsideGuard {
    /// Handle to the pending rAF callback, used for cancellation.
    pending_raf: Option<RafHandle>,
    /// Handle to the active click-outside listener, used for cleanup.
    active_listener: Option<ListenerHandle>,
}

impl ClickOutsideGuard {
    /// Attach the click-outside listener deferred.
    fn attach_deferred(&mut self, content_el: ElementRef, on_close: impl Fn() + 'static) {
        // Always remove existing listener first (cleanup ordering).
        self.detach();

        let guard_active = Rc::new(Cell::new(true));
        let guard_clone = guard_active.clone();

        self.pending_raf = Some(request_animation_frame(move || {
            if !guard_clone.get() {
                // State transitioned to Closed before rAF fired — bail out.
                return;
            }
            // Now safe to listen: the triggering click has fully propagated.
            let handle = document().add_event_listener("pointerdown", move |e: PointerEvent| {
                if !content_el.contains(e.target_element()) {
                    on_close();
                }
            });
            // Store handle for later cleanup (via detach).
            // In practice the adapter stores this in its own reactive state.
            let _ = handle;
        }));
    }

    /// Detach the click-outside listener.
    fn detach(&mut self) {
        // Cancel pending rAF if state closed before it fired.
        if let Some(raf) = self.pending_raf.take() {
            cancel_animation_frame(raf);
        }
        // Remove active listener.
        if let Some(listener) = self.active_listener.take() {
            listener.remove();
        }
    }
}
```

```rust,ignore
/// Strategy 2: Timestamp comparison (fallback for environments without rAF)
struct TimestampClickOutsideGuard {
    /// timeStamp of the pointer event that triggered the open transition.
    trigger_timestamp: f64,
}

impl TimestampClickOutsideGuard {
    /// Create a new timestamp click-outside guard.
    fn new(trigger_event: &PointerEvent) -> Self {
        Self { trigger_timestamp: trigger_event.time_stamp() }
    }

    /// Whether the click-outside event should close the popover.
    fn should_close(&self, outside_event: &PointerEvent) -> bool {
        // Ignore events with the same or earlier timestamp — they are
        // the originating click still propagating.
        outside_event.time_stamp() > self.trigger_timestamp
    }
}
```

### 1.6 Full Machine Implementation

The agnostic core never reaches into the DOM. Click-outside containment, focus
restoration, portal host resolution, and geometry measurement all stay in the
adapter layer; the state machine surfaces those responsibilities as named
`PendingEffect` intents that adapters listen for and dispatch reciprocal events
back through:

| Effect intent constant        | Effect name                    | Emitted on                                        | Adapter responsibility                                                                                                                       |
| ----------------------------- | ------------------------------ | ------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| `EFFECT_OPEN_CHANGE`          | `popover-open-change`          | every state-flipping transition + non-Closed init | invoke `Props::on_open_change` with the post-transition open state                                                                           |
| `EFFECT_ATTACH_CLICK_OUTSIDE` | `popover-attach-click-outside` | `Closed → Open` + non-Closed init                 | install a click-outside listener using the §1.5 race-prevention strategy and dispatch `Event::CloseOnInteractOutside` on outside interaction |
| `EFFECT_DETACH_CLICK_OUTSIDE` | `popover-detach-click-outside` | every `Open → Closed`                             | remove the previously installed listener and cancel any pending rAF callback                                                                 |
| `EFFECT_ALLOCATE_Z_INDEX`     | `popover-allocate-z-index`     | `Closed → Open` + non-Closed init                 | allocate a z-index from `z_index_allocator::Context` and dispatch `Event::SetZIndex` back so the value is rendered on the positioner         |
| `EFFECT_RELEASE_Z_INDEX`      | `popover-release-z-index`      | every `Open → Closed`                             | release the previously allocated z-index claim                                                                                               |
| `EFFECT_RESTORE_FOCUS`        | `popover-restore-focus`        | every `Open → Closed`                             | move focus back to the trigger element (non-modal equivalent of `dialog-restore-focus`)                                                      |
| `EFFECT_FOCUS_INITIAL`        | `popover-focus-initial`        | `Closed → Open` + non-Closed init                 | move focus to the first tabbable inside the content; if none exists, focus the content container itself (which has `tabindex="-1"`)          |

The "+ non-Closed init" column means the same intent fires from
`Machine::initial_effects` when the popover boots straight into `State::Open`
(via `default_open: true` or controlled `open: Some(true)`). Adapters drain
those effects with `Service::take_initial_effects` on first mount; see
`spec/foundation/01-architecture.md` §X.

The popover machine declares a typed `Effect` enum (associated type
`type Effect = Effect;` on `impl Machine`). Adapters that route on names
use exhaustive `match` on the enum so a new variant added in this
component forces every adapter dispatcher to handle it:

```rust,ignore
use ars_components::overlay::popover::{Effect, /* … */};

for effect in &result.pending_effects {
    match effect.name {
        Effect::OpenChange    => { /* invoke `props.on_open_change` */ }
        Effect::FocusInitial  => { /* move focus into the content */ }
        Effect::AllocateZIndex => { /* allocate from z_index_allocator::Context */ }
        // … remaining variants …
    }
}
```

Adapter logs and devtools panels print the variant directly via the
enum's `Debug` impl — the bound on `Machine::Effect` includes `Debug`
specifically so adapters can render `effect.name` without a parallel
identifier table. See `spec/foundation/01-architecture.md` §2.1.2 for
the full `Machine::Effect` contract.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    OpenChange,
    AttachClickOutside,
    DetachClickOutside,
    AllocateZIndex,
    ReleaseZIndex,
    RestoreFocus,
    FocusInitial,
}

/// The machine for the `Popover` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let ids = ComponentIds::from_id(&props.id);
        // Apply convenience offset/cross_offset aliases into positioning options.
        // Explicit Props.offset / Props.cross_offset override the corresponding
        // PositioningOptions.offset axis when non-zero.
        let mut positioning = props.positioning.clone();
        if props.offset != 0.0 { positioning.offset.main_axis = props.offset; }
        if props.cross_offset != 0.0 { positioning.offset.cross_axis = props.cross_offset; }
        let initial_open = props.open.unwrap_or(props.default_open);
        let initial_state = if initial_open { State::Open } else { State::Closed };
        let current_placement = positioning.placement;
        (initial_state, Context {
            locale: env.locale.clone(),
            open: initial_open,
            modal: props.modal,
            trigger_id: ids.part("trigger"),
            content_id: ids.part("content"),
            title_id: None,
            description_id: None,
            positioning,
            current_placement,
            arrow_offset: None,
            z_index: None,
            messages: messages.clone(),
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Closed, Event::Open | Event::Toggle) => Some(open_plan()),
            (State::Open, Event::Close | Event::Toggle) => Some(close_plan()),
            (State::Open, Event::CloseOnEscape) if props.close_on_escape => Some(close_plan()),
            (State::Open, Event::CloseOnInteractOutside) if props.close_on_interact_outside => {
                Some(close_plan())
            }
            (State::Open, Event::PositioningUpdate(snap)) => {
                let snap = *snap;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.current_placement = snap.placement;
                    ctx.arrow_offset = snap.arrow;
                }))
            }
            (_, Event::SetZIndex(z)) => {
                let z = *z;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.z_index = Some(z);
                }))
            }
            (_, Event::RegisterTitle) if ctx.title_id.is_none() => {
                let title_id = ComponentIds::from_id(&props.id).part("title");
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.title_id = Some(title_id);
                }))
            }
            (_, Event::RegisterDescription) if ctx.description_id.is_none() => {
                let description_id = ComponentIds::from_id(&props.id).part("description");
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.description_id = Some(description_id);
                }))
            }
            (_, Event::SyncProps) => {
                let modal = props.modal;
                let positioning = resolved_positioning(props);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.modal = modal;
                    ctx.current_placement = positioning.placement;
                    ctx.arrow_offset = None;
                    ctx.positioning = positioning;
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

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        // Popover IDs are baked into Context::trigger_id / content_id (and
        // every aria-* relationship that points at them) at init time —
        // a runtime id change would silently break ARIA wiring.
        assert_eq!(old.id, new.id, "Popover id cannot change after initialization");

        let mut events = Vec::new();

        if let (was, Some(now)) = (old.open, new.open)
            && was != Some(now)
        {
            events.push(if now { Event::Open } else { Event::Close });
        }

        if context_relevant_props_changed(old, new) {
            events.push(Event::SyncProps);
        }

        events
    }

    fn initial_effects(
        state: &Self::State,
        _ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        // When `default_open: true` (or controlled `open: Some(true)`),
        // `init` returns `(State::Open, ctx)` directly — no `Closed → Open`
        // transition runs, so the regular open-plan effects never fire.
        // Mirror them here so adapters can drive the same lifecycle on
        // first mount via `Service::take_initial_effects`.
        if matches!(state, State::Open) {
            open_lifecycle_effects().into_iter().collect()
        } else {
            Vec::new()
        }
    }
}

/// Helper: returns the named effect intents that the open lifecycle
/// produces. Used by both `open_plan` (the regular `Closed → Open`
/// transition path) and `Machine::initial_effects` (the `default_open: true`
/// boot path) so the two entry points stay in lock-step.
fn open_lifecycle_effects() -> [PendingEffect<Machine>; 4] {
    [
        PendingEffect::named(EFFECT_OPEN_CHANGE),
        PendingEffect::named(EFFECT_ALLOCATE_Z_INDEX),
        PendingEffect::named(EFFECT_ATTACH_CLICK_OUTSIDE),
        PendingEffect::named(EFFECT_FOCUS_INITIAL),
    ]
}

/// `Closed → Open` plan. Bundled here so both `Event::Open` and
/// `Event::Toggle` produce the same effect set.
fn open_plan() -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Open).apply(|ctx: &mut Context| {
        ctx.open = true;
    });

    for effect in open_lifecycle_effects() {
        plan = plan.with_effect(effect);
    }

    plan
}

/// `Open → Closed` plan. Bundled here so `Close`, `Toggle`, `CloseOnEscape`,
/// and `CloseOnInteractOutside` produce the same effect set when their
/// guards permit the transition.
fn close_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Closed)
        .apply(|ctx: &mut Context| {
            ctx.open = false;
            ctx.arrow_offset = None;
            ctx.z_index = None;
        })
        .with_effect(PendingEffect::named(EFFECT_OPEN_CHANGE))
        .with_effect(PendingEffect::named(EFFECT_DETACH_CLICK_OUTSIDE))
        .with_effect(PendingEffect::named(EFFECT_RELEASE_Z_INDEX))
        .with_effect(PendingEffect::named(EFFECT_RESTORE_FOCUS))
}

/// Apply convenience offset/cross_offset aliases to a positioning options
/// snapshot so init and SyncProps share one rule.
fn resolved_positioning(props: &Props) -> PositioningOptions {
    let mut positioning = props.positioning.clone();
    if props.offset != 0.0 { positioning.offset.main_axis = props.offset; }
    if props.cross_offset != 0.0 { positioning.offset.cross_axis = props.cross_offset; }
    positioning
}

/// Returns `true` when any context-backed non-`open` prop differs.
fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.modal != new.modal
        || old.positioning != new.positioning
        || old.offset != new.offset
        || old.cross_offset != new.cross_offset
}
```

The §1.5 click-outside race-prevention strategies (rAF deferral, timestamp
comparison, cleanup ordering, rapid open/close guard) all live in the adapter
that handles the `popover-attach-click-outside` and
`popover-detach-click-outside` intents — the agnostic core only owns the
intent strings.

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "popover"]
pub enum Part {
    Root,
    Anchor,
    Trigger,
    Positioner,
    Content,
    Arrow,
    Title,
    Description,
    CloseTrigger,
}

/// The API of the `Popover` component.
pub struct Api<'a> {
    /// The state of the popover.
    state: &'a State,
    /// The context of the popover.
    ctx: &'a Context,
    /// The props of the popover.
    props: &'a Props,
    /// The send callback.
    send: &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    // ── Read-only accessors ─────────────────────────────────────────
    //
    // Adapters read these through the connected API rather than reaching
    // into `Props` directly, so the agnostic core remains the single
    // source of truth for runtime state.

    /// Whether the popover is open.
    pub const fn is_open(&self) -> bool { matches!(self.state, State::Open) }

    /// Whether the popover is configured as modal (drives `role="dialog"`
    /// + `aria-modal="true"` on the content element).
    pub const fn is_modal(&self) -> bool { self.ctx.modal }

    /// Current resolved placement (initial = `props.positioning.placement`,
    /// updated by `Event::PositioningUpdate` after the adapter measures and
    /// flips placement).
    pub const fn placement(&self) -> Placement { self.ctx.current_placement }

    /// Forwards `Props::lazy_mount` so adapters can defer mount.
    pub const fn lazy_mount(&self) -> bool { self.props.lazy_mount }

    /// Forwards `Props::unmount_on_exit` so adapters can drop content
    /// on close.
    pub const fn unmount_on_exit(&self) -> bool { self.props.unmount_on_exit }

    /// Forwards `Props::portal` so adapters can portal content into the
    /// shared portal root.
    pub const fn portal(&self) -> bool { self.props.portal }

    /// Forwards `Props::same_width` so adapters can apply
    /// `min-width: <trigger-width>px` on the positioner.
    pub const fn same_width(&self) -> bool { self.props.same_width }

    // ── Anatomy attribute methods ───────────────────────────────────

    /// The attributes for the root element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs
    }

    /// The attributes for the anchor element.
    pub fn anchor_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Anchor.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs
    }

    /// The attributes for the trigger element.
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.trigger_id);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog");
        attrs.set(HtmlAttr::Aria(AriaAttr::Expanded), if self.is_open() { "true" } else { "false" });
        if self.is_open() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Controls), &self.ctx.content_id);
        }
        attrs
    }

    /// The handler for the trigger click event.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// The attributes for the positioner element.
    ///
    /// Renders the resolved placement as `data-ars-placement` and the
    /// adapter-allocated z-index as a `--ars-z-index` custom property
    /// (matches the Tooltip convention so the same stylesheet rule applies
    /// to both components). The agnostic core never emits `top` / `left`
    /// inline styles — those are owned by the adapter that performed the
    /// actual measurement.
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::Data("ars-placement"), self.ctx.current_placement.as_str());
        if let Some(z_index) = self.ctx.z_index {
            attrs.set_style(CssProperty::Custom("ars-z-index"), z_index.to_string());
        }
        attrs
    }

    /// The attributes for the content element.
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, &self.ctx.content_id);
        if self.ctx.modal {
            attrs.set(HtmlAttr::Role, "dialog");
            attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
        } else {
            attrs.set(HtmlAttr::Role, "group");
        }
        attrs.set(HtmlAttr::Data("ars-state"), if self.is_open() { "open" } else { "closed" });
        attrs.set(HtmlAttr::TabIndex, "-1");
        if let Some(title_id) = &self.ctx.title_id {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), title_id);
        }
        if let Some(desc_id) = &self.ctx.description_id {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), desc_id);
        }
        attrs
    }

    /// The handler for the content keydown event.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::CloseOnEscape);
        }
    }

    /// The attributes for the arrow element.
    ///
    /// Inline `top` / `left` styles are emitted only when the adapter has
    /// reported an offset via `Event::PositioningUpdate`. The agnostic core
    /// never measures the arrow itself.
    pub fn arrow_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Arrow.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-placement"), self.ctx.current_placement.as_str());
        if let Some(offset) = self.ctx.arrow_offset {
            attrs.set_style(CssProperty::Top, format!("{}px", offset.main_axis));
            attrs.set_style(CssProperty::Left, format!("{}px", offset.cross_axis));
        }
        attrs
    }

    /// The attributes for the title element.
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(title_id) = &self.ctx.title_id {
            attrs.set(HtmlAttr::Id, title_id);
        }
        attrs
    }

    /// The attributes for the description element.
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if let Some(desc_id) = &self.ctx.description_id {
            attrs.set(HtmlAttr::Id, desc_id);
        }
        attrs
    }

    /// The attributes for the close trigger element.
    ///
    /// `type="button"` is mandatory so a close button placed inside a
    /// `<form>` does not double as the implicit submit button.
    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), (self.ctx.messages.dismiss_label)(&self.ctx.locale));
        attrs
    }

    /// The handler for the close trigger click event.
    pub fn on_close_trigger_click(&self) {
        (self.send)(Event::Close);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Anchor => self.anchor_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Arrow => self.arrow_attrs(),
            Part::Title => self.title_attrs(),
            Part::Description => self.description_attrs(),
            Part::CloseTrigger => self.close_trigger_attrs(),
        }
    }
}
```

## 2. Anatomy

```text
Popover
├── Root                 (required)
├── Trigger              (required)
├── Anchor               (optional — alternative positioning reference)
├── Positioner           (required)
│   ├── Arrow            (optional)
│   └── Content          (required)
│       ├── Title        (optional)
│       ├── Description  (optional)
│       └── CloseTrigger (optional)
```

| Part                     | Element    | Key Attributes                                                                                                                                                                                            |
| ------------------------ | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Root                     | `<div>`    | `data-ars-scope="popover"`, `data-ars-part="root"`, `data-ars-state`                                                                                                                                      |
| Anchor                   | any        | `data-ars-scope="popover"`, `data-ars-part="anchor"`                                                                                                                                                      |
| Trigger                  | `<button>` | `id`, `type="button"`, `aria-haspopup="dialog"`, `aria-expanded`, `aria-controls` (when open)                                                                                                             |
| Positioner               | `<div>`    | `data-ars-scope="popover"`, `data-ars-part="positioner"`, `data-ars-state`, `data-ars-placement`, `--ars-z-index` style (when allocated)                                                                  |
| Content                  | `<div>`    | `id`, `role="group"` or `role="dialog"`, `aria-modal="true"` (modal only), `tabindex="-1"`, `data-ars-state`, `aria-labelledby` (when title registered), `aria-describedby` (when description registered) |
| Arrow                    | `<div>`    | `data-ars-scope="popover"`, `data-ars-part="arrow"`, `data-ars-placement`, inline `top`/`left` styles (when offset reported)                                                                              |
| Title                    | any        | `data-ars-scope="popover"`, `data-ars-part="title"`, `id` (when `Event::RegisterTitle` has fired)                                                                                                         |
| Description              | any        | `data-ars-scope="popover"`, `data-ars-part="description"`, `id` (when `Event::RegisterDescription` has fired)                                                                                             |
| CloseTrigger             | `<button>` | `data-ars-scope="popover"`, `data-ars-part="close-trigger"`, `type="button"`, `aria-label` (from `Messages::dismiss_label`)                                                                               |
| DismissButton (composed) | `<button>` | Rendered by the adapter using `dismissable::dismiss_button_attrs(label)` — see §3.3                                                                                                                       |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part    | Property           | Value                                              |
| ------- | ------------------ | -------------------------------------------------- |
| Content | `role`             | `"group"` (non-modal) or `"dialog"` (modal)        |
| Content | `aria-modal`       | `"true"` (modal only)                              |
| Content | `aria-labelledby`  | Title part ID (when title is rendered)             |
| Content | `aria-describedby` | Description part ID (when description is rendered) |
| Content | `tabindex`         | `"-1"` (allows programmatic focus)                 |
| Trigger | `aria-haspopup`    | `"dialog"` (announces the trigger opens a popup)   |
| Trigger | `aria-expanded`    | `"true"` / `"false"`                               |
| Trigger | `aria-controls`    | Content part ID (when open)                        |

- No `aria-modal` for non-modal popovers.
- Return focus to trigger on close.
- Tab cycles through interactive content but can leave (non-modal).

### 3.2 Focus Management

When `modal=false` (default for Popover, HoverCard):

- **On open**: focus moves to the first tabbable element inside the popover content. If no tabbable element exists, focus moves to the content container itself (which should have `tabindex="-1"`)
- Focus is NOT trapped — Tab moves to the next element in document order (natural tab flow). After the last tabbable element inside the popover, Tab continues to the next element in the page
- `FocusScope::popover()` preset is used: `contain=false`, `restore_focus=true`
- Clicking outside closes the popover and focus returns to trigger
- Escape closes the popover and restores focus to trigger
- `DismissButton` provides screen reader close mechanism
- Content is NOT rendered with `aria-modal="true"`
- Background is NOT made inert

### 3.3 DismissButton

A visually-hidden button placed at the start and/or end of non-modal overlay
content (Popover, HoverCard, Tooltip with interactive content). It allows
screen-reader users to dismiss the overlay without relying on Escape-key
discovery.

DismissButton is **not a popover anatomy part** — the agnostic core does not
publish a `Part::DismissButton`. Instead, adapters compose the shared
`dismissable::dismiss_button_attrs(label)` helper from
`crates/ars-components/src/utility/dismissable.rs`, which is reused by every
overlay that needs the same screen-reader affordance. The helper produces:

```text
data-ars-scope="dismissable"
data-ars-part="dismiss-button"
role="button"
type="button"
tabindex="0"
aria-label=<caller-provided label>
data-ars-visually-hidden=""
```

`spec/components/utility/dismissable.md` §3 documents the rationale for
rendering **two** dismiss buttons (one at the start of the content, one at
the end) — the duplication serves only assistive-technology paths
(forward/backward tab exits, reading-order proximity for screen readers,
rotor / element-list discovery).

**Popover accessibility checklist:** when popover content is interactive,
adapters MUST render a DismissButton (preferably two — start and end) and
wire the click handler to the popover's `Api::on_close_trigger_click()` (or
directly send `Event::Close`). Sighted users never see either button. The
label is sourced from `Messages::dismiss_label` resolved against the active
locale.

## 4. Internationalization

### 4.1 Messages

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Dismiss button label for screen readers (default: "Dismiss popover")
    pub dismiss_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self { dismiss_label: MessageFn::static_str("Dismiss popover") }
    }
}

impl ComponentMessages for Messages {}
```

`Context` derives `PartialEq` and contains `messages: Messages`, so `Messages`
must implement `PartialEq` too. The derive composes naturally because
`MessageFn<T>` already provides `PartialEq` via `Arc::ptr_eq`.

## 5. Differences from Dialog

| Feature                | Dialog                 | Popover                                               |
| ---------------------- | ---------------------- | ----------------------------------------------------- |
| Modal                  | Yes (default)          | No (default); `Props::modal: true` switches to modal  |
| Focus trap             | Yes                    | No (Tab moves through, can leave) — even when modal   |
| Backdrop               | Yes                    | No                                                    |
| Anchored to trigger    | No (centered viewport) | Yes                                                   |
| Close on click outside | Optional               | Yes (default); guarded by `close_on_interact_outside` |

## 6. Library Parity

> Compared against: Ark UI (`Popover`), Radix UI (`Popover`), React Aria (`Popover`).

### 6.1 Props

| Feature                | ars-ui                      | Ark UI                   | Radix UI             | React Aria                         | Notes                                                 |
| ---------------------- | --------------------------- | ------------------------ | -------------------- | ---------------------------------- | ----------------------------------------------------- |
| Controlled open        | `open`                      | `open`                   | `open`               | `isOpen`                           | All libraries                                         |
| Default open           | `default_open`              | `defaultOpen`            | `defaultOpen`        | `defaultOpen`                      | All libraries                                         |
| Modal mode             | `modal`                     | `modal`                  | `modal`              | `isNonModal` (inverse)             | All libraries                                         |
| Close on Escape        | `close_on_escape`           | `closeOnEscape`          | (onEscapeKeyDown)    | `isKeyboardDismissDisabled`        | All libraries                                         |
| Close on outside click | `close_on_interact_outside` | `closeOnInteractOutside` | (onInteractOutside)  | `shouldCloseOnInteractOutside`     | All libraries                                         |
| Positioning options    | `positioning`               | `positioning`            | (side/align/offset)  | `placement`/`offset`/`crossOffset` | ars-ui unified; Radix/React Aria use individual props |
| Offset                 | `offset`                    | (in positioning)         | `sideOffset`         | `offset`                           | Convenience alias                                     |
| Cross offset           | `cross_offset`              | (in positioning)         | `alignOffset`        | `crossOffset`                      | Convenience alias                                     |
| Same width             | `same_width`                | (in positioning)         | --                   | --                                 | Dropdown-style alignment                              |
| Auto focus             | (implicit)                  | `autoFocus`              | (onOpenAutoFocus)    | (implicit)                         | Ark UI has explicit prop                              |
| Initial focus el       | --                          | `initialFocusEl`         | (onOpenAutoFocus)    | --                                 | Ark UI only                                           |
| Portal                 | `portal`                    | `portalled`              | (Portal part)        | `UNSTABLE_portalContainer`         | All libraries                                         |
| Lazy mount             | `lazy_mount`                | `lazyMount`              | --                   | --                                 | Ark UI parity                                         |
| Unmount on exit        | `unmount_on_exit`           | `unmountOnExit`          | (forceMount inverse) | --                                 | Ark UI parity                                         |
| Open change callback   | `on_open_change`            | `onOpenChange`           | `onOpenChange`       | `onOpenChange`                     | All libraries                                         |
| Should flip            | (in positioning)            | (in positioning)         | `avoidCollisions`    | `shouldFlip`                       | All libraries via positioning engine                  |
| Container padding      | (in positioning)            | (in positioning)         | `collisionPadding`   | `containerPadding`                 | All libraries                                         |
| Max height             | (in positioning)            | --                       | --                   | `maxHeight`                        | React Aria only                                       |
| Arrow padding          | (in positioning)            | (in positioning)         | `arrowPadding`       | `arrowBoundaryOffset`              | All libraries                                         |
| Hide when detached     | (in positioning)            | --                       | `hideWhenDetached`   | --                                 | Radix only                                            |

**Gaps:** None. Positioning features are handled through the unified `PositioningOptions` struct.

### 6.2 Anatomy

| Part         | ars-ui       | Ark UI       | Radix UI | React Aria   | Notes                       |
| ------------ | ------------ | ------------ | -------- | ------------ | --------------------------- |
| Root         | Root         | Root         | Root     | --           | Container                   |
| Trigger      | Trigger      | Trigger      | Trigger  | --           | Open button                 |
| Anchor       | Anchor       | Anchor       | Anchor   | (triggerRef) | Alternative positioning ref |
| Positioner   | Positioner   | Positioner   | --       | --           | Ark UI parity               |
| Content      | Content      | Content      | Content  | Popover      | Main content                |
| Arrow        | Arrow        | Arrow        | Arrow    | OverlayArrow | All libraries               |
| Title        | Title        | Title        | --       | --           | ars-ui/Ark UI               |
| Description  | Description  | Description  | --       | --           | ars-ui/Ark UI               |
| CloseTrigger | CloseTrigger | CloseTrigger | Close    | --           | ars-ui/Ark UI/Radix         |
| Indicator    | --           | Indicator    | --       | --           | Ark UI open-state indicator |

**Gaps:** None. Ark UI's `Indicator` part is purely visual and covered by `data-ars-state` attribute on Root/Trigger.

### 6.3 Events

| Callback             | ars-ui                          | Ark UI                 | Radix UI               | React Aria     | Notes                                       |
| -------------------- | ------------------------------- | ---------------------- | ---------------------- | -------------- | ------------------------------------------- |
| Open change          | `on_open_change`                | `onOpenChange`         | `onOpenChange`         | `onOpenChange` | All libraries                               |
| Escape key           | (via close_on_escape)           | `onEscapeKeyDown`      | `onEscapeKeyDown`      | --             | ars-ui uses boolean; Ark/Radix use callback |
| Outside interaction  | (via close_on_interact_outside) | `onInteractOutside`    | `onInteractOutside`    | --             | ars-ui uses boolean prop                    |
| Focus outside        | --                              | `onFocusOutside`       | `onFocusOutside`       | --             | Subsumed by interact outside                |
| Pointer down outside | --                              | `onPointerDownOutside` | `onPointerDownOutside` | --             | Subsumed by interact outside                |
| Exit complete        | (Presence)                      | `onExitComplete`       | --                     | --             | Handled by Presence composition             |

**Gaps:** None.

### 6.4 Features

| Feature                       | ars-ui         | Ark UI        | Radix UI         | React Aria         |
| ----------------------------- | -------------- | ------------- | ---------------- | ------------------ |
| Non-modal (default)           | Yes            | Yes (default) | Yes (default)    | Yes                |
| Modal mode                    | Yes            | Yes           | Yes              | Yes                |
| Focus management              | Yes            | Yes           | Yes              | Yes                |
| Anchored positioning          | Yes            | Yes           | Yes              | Yes                |
| Arrow                         | Yes            | Yes           | Yes              | Yes                |
| Click-outside close           | Yes            | Yes           | Yes              | Yes                |
| Click-outside race prevention | Yes            | --            | --               | --                 |
| Animation support             | Yes (Presence) | Yes           | Yes (forceMount) | Yes (render props) |
| DismissButton                 | Yes            | --            | --               | Yes (implicit)     |
| Lazy mount                    | Yes            | Yes           | --               | --                 |

**Gaps:** None.

### 6.5 Summary

- **Overall:** Full parity.
- **Divergences:** (1) ars-ui uses a unified `PositioningOptions` struct instead of individual `side`/`align`/`sideOffset` props like Radix; convenience aliases (`offset`, `cross_offset`) provide a simpler API. (2) Dismiss interception uses **both** a boolean policy prop (`close_on_*`) and an optional preventable callback (`on_*_key_down`/`on_interact_outside` carrying `DismissAttempt<()>`) — the boolean is the always-on policy, the callback is the per-event veto. (3) Click-outside race prevention is explicitly specified with two strategies (rAF delay and timestamp comparison).
- **Recommended additions:** None.

## 7. Tests

The agnostic-core test surface has three layers, all covered in
`crates/ars-components/src/overlay/popover.rs` under `#[cfg(test)]`:

### 7.1 State-machine invariants

- `Closed → Open` on `Event::Open`, `Event::Toggle`.
- `Open → Closed` on `Event::Close`, `Event::Toggle`.
- `Open → Closed` on `Event::CloseOnEscape` only when `props.close_on_escape`.
- `Open → Closed` on `Event::CloseOnInteractOutside` only when
  `props.close_on_interact_outside`.
- `Event::SetZIndex(z)` updates `Context::z_index` regardless of state
  (covers the rare adapter race where the response arrives after a rapid
  close).
- `Event::PositioningUpdate(snap)` updates `Context::current_placement` /
  `Context::arrow_offset` only while open (closed popovers ignore stale
  measurements).
- `Event::RegisterTitle` / `Event::RegisterDescription` are idempotent —
  guard ensures `context_changed` is `false` on the second send.
- `Event::SyncProps` re-applies `modal`, `positioning`, and resets
  `arrow_offset` so the adapter re-measures.
- `Machine::on_props_changed` panics if `props.id` changes.

### 7.2 Effect contract

- `Closed → Open` emits exactly `EFFECT_OPEN_CHANGE`,
  `EFFECT_ALLOCATE_Z_INDEX`, `EFFECT_ATTACH_CLICK_OUTSIDE`,
  `EFFECT_FOCUS_INITIAL`.
- `Open → Closed` (via `Close`, `Toggle`, or guarded `CloseOnEscape`/
  `CloseOnInteractOutside`) emits exactly `EFFECT_OPEN_CHANGE`,
  `EFFECT_DETACH_CLICK_OUTSIDE`, `EFFECT_RELEASE_Z_INDEX`,
  `EFFECT_RESTORE_FOCUS`.
- Guard misses (`close_on_escape: false` etc.) emit zero effects.
- All effects carry no metadata payload — every name is one of the seven
  documented intent strings.
- `Service::take_initial_effects()` returns the open-lifecycle effect set
  when `default_open: true` (or controlled `open: Some(true)`), and is
  drained exactly once: subsequent calls return an empty buffer.

### 7.3 Connect API snapshots

Per-part × per-output-affecting-branch snapshots stored under
`crates/ars-components/src/overlay/snapshots/`. Required coverage:

- Root: closed, open
- Anchor: default
- Trigger: closed, open (open includes `aria-controls`; both include
  `aria-haspopup`)
- Positioner: default, with placement, with z-index allocated
- Content: closed, open non-modal (`role="group"`), open modal
  (`role="dialog"` + `aria-modal`), open with title registered
  (adds `aria-labelledby`), open with description registered (adds
  `aria-describedby`), open with title + description registered
- Arrow: default, with offset reported
- Title: with id (after `RegisterTitle`)
- Description: with id (after `RegisterDescription`)
- CloseTrigger: default label, custom localized label

CI enforces `cargo insta test --unreferenced=reject` and the per-component
snapshot-count budget (≤ 20 snapshots for popover, see
`xtask/src/lint.rs`).
