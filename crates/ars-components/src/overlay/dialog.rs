//! Dialog modal/non-modal overlay machine.
//!
//! Owns the binary `Closed`/`Open` state, modal flag, dismissal policy, semantic
//! IDs, ARIA/data attribute output, and the adapter-resolvable named effect
//! intents listed in `spec/components/overlay/dialog.md` §1.11.
//!
//! The agnostic core never traverses the DOM, never resolves elements by ID,
//! never attaches document listeners, and never inspects real event targets —
//! those responsibilities belong to the framework adapter (`ars-leptos`,
//! `ars-dioxus`). Adapters obtain live element references via `NodeRef`
//! (Leptos) / `MountedData` (Dioxus); the ID fields in `Context` exist purely
//! for ARIA wiring and hydration-stable `id` attributes.

use alloc::{
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_a11y::FocusTarget;
use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

use crate::utility::dismissable::DismissAttempt;

// ────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────

/// States for the [`Dialog`](self) component.
///
/// Dialog uses a binary lifecycle. Mount/unmount animation lifecycle is
/// delegated to the [`Presence`](super::presence) machine and composed by the
/// adapter — see `spec/components/overlay/dialog.md` §5.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The dialog is closed and not visible.
    #[default]
    Closed,

    /// The dialog is open and visible.
    Open,
}

// ────────────────────────────────────────────────────────────────────
// Event
// ────────────────────────────────────────────────────────────────────

/// Events accepted by the [`Dialog`](self) state machine.
///
/// Mount/unmount animation events (`AnimationStart` / `AnimationEnd`) are not
/// part of this enum — animation lifecycle is owned by
/// [`Presence`](super::presence) and composed at the adapter layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Open the dialog (force-open from closed).
    Open,

    /// Close the dialog (force-close from open).
    Close,

    /// Toggle the dialog open/closed.
    Toggle,

    /// User clicked the backdrop. The state machine guards on
    /// [`Context::close_on_backdrop`] before transitioning.
    ///
    /// Adapters MUST invoke [`Props::on_interact_outside`] with a
    /// [`DismissAttempt`] before sending this event, and MUST NOT send the
    /// event when [`DismissAttempt::is_prevented`] returns `true`.
    CloseOnBackdropClick,

    /// User pressed the Escape key. The state machine guards on
    /// [`Context::close_on_escape`] before transitioning.
    ///
    /// Adapters MUST invoke [`Props::on_escape_key_down`] with a
    /// [`DismissAttempt`] before sending this event, and MUST NOT send the
    /// event when [`DismissAttempt::is_prevented`] returns `true`.
    CloseOnEscape,

    /// A title element was rendered; sets [`Context::has_title`] so the
    /// content `aria-labelledby` attribute is emitted.
    RegisterTitle,

    /// A description element was rendered; sets [`Context::has_description`]
    /// so the content `aria-describedby` attribute is emitted.
    RegisterDescription,

    /// Re-apply context-backed [`Props`] fields after a prop change.
    /// Emitted by `Machine::on_props_changed` (the `Machine` trait method
    /// that the [`Machine`] state machine implements) when any non-`open`
    /// field that drives [`Context`] differs between old and new props
    /// (`modal`, `close_on_backdrop`, `close_on_escape`, `prevent_scroll`,
    /// `restore_focus`, `initial_focus`, `final_focus`, `role`).
    ///
    /// The transition is context-only; it does not change [`State`] and
    /// emits no adapter intents — the next state-flipping transition will
    /// emit intents using the freshly-synced context.
    SyncProps,
}

// ────────────────────────────────────────────────────────────────────
// Role
// ────────────────────────────────────────────────────────────────────

/// Semantic role applied to the dialog content element.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    /// Standard dialog (`role="dialog"`).
    #[default]
    Dialog,

    /// Alert dialog (`role="alertdialog"`).
    AlertDialog,
}

impl Role {
    /// ARIA role token rendered on the content element.
    #[must_use]
    pub const fn as_aria_role(self) -> &'static str {
        match self {
            Self::Dialog => "dialog",
            Self::AlertDialog => "alertdialog",
        }
    }
}

