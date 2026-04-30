//! Popover non-modal anchored overlay machine.
//!
//! Owns the binary `Closed`/`Open` state, modal flag, dismissal policy,
//! semantic IDs, ARIA/data attribute output, the adapter-allocated z-index,
//! the runtime placement reported by the adapter, and the adapter-resolvable
//! named effect intents listed in `spec/components/overlay/popover.md` §1.6.
//!
//! The agnostic core never traverses the DOM, never resolves elements by ID,
//! never attaches document listeners, and never inspects real event targets —
//! those responsibilities belong to the framework adapter (`ars-leptos`,
//! `ars-dioxus`). Adapters obtain live element references via `NodeRef`
//! (Leptos) / `MountedData` (Dioxus); the ID fields in `Context` exist purely
//! for ARIA wiring and hydration-stable `id` attributes.
//!
//! Click-outside containment, focus restoration, portal host resolution, and
//! geometry measurement all stay in the adapter layer. The state machine
//! receives back the resulting placement/arrow/z-index data through the
//! [`Event::PositioningUpdate`] and [`Event::SetZIndex`] events.
//!
//! # Adapter mounting flow
//!
//! Adapter integrations (`ars-leptos::use_machine`, `ars-dioxus::use_machine`)
//! always follow the same first-mount sequence. The example below sketches
//! the flow against the agnostic [`Service`](ars_core::Service) directly so
//! the contract is visible without framework noise:
//!
//! ```
//! use ars_components::overlay::popover::{Effect, Event, Machine, Messages, Props};
//! use ars_core::{Env, Service};
//!
//! // 1. Construct the service. `default_open: true` boots straight into
//! //    `State::Open`; the regular open-plan transition does not run.
//! let mut service = Service::<Machine>::new(
//!     Props::new().id("popover-readme").default_open(true),
//!     &Env::default(),
//!     &Messages::default(),
//! );
//!
//! // 2. Drain the initial effects exactly once. The adapter would now
//! //    dispatch each named intent (allocate z-index, attach click-outside
//! //    listener, move focus into the content, fire `on_open_change`).
//! //    Adapter dispatch typically uses an exhaustive `match` on
//! //    `effect.name`; here we collect into the wire-string names so the
//! //    doctest is self-contained.
//! let initial: Vec<Effect> = service
//!     .take_initial_effects()
//!     .into_iter()
//!     .map(|effect| effect.name)
//!     .collect();
//!
//! assert!(initial.contains(&Effect::OpenChange));
//! assert!(initial.contains(&Effect::AllocateZIndex));
//! assert!(initial.contains(&Effect::AttachClickOutside));
//! assert!(initial.contains(&Effect::FocusInitial));
//!
//! // 3. Subsequent calls observe an empty buffer — the contract guarantees
//! //    each effect fires exactly once.
//! assert!(service.take_initial_effects().is_empty());
//!
//! // 4. From here on, regular event dispatch produces effects through
//! //    `SendResult::pending_effects` like any other component.
//! let close = service.send(Event::Close);
//! assert!(close.state_changed);
//! ```

use alloc::{
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    CssProperty, Env, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

use super::positioning::{ArrowOffset, Placement, PositioningOptions, PositioningSnapshot};
use crate::utility::dismissable::DismissAttempt;

// ────────────────────────────────────────────────────────────────────
// Effect
// ────────────────────────────────────────────────────────────────────

/// Typed identifier for every named effect intent the popover machine
/// emits.
///
/// Adapters that dispatch on names write
/// `match effect.name { popover::Effect::OpenChange => …, … }`
/// exhaustively, so name typos and unhandled variants surface at compile
/// time — the same convention used by the dialog and tooltip machines.
/// The variant names themselves are the contract; there is no parallel
/// kebab-case wire form to keep in sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Emitted on every state-flipping transition (`Closed → Open` and
    /// `Open → Closed`) and on a non-Closed initial mount. The adapter
    /// resolves the intent by reading the post-transition open state from
    /// the connected [`Api`] (via [`Api::is_open`]) and invoking
    /// [`Props::on_open_change`] with that value.
    OpenChange,

    /// Emitted on `Closed → Open` and on a non-Closed initial mount.
    /// Adapters install a click-outside listener using the
    /// click-outside race-prevention strategy documented in spec §1.5
    /// (rAF deferral or timestamp comparison) and dispatch
    /// [`Event::CloseOnInteractOutside`] when an outside interaction is
    /// detected.
    AttachClickOutside,

    /// Emitted on every `Open → Closed` transition. Adapters MUST remove
    /// the previously installed click-outside listener and cancel any
    /// pending rAF callback so a stale listener does not attach to an
    /// already-closed popover.
    DetachClickOutside,

    /// Emitted on `Closed → Open` and on a non-Closed initial mount
    /// (see spec §1.6 and `spec/shared/z-index-stacking.md`). Adapters
    /// MUST allocate a z-index from the active
    /// `z_index_allocator::Context` (or the compatibility
    /// `next_z_index()` free function) and dispatch
    /// [`Event::SetZIndex`] back to the machine so the value is stored
    /// in [`Context::z_index`] and rendered through
    /// [`Api::positioner_attrs`].
    AllocateZIndex,

    /// Emitted on every `Open → Closed` transition. Adapters MUST
    /// release the previously allocated z-index claim so subsequent
    /// allocations remain monotonic and bounded.
    ReleaseZIndex,

    /// Emitted on every `Open → Closed` transition. Adapters move
    /// focus back to the trigger element so keyboard users return to
    /// the activator after dismissal — this is the non-modal
    /// equivalent of the dialog `RestoreFocus` intent.
    RestoreFocus,

    /// Emitted on every `Closed → Open` transition and on a non-Closed
    /// initial mount. Adapters MUST move focus to the first tabbable
    /// element inside the popover content; if the content has no
    /// tabbable descendants, focus moves to the content container
    /// itself (which carries `tabindex="-1"`). This is the popover
    /// counterpart of the dialog `FocusInitial` /
    /// `FocusFirstTabbable` intents rolled into a single named effect
    /// because non-modal popovers do not trap focus.
    FocusInitial,
}

// ────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────

/// States for the [`Popover`](self) component.
///
/// Popover uses a binary lifecycle. Mount/unmount animation lifecycle is
/// delegated to the [`Presence`](super::presence) machine and composed by the
/// adapter — see `spec/components/overlay/popover.md` §1.4 (`unmount_on_exit`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The popover is closed and not visible.
    #[default]
    Closed,

    /// The popover is open and visible.
    Open,
}

// ────────────────────────────────────────────────────────────────────
// Event
// ────────────────────────────────────────────────────────────────────