// PreventableEvent — shared. Dialog used to ship a local copy of this
// pattern; the canonical type is now [`DismissAttempt`], which Popover and
// the dismissable utility component also share. The veto flag is backed by
// `Arc<AtomicBool>` so a clone passed into a `Callback` (with `Args:
// 'static`) still propagates vetoes back to the adapter that constructed
// the attempt.

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Runtime context for [`Dialog`](self).
///
/// The IDs derived from [`Context::ids`] (via `ids.part("trigger" |
/// "content" | "title" | "description")`) are semantic strings used for
/// ARIA wiring and the rendered `id` attribute only. They are never used
/// by the agnostic core or adapters as element-lookup keys; live element
/// references are captured by the adapter via `NodeRef` / `MountedData`.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the dialog is logically open.
    pub open: bool,

    /// Whether the dialog is modal (drives `aria-modal` and the
    /// `set-background-inert` adapter intent).
    pub modal: bool,

    /// Whether backdrop clicks may close the dialog.
    pub close_on_backdrop: bool,

    /// Whether the Escape key may close the dialog.
    pub close_on_escape: bool,

    /// Whether the body should be scroll-locked while the dialog is open.
    pub prevent_scroll: bool,

    /// Whether focus should be restored to the trigger when the dialog
    /// closes.
    pub restore_focus: bool,

    /// Initial focus target resolved by the adapter when the dialog opens.
    pub initial_focus: Option<FocusTarget>,

    /// Final focus target resolved by the adapter when the dialog closes.
    pub final_focus: Option<FocusTarget>,

    /// Semantic role applied to the content element.
    pub role: Role,

    /// Hydration-stable IDs derived from [`Props::id`]. Adapters render
    /// each part's `id` attribute via `ids.part("trigger" | "content" |
    /// "title" | "description")`; ARIA wiring (`aria-controls`,
    /// `aria-labelledby`, `aria-describedby`) reads from the same
    /// `part(...)` lookup. This matches the workspace convention used by
    /// `form`, `field`, `fieldset`, `checkbox`, `textarea`, `text_field`,
    /// and `date_field`.
    pub ids: ComponentIds,

    /// Whether a title was registered; controls emission of the content
    /// `aria-labelledby` attribute.
    pub has_title: bool,

    /// Whether a description was registered; controls emission of the
    /// content `aria-describedby` attribute.
    pub has_description: bool,

    /// Active locale used to resolve [`Messages`].
    pub locale: Locale,

    /// Localized message bundle.
    pub messages: Messages,
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Localizable strings for [`Dialog`](self).
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the close trigger button. Defaults to
    /// `"Close dialog"`.
    pub close_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            close_label: MessageFn::static_str("Close dialog"),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Immutable configuration for a [`Dialog`](self) instance.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Controlled open state. When `Some`, overrides
    /// [`default_open`](Self::default_open).
    pub open: Option<bool>,

    /// Initial open state in uncontrolled mode. Default `false`.
    pub default_open: bool,

    /// Whether the dialog is modal. Default `true`.
    pub modal: bool,

    /// Whether backdrop clicks may close the dialog. Default `true`.
    pub close_on_backdrop: bool,

    /// Whether the Escape key may close the dialog. Default `true`.
    pub close_on_escape: bool,

    /// Whether to scroll-lock the body while the dialog is open.
    /// Default `true`.
    pub prevent_scroll: bool,

    /// Whether focus is restored to the trigger when the dialog closes.
    /// Default `true`.
    pub restore_focus: bool,

    /// Initial focus target the adapter resolves when the dialog opens.
    pub initial_focus: Option<FocusTarget>,

    /// Final focus target the adapter resolves when the dialog closes.
    pub final_focus: Option<FocusTarget>,

    /// Semantic role applied to the content element. Default
    /// [`Role::Dialog`].
    pub role: Role,

    /// Heading level for the title (`<h{level}>`), clamped to `1..=6`.
    /// Default `2`.
    pub title_level: u8,

    /// When `true`, dialog content is not rendered until first opened.
    /// Default `false`.
    pub lazy_mount: bool,

    /// When `true`, dialog content is removed from the DOM on close.
    /// Default `false`.
    pub unmount_on_exit: bool,

    /// Callback invoked after the open state changes, with the new value.
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,

    /// Callback invoked before [`Event::CloseOnEscape`] is dispatched. The
    /// adapter passes a clone of the [`DismissAttempt`] it constructed; if
    /// the consumer calls
    /// [`DismissAttempt::prevent_dismiss`] the close is cancelled (the veto
    /// flag is shared between clones).
    pub on_escape_key_down: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,

    /// Callback invoked before [`Event::CloseOnBackdropClick`] is
    /// dispatched, on outside pointer/focus interactions. The adapter passes
    /// a clone of the [`DismissAttempt`] it constructed; if the consumer
    /// calls [`DismissAttempt::prevent_dismiss`] the close is cancelled (the
    /// veto flag is shared between clones).
    pub on_interact_outside: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            modal: true,
            close_on_backdrop: true,
            close_on_escape: true,
            prevent_scroll: true,
            restore_focus: true,
            initial_focus: None,
            final_focus: None,
            role: Role::Dialog,
            title_level: 2,
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
    /// `..Props::default()` ceremony struct-literal construction requires:
    ///
    /// ```
    /// use ars_components::overlay::dialog::{Props, Role};
    ///
    /// let props = Props::new()
    ///     .id("confirm")
    ///     .role(Role::AlertDialog)
    ///     .modal(true)
    ///     .close_on_backdrop(false);
    ///
    /// assert_eq!(props.id, "confirm");
    /// assert_eq!(props.role, Role::AlertDialog);
    /// assert!(props.modal);
    /// assert!(!props.close_on_backdrop);
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

    /// Sets [`close_on_backdrop`](Self::close_on_backdrop).
    #[must_use]
    pub const fn close_on_backdrop(mut self, value: bool) -> Self {
        self.close_on_backdrop = value;
        self
    }

    /// Sets [`close_on_escape`](Self::close_on_escape).
    #[must_use]
    pub const fn close_on_escape(mut self, value: bool) -> Self {
        self.close_on_escape = value;
        self
    }

    /// Sets [`prevent_scroll`](Self::prevent_scroll).
    #[must_use]
    pub const fn prevent_scroll(mut self, value: bool) -> Self {
        self.prevent_scroll = value;
        self
    }

    /// Sets [`restore_focus`](Self::restore_focus).
    #[must_use]
    pub const fn restore_focus(mut self, value: bool) -> Self {
        self.restore_focus = value;
        self
    }

    /// Sets [`initial_focus`](Self::initial_focus).
    #[must_use]
    pub const fn initial_focus(mut self, value: Option<FocusTarget>) -> Self {
        self.initial_focus = value;
        self
    }

    /// Sets [`final_focus`](Self::final_focus).
    #[must_use]
    pub const fn final_focus(mut self, value: Option<FocusTarget>) -> Self {
        self.final_focus = value;
        self
    }

    /// Sets [`role`](Self::role).
    #[must_use]
    pub const fn role(mut self, value: Role) -> Self {
        self.role = value;
        self
    }

    /// Sets [`title_level`](Self::title_level).
    #[must_use]
    pub const fn title_level(mut self, value: u8) -> Self {
        self.title_level = value;
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

/// Anatomy parts exposed by the [`Dialog`](self) connect API.
#[derive(ComponentPart)]
#[scope = "dialog"]
pub enum Part {
    /// The root container element.
    Root,

    /// The trigger button that opens the dialog.
    Trigger,

    /// The semi-transparent backdrop behind the content.
    Backdrop,

    /// The wrapper element that positions the content.
    Positioner,

    /// The dialog content (`role="dialog"` or `"alertdialog"`).
    Content,

    /// The optional title element used for `aria-labelledby` wiring.
    Title,

    /// The optional description element used for `aria-describedby` wiring.
    Description,

    /// The optional close trigger button rendered inside the content.
    CloseTrigger,
}

// ────────────────────────────────────────────────────────────────────
// Effect
// ────────────────────────────────────────────────────────────────────

/// Typed identifier for every named effect intent the dialog machine
/// emits.
///
/// Adapters that dispatch on names use exhaustive
/// `match effect.name { dialog::Effect::OpenChange => …, … }` so name
/// typos and unhandled variants fail at compile time. The variant
/// names themselves are the contract; there is no parallel kebab-case
/// wire form to keep in sync.
///
/// Adapter dispatch sketch:
///
/// ```
/// use ars_components::overlay::dialog::{Effect, Event, Machine, Messages, Props};
/// use ars_core::{Env, Service};
///
/// let mut service = Service::<Machine>::new(
///     Props { default_open: true, id: "dialog".into(), ..Props::default() },
///     &Env::default(),
///     &Messages::default(),
/// );
///
/// let result = service.send(Event::Close);
///
/// for effect in &result.pending_effects {
///     match effect.name {
///         Effect::OpenChange => { /* notify consumer */ }
///         Effect::RestoreFocus => { /* focus the trigger handle */ }
///         _ => { /* ...other intents handled elsewhere... */ }
///     }
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Emitted on `Closed → Open` for modal dialogs (see spec §1.11).
    SetBackgroundInert,

    /// Emitted on `Open → Closed` for modal dialogs (see spec §1.11).
    RemoveBackgroundInert,

    /// Emitted on `Closed → Open` when `prevent_scroll` is `true`
    /// (see spec §1.11).
    ScrollLockAcquire,

    /// Emitted on `Open → Closed` when `prevent_scroll` is `true`
    /// (see spec §1.11).
    ScrollLockRelease,

    /// Emitted on `Closed → Open` to move focus to the explicitly
    /// configured initial focus target inside the dialog (see spec
    /// §1.11).
    FocusInitial,

    /// Emitted on `Closed → Open` when no explicit initial-focus
    /// target was configured; adapters move focus to the first
    /// tabbable descendant of the content (see spec §1.11).
    FocusFirstTabbable,

    /// Emitted on `Open → Closed`; adapters return focus to the
    /// trigger element captured before opening (see spec §1.11).
    RestoreFocus,

    /// Emitted on every state-flipping transition; adapters dispatch
    /// the `on_open_change` callback (see spec §1.11).
    OpenChange,
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// State machine for the [`Dialog`](self) component.
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
        let open = props.open.unwrap_or(props.default_open);

        let state = if open { State::Open } else { State::Closed };

        let ids = ComponentIds::from_id(&props.id);

        (
            state,
            Context {
                open,
                modal: props.modal,
                close_on_backdrop: props.close_on_backdrop,
                close_on_escape: props.close_on_escape,
                prevent_scroll: props.prevent_scroll,
                restore_focus: props.restore_focus,
                initial_focus: props.initial_focus,
                final_focus: props.final_focus,
                role: props.role,
                ids,
                has_title: false,
                has_description: false,
                locale: env.locale.clone(),
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
            (State::Closed, Event::Open | Event::Toggle) => {
                let mut plan = TransitionPlan::to(State::Open)
                    .apply(|ctx: &mut Context| {
                        ctx.open = true;
                    })
                    .with_effect(PendingEffect::named(Effect::OpenChange))
                    .with_effect(PendingEffect::named(Effect::FocusInitial))
                    .with_effect(PendingEffect::named(Effect::FocusFirstTabbable));

                if ctx.prevent_scroll {
                    plan = plan.with_effect(PendingEffect::named(Effect::ScrollLockAcquire));
                }

                if ctx.modal {
                    plan = plan.with_effect(PendingEffect::named(Effect::SetBackgroundInert));
                }

                Some(plan)
            }

            (State::Open, Event::Close | Event::Toggle) => {
                let mut plan = TransitionPlan::to(State::Closed)
                    .apply(|ctx: &mut Context| {
                        ctx.open = false;
                    })
                    .with_effect(PendingEffect::named(Effect::OpenChange));

                if ctx.prevent_scroll {
                    plan = plan.with_effect(PendingEffect::named(Effect::ScrollLockRelease));
                }

                if ctx.modal {
                    plan = plan.with_effect(PendingEffect::named(Effect::RemoveBackgroundInert));
                }

                if ctx.restore_focus {
                    plan = plan.with_effect(PendingEffect::named(Effect::RestoreFocus));
                }

                Some(plan)
            }

            (State::Open, Event::CloseOnBackdropClick) if ctx.close_on_backdrop => Some(
                TransitionPlan::to(State::Closed)
                    .apply(|ctx: &mut Context| {
                        ctx.open = false;
                    })
                    .with_effect(PendingEffect::named(Effect::OpenChange)),
            ),

            (State::Open, Event::CloseOnEscape) if ctx.close_on_escape => Some(
                TransitionPlan::to(State::Closed)
                    .apply(|ctx: &mut Context| {
                        ctx.open = false;
                    })
                    .with_effect(PendingEffect::named(Effect::OpenChange)),
            ),

            (_, Event::RegisterTitle) if !ctx.has_title => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.has_title = true;
                }))
            }

            (_, Event::RegisterDescription) if !ctx.has_description => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.has_description = true;
                }))
            }

            (_, Event::SyncProps) => {
                // Replay context-backed prop fields. The `apply` closure
                // captures the values by copy/move so the agnostic core
                // does not retain `&props`.
                let modal = props.modal;
                let close_on_backdrop = props.close_on_backdrop;
                let close_on_escape = props.close_on_escape;
                let prevent_scroll = props.prevent_scroll;
                let restore_focus = props.restore_focus;
                let initial_focus = props.initial_focus;
                let final_focus = props.final_focus;
                let role = props.role;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.modal = modal;
                    ctx.close_on_backdrop = close_on_backdrop;
                    ctx.close_on_escape = close_on_escape;
                    ctx.prevent_scroll = prevent_scroll;
                    ctx.restore_focus = restore_focus;
                    ctx.initial_focus = initial_focus;
                    ctx.final_focus = final_focus;
                    ctx.role = role;
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
        context: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        // When `default_open: true` (or controlled `open: Some(true)`) the
        // dialog boots straight into `Open` without any `Closed → Open`
        // transition — the regular open-plan effects (focus, scroll-lock,
        // background-inert, open-change) are therefore never emitted. Mirror
        // them here so adapters can drive the same lifecycle on first
        // mount via `Service::take_initial_effects`. The same context-flag
        // guards that `transition` consults are honoured.
        if !matches!(state, State::Open) {
            return Vec::new();
        }

        let mut effects = vec![
            PendingEffect::named(Effect::OpenChange),
            PendingEffect::named(Effect::FocusInitial),
            PendingEffect::named(Effect::FocusFirstTabbable),
        ];

        if context.prevent_scroll {
            effects.push(PendingEffect::named(Effect::ScrollLockAcquire));
        }

        if context.modal {
            effects.push(PendingEffect::named(Effect::SetBackgroundInert));
        }

        effects
    }
}