/// Events accepted by the [`Popover`](self) state machine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Open the popover (force-open from closed).
    Open,

    /// Close the popover (force-close from open).
    Close,

    /// Toggle the popover open/closed.
    Toggle,

    /// User pressed the Escape key. The state machine guards on
    /// [`Props::close_on_escape`] before transitioning.
    ///
    /// Adapters MUST invoke [`Props::on_escape_key_down`] with a
    /// [`DismissAttempt`] before sending this event, and MUST NOT send the
    /// event when [`DismissAttempt::is_prevented`] returns `true`.
    CloseOnEscape,

    /// An outside interaction (pointerdown / focus) occurred. The state
    /// machine guards on [`Props::close_on_interact_outside`] before
    /// transitioning.
    ///
    /// Adapters MUST invoke [`Props::on_interact_outside`] with a
    /// [`DismissAttempt`] before sending this event, and MUST NOT send the
    /// event when [`DismissAttempt::is_prevented`] returns `true`.
    CloseOnInteractOutside,

    /// Adapter reported a positioning measurement (placement and optional
    /// arrow offset) for the open popover. Updates [`Context::current_placement`]
    /// and [`Context::arrow_offset`] without affecting the open state.
    ///
    /// The payload is intentionally DOM-free — adapters compute the snapshot
    /// using their framework-specific positioning engine and only forward
    /// the resolved placement and arrow offset, never bounding rectangles or
    /// element references.
    PositioningUpdate(PositioningSnapshot),

    /// Adapter reported the z-index allocated for this popover instance.
    /// Stored in [`Context::z_index`] and rendered through
    /// [`Api::positioner_attrs`] as a `--ars-z-index` custom property.
    SetZIndex(u32),

    /// A title element was rendered; sets [`Context::title_id`] so the
    /// content `aria-labelledby` attribute is emitted.
    RegisterTitle,

    /// A description element was rendered; sets [`Context::description_id`]
    /// so the content `aria-describedby` attribute is emitted.
    RegisterDescription,

    /// Re-apply context-backed [`Props`] fields after a prop change.
    /// Emitted by `Machine::on_props_changed` (the `Machine` trait method
    /// that the [`Machine`] state machine implements) when any non-`open`
    /// field that drives [`Context`] differs between old and new props
    /// (`modal`, `positioning`).
    ///
    /// The transition is context-only; it does not change [`State`] and
    /// emits no adapter intents.
    SyncProps,
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Localizable strings for [`Popover`](self).
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the close trigger button. Defaults to
    /// `"Dismiss popover"`.
    pub dismiss_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            dismiss_label: MessageFn::static_str("Dismiss popover"),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Runtime context for [`Popover`](self).
///
/// The IDs stored here (derived from [`Props::id`] via [`ComponentIds`]) are
/// semantic strings used for ARIA wiring and the rendered `id` attribute
/// only. They are never used by the agnostic core or adapters as
/// element-lookup keys; live element references are captured by the adapter
/// via `NodeRef` / `MountedData`.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Active locale used to resolve [`Messages`].
    pub locale: Locale,

    /// Whether the popover is logically open.
    pub open: bool,

    /// Whether the popover is rendered in modal mode (drives
    /// `role="dialog"` + `aria-modal="true"` on the content element rather
    /// than the default non-modal `role="group"`).
    pub modal: bool,

    /// Hydration-stable id for the trigger element. Derived from
    /// [`Props::id`] at init time.
    pub trigger_id: String,

    /// Hydration-stable id for the content element. Derived from
    /// [`Props::id`] at init time.
    pub content_id: String,

    /// Hydration-stable id for the title element, populated when
    /// [`Event::RegisterTitle`] has been dispatched.
    pub title_id: Option<String>,

    /// Hydration-stable id for the description element, populated when
    /// [`Event::RegisterDescription`] has been dispatched.
    pub description_id: Option<String>,

    /// Adapter-supplied positioning configuration (mirror of
    /// [`Props::positioning`] after applying the `offset` / `cross_offset`
    /// convenience aliases). Adapters read this when measuring the floating
    /// element so the agnostic core retains the canonical resolved value.
    pub positioning: PositioningOptions,

    /// Current resolved placement of the floating element. Initialized from
    /// [`PositioningOptions::placement`] and updated by
    /// [`Event::PositioningUpdate`] when the adapter flips placement after
    /// measurement.
    pub current_placement: Placement,

    /// Latest arrow offset reported by the adapter, when an arrow part is
    /// rendered. `None` until the adapter dispatches a measurement.
    pub arrow_offset: Option<ArrowOffset>,

    /// Adapter-allocated z-index for the positioner. `None` until
    /// [`Event::SetZIndex`] is dispatched (typically immediately after the
    /// `popover-allocate-z-index` effect resolves).
    pub z_index: Option<u32>,

    /// Localized message bundle.
    pub messages: Messages,
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Immutable configuration for a [`Popover`](self) instance.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id. Used as the base for all derived part IDs and
    /// must be hydration-stable across SSR/CSR.
    pub id: String,

    /// Controlled open state. When `Some`, the consumer owns the open state.
    pub open: Option<bool>,

    /// Default open state for uncontrolled mode. Default `false`.
    pub default_open: bool,

    /// Whether the popover is rendered in modal mode. Default `false` —
    /// popovers default to non-modal `role="group"` per spec §3.1.
    pub modal: bool,

    /// Whether the popover closes on Escape. Default `true`.
    pub close_on_escape: bool,

    /// Whether the popover closes on outside pointer/focus interaction.
    /// Default `true`.
    pub close_on_interact_outside: bool,

    /// Positioning options forwarded to the adapter's measurement engine.
    pub positioning: PositioningOptions,

    /// Convenience alias that populates [`PositioningOptions::offset`]'s main
    /// axis. Distance (in CSS pixels) between the trigger and the popover
    /// along the placement direction. Default `0.0`.
    ///
    /// When non-zero this overrides
    /// [`positioning.offset.main_axis`](PositioningOptions::offset).
    pub offset: f64,

    /// Convenience alias that populates [`PositioningOptions::offset`]'s
    /// cross axis. Distance (in CSS pixels) between the trigger and the
    /// popover perpendicular to the placement direction. Default `0.0`.
    ///
    /// When non-zero this overrides
    /// [`positioning.offset.cross_axis`](PositioningOptions::offset).
    pub cross_offset: f64,

    /// When `true`, the popover content matches the trigger (or anchor)
    /// element's width. Useful for dropdown-style popovers. Adapter-only
    /// hint; the agnostic core forwards the value via
    /// [`Api::same_width`]. Default `false`.
    pub same_width: bool,

    /// Whether the popover content is rendered into the portal root.
    /// Adapter-only hint; the agnostic core forwards the value via
    /// [`Api::portal`]. Default `true`.
    pub portal: bool,

    /// When `true`, popover content is not mounted until first opened.
    /// Adapter-only hint; the agnostic core forwards the value via
    /// [`Api::lazy_mount`]. Default `false`.
    pub lazy_mount: bool,

    /// Whether the popover content is removed from the DOM after closing.
    /// Adapter-only hint; the agnostic core forwards the value via
    /// [`Api::unmount_on_exit`]. Default `false`.
    pub unmount_on_exit: bool,

    /// Callback invoked after the open state changes, with the new value.
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,

    /// Callback invoked before [`Event::CloseOnEscape`] is dispatched. The
    /// adapter passes a clone of the [`DismissAttempt`] it constructed; if
    /// the consumer calls
    /// [`DismissAttempt::prevent_dismiss`] the close is cancelled (the veto
    /// flag is shared between clones).
    pub on_escape_key_down: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,

    /// Callback invoked before [`Event::CloseOnInteractOutside`] is
    /// dispatched. The adapter passes a clone of the [`DismissAttempt`] it
    /// constructed; if the consumer calls
    /// [`DismissAttempt::prevent_dismiss`] the close is cancelled (the veto
    /// flag is shared between clones).
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

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    ///
    /// `Props::new()` is the documented entry point for the fluent
    /// builder. Chain setters to populate configuration without the
    /// `..Props::default()` ceremony struct-literal construction requires.
    ///
    /// ```
    /// use ars_components::overlay::popover::Props;
    ///
    /// let props = Props::new()
    ///     .id("settings")
    ///     .modal(true)
    ///     .close_on_escape(false);
    ///
    /// assert_eq!(props.id, "settings");
    /// assert!(props.modal);
    /// assert!(!props.close_on_escape);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id).
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`open`](Self::open) (controlled state).
    #[must_use]
    pub const fn open(mut self, value: Option<bool>) -> Self {
        self.open = value;
        self
    }

    /// Sets [`default_open`](Self::default_open).
    #[must_use]
    pub const fn default_open(mut self, value: bool) -> Self {
        self.default_open = value;
        self
    }

    /// Sets [`modal`](Self::modal).
    #[must_use]
    pub const fn modal(mut self, value: bool) -> Self {
        self.modal = value;
        self
    }

    /// Sets [`close_on_escape`](Self::close_on_escape).
    #[must_use]
    pub const fn close_on_escape(mut self, value: bool) -> Self {
        self.close_on_escape = value;
        self
    }

    /// Sets [`close_on_interact_outside`](Self::close_on_interact_outside).
    #[must_use]
    pub const fn close_on_interact_outside(mut self, value: bool) -> Self {
        self.close_on_interact_outside = value;
        self
    }

    /// Sets [`positioning`](Self::positioning).
    #[must_use]
    pub fn positioning(mut self, value: PositioningOptions) -> Self {
        self.positioning = value;
        self
    }

    /// Sets [`offset`](Self::offset).
    #[must_use]
    pub const fn offset(mut self, value: f64) -> Self {
        self.offset = value;
        self
    }

    /// Sets [`cross_offset`](Self::cross_offset).
    #[must_use]
    pub const fn cross_offset(mut self, value: f64) -> Self {
        self.cross_offset = value;
        self
    }

    /// Sets [`same_width`](Self::same_width).
    #[must_use]
    pub const fn same_width(mut self, value: bool) -> Self {
        self.same_width = value;
        self
    }

    /// Sets [`portal`](Self::portal).
    #[must_use]
    pub const fn portal(mut self, value: bool) -> Self {
        self.portal = value;
        self
    }

    /// Sets [`lazy_mount`](Self::lazy_mount).
    #[must_use]
    pub const fn lazy_mount(mut self, value: bool) -> Self {
        self.lazy_mount = value;
        self
    }

    /// Sets [`unmount_on_exit`](Self::unmount_on_exit).
    #[must_use]
    pub const fn unmount_on_exit(mut self, value: bool) -> Self {
        self.unmount_on_exit = value;
        self
    }

    /// Registers [`on_open_change`](Self::on_open_change).
    #[must_use]
    pub fn on_open_change<F>(mut self, f: F) -> Self
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        self.on_open_change = Some(Callback::new(f));
        self
    }

    /// Registers [`on_escape_key_down`](Self::on_escape_key_down).
    #[must_use]
    pub fn on_escape_key_down<F>(mut self, f: F) -> Self
    where
        F: Fn(DismissAttempt<()>) + Send + Sync + 'static,
    {
        self.on_escape_key_down = Some(Callback::new(f));
        self
    }

    /// Registers [`on_interact_outside`](Self::on_interact_outside).
    #[must_use]
    pub fn on_interact_outside<F>(mut self, f: F) -> Self
    where
        F: Fn(DismissAttempt<()>) + Send + Sync + 'static,
    {
        self.on_interact_outside = Some(Callback::new(f));
        self
    }
}

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// Anatomy parts exposed by the [`Popover`](self) connect API.
#[derive(ComponentPart)]
#[scope = "popover"]
pub enum Part {
    /// The root container element.
    Root,

    /// Optional alternative anchor element used as the positioning reference
    /// when the trigger is not the visual anchor.
    Anchor,

    /// The trigger button that toggles the popover.
    Trigger,

    /// The wrapper element that positions the content relative to the
    /// trigger / anchor.
    Positioner,

    /// The popover content (`role="group"` non-modal or `"dialog"` modal).
    Content,

    /// The optional arrow element rendered alongside the content.
    Arrow,

    /// The optional title element used for `aria-labelledby` wiring.
    Title,

    /// The optional description element used for `aria-describedby` wiring.
    Description,

    /// The optional close trigger button rendered inside the content.
    CloseTrigger,
}

// ────────────────────────────────────────────────────────────────────
// Helpers used by Machine and `init`
// ────────────────────────────────────────────────────────────────────

/// Returns a copy of `props.positioning` with the convenience `offset` and
/// `cross_offset` aliases applied. Non-zero alias values override the
/// corresponding [`PositioningOptions::offset`] axis. Mirrors the spec §1.6
/// init logic.
fn resolved_positioning(props: &Props) -> PositioningOptions {
    let mut positioning = props.positioning.clone();

    if props.offset != 0.0 {
        positioning.offset.main_axis = props.offset;
    }

    if props.cross_offset != 0.0 {
        positioning.offset.cross_axis = props.cross_offset;
    }

    positioning
}

/// Returns the named effect intents that the open lifecycle produces.
///
/// Used by both `open_plan` (the regular `Closed → Open` transition path)
/// and `Machine::initial_effects` (the `default_open: true` boot path) so
/// the two entry points stay in lock-step. Any change to the open lifecycle
/// only needs to be made here.
fn open_lifecycle_effects() -> [PendingEffect<Machine>; 4] {
    [
        PendingEffect::named(Effect::OpenChange),
        PendingEffect::named(Effect::AllocateZIndex),
        PendingEffect::named(Effect::AttachClickOutside),
        PendingEffect::named(Effect::FocusInitial),
    ]
}

/// Returns the open transition plan emitted by `Closed → Open`. Bundled here
/// so the same plan is produced from both [`Event::Open`] and
/// [`Event::Toggle`] without duplicating the effect list.
fn open_plan() -> TransitionPlan<Machine> {
    let mut plan = TransitionPlan::to(State::Open).apply(|ctx: &mut Context| {
        ctx.open = true;
    });

    for effect in open_lifecycle_effects() {
        plan = plan.with_effect(effect);
    }

    plan
}

/// Returns the close transition plan emitted by `Open → Closed`. Releases
/// the z-index allocation, detaches the click-outside listener, fires the
/// open-change intent so consumers see the new state, and asks the adapter
/// to restore focus to the trigger.
fn close_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Closed)
        .apply(|ctx: &mut Context| {
            ctx.open = false;
            ctx.arrow_offset = None;
            ctx.z_index = None;
        })
        .with_effect(PendingEffect::named(Effect::OpenChange))
        .with_effect(PendingEffect::named(Effect::DetachClickOutside))
        .with_effect(PendingEffect::named(Effect::ReleaseZIndex))
        .with_effect(PendingEffect::named(Effect::RestoreFocus))
}