/// Returns `true` when any context-backed non-`open` prop differs between
/// `old` and `new`. Used by [`Machine::on_props_changed`] to decide whether
/// to emit [`Event::SyncProps`].
fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.modal != new.modal
        || old.close_on_backdrop != new.close_on_backdrop
        || old.close_on_escape != new.close_on_escape
        || old.prevent_scroll != new.prevent_scroll
        || old.restore_focus != new.restore_focus
        || old.initial_focus != new.initial_focus
        || old.final_focus != new.final_focus
        || old.role != new.role
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// Connected API surface for the [`Dialog`](self) component.
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
    /// Returns `true` when the dialog is in [`State::Open`].
    ///
    /// ```
    /// use ars_components::overlay::dialog::{Machine, Messages, Props};
    /// use ars_core::{Env, Service};
    ///
    /// let service = Service::<Machine>::new(
    ///     Props { default_open: true, id: "dialog".into(), ..Props::default() },
    ///     &Env::default(),
    ///     &Messages::default(),
    /// );
    ///
    /// assert!(service.connect(&|_| {}).is_open());
    /// ```
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.state, State::Open)
    }

    /// Returns `true` when the dialog is configured as modal.
    ///
    /// Reads the live [`Context::modal`] flag — which is initialised from
    /// [`Props::modal`] and re-applied by the
    /// [`Event::SyncProps`] transition when props change at runtime.
    /// Useful for adapters that conditionally render the
    /// [`Part::Backdrop`] only for modal dialogs.
    #[must_use]
    pub const fn is_modal(&self) -> bool {
        self.ctx.modal
    }

    /// Returns the active [`Role`] of the dialog content element.
    ///
    /// Reads the live [`Context::role`]. Adapters that need the role for
    /// logging, test fixtures, or conditional behaviour read it through
    /// this accessor; the same value drives the `role="..."` attribute on
    /// [`Part::Content`] via [`Role::as_aria_role`].
    #[must_use]
    pub const fn role(&self) -> Role {
        self.ctx.role
    }

    /// Returns the value of [`Props::lazy_mount`].
    ///
    /// Adapter-only hint: the agnostic core never reads this flag, but
    /// adapters composing with [`Presence`](super::presence) need to defer
    /// content rendering and CSS animation start until lazy content
    /// settles. Exposed through the `Api` so adapters do not need to
    /// thread `Props` separately.
    #[must_use]
    pub const fn lazy_mount(&self) -> bool {
        self.props.lazy_mount
    }

    /// Returns the value of [`Props::unmount_on_exit`].
    ///
    /// Adapter-only hint: when `true`, adapters MUST remove the dialog
    /// content from the DOM on close (after the exit animation finishes).
    /// The agnostic core does not consume this flag.
    #[must_use]
    pub const fn unmount_on_exit(&self) -> bool {
        self.props.unmount_on_exit
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

    /// Attributes for the trigger button.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("trigger"))
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.is_open() { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            );

        attrs
    }

    /// Adapter handler: the trigger element was clicked.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Attributes for the backdrop element.
    ///
    /// `aria-hidden="true"` and `inert` are always emitted because the
    /// backdrop is decorative. The `set-background-inert` adapter intent
    /// (see spec §1.11) governs the *sibling* inert state, not this
    /// element.
    #[must_use]
    pub fn backdrop_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Backdrop.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::Inert, "");

        attrs
    }

    /// Adapter handler: a pointer event closed the dialog from the
    /// backdrop. Adapters MUST consult [`Props::on_interact_outside`] (with
    /// a [`DismissAttempt`]) before calling this method.
    pub fn on_backdrop_click(&self) {
        (self.send)(Event::CloseOnBackdropClick);
    }

    /// Attributes for the positioner wrapper.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Attributes for the content element (the dialog itself).
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, self.ctx.role.as_aria_role())
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(
                HtmlAttr::TabIndex,
                // Content is a programmatic focus target during the focus
                // delay window; adapters override with `tabindex="0"`
                // dynamically when needed. We render -1 by default so the
                // content is reachable via `.focus()` but not via Tab.
                "-1",
            );

        if self.ctx.modal {
            attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
        }

        if self.ctx.has_title {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("title"),
            );
        }

        if self.ctx.has_description {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                self.ctx.ids.part("description"),
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

    /// Attributes for the title element.
    ///
    /// The `data-ars-heading-level` attribute is the clamped
    /// [`Props::title_level`] value; adapters render `<h{level}>` accordingly.
    #[must_use]
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();

        let level = self.props.title_level.clamp(1, 6);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("title"))
            .set(HtmlAttr::Data("ars-heading-level"), level.to_string());

        attrs
    }

    /// Attributes for the description element.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("description"));

        attrs
    }

    /// Attributes for the close trigger button.
    #[must_use]
    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();

        let label = (self.ctx.messages.close_label)(&self.ctx.locale);

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
            Part::Trigger => self.trigger_attrs(),
            Part::Backdrop => self.backdrop_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
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
            id: "dialog".to_string(),
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
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
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

    // Test 1
    #[test]
    fn init_default_open_false_starts_closed() {
        let service = fresh_service(test_props());

        assert_eq!(service.state(), &State::Closed);

        let ctx = service.context();

        assert!(!ctx.open);
        assert!(!ctx.has_title);
        assert!(!ctx.has_description);
        assert_eq!(ctx.ids.part("trigger"), "dialog-trigger");
        assert_eq!(ctx.ids.part("content"), "dialog-content");
        assert_eq!(ctx.ids.part("title"), "dialog-title");
        assert_eq!(ctx.ids.part("description"), "dialog-description");
    }

    // Test 2
    #[test]
    fn init_default_open_true_starts_open() {
        let service = fresh_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
    }

    // Test 3
    #[test]
    fn init_controlled_open_starts_open() {
        let service = fresh_service(Props {
            open: Some(true),
            default_open: false,
            ..test_props()
        });

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
    }

    // Test 4
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

    // Test 5
    #[test]
    fn init_role_alertdialog_preserved() {
        let service = fresh_service(Props {
            role: Role::AlertDialog,
            ..test_props()
        });

        assert_eq!(service.context().role, Role::AlertDialog);
    }

    // Test 6
    #[test]
    fn init_locale_and_messages_passed_through_env() {
        let env = Env::default();

        let messages = Messages {
            close_label: MessageFn::static_str("Cerrar"),
        };

        let service = Service::<Machine>::new(test_props(), &env, &messages);

        assert_eq!(service.context().locale, env.locale);
        assert_eq!(
            (service.context().messages.close_label)(&service.context().locale),
            "Cerrar"
        );
    }

    // Test 7
    #[test]
    fn event_open_transitions_closed_to_open() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Open);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
    }

    // Test 8
    #[test]
    fn event_close_transitions_open_to_closed() {
        let mut service = open_service(test_props());

        let result = service.send(Event::Close);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
    }

    // Test 9
    #[test]
    fn event_toggle_from_closed_opens() {
        let mut service = fresh_service(test_props());

        drop(service.send(Event::Toggle));

        assert_eq!(service.state(), &State::Open);
    }

    // Test 10
    #[test]
    fn event_toggle_from_open_closes() {
        let mut service = open_service(test_props());

        drop(service.send(Event::Toggle));

        assert_eq!(service.state(), &State::Closed);
    }

    // Test 11
    #[test]
    fn event_close_on_backdrop_click_closes_when_allowed() {
        let mut service = open_service(Props {
            close_on_backdrop: true,
            ..test_props()
        });

        drop(service.send(Event::CloseOnBackdropClick));

        assert_eq!(service.state(), &State::Closed);
    }

    // Test 12
    #[test]
    fn event_close_on_backdrop_click_no_op_when_disabled() {
        let mut service = open_service(Props {
            close_on_backdrop: false,
            ..test_props()
        });

        let result = service.send(Event::CloseOnBackdropClick);

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
    }

    // Test 13
    #[test]
    fn event_close_on_escape_closes_when_allowed() {
        let mut service = open_service(Props {
            close_on_escape: true,
            ..test_props()
        });

        drop(service.send(Event::CloseOnEscape));

        assert_eq!(service.state(), &State::Closed);
    }

    // Test 14
    #[test]
    fn event_close_on_escape_no_op_when_disabled() {
        let mut service = open_service(Props {
            close_on_escape: false,
            ..test_props()
        });

        let result = service.send(Event::CloseOnEscape);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Open);
    }

    // Test 15
    #[test]
    fn event_register_title_sets_has_title() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::RegisterTitle);

        assert!(result.context_changed);
        assert!(service.context().has_title);

        // Guarded: re-sending RegisterTitle when `has_title` is already
        // true is a no-op — no transition plan, no `context_changed` signal,
        // no spurious re-render trigger.
        let result_again = service.send(Event::RegisterTitle);

        assert!(service.context().has_title);
        assert!(!result_again.state_changed);
        assert!(!result_again.context_changed);
    }

    // Test 16
    #[test]
    fn event_register_description_sets_has_description() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::RegisterDescription);

        assert!(result.context_changed);
        assert!(service.context().has_description);

        // Guarded — see RegisterTitle.
        let result_again = service.send(Event::RegisterDescription);

        assert!(service.context().has_description);
        assert!(!result_again.state_changed);
        assert!(!result_again.context_changed);
    }

    // Test 17
    #[test]
    fn event_open_in_open_state_no_op() {
        let mut service = open_service(test_props());

        let result = service.send(Event::Open);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Open);
    }

    // Test 18
    #[test]
    fn event_close_in_closed_state_no_op() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Close);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Closed);
    }

    // Test 19
    #[test]
    fn open_emits_scroll_lock_intent_when_prevent_scroll() {
        let mut service = fresh_service(Props {
            prevent_scroll: true,
            ..test_props()
        });

        let result = service.send(Event::Open);

        assert!(effect_names(&result).contains(&Effect::ScrollLockAcquire));
    }

    // Test 20
    #[test]
    fn open_skips_scroll_lock_intent_when_prevent_scroll_false() {
        let mut service = fresh_service(Props {
            prevent_scroll: false,
            ..test_props()
        });

        let result = service.send(Event::Open);

        assert!(!effect_names(&result).contains(&Effect::ScrollLockAcquire));
    }

    // Test 21
    #[test]
    fn open_emits_set_background_inert_intent_when_modal() {
        let mut service = fresh_service(Props {
            modal: true,
            ..test_props()
        });

        let result = service.send(Event::Open);

        assert!(effect_names(&result).contains(&Effect::SetBackgroundInert));
    }

    // Test 22
    #[test]
    fn open_skips_set_background_inert_intent_when_non_modal() {
        let mut service = fresh_service(Props {
            modal: false,
            ..test_props()
        });

        let result = service.send(Event::Open);

        assert!(!effect_names(&result).contains(&Effect::SetBackgroundInert));
    }

    // Test 23
    #[test]
    fn open_emits_focus_initial_intent() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Open);

        let names = effect_names(&result);

        assert!(names.contains(&Effect::FocusInitial));
        assert!(names.contains(&Effect::FocusFirstTabbable));
    }

    // ── initial_effects override ─────────────────────────────────

    fn initial_effect_names(service: &mut Service<Machine>) -> Vec<Effect> {
        service
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect()
    }

    #[test]
    fn initial_effects_empty_when_default_open_false() {
        let mut service = fresh_service(test_props());

        assert!(service.take_initial_effects().is_empty());
        assert!(service.initial_effects_taken());
    }

    #[test]
    fn initial_effects_emit_full_open_lifecycle_when_default_open_true() {
        let mut service = open_service(test_props());

        let names = initial_effect_names(&mut service);

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::FocusInitial));
        assert!(names.contains(&Effect::FocusFirstTabbable));
    }

    #[test]
    fn initial_effects_include_scroll_lock_when_modal_and_prevent_scroll_true() {
        let mut service = open_service(Props {
            modal: true,
            prevent_scroll: true,
            ..test_props()
        });

        let names = initial_effect_names(&mut service);

        assert!(names.contains(&Effect::ScrollLockAcquire));
        assert!(names.contains(&Effect::SetBackgroundInert));
    }

    #[test]
    fn initial_effects_skip_scroll_lock_when_prevent_scroll_false() {
        let mut service = open_service(Props {
            prevent_scroll: false,
            ..test_props()
        });

        let names = initial_effect_names(&mut service);

        assert!(!names.contains(&Effect::ScrollLockAcquire));
    }

    #[test]
    fn initial_effects_skip_set_background_inert_when_non_modal() {
        let mut service = open_service(Props {
            modal: false,
            ..test_props()
        });

        let names = initial_effect_names(&mut service);

        assert!(!names.contains(&Effect::SetBackgroundInert));
    }

    #[test]
    fn initial_effects_drain_idempotently() {
        let mut service = open_service(test_props());

        assert!(!service.take_initial_effects().is_empty());

        // Subsequent calls observe an empty buffer.
        assert!(service.take_initial_effects().is_empty());
        assert!(service.initial_effects_taken());
    }

    // Test 24
    #[test]
    fn close_emits_release_scroll_lock_intent_when_prevent_scroll() {
        let mut service = open_service(Props {
            prevent_scroll: true,
            ..test_props()
        });

        let result = service.send(Event::Close);

        assert!(effect_names(&result).contains(&Effect::ScrollLockRelease));
    }

    // Test 25
    #[test]
    fn close_emits_remove_background_inert_intent_when_modal() {
        let mut service = open_service(Props {
            modal: true,
            ..test_props()
        });

        let result = service.send(Event::Close);

        assert!(effect_names(&result).contains(&Effect::RemoveBackgroundInert));
    }

    // Test 26
    #[test]
    fn close_emits_restore_focus_intent_when_restore_focus_true() {
        let mut service = open_service(Props {
            restore_focus: true,
            ..test_props()
        });

        let result = service.send(Event::Close);

        assert!(effect_names(&result).contains(&Effect::RestoreFocus));
    }

    // Test 27
    #[test]
    fn close_skips_restore_focus_intent_when_restore_focus_false() {
        let mut service = open_service(Props {
            restore_focus: false,
            ..test_props()
        });

        let result = service.send(Event::Close);

        assert!(!effect_names(&result).contains(&Effect::RestoreFocus));
    }

    // Test 27a — regression guard: every effect emitted by Open and Close is
    // one of the documented payload-free named intents from spec §1.11.
    #[test]
    fn effects_carry_no_id_payload() {
        const ALLOWED: &[Effect] = &[
            Effect::FocusInitial,
            Effect::FocusFirstTabbable,
            Effect::ScrollLockAcquire,
            Effect::ScrollLockRelease,
            Effect::SetBackgroundInert,
            Effect::RemoveBackgroundInert,
            Effect::RestoreFocus,
            Effect::OpenChange,
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

    // ── open-change adapter intent tests (Finding 1) ──────────────

    #[test]
    fn open_event_emits_open_change_intent() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Open);

        assert!(effect_names(&result).contains(&Effect::OpenChange));
    }

    #[test]
    fn close_event_emits_open_change_intent() {
        let mut service = open_service(test_props());

        let result = service.send(Event::Close);

        assert!(effect_names(&result).contains(&Effect::OpenChange));
    }

    #[test]
    fn toggle_event_emits_open_change_intent_in_either_direction() {
        let mut closed = fresh_service(test_props());

        assert!(effect_names(&closed.send(Event::Toggle)).contains(&Effect::OpenChange));

        let mut opened = open_service(test_props());

        assert!(effect_names(&opened.send(Event::Toggle)).contains(&Effect::OpenChange));
    }

    #[test]
    fn close_on_backdrop_emits_open_change_intent_when_allowed() {
        let mut service = open_service(test_props());

        let result = service.send(Event::CloseOnBackdropClick);

        assert!(effect_names(&result).contains(&Effect::OpenChange));
    }

    #[test]
    fn close_on_escape_emits_open_change_intent_when_allowed() {
        let mut service = open_service(test_props());

        let result = service.send(Event::CloseOnEscape);

        assert!(effect_names(&result).contains(&Effect::OpenChange));
    }

    #[test]
    fn close_on_backdrop_skips_open_change_intent_when_guarded() {
        let mut service = open_service(Props {
            close_on_backdrop: false,
            ..test_props()
        });

        let result = service.send(Event::CloseOnBackdropClick);

        assert!(!effect_names(&result).contains(&Effect::OpenChange));
    }

    #[test]
    fn close_on_escape_skips_open_change_intent_when_guarded() {
        let mut service = open_service(Props {
            close_on_escape: false,
            ..test_props()
        });

        let result = service.send(Event::CloseOnEscape);

        assert!(!effect_names(&result).contains(&Effect::OpenChange));
    }

    #[test]
    fn register_title_does_not_emit_open_change_intent() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::RegisterTitle);

        assert!(!effect_names(&result).contains(&Effect::OpenChange));
    }

    #[test]
    fn register_description_does_not_emit_open_change_intent() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::RegisterDescription);

        assert!(!effect_names(&result).contains(&Effect::OpenChange));
    }

    #[test]
    fn no_op_open_in_open_state_does_not_emit_open_change_intent() {
        let mut service = open_service(test_props());

        let result = service.send(Event::Open);

        assert!(!effect_names(&result).contains(&Effect::OpenChange));
    }

    #[test]
    fn no_op_close_in_closed_state_does_not_emit_open_change_intent() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Close);

        assert!(!effect_names(&result).contains(&Effect::OpenChange));
    }

    // ── Api event-handler dispatch tests ──────────────────────────

    fn drain_to_vec(events: &Rc<RefCell<Vec<Event>>>) -> Vec<Event> {
        events.borrow().clone()
    }

    // Test 28
    #[test]
    fn on_trigger_click_sends_toggle() {
        let service = fresh_service(test_props());

        let captured = Rc::new(RefCell::new(Vec::new()));

        let captured_for_send = Rc::clone(&captured);

        let send = move |event: Event| captured_for_send.borrow_mut().push(event);

        service.connect(&send).on_trigger_click();

        assert_eq!(drain_to_vec(&captured), [Event::Toggle]);
    }

    // Test 29
    #[test]
    fn on_backdrop_click_sends_close_on_backdrop_click() {
        let service = open_service(test_props());

        let captured = Rc::new(RefCell::new(Vec::new()));

        let captured_for_send = Rc::clone(&captured);

        let send = move |event: Event| captured_for_send.borrow_mut().push(event);

        service.connect(&send).on_backdrop_click();

        assert_eq!(drain_to_vec(&captured), [Event::CloseOnBackdropClick]);
    }

    // Test 30
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

    // Test 31
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

    // Test 32
    #[test]
    fn on_close_trigger_click_sends_close() {
        let service = open_service(test_props());

        let captured = Rc::new(RefCell::new(Vec::new()));

        let captured_for_send = Rc::clone(&captured);

        let send = move |event: Event| captured_for_send.borrow_mut().push(event);

        service.connect(&send).on_close_trigger_click();

        assert_eq!(drain_to_vec(&captured), [Event::Close]);
    }

    // Test 33
    #[test]
    fn is_open_returns_true_when_state_open() {
        let service = open_service(test_props());

        assert!(service.connect(&|_| {}).is_open());
    }

    // Test 34
    #[test]
    fn is_open_returns_false_when_state_closed() {
        let service = fresh_service(test_props());

        assert!(!service.connect(&|_| {}).is_open());
    }

    // ── DismissAttempt integration tests ───────────────────────────

    // Test 35
    #[test]
    fn dismiss_attempt_starts_not_prevented() {
        let attempt = DismissAttempt::new(());

        assert!(!attempt.is_prevented());
    }

    // Test 36
    #[test]
    fn dismiss_attempt_prevent_dismiss_marks_prevented() {
        let attempt = DismissAttempt::new(());

        attempt.prevent_dismiss();

        assert!(attempt.is_prevented());
    }

    // Test 37
    #[test]
    fn dismiss_attempt_repeated_prevent_idempotent_and_clones_share_veto() {
        let attempt = DismissAttempt::new(());

        attempt.prevent_dismiss();
        attempt.prevent_dismiss();

        assert!(attempt.is_prevented());

        // Cloned views observe the shared veto.
        let cloned = attempt.clone();

        assert!(cloned.is_prevented());
    }

    // ── on_props_changed tests ────────────────────────────────────

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
    fn on_props_changed_emits_nothing_when_open_unchanged() {
        let old = Props {
            open: Some(true),
            ..test_props()
        };

        assert!(Machine::on_props_changed(&old, &old).is_empty());
    }

    #[test]
    fn on_props_changed_ignores_uncontrolled_to_uncontrolled() {
        let old = test_props();

        let new = Props {
            default_open: true,
            ..test_props()
        };

        assert!(Machine::on_props_changed(&old, &new).is_empty());
    }

    // ── Role helper ───────────────────────────────────────────────

    #[test]
    fn props_debug_redacts_callback_closures() {
        let props = Props::new()
            .id("dbg")
            .on_open_change(|_| {})
            .on_escape_key_down(|_| {})
            .on_interact_outside(|_| {});

        let rendered = format!("{props:?}");

        assert!(rendered.contains("Props"));

        // `Callback`'s `Debug` impl writes `Callback(..)` rather than the
        // closure pointer, so consumers never see opaque function addresses
        // in error messages or logs.
        assert!(rendered.contains("Callback(..)"));
    }

    #[test]
    fn api_debug_renders_without_send_field() {
        let service = fresh_service(test_props());

        let send: &dyn Fn(Event) = &|_| {};

        let api = service.connect(&send);

        let rendered = format!("{api:?}");

        assert!(rendered.contains("Api"));

        // The `send` closure is intentionally omitted from Debug output.
        assert!(!rendered.contains("send:"));
    }

    #[test]
    fn connect_api_part_attrs_match_direct_methods() {
        use ars_core::ConnectApi as _;

        let mut service = open_service(test_props());

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Trigger), api.trigger_attrs());
        assert_eq!(api.part_attrs(Part::Backdrop), api.backdrop_attrs());
        assert_eq!(api.part_attrs(Part::Positioner), api.positioner_attrs());
        assert_eq!(api.part_attrs(Part::Content), api.content_attrs());
        assert_eq!(api.part_attrs(Part::Title), api.title_attrs());
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::CloseTrigger),
            api.close_trigger_attrs()
        );
    }

    #[test]
    fn role_as_aria_role_returns_dialog_for_dialog_variant() {
        assert_eq!(Role::Dialog.as_aria_role(), "dialog");
    }

    #[test]
    fn role_as_aria_role_returns_alertdialog_for_alertdialog_variant() {
        assert_eq!(Role::AlertDialog.as_aria_role(), "alertdialog");
    }

    // ── Props builder coverage ────────────────────────────────────

    #[test]
    fn props_new_returns_default() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new()
            .id("dialog-builder")
            .open(Some(true))
            .default_open(true)
            .modal(false)
            .close_on_backdrop(false)
            .close_on_escape(false)
            .prevent_scroll(false)
            .restore_focus(false)
            .initial_focus(Some(FocusTarget::First))
            .final_focus(Some(FocusTarget::Last))
            .role(Role::AlertDialog)
            .title_level(4)
            .lazy_mount(true)
            .unmount_on_exit(true)
            .on_open_change(|_| {})
            .on_escape_key_down(|_| {})
            .on_interact_outside(|_| {});

        assert_eq!(props.id, "dialog-builder");
        assert_eq!(props.open, Some(true));
        assert!(props.default_open);
        assert!(!props.modal);
        assert!(!props.close_on_backdrop);
        assert!(!props.close_on_escape);
        assert!(!props.prevent_scroll);
        assert!(!props.restore_focus);
        assert_eq!(props.initial_focus, Some(FocusTarget::First));
        assert_eq!(props.final_focus, Some(FocusTarget::Last));
        assert_eq!(props.role, Role::AlertDialog);
        assert_eq!(props.title_level, 4);
        assert!(props.lazy_mount);
        assert!(props.unmount_on_exit);
        assert!(props.on_open_change.is_some());
        assert!(props.on_escape_key_down.is_some());
        assert!(props.on_interact_outside.is_some());
    }

    // ── Snapshot tests (38–59) ────────────────────────────────────

    fn snapshot_service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_service_with_messages(props: Props, messages: &Messages) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), messages)
    }

    // 38
    #[test]
    fn snapshot_root_closed() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "dialog_root_closed",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    // 39
    #[test]
    fn snapshot_root_open() {
        let service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_root_open",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    // 40
    #[test]
    fn snapshot_trigger_closed() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "dialog_trigger_closed",
            snapshot_attrs(&service.connect(&|_| {}).trigger_attrs())
        );
    }

    // 41
    #[test]
    fn snapshot_trigger_open() {
        let service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_trigger_open",
            snapshot_attrs(&service.connect(&|_| {}).trigger_attrs())
        );
    }

    // 42
    #[test]
    fn snapshot_backdrop_closed() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "dialog_backdrop_closed",
            snapshot_attrs(&service.connect(&|_| {}).backdrop_attrs())
        );
    }

    // 43
    #[test]
    fn snapshot_backdrop_open() {
        let service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_backdrop_open",
            snapshot_attrs(&service.connect(&|_| {}).backdrop_attrs())
        );
    }

    // 44
    #[test]
    fn snapshot_positioner() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "dialog_positioner",
            snapshot_attrs(&service.connect(&|_| {}).positioner_attrs())
        );
    }

    // 45
    #[test]
    fn snapshot_content_open_modal_dialog() {
        let service = snapshot_service(Props {
            default_open: true,
            modal: true,
            role: Role::Dialog,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_content_open_modal_dialog",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    // 46
    #[test]
    fn snapshot_content_open_modal_alertdialog() {
        let service = snapshot_service(Props {
            default_open: true,
            modal: true,
            role: Role::AlertDialog,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_content_open_modal_alertdialog",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    // 47
    #[test]
    fn snapshot_content_open_non_modal() {
        let service = snapshot_service(Props {
            default_open: true,
            modal: false,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_content_open_non_modal",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    // 48
    #[test]
    fn snapshot_content_open_with_title() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::RegisterTitle));

        assert_snapshot!(
            "dialog_content_open_with_title",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    // 49
    #[test]
    fn snapshot_content_open_with_description() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "dialog_content_open_with_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    // 50
    #[test]
    fn snapshot_content_open_with_title_and_description() {
        let mut service = snapshot_service(Props {
            default_open: true,
            ..test_props()
        });

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "dialog_content_open_with_title_and_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    // 51
    #[test]
    fn snapshot_content_closed() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "dialog_content_closed",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    // 52
    #[test]
    fn snapshot_title_default_h2() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "dialog_title_default_h2",
            snapshot_attrs(&service.connect(&|_| {}).title_attrs())
        );
    }

    // 53 — title_level=1 is exercised by `title_clamped_below_one`, which
    // verifies the clamp path produces the same `data-ars-heading-level=1`
    // attribute. Snapshot is omitted to stay under the per-component
    // snapshot-count cap; the assertion below proves the structural value.
    #[test]
    fn title_level_one_renders_data_ars_heading_level_1() {
        let service = snapshot_service(Props {
            title_level: 1,
            ..test_props()
        });

        let attrs = service.connect(&|_| {}).title_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-heading-level")), Some("1"));
    }

    // 54
    #[test]
    fn snapshot_title_clamped_above_six() {
        let service = snapshot_service(Props {
            title_level: 10,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_title_clamped_above_six",
            snapshot_attrs(&service.connect(&|_| {}).title_attrs())
        );
    }

    // 55
    #[test]
    fn snapshot_title_clamped_below_one() {
        let service = snapshot_service(Props {
            title_level: 0,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_title_clamped_below_one",
            snapshot_attrs(&service.connect(&|_| {}).title_attrs())
        );
    }

    // 56
    #[test]
    fn snapshot_description() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "dialog_description",
            snapshot_attrs(&service.connect(&|_| {}).description_attrs())
        );
    }

    // 57
    #[test]
    fn snapshot_close_trigger_default_label() {
        let service = snapshot_service(test_props());

        assert_snapshot!(
            "dialog_close_trigger_default_label",
            snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs())
        );
    }

    // 58
    #[test]
    fn snapshot_close_trigger_custom_messages_label() {
        let messages = Messages {
            close_label: MessageFn::static_str("Cerrar diálogo"),
        };

        let service = snapshot_service_with_messages(test_props(), &messages);

        assert_snapshot!(
            "dialog_close_trigger_custom_messages_label",
            snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs())
        );
    }

    // 59 — locale-aware label resolution. The MessageFn closure inspects
    // the active locale; we exercise BOTH branches of the same closure
    // (shared via the `Arc<dyn Fn>` Messages clones) so the agnostic core's
    // closure-form `MessageFn` path is fully covered. Structural
    // verification of the close trigger AttrMap is already covered by
    // `snapshot_close_trigger_default_label` and
    // `snapshot_close_trigger_custom_messages_label`; this test uses
    // direct assertions to stay under the per-component snapshot cap.
    #[test]
    fn close_trigger_resolves_locale_aware_label_for_both_branches() {
        use ars_core::Locale;

        let messages = Messages {
            close_label: MessageFn::new(|locale: &Locale| {
                if locale.to_bcp47().starts_with("es") {
                    "Cerrar".to_string()
                } else {
                    "Close".to_string()
                }
            }),
        };

        // English / undefined locale → "Close" branch.
        let en_service = Service::<Machine>::new(test_props(), &Env::default(), &messages);

        let en_attrs = en_service.connect(&|_| {}).close_trigger_attrs();

        assert_eq!(
            en_attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Close")
        );

        // Spanish locale → "Cerrar" branch (same closure, different
        // input locale → both paths now executed under coverage).
        let es_env = Env {
            locale: Locale::parse("es").expect("`es` is a valid BCP-47 tag"),
            ..Env::default()
        };

        let es_service = Service::<Machine>::new(test_props(), &es_env, &messages);

        let es_attrs = es_service.connect(&|_| {}).close_trigger_attrs();
        assert_eq!(
            es_attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Cerrar")
        );
    }

    // ── Additional snapshot coverage (within the 40-snapshot budget) ──

    #[test]
    fn snapshot_root_open_modal_alertdialog() {
        let service = snapshot_service(Props {
            default_open: true,
            modal: true,
            role: Role::AlertDialog,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_root_open_modal_alertdialog",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_trigger_open_with_alertdialog_role() {
        // The trigger emits `aria-haspopup="dialog"` regardless of the
        // content role, but `aria-controls` still points at the
        // alertdialog content id when open. This locks down that
        // behaviour as part of the trigger contract.
        let service = snapshot_service(Props {
            default_open: true,
            modal: true,
            role: Role::AlertDialog,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_trigger_open_with_alertdialog_role",
            snapshot_attrs(&service.connect(&|_| {}).trigger_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_alertdialog_with_title() {
        let mut service = snapshot_service(Props {
            default_open: true,
            modal: true,
            role: Role::AlertDialog,
            ..test_props()
        });

        drop(service.send(Event::RegisterTitle));

        assert_snapshot!(
            "dialog_content_open_alertdialog_with_title",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_alertdialog_with_description() {
        let mut service = snapshot_service(Props {
            default_open: true,
            modal: true,
            role: Role::AlertDialog,
            ..test_props()
        });

        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "dialog_content_open_alertdialog_with_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_non_modal_with_title() {
        // Non-modal dialog keeps `role="dialog"` but omits
        // `aria-modal="true"`; the title still wires through
        // `aria-labelledby`. Verifies the modal flag and the title flag
        // compose independently.
        let mut service = snapshot_service(Props {
            default_open: true,
            modal: false,
            ..test_props()
        });

        drop(service.send(Event::RegisterTitle));

        assert_snapshot!(
            "dialog_content_open_non_modal_with_title",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_open_non_modal_with_title_and_description() {
        let mut service = snapshot_service(Props {
            default_open: true,
            modal: false,
            ..test_props()
        });

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "dialog_content_open_non_modal_with_title_and_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_title_h1_level() {
        // Heading-level clamping is already covered for the `> 6` and
        // `< 1` branches; this snapshot pins the in-range edge (level=1)
        // so reviewers can see what the rendered `data-ars-heading-level`
        // attribute looks like at the lowest legal value.
        let service = snapshot_service(Props {
            title_level: 1,
            ..test_props()
        });

        assert_snapshot!(
            "dialog_title_h1_level",
            snapshot_attrs(&service.connect(&|_| {}).title_attrs())
        );
    }

    #[test]
    fn snapshot_close_trigger_localized_es() {
        // Locale-aware close-trigger label: the same MessageFn closure
        // (shared between English and Spanish locales) exercises the
        // Spanish branch. The structural shape matches
        // `dialog_close_trigger_default_label` but the aria-label value
        // differs — pins the locale-aware path under snapshot review.
        use ars_core::Locale;

        let messages = Messages {
            close_label: MessageFn::new(|locale: &Locale| {
                if locale.to_bcp47().starts_with("es") {
                    "Cerrar".to_string()
                } else {
                    "Close".to_string()
                }
            }),
        };

        let es_env = Env {
            locale: Locale::parse("es").expect("`es` is a valid BCP-47 tag"),
            ..Env::default()
        };

        let service = Service::<Machine>::new(test_props(), &es_env, &messages);

        assert_snapshot!(
            "dialog_close_trigger_localized_es",
            snapshot_attrs(&service.connect(&|_| {}).close_trigger_attrs())
        );
    }

    // ── Additional regression / contract tests ─────────────────────

    // B — on_props_changed coverage gaps

    #[test]
    fn on_props_changed_emits_close_when_uncontrolled_to_some_false() {
        let old = test_props();

        let new = Props {
            open: Some(false),
            ..test_props()
        };

        assert_eq!(Machine::on_props_changed(&old, &new), [Event::Close]);
    }

    #[test]
    fn on_props_changed_emits_nothing_when_some_true_to_none() {
        let old = Props {
            open: Some(true),
            ..test_props()
        };

        let new = test_props();

        assert!(Machine::on_props_changed(&old, &new).is_empty());
    }

    #[test]
    fn on_props_changed_emits_nothing_when_some_false_to_none() {
        let old = Props {
            open: Some(false),
            ..test_props()
        };

        let new = test_props();

        assert!(Machine::on_props_changed(&old, &new).is_empty());
    }

    // C — DismissAttempt veto symmetry

    #[test]
    fn dismiss_attempt_veto_set_on_clone_observable_on_original() {
        let original = DismissAttempt::new(());

        let cloned = original.clone();

        cloned.prevent_dismiss();

        assert!(original.is_prevented());
    }

    // D — Default trio

    #[test]
    fn state_default_returns_closed() {
        assert_eq!(State::default(), State::Closed);
    }

    #[test]
    fn role_default_returns_dialog() {
        assert_eq!(Role::default(), Role::Dialog);
    }

    // E — adapter-only prop accessors

    #[test]
    fn api_exposes_lazy_mount_and_unmount_on_exit_for_adapter() {
        let service = fresh_service(Props {
            lazy_mount: true,
            unmount_on_exit: true,
            ..test_props()
        });

        let api = service.connect(&|_| {});

        assert!(api.lazy_mount());
        assert!(api.unmount_on_exit());
    }

    #[test]
    fn api_lazy_mount_and_unmount_on_exit_default_false() {
        let service = fresh_service(test_props());

        let api = service.connect(&|_| {});

        assert!(!api.lazy_mount());
        assert!(!api.unmount_on_exit());
    }

    // F — Send + Sync compile-time assertion

    #[test]
    fn dismiss_attempt_is_send_sync() {
        const fn assert_send_sync<T: Send + Sync>() {}

        // Will fail to compile if `DismissAttempt` ever loses `Send` or
        // `Sync` (e.g., if `Arc<AtomicBool>` is replaced with `Cell<bool>`).
        assert_send_sync::<DismissAttempt<()>>();
    }

    // G — controlled-mode round-trip via Service::set_props

    #[test]
    fn controlled_set_props_round_trip_flips_state_and_emits_open_change() {
        let mut service = fresh_service(test_props());

        assert_eq!(service.state(), &State::Closed);

        let result = service.set_props(Props {
            open: Some(true),
            ..test_props()
        });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Open);
        assert!(effect_names(&result).contains(&Effect::OpenChange));

        let result = service.set_props(Props {
            open: Some(false),
            ..test_props()
        });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Closed);
        assert!(effect_names(&result).contains(&Effect::OpenChange));
    }

    // I — on_open_change registration round-trip

    // ── Effect ORDER assertions (Finding 2) ────────────────────────

    #[test]
    fn open_transition_effects_emitted_in_documented_order() {
        // Adapter expectations: scroll-lock SHOULD acquire before focus
        // moves so DOM measurements stabilise; open-change fires first so
        // observers can react before background mutations land. Pin the
        // canonical order from §1.9 for the all-flags-enabled case.
        let mut service = fresh_service(Props {
            modal: true,
            prevent_scroll: true,
            ..test_props()
        });

        let result = service.send(Event::Open);

        assert_eq!(
            effect_names(&result),
            vec![
                Effect::OpenChange,
                Effect::FocusInitial,
                Effect::FocusFirstTabbable,
                Effect::ScrollLockAcquire,
                Effect::SetBackgroundInert,
            ]
        );
    }

    #[test]
    fn close_transition_effects_emitted_in_documented_order() {
        let mut service = open_service(Props {
            modal: true,
            prevent_scroll: true,
            restore_focus: true,
            ..test_props()
        });

        let result = service.send(Event::Close);

        assert_eq!(
            effect_names(&result),
            vec![
                Effect::OpenChange,
                Effect::ScrollLockRelease,
                Effect::RemoveBackgroundInert,
                Effect::RestoreFocus,
            ]
        );
    }

    // ── Multi-event sequence (Finding 3) ───────────────────────────

    #[test]
    fn open_close_open_cycle_preserves_state_machine_invariants() {
        let mut service = fresh_service(test_props());

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);

        // 1. Open
        let r1 = service.send(Event::Open);

        assert!(r1.state_changed);
        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
        assert!(effect_names(&r1).contains(&Effect::OpenChange));

        // 2. Close via backdrop click
        let r2 = service.send(Event::CloseOnBackdropClick);

        assert!(r2.state_changed);
        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert!(effect_names(&r2).contains(&Effect::OpenChange));

        // 3. Re-open via Toggle
        let r3 = service.send(Event::Toggle);

        assert!(r3.state_changed);
        assert_eq!(service.state(), &State::Open);
        assert!(effect_names(&r3).contains(&Effect::OpenChange));

        // 4. Register title and description while open — both flips happen
        let r4 = service.send(Event::RegisterTitle);

        let r5 = service.send(Event::RegisterDescription);

        assert!(r4.context_changed);
        assert!(r5.context_changed);
        assert!(service.context().has_title);
        assert!(service.context().has_description);

        // 5. Close via Escape
        let r6 = service.send(Event::CloseOnEscape);

        assert!(r6.state_changed);
        assert_eq!(service.state(), &State::Closed);

        // has_title / has_description survive the close (they're
        // monotonically non-decreasing across the lifecycle).
        assert!(service.context().has_title);
        assert!(service.context().has_description);
    }

    // ── Toggle ≡ Open/Close equivalence (Finding 4) ────────────────

    #[test]
    fn toggle_from_closed_emits_same_effects_as_open() {
        // Two services with identical config; one driven by Open, the
        // other by Toggle. The emitted effect names MUST match.
        let mut by_open = fresh_service(test_props());
        let mut by_toggle = fresh_service(test_props());

        let open_effects = effect_names(&by_open.send(Event::Open));
        let toggle_effects = effect_names(&by_toggle.send(Event::Toggle));

        assert_eq!(open_effects, toggle_effects);
        assert_eq!(by_open.state(), by_toggle.state());
        assert_eq!(by_open.context().open, by_toggle.context().open);
    }

    #[test]
    fn toggle_from_open_emits_same_effects_as_close() {
        let mut by_close = open_service(test_props());
        let mut by_toggle = open_service(test_props());

        let close_effects = effect_names(&by_close.send(Event::Close));
        let toggle_effects = effect_names(&by_toggle.send(Event::Toggle));

        assert_eq!(close_effects, toggle_effects);
        assert_eq!(by_close.state(), by_toggle.state());
        assert_eq!(by_close.context().open, by_toggle.context().open);
    }

    // ── Catch-all `_ => None` direct assertion (Finding 5) ─────────

    #[test]
    fn transition_returns_none_for_register_title_when_already_registered() {
        let mut ctx = Context {
            open: false,
            modal: true,
            close_on_backdrop: true,
            close_on_escape: true,
            prevent_scroll: true,
            restore_focus: true,
            initial_focus: None,
            final_focus: None,
            role: Role::Dialog,
            ids: ComponentIds::from_id("x"),
            has_title: true,
            has_description: false,
            locale: Env::default().locale,
            messages: Messages::default(),
        };

        let props = test_props();

        let plan = Machine::transition(&State::Closed, &Event::RegisterTitle, &ctx, &props);

        assert!(
            plan.is_none(),
            "RegisterTitle on has_title=true must route to the catch-all `_ => None` arm"
        );

        ctx.has_title = false;
        ctx.has_description = true;

        let plan = Machine::transition(&State::Closed, &Event::RegisterDescription, &ctx, &props);

        assert!(
            plan.is_none(),
            "RegisterDescription on has_description=true must route to the catch-all"
        );
    }

    // K — SyncProps event runtime context-backed prop sync

    #[test]
    fn event_sync_props_replays_context_backed_fields() {
        let mut service = open_service(test_props());

        // Sanity: defaults set modal=true, close_on_backdrop=true, etc.
        assert!(service.context().modal);
        assert!(service.context().close_on_backdrop);
        assert!(service.context().close_on_escape);
        assert!(service.context().prevent_scroll);
        assert!(service.context().restore_focus);
        assert_eq!(service.context().role, Role::Dialog);

        // Replace props with non-default values for every context-backed
        // field, then send SyncProps directly (without going through
        // Service::set_props, which is exercised separately below).
        *service.props_mut() = Props {
            id: "dialog".to_string(),
            modal: false,
            close_on_backdrop: false,
            close_on_escape: false,
            prevent_scroll: false,
            restore_focus: false,
            initial_focus: Some(FocusTarget::First),
            final_focus: Some(FocusTarget::Last),
            role: Role::AlertDialog,
            default_open: true,
            ..Props::default()
        };

        let result = service.send(Event::SyncProps);

        assert!(!result.state_changed);
        assert!(result.context_changed);

        let ctx = service.context();

        assert!(!ctx.modal);
        assert!(!ctx.close_on_backdrop);
        assert!(!ctx.close_on_escape);
        assert!(!ctx.prevent_scroll);
        assert!(!ctx.restore_focus);
        assert_eq!(ctx.initial_focus, Some(FocusTarget::First));
        assert_eq!(ctx.final_focus, Some(FocusTarget::Last));
        assert_eq!(ctx.role, Role::AlertDialog);
    }

    #[test]
    fn event_sync_props_emits_no_adapter_intents() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::SyncProps);

        // SyncProps is a context-only update; it MUST NOT emit any
        // adapter intent. No focus moves, no scroll lock changes, no
        // open-change signal.
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn on_props_changed_emits_sync_props_when_modal_flips() {
        let old = test_props();

        let new = Props {
            modal: false,
            ..test_props()
        };

        assert_eq!(Machine::on_props_changed(&old, &new), [Event::SyncProps]);
    }

    #[test]
    fn on_props_changed_emits_sync_props_when_role_flips() {
        let old = test_props();

        let new = Props {
            role: Role::AlertDialog,
            ..test_props()
        };

        assert_eq!(Machine::on_props_changed(&old, &new), [Event::SyncProps]);
    }

    #[test]
    fn on_props_changed_emits_sync_props_when_close_policy_changes() {
        for new in [
            Props {
                close_on_backdrop: false,
                ..test_props()
            },
            Props {
                close_on_escape: false,
                ..test_props()
            },
            Props {
                prevent_scroll: false,
                ..test_props()
            },
            Props {
                restore_focus: false,
                ..test_props()
            },
        ] {
            assert_eq!(
                Machine::on_props_changed(&test_props(), &new),
                [Event::SyncProps],
                "expected SyncProps for prop change to {new:?}"
            );
        }
    }

    #[test]
    fn on_props_changed_emits_sync_props_when_focus_targets_change() {
        let new = Props {
            initial_focus: Some(FocusTarget::First),
            final_focus: Some(FocusTarget::Last),
            ..test_props()
        };

        assert_eq!(
            Machine::on_props_changed(&test_props(), &new),
            [Event::SyncProps]
        );
    }

    #[test]
    fn on_props_changed_emits_open_and_sync_props_together_when_both_change() {
        let old = test_props();

        let new = Props {
            open: Some(true),
            modal: false,
            ..test_props()
        };

        assert_eq!(
            Machine::on_props_changed(&old, &new),
            [Event::Open, Event::SyncProps]
        );
    }

    #[test]
    fn on_props_changed_emits_nothing_when_only_id_changes() {
        let old = test_props();

        let new = Props {
            id: "different-dialog".to_string(),
            ..test_props()
        };

        // `id` is not a context-backed field — it only feeds
        // `ComponentIds::from_id` at init time. Changing it is meaningless
        // post-init and must not produce events.
        assert!(Machine::on_props_changed(&old, &new).is_empty());
    }

    #[test]
    fn controlled_modal_flip_via_set_props_propagates_to_context() {
        // End-to-end: `Service::set_props` flips `modal`. The next close
        // transition must read the freshly-synced context (`ctx.modal ==
        // false`) and skip the `dialog-remove-background-inert` intent.
        let mut service = open_service(test_props());

        drop(service.set_props(Props {
            default_open: true,
            modal: false,
            ..test_props()
        }));

        assert!(!service.context().modal);

        let close_result = service.send(Event::Close);

        assert!(
            !effect_names(&close_result).contains(&Effect::RemoveBackgroundInert),
            "non-modal dialog must not request inert removal"
        );
    }

    // L — Api::is_modal() / Api::role() accessors

    #[test]
    fn api_is_modal_reflects_ctx_modal() {
        let modal_service = fresh_service(Props {
            modal: true,
            ..test_props()
        });

        assert!(modal_service.connect(&|_| {}).is_modal());

        let non_modal_service = fresh_service(Props {
            modal: false,
            ..test_props()
        });

        assert!(!non_modal_service.connect(&|_| {}).is_modal());
    }

    #[test]
    fn api_role_reflects_ctx_role() {
        let dialog_service = fresh_service(Props {
            role: Role::Dialog,
            ..test_props()
        });

        assert_eq!(dialog_service.connect(&|_| {}).role(), Role::Dialog);

        let alertdialog_service = fresh_service(Props {
            role: Role::AlertDialog,
            ..test_props()
        });

        assert_eq!(
            alertdialog_service.connect(&|_| {}).role(),
            Role::AlertDialog
        );
    }

    // M — backdrop_attrs invariance regression guard

    #[test]
    fn backdrop_attrs_invariant_across_modal_and_non_modal() {
        // The backdrop is decorative when rendered. Whether the dialog is
        // modal or non-modal, the backdrop's AttrMap output MUST be
        // identical — adapters skip rendering the backdrop entirely for
        // non-modal dialogs, but the agnostic-core surface stays uniform.
        let modal = open_service(Props {
            modal: true,
            ..test_props()
        });

        let non_modal = open_service(Props {
            modal: false,
            ..test_props()
        });

        assert_eq!(
            modal.connect(&|_| {}).backdrop_attrs(),
            non_modal.connect(&|_| {}).backdrop_attrs()
        );
    }

    #[test]
    fn on_open_change_callback_registered_via_builder_is_invokable() {
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicBool, Ordering};

        let observed = Arc::new(AtomicBool::new(false));

        let observed_for_callback = Arc::clone(&observed);

        let props = Props::new().id("dialog").on_open_change(move |open| {
            observed_for_callback.store(open, Ordering::SeqCst);
        });

        // The agnostic core stores the callback in props; an adapter handling
        // `Effect::OpenChange` would invoke `props.on_open_change` with the
        // post-transition state. We exercise that end-to-end here.
        let cb = props
            .on_open_change
            .as_ref()
            .expect("callback registered via builder");

        cb(true);

        assert!(observed.load(Ordering::SeqCst));

        cb(false);

        assert!(!observed.load(Ordering::SeqCst));
    }
}