/// Returns `true` when any context-backed non-`open` prop differs between
/// `old` and `new`. Used by [`Machine::on_props_changed`] to decide whether
/// to emit [`Event::SyncProps`].
fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.modal != new.modal
        || old.positioning != new.positioning
        || old.offset != new.offset
        || old.cross_offset != new.cross_offset
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// State machine for the [`Popover`](self) component.
#[derive(Debug)]
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
        let initial_open = props.open.unwrap_or(props.default_open);

        let state = if initial_open {
            State::Open
        } else {
            State::Closed
        };

        let ids = ComponentIds::from_id(&props.id);

        let positioning = resolved_positioning(props);

        let current_placement = positioning.placement;

        (
            state,
            Context {
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
            },
        )
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
        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        // Popover IDs are baked into Context::trigger_id / content_id (and
        // every aria-* relationship that points at them) at init time.
        // Allowing the id to change at runtime would silently break ARIA
        // wiring — the trigger's `aria-controls` would keep pointing at
        // the old content id while the rendered content emits the new one.
        // Tooltip enforces the same invariant.
        assert_eq!(
            old.id, new.id,
            "Popover id cannot change after initialization"
        );

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
        _context: &Self::Context,
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

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// Connected API surface for the [`Popover`](self) component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish()
    }
}

impl Api<'_> {
    /// Returns `true` when the popover is in [`State::Open`].
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.state, State::Open)
    }

    /// Returns `true` when the popover is configured as modal.
    #[must_use]
    pub const fn is_modal(&self) -> bool {
        self.ctx.modal
    }

    /// Returns the current resolved [`Placement`] for the floating element.
    #[must_use]
    pub const fn placement(&self) -> Placement {
        self.ctx.current_placement
    }

    /// Returns the value of [`Props::lazy_mount`].
    #[must_use]
    pub const fn lazy_mount(&self) -> bool {
        self.props.lazy_mount
    }

    /// Returns the value of [`Props::unmount_on_exit`].
    #[must_use]
    pub const fn unmount_on_exit(&self) -> bool {
        self.props.unmount_on_exit
    }

    /// Returns the value of [`Props::portal`].
    #[must_use]
    pub const fn portal(&self) -> bool {
        self.props.portal
    }

    /// Returns the value of [`Props::same_width`].
    #[must_use]
    pub const fn same_width(&self) -> bool {
        self.props.same_width
    }

    const fn state_token(&self) -> &'static str {
        if self.is_open() { "open" } else { "closed" }
    }

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        attrs
    }

    /// Attributes for the optional anchor element.
    #[must_use]
    pub fn anchor_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Anchor.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Attributes for the trigger button.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.trigger_id.clone())
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.is_open() { "true" } else { "false" },
            );

        if self.is_open() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.content_id.clone(),
            );
        }

        attrs
    }

    /// Adapter handler: the trigger element was clicked.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Attributes for the positioner wrapper.
    ///
    /// Renders the resolved placement as `data-ars-placement` and the
    /// adapter-allocated z-index as a `--ars-z-index` custom property
    /// (matches the Tooltip convention so the same stylesheet rule applies
    /// to both components).
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(
                HtmlAttr::Data("ars-placement"),
                self.ctx.current_placement.as_str(),
            );

        if let Some(z_index) = self.ctx.z_index {
            attrs.set_style(CssProperty::Custom("ars-z-index"), z_index.to_string());
        }

        attrs
    }

    /// Attributes for the content element (the popover surface).
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        let role = if self.ctx.modal { "dialog" } else { "group" };

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.content_id.clone())
            .set(HtmlAttr::Role, role)
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(HtmlAttr::TabIndex, "-1");

        if self.ctx.modal {
            attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
        }

        if let Some(title_id) = &self.ctx.title_id {
            attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), title_id.clone());
        }

        if let Some(description_id) = &self.ctx.description_id {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                description_id.clone(),
            );
        }

        attrs
    }

    /// Adapter handler: a key was pressed on the content element. Sends
    /// [`Event::CloseOnEscape`] when the key is Escape (after the adapter
    /// has cleared the [`Props::on_escape_key_down`] callback).
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::CloseOnEscape);
        }
    }

    /// Attributes for the optional arrow element.
    ///
    /// Inline `top` / `left` styles are emitted only when the adapter has
    /// reported an offset via [`Event::PositioningUpdate`]. The agnostic
    /// core never measures the arrow itself.
    #[must_use]
    pub fn arrow_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Arrow.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-placement"),
                self.ctx.current_placement.as_str(),
            );

        if let Some(offset) = self.ctx.arrow_offset {
            attrs
                .set_style(CssProperty::Top, format!("{}px", offset.main_axis))
                .set_style(CssProperty::Left, format!("{}px", offset.cross_axis));
        }

        attrs
    }

    /// Attributes for the optional title element.
    ///
    /// The `id` attribute is emitted only when [`Event::RegisterTitle`] has
    /// been dispatched — adapters render the element conditionally and the
    /// attribute is wired to the content's `aria-labelledby` only when the
    /// title is present.
    #[must_use]
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if let Some(title_id) = &self.ctx.title_id {
            attrs.set(HtmlAttr::Id, title_id.clone());
        }

        attrs
    }

    /// Attributes for the optional description element.
    ///
    /// The `id` attribute is emitted only when [`Event::RegisterDescription`]
    /// has been dispatched.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if let Some(description_id) = &self.ctx.description_id {
            attrs.set(HtmlAttr::Id, description_id.clone());
        }

        attrs
    }

    /// Attributes for the optional close trigger button.
    ///
    /// The accessible label is resolved via [`Messages::dismiss_label`] using
    /// the active context locale.
    #[must_use]
    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();

        let label = (self.ctx.messages.dismiss_label)(&self.ctx.locale);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label);

        attrs
    }

    /// Adapter handler: the close trigger was activated.
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

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloc::{
        rc::Rc,
        string::{String, ToString},
        vec::Vec,
    };
    use core::cell::RefCell;

    use ars_core::{Machine as MachineTrait, MessageFn, SendResult, Service};
    use insta::assert_snapshot;

    use super::*;

    // ── Test fixtures ───────────────────────────────────────────────

    fn test_props() -> Props {
        Props {
            id: "popover".to_string(),
            ..Props::default()
        }
    }

    fn keyboard_data(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    fn effect_names(result: &SendResult<Machine>) -> Vec<Effect> {
        result.pending_effects.iter().map(|e| e.name).collect()
    }

    fn fresh_service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn open_service(props: Props) -> Service<Machine> {
        let mut props = props;

        props.default_open = true;

        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn drain_to_vec(events: &Rc<RefCell<Vec<Event>>>) -> Vec<Event> {
        events.borrow().clone()
    }

    // ── init coverage ──────────────────────────────────────────────

    #[test]
    fn init_default_open_false_starts_closed() {
        let service = fresh_service(test_props());

        assert_eq!(service.state(), &State::Closed);

        let ctx = service.context();

        assert!(!ctx.open);
        assert!(!ctx.modal);
        assert_eq!(ctx.trigger_id, "popover-trigger");
        assert_eq!(ctx.content_id, "popover-content");
        assert!(ctx.title_id.is_none());
        assert!(ctx.description_id.is_none());
        assert_eq!(ctx.current_placement, Placement::Bottom);
        assert!(ctx.arrow_offset.is_none());
        assert!(ctx.z_index.is_none());
    }

    #[test]
    fn init_default_open_true_starts_open() {
        let service = fresh_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
    }

    #[test]
    fn init_controlled_open_overrides_default_open() {
        let service = fresh_service(Props {
            open: Some(false),
            default_open: true,
            ..test_props()
        });

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
    }

    #[test]
    fn init_offset_alias_overrides_positioning_main_axis() {
        let mut positioning = PositioningOptions::default();

        positioning.offset.main_axis = 0.0;

        let service = fresh_service(Props {
            offset: 12.0,
            positioning,
            ..test_props()
        });

        assert!((service.context().positioning.offset.main_axis - 12.0).abs() < f64::EPSILON);
    }

    #[test]
    fn init_cross_offset_alias_overrides_positioning_cross_axis() {
        let service = fresh_service(Props {
            cross_offset: 4.5,
            ..test_props()
        });

        assert!((service.context().positioning.offset.cross_axis - 4.5).abs() < f64::EPSILON);
    }

    #[test]
    fn init_messages_locale_passes_through_env() {
        let env = Env::default();

        let messages = Messages {
            dismiss_label: MessageFn::static_str("Cerrar"),
        };

        let service = Service::<Machine>::new(test_props(), &env, &messages);

        assert_eq!(service.context().locale, env.locale);
        assert_eq!(
            (service.context().messages.dismiss_label)(&service.context().locale),
            "Cerrar"
        );
    }

    // ── State-machine transitions ─────────────────────────────────

    #[test]
    fn closed_to_open_on_trigger_toggle() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Toggle);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
    }

    #[test]
    fn closed_to_open_on_open_event() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Open);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Open);
    }

    #[test]
    fn open_to_closed_on_close_event() {
        let mut service = open_service(test_props());

        let result = service.send(Event::Close);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
    }

    #[test]
    fn open_to_closed_on_toggle() {
        let mut service = open_service(test_props());

        drop(service.send(Event::Toggle));

        assert_eq!(service.state(), &State::Closed);
    }

    #[test]
    fn open_event_in_open_state_no_op() {
        let mut service = open_service(test_props());

        let result = service.send(Event::Open);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Open);
    }

    #[test]
    fn close_event_in_closed_state_no_op() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Close);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Closed);
    }

    #[test]
    fn close_on_escape_closes_when_allowed() {
        let mut service = open_service(Props {
            close_on_escape: true,
            ..test_props()
        });

        let result = service.send(Event::CloseOnEscape);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Closed);
    }

    #[test]
    fn close_on_escape_no_op_when_disabled() {
        let mut service = open_service(Props {
            close_on_escape: false,
            ..test_props()
        });

        let result = service.send(Event::CloseOnEscape);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Open);
    }

    #[test]
    fn close_on_interact_outside_closes_when_allowed() {
        let mut service = open_service(Props {
            close_on_interact_outside: true,
            ..test_props()
        });

        let result = service.send(Event::CloseOnInteractOutside);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Closed);
    }

    #[test]
    fn close_on_interact_outside_no_op_when_disabled() {
        let mut service = open_service(Props {
            close_on_interact_outside: false,
            ..test_props()
        });

        let result = service.send(Event::CloseOnInteractOutside);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Open);
    }

    #[test]
    fn set_z_index_records_value() {
        let mut service = open_service(test_props());

        let result = service.send(Event::SetZIndex(1234));

        assert!(result.context_changed);
        assert_eq!(service.context().z_index, Some(1234));
    }

    #[test]
    fn set_z_index_works_when_closed_too() {
        // SetZIndex is event-only context-only and intentionally unguarded
        // by state — it covers the rare case where the adapter responds to
        // `popover-allocate-z-index` after the user has rapidly closed.
        let mut service = fresh_service(test_props());

        drop(service.send(Event::SetZIndex(42)));

        assert_eq!(service.context().z_index, Some(42));
    }

    #[test]
    fn positioning_update_records_placement_and_arrow_offset() {
        let mut service = open_service(test_props());

        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::TopStart,
            arrow: Some(ArrowOffset {
                main_axis: 7.0,
                cross_axis: -2.0,
            }),
        })));

        assert_eq!(service.context().current_placement, Placement::TopStart);

        let arrow = service
            .context()
            .arrow_offset
            .expect("PositioningUpdate should record arrow offset");

        assert!((arrow.main_axis - 7.0).abs() < f64::EPSILON);
        assert!((arrow.cross_axis + 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn positioning_update_no_op_when_closed() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::TopStart,
            arrow: None,
        }));

        assert!(!result.context_changed);
        assert_eq!(service.context().current_placement, Placement::Bottom);
    }

    #[test]
    fn register_title_sets_title_id() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::RegisterTitle);

        assert!(result.context_changed);
        assert_eq!(service.context().title_id.as_deref(), Some("popover-title"));

        // Idempotent: re-sending RegisterTitle does not flag context_changed
        // a second time. (The current id-derivation runs each call but the
        // assignment is guarded so the field value is unchanged.)
        let result_again = service.send(Event::RegisterTitle);

        assert_eq!(service.context().title_id.as_deref(), Some("popover-title"));
        assert!(!result_again.context_changed);
    }

    #[test]
    fn register_description_sets_description_id() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::RegisterDescription);

        assert!(result.context_changed);
        assert_eq!(
            service.context().description_id.as_deref(),
            Some("popover-description")
        );

        // Idempotent: re-sending RegisterDescription does not flag context_changed
        // a second time. The match guard `ctx.description_id.is_none()` ensures
        // duplicate registrations are no-ops; this assertion locks the guard
        // against silent regressions (mutation testing originally surfaced the
        // gap between this test and `register_title_sets_title_id`).
        let result_again = service.send(Event::RegisterDescription);

        assert_eq!(
            service.context().description_id.as_deref(),
            Some("popover-description")
        );
        assert!(!result_again.context_changed);
    }

    // ── Effect emission tests ─────────────────────────────────────

    #[test]
    fn open_emits_full_open_lifecycle_effect_set() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Open);

        let names = effect_names(&result);

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::AttachClickOutside));
        assert!(names.contains(&Effect::AllocateZIndex));
        assert!(names.contains(&Effect::FocusInitial));
    }

    #[test]
    fn initial_open_state_returns_full_open_lifecycle_via_take_initial_effects() {
        // When the popover boots in `default_open: true`, no transition runs
        // — the open lifecycle has to come from `Service::take_initial_effects`
        // so the adapter can install listeners, allocate z-index, and move
        // focus on first mount.
        let mut service = open_service(test_props());

        let initial = service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        assert!(initial.contains(&Effect::OpenChange));
        assert!(initial.contains(&Effect::AllocateZIndex));
        assert!(initial.contains(&Effect::AttachClickOutside));
        assert!(initial.contains(&Effect::FocusInitial));

        // Subsequent calls return an empty buffer — the contract guarantees
        // each effect fires exactly once.
        assert!(service.take_initial_effects().is_empty());
    }

    #[test]
    fn initial_closed_state_returns_no_initial_effects() {
        let mut service = fresh_service(test_props());

        assert!(service.take_initial_effects().is_empty());
    }

    #[test]
    fn open_does_not_emit_release_or_detach_effects() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Open);

        let names = effect_names(&result);

        assert!(!names.contains(&Effect::DetachClickOutside));
        assert!(!names.contains(&Effect::ReleaseZIndex));
        assert!(!names.contains(&Effect::RestoreFocus));
    }

    #[test]
    fn close_emits_open_change_detach_release_and_restore_effects() {
        let mut service = open_service(test_props());

        let result = service.send(Event::Close);

        let names = effect_names(&result);

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::DetachClickOutside));
        assert!(names.contains(&Effect::ReleaseZIndex));
        assert!(names.contains(&Effect::RestoreFocus));
    }

    #[test]
    fn close_on_escape_emits_full_close_effect_set_when_allowed() {
        let mut service = open_service(test_props());

        let result = service.send(Event::CloseOnEscape);

        let names = effect_names(&result);

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::DetachClickOutside));
        assert!(names.contains(&Effect::ReleaseZIndex));
        assert!(names.contains(&Effect::RestoreFocus));
    }

    #[test]
    fn close_on_interact_outside_emits_full_close_effect_set_when_allowed() {
        let mut service = open_service(test_props());

        let result = service.send(Event::CloseOnInteractOutside);

        let names = effect_names(&result);

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::DetachClickOutside));
        assert!(names.contains(&Effect::ReleaseZIndex));
        assert!(names.contains(&Effect::RestoreFocus));
    }

    #[test]
    fn close_on_escape_emits_no_effects_when_guarded() {
        let mut service = open_service(Props {
            close_on_escape: false,
            ..test_props()
        });

        let result = service.send(Event::CloseOnEscape);

        assert!(effect_names(&result).is_empty());
    }

    #[test]
    fn close_on_interact_outside_emits_no_effects_when_guarded() {
        let mut service = open_service(Props {
            close_on_interact_outside: false,
            ..test_props()
        });

        let result = service.send(Event::CloseOnInteractOutside);

        assert!(effect_names(&result).is_empty());
    }

    #[test]
    fn open_and_close_effects_carry_no_metadata_payload() {
        const ALLOWED: &[Effect] = &[
            Effect::OpenChange,
            Effect::AttachClickOutside,
            Effect::DetachClickOutside,
            Effect::AllocateZIndex,
            Effect::ReleaseZIndex,
            Effect::RestoreFocus,
            Effect::FocusInitial,
        ];

        let mut service = fresh_service(test_props());

        let open_result = service.send(Event::Open);

        for effect in &open_result.pending_effects {
            assert!(
                ALLOWED.contains(&effect.name),
                "open emitted unknown effect {:?}",
                effect.name
            );
            assert!(
                effect.metadata.is_none(),
                "open effect {:?} carries metadata payload",
                effect.name
            );
        }

        let close_result = service.send(Event::Close);

        for effect in &close_result.pending_effects {
            assert!(
                ALLOWED.contains(&effect.name),
                "close emitted unknown effect {:?}",
                effect.name
            );
            assert!(
                effect.metadata.is_none(),
                "close effect {:?} carries metadata payload",
                effect.name
            );
        }
    }

    // ── Api dispatch & accessor tests ─────────────────────────────

    #[test]
    fn on_trigger_click_sends_toggle() {
        let service = fresh_service(test_props());

        let captured = Rc::new(RefCell::new(Vec::new()));
        let captured_for_send = Rc::clone(&captured);
        let send = move |event: Event| captured_for_send.borrow_mut().push(event);

        service.connect(&send).on_trigger_click();

        assert_eq!(drain_to_vec(&captured), [Event::Toggle]);
    }

    #[test]
    fn on_content_keydown_escape_sends_close_on_escape() {
        let service = open_service(test_props());

        let captured = Rc::new(RefCell::new(Vec::new()));
        let captured_for_send = Rc::clone(&captured);
        let send = move |event: Event| captured_for_send.borrow_mut().push(event);

        service
            .connect(&send)
            .on_content_keydown(&keyboard_data(KeyboardKey::Escape));

        assert_eq!(drain_to_vec(&captured), [Event::CloseOnEscape]);
    }

    #[test]
    fn on_content_keydown_non_escape_no_op() {
        let service = open_service(test_props());

        let captured = Rc::new(RefCell::new(Vec::new()));
        let captured_for_send = Rc::clone(&captured);
        let send = move |event: Event| captured_for_send.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_content_keydown(&keyboard_data(KeyboardKey::Tab));
        api.on_content_keydown(&keyboard_data(KeyboardKey::Enter));

        assert!(drain_to_vec(&captured).is_empty());
    }

    #[test]
    fn on_close_trigger_click_sends_close() {
        let service = open_service(test_props());

        let captured = Rc::new(RefCell::new(Vec::new()));
        let captured_for_send = Rc::clone(&captured);
        let send = move |event: Event| captured_for_send.borrow_mut().push(event);

        service.connect(&send).on_close_trigger_click();

        assert_eq!(drain_to_vec(&captured), [Event::Close]);
    }

    #[test]
    fn is_open_returns_true_when_state_open() {
        let service = open_service(test_props());

        assert!(service.connect(&|_| {}).is_open());
    }

    #[test]
    fn is_open_returns_false_when_state_closed() {
        let service = fresh_service(test_props());

        assert!(!service.connect(&|_| {}).is_open());
    }

    #[test]
    fn is_modal_reflects_context_modal_flag() {
        let modal = open_service(Props {
            modal: true,
            ..test_props()
        });

        let non_modal = open_service(test_props());

        assert!(modal.connect(&|_| {}).is_modal());
        assert!(!non_modal.connect(&|_| {}).is_modal());
    }

    #[test]
    fn placement_returns_current_placement_from_context() {
        let mut service = open_service(test_props());

        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::Right,
            arrow: None,
        })));

        assert_eq!(service.connect(&|_| {}).placement(), Placement::Right);
    }

    #[test]
    fn adapter_hint_accessors_mirror_props() {
        let service = fresh_service(Props {
            portal: false,
            same_width: true,
            lazy_mount: true,
            unmount_on_exit: true,
            ..test_props()
        });

        let api = service.connect(&|_| {});

        assert!(!api.portal());
        assert!(api.same_width());
        assert!(api.lazy_mount());
        assert!(api.unmount_on_exit());
    }

    // ── ConnectApi wiring ─────────────────────────────────────────

    #[test]
    fn connect_api_part_attrs_match_direct_methods() {
        use ars_core::ConnectApi as _;

        let mut service = open_service(test_props());

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));
        drop(service.send(Event::SetZIndex(1500)));
        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::TopEnd,
            arrow: Some(ArrowOffset {
                main_axis: 5.0,
                cross_axis: 1.0,
            }),
        })));

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Anchor), api.anchor_attrs());
        assert_eq!(api.part_attrs(Part::Trigger), api.trigger_attrs());
        assert_eq!(api.part_attrs(Part::Positioner), api.positioner_attrs());
        assert_eq!(api.part_attrs(Part::Content), api.content_attrs());
        assert_eq!(api.part_attrs(Part::Arrow), api.arrow_attrs());
        assert_eq!(api.part_attrs(Part::Title), api.title_attrs());
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::CloseTrigger),
            api.close_trigger_attrs()
        );
    }

    // ── on_props_changed ──────────────────────────────────────────

    #[test]
    fn on_props_changed_emits_open_when_controlled_open_set_true() {
        let old = test_props();

        let new = Props {
            open: Some(true),
            ..test_props()
        };

        assert_eq!(Machine::on_props_changed(&old, &new), [Event::Open]);
    }

    #[test]
    fn on_props_changed_emits_close_when_controlled_open_set_false() {
        let old = Props {
            open: Some(true),
            ..test_props()
        };

        let new = Props {
            open: Some(false),
            ..test_props()
        };

        assert_eq!(Machine::on_props_changed(&old, &new), [Event::Close]);
    }

    #[test]
    fn on_props_changed_emits_sync_props_when_modal_changes() {
        let old = test_props();

        let new = Props {
            modal: true,
            ..test_props()
        };

        assert_eq!(Machine::on_props_changed(&old, &new), [Event::SyncProps]);
    }

    #[test]
    fn on_props_changed_emits_sync_props_when_offset_changes() {
        let old = test_props();

        let new = Props {
            offset: 8.0,
            ..test_props()
        };

        assert_eq!(Machine::on_props_changed(&old, &new), [Event::SyncProps]);
    }

    #[test]
    fn on_props_changed_emits_nothing_when_unchanged() {
        let old = test_props();

        assert!(Machine::on_props_changed(&old, &old).is_empty());
    }

    #[test]
    fn sync_props_event_updates_modal_flag_in_context() {
        let mut service = fresh_service(test_props());

        // Mutate via a fresh service whose props differ.
        let mut new_props = test_props();

        new_props.modal = true;

        service.set_props(new_props);

        // `set_props` calls `on_props_changed` and dispatches `SyncProps`.
        assert!(service.context().modal);
    }

    // ── Coverage fillers for branches that don't merit a snapshot ─

    #[test]
    fn title_attrs_omits_id_until_register_title_fires() {
        let service = fresh_service(test_props());

        let attrs = service.connect(&|_| {}).title_attrs();

        // Without `RegisterTitle`, the id attribute is absent — the
        // adapter renders the title element conditionally and the
        // content's `aria-labelledby` is not wired.
        assert!(!format!("{attrs:?}").contains("Id,"));
    }

    #[test]
    fn description_attrs_omits_id_until_register_description_fires() {
        let service = fresh_service(test_props());

        let attrs = service.connect(&|_| {}).description_attrs();

        assert!(!format!("{attrs:?}").contains("Id,"));
    }

    #[test]
    fn close_plan_resets_arrow_offset_and_z_index() {
        let mut service = open_service(test_props());

        drop(service.send(Event::SetZIndex(2400)));
        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::TopStart,
            arrow: Some(ArrowOffset {
                main_axis: 4.0,
                cross_axis: 1.0,
            }),
        })));

        assert!(service.context().z_index.is_some());
        assert!(service.context().arrow_offset.is_some());

        drop(service.send(Event::Close));

        // close_plan resets both fields so a re-open does not surface
        // stale measurement / allocation data before the adapter has
        // had a chance to re-allocate / re-measure.
        assert!(service.context().z_index.is_none());
        assert!(service.context().arrow_offset.is_none());
        // current_placement is intentionally NOT reset — the adapter's
        // PositioningUpdate after re-open will overwrite it.
        assert_eq!(service.context().current_placement, Placement::TopStart);
    }

    #[test]
    #[should_panic(expected = "Popover id cannot change after initialization")]
    fn on_props_changed_panics_when_props_id_changes() {
        let mut service = fresh_service(test_props());

        service.set_props(Props {
            id: "popover-renamed".to_string(),
            ..test_props()
        });
    }

    #[test]
    fn api_passthrough_getters_reflect_props_for_adapter_rendering() {
        // The four passthrough getters (`lazy_mount`, `unmount_on_exit`,
        // `portal`, `same_width`) carry props through to the adapter so it
        // can decide rendering structure (mount lifecycle, portal wrapping,
        // width matching). They never appear in any AttrMap, so snapshot
        // tests miss them — assert each in both polarities to lock the
        // delegation against silent prop-rename or default-flip regressions.
        let with_defaults = fresh_service(test_props());

        let api_default = with_defaults.connect(&|_| {});

        assert!(!api_default.lazy_mount());
        assert!(!api_default.unmount_on_exit());
        assert!(api_default.portal()); // Default is `true` per `Props::default`.
        assert!(!api_default.same_width());

        let with_overrides = fresh_service(Props {
            lazy_mount: true,
            unmount_on_exit: true,
            portal: false,
            same_width: true,
            ..test_props()
        });

        let api_override = with_overrides.connect(&|_| {});

        assert!(api_override.lazy_mount());
        assert!(api_override.unmount_on_exit());
        assert!(!api_override.portal());
        assert!(api_override.same_width());
    }

    #[test]
    fn api_debug_impl_renders_state_context_props_without_send_field() {
        let service = open_service(test_props());

        let api = service.connect(&|_| {});

        let rendered = format!("{api:?}");

        // The Debug impl exists primarily for diagnostics — verify it
        // produces something usable and intentionally omits the `send`
        // closure (which would otherwise print an opaque function
        // pointer in adapter logs).
        assert!(rendered.contains("Api"));
        assert!(rendered.contains("state"));
        assert!(rendered.contains("ctx"));
        assert!(rendered.contains("props"));
        assert!(!rendered.contains("send:"));
    }

    // ── DismissAttempt integration ────────────────────────────────

    #[test]
    fn props_accept_dismiss_attempt_callbacks() {
        use std::sync::{Arc, Mutex};

        // Compile-test that on_escape_key_down / on_interact_outside accept
        // closures keyed on the shared `dismissable::DismissAttempt<()>`,
        // and that the veto flag round-trips between adapter and consumer.
        // Send + Sync is required for the callback type, so the captures use
        // Arc<Mutex<_>> rather than Rc<RefCell<_>>.
        let observed_escape = Arc::new(Mutex::new(false));
        let observed_outside = Arc::new(Mutex::new(false));

        let escape_observed = Arc::clone(&observed_escape);
        let outside_observed = Arc::clone(&observed_outside);

        let props = Props::new()
            .id("popover-dismiss")
            .on_escape_key_down(move |attempt: DismissAttempt<()>| {
                attempt.prevent_dismiss();
                *escape_observed.lock().expect("escape observed mutex") = attempt.is_prevented();
            })
            .on_interact_outside(move |attempt: DismissAttempt<()>| {
                *outside_observed.lock().expect("outside observed mutex") = !attempt.is_prevented();
            });

        let escape_attempt = DismissAttempt::new(());

        if let Some(cb) = &props.on_escape_key_down {
            cb(escape_attempt.clone());
        }

        assert!(escape_attempt.is_prevented());
        assert!(*observed_escape.lock().expect("escape observed mutex"));

        let outside_attempt = DismissAttempt::new(());

        if let Some(cb) = &props.on_interact_outside {
            cb(outside_attempt.clone());
        }

        assert!(!outside_attempt.is_prevented());
        assert!(*observed_outside.lock().expect("outside observed mutex"));
    }

    // ── Props builder ────────────────────────────────────────────

    #[test]
    fn props_new_returns_default() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let positioning = PositioningOptions {
            placement: Placement::RightStart,
            ..PositioningOptions::default()
        };

        let props = Props::new()
            .id("popover-builder")
            .open(Some(true))
            .default_open(true)
            .modal(true)
            .close_on_escape(false)
            .close_on_interact_outside(false)
            .positioning(positioning.clone())
            .offset(6.0)
            .cross_offset(-1.5)
            .same_width(true)
            .portal(false)
            .lazy_mount(true)
            .unmount_on_exit(true)
            .on_open_change(|_| {})
            .on_escape_key_down(|_| {})
            .on_interact_outside(|_| {});

        assert_eq!(props.id, "popover-builder");
        assert_eq!(props.open, Some(true));
        assert!(props.default_open);
        assert!(props.modal);
        assert!(!props.close_on_escape);
        assert!(!props.close_on_interact_outside);
        assert_eq!(props.positioning, positioning);
        assert!((props.offset - 6.0).abs() < f64::EPSILON);
        assert!((props.cross_offset - -1.5).abs() < f64::EPSILON);
        assert!(props.same_width);
        assert!(!props.portal);
        assert!(props.lazy_mount);
        assert!(props.unmount_on_exit);
        assert!(props.on_open_change.is_some());
        assert!(props.on_escape_key_down.is_some());
        assert!(props.on_interact_outside.is_some());
    }

    // ── Snapshot tests (per part × per output-affecting branch) ──

    fn snapshot_service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_service_with_messages(props: Props, messages: &Messages) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), messages)
    }

    #[test]
    fn snapshot_root_closed() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "popover_root_closed",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_open() {
        let service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_snapshot!(
            "popover_root_open",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_anchor() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "popover_anchor",
            snapshot_attrs(&service.connect(&|_| {}).anchor_attrs())
        );
    }

    #[test]
    fn snapshot_trigger_closed() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "popover_trigger_closed",
            snapshot_attrs(&service.connect(&|_| {}).trigger_attrs())
        );
    }

    #[test]
    fn snapshot_trigger_open() {
        let service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_snapshot!(
            "popover_trigger_open",
            snapshot_attrs(&service.connect(&|_| {}).trigger_attrs())
        );
    }

    #[test]
    fn snapshot_positioner_default() {
        let service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_snapshot!(
            "popover_positioner_default",
            snapshot_attrs(&service.connect(&|_| {}).positioner_attrs())
        );
    }

    #[test]
    fn snapshot_positioner_with_placement_top() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::Top,
            arrow: None,
        })));

        assert_snapshot!(
            "popover_positioner_with_placement_top",
            snapshot_attrs(&service.connect(&|_| {}).positioner_attrs())
        );
    }

    #[test]
    fn snapshot_positioner_with_z_index() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::SetZIndex(1500)));

        assert_snapshot!(
            "popover_positioner_with_z_index",
            snapshot_attrs(&service.connect(&|_| {}).positioner_attrs())
        );
    }

    #[test]
    fn snapshot_content_closed() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "popover_content_closed",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_non_modal_group() {
        let service = snapshot_service(Props {
            default_open: true,
            modal: false,
            ..test_props()
        });

        assert_snapshot!(
            "popover_content_open_non_modal_group",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_modal_dialog() {
        let service = snapshot_service(Props {
            default_open: true,
            modal: true,
            ..test_props()
        });

        assert_snapshot!(
            "popover_content_open_modal_dialog",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_with_title() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::RegisterTitle));

        assert_snapshot!(
            "popover_content_open_with_title",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_with_description() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "popover_content_open_with_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_with_title_and_description() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "popover_content_open_with_title_and_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_arrow_default() {
        let service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_snapshot!(
            "popover_arrow_default",
            snapshot_attrs(&service.connect(&|_| {}).arrow_attrs())
        );
    }

    #[test]
    fn snapshot_arrow_with_offset() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::Top,
            arrow: Some(ArrowOffset {
                main_axis: 12.0,
                cross_axis: 4.0,
            }),
        })));

        assert_snapshot!(
            "popover_arrow_with_offset",
            snapshot_attrs(&service.connect(&|_| {}).arrow_attrs())
        );
    }

    #[test]
    fn snapshot_title_without_id() {
        // Pre-registration: only scope/part attrs are emitted. Distinguishes
        // the "title element rendered without `RegisterTitle`" case from the
        // anchor part — both use the same shape but with different scope.
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "popover_title_without_id",
            snapshot_attrs(&service.connect(&|_| {}).title_attrs())
        );
    }

    #[test]
    fn snapshot_title_with_id() {
        let mut service = snapshot_service(test_props());

        drop(service.send(Event::RegisterTitle));

        assert_snapshot!(
            "popover_title_with_id",
            snapshot_attrs(&service.connect(&|_| {}).title_attrs())
        );
    }

    #[test]
    fn snapshot_description_without_id() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "popover_description_without_id",
            snapshot_attrs(&service.connect(&|_| {}).description_attrs())
        );
    }

    #[test]
    fn snapshot_description_with_id() {
        let mut service = snapshot_service(test_props());

        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "popover_description_with_id",
            snapshot_attrs(&service.connect(&|_| {}).description_attrs())
        );
    }

    #[test]
    fn snapshot_close_trigger_default_label() {
        let service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_snapshot!(
            "popover_close_trigger_default_label",
            snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_close_trigger_custom_messages_label() {
        let messages = Messages {
            dismiss_label: MessageFn::static_str("Cerrar ventana"),
        };

        let service = snapshot_service_with_messages(
            Props {
                default_open: true,
                ..test_props()
            },
            &messages,
        );

        assert_snapshot!(
            "popover_close_trigger_custom_messages_label",
            snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs())
        );
    }

    // ── Additional branch coverage (within the 40-snapshot budget) ──

    #[test]
    fn snapshot_positioner_with_placement_left() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::Left,
            arrow: None,
        })));

        assert_snapshot!(
            "popover_positioner_with_placement_left",
            snapshot_attrs(&service.connect(&|_| {}).positioner_attrs())
        );
    }

    #[test]
    fn snapshot_positioner_with_placement_right_end() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::RightEnd,
            arrow: None,
        })));

        assert_snapshot!(
            "popover_positioner_with_placement_right_end",
            snapshot_attrs(&service.connect(&|_| {}).positioner_attrs())
        );
    }

    #[test]
    fn snapshot_positioner_with_placement_and_z_index() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::TopStart,
            arrow: None,
        })));
        drop(service.send(Event::SetZIndex(1500)));

        assert_snapshot!(
            "popover_positioner_with_placement_and_z_index",
            snapshot_attrs(&service.connect(&|_| {}).positioner_attrs())
        );
    }

    #[test]
    fn snapshot_arrow_with_offset_at_top_placement() {
        // Arrow offset combined with a non-default placement — exercises
        // the data-ars-placement attribute alongside inline top/left styles
        // on the same element.
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::PositioningUpdate(PositioningSnapshot {
            placement: Placement::Top,
            arrow: Some(ArrowOffset {
                main_axis: -8.0,
                cross_axis: 24.0,
            }),
        })));

        assert_snapshot!(
            "popover_arrow_with_offset_at_top_placement",
            snapshot_attrs(&service.connect(&|_| {}).arrow_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_modal_with_title() {
        let mut service = snapshot_service(Props {
            default_open: true,
            modal: true,
            ..test_props()
        });

        drop(service.send(Event::RegisterTitle));

        assert_snapshot!(
            "popover_content_open_modal_with_title",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_modal_with_description() {
        let mut service = snapshot_service(Props {
            default_open: true,
            modal: true,
            ..test_props()
        });

        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "popover_content_open_modal_with_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_modal_with_title_and_description() {
        let mut service = snapshot_service(Props {
            default_open: true,
            modal: true,
            ..test_props()
        });

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "popover_content_open_modal_with_title_and_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_trigger_open_with_modal_props() {
        // `trigger_attrs` keeps `aria-haspopup="dialog"` regardless of
        // the modal flag; this snapshot proves the trigger output is
        // identical for modal vs non-modal popovers (the spec requires
        // it: trigger announces "opens a popup", the role of which is
        // determined later on the content element).
        let service = snapshot_service(Props {
            default_open: true,
            modal: true,
            ..test_props()
        });

        assert_snapshot!(
            "popover_trigger_open_with_modal_props",
            snapshot_attrs(&service.connect(&|_| {}).trigger_attrs())
        );
    }
}
