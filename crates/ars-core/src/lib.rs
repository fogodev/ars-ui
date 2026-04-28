//! Framework-agnostic state machine engine for UI components.
//!
//! This crate provides the foundational finite state machine runtime that powers all
//! ars-ui components. Component logic is defined as pure [`Machine`] implementations
//! with typed states, events, and context — then run via [`Service`] which manages
//! transitions and side effects.
//!
//! Key abstractions:
//! - [`Machine`] — trait defining a component's FSM (states, events, transitions)
//! - [`Service`] — running machine instance that applies transitions and collects effects
//! - [`ConnectApi`] — bridges machine state to DOM attributes via [`ComponentPart`]
//! - [`Bindable`] — controlled/uncontrolled value pattern for two-way binding
//! - [`BindableValue`] — marker trait for values usable with [`Bindable`]
//! - [`TransitionPlan`] — declarative transition result with closures, effects, and follow-ups
//! - [`PendingEffect`] — named side effect with setup closure and cleanup lifecycle
//! - [`Callback`] — shared callback wrapper built on [`Arc`]
//! - [`ComponentIds`] — adapter-provided base IDs expanded into stable part IDs
//! - [`ComponentError`] — standardized recoverable component misuse errors
//! - [`SafeUrl`] — validated URL type for URL-valued HTML attributes
//! - [`SharedState`] — shared interior-mutable state (`Rc<RefCell>` on wasm, `Arc<Mutex>` on native)
//! - [`WeakSend`] — weak event sender for safe effect cleanup
//! - [`SendResult`] — structured result from [`Service::send`]

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;
extern crate self as ars_core;

use alloc::{boxed::Box, collections::VecDeque, string::String, sync::Arc, vec::Vec};
use core::fmt::{self, Debug};

mod callback;
pub mod companion_css;
mod component_ids;
mod connect;
mod error;
mod i18n_registry;
mod message_fn;
pub mod modality;
pub mod platform;
pub mod provider;
mod shared_flag;
mod shared_state;
mod url;
mod weak_send;

/// Hidden re-exports used by proc macros to stay hygienic without forcing
/// downstream crates to import `alloc`.
#[doc(hidden)]
pub mod __private {
    pub use alloc::{string::String, vec::Vec};
}

// ── Derive macros ───────────────────────────────────────────────────
#[doc(inline)]
pub use ars_derive::{ComponentPart, HasId};
// ── External re-exports ─────────────────────────────────────────────
pub use ars_i18n::{
    Direction, IntlBackend, IsolateDirection, Locale, LocaleParseError, Orientation,
    ResolvedDirection, StubIntlBackend, Weekday, isolate_text_safe,
};
// ── Platform-conditional smart pointers (extracted modules) ─────────
pub use callback::{Callback, callback};
pub use component_ids::ComponentIds;
// ── DOM attribute / connect primitives ──────────────────────────────
pub use connect::{
    AriaAttr, AttrMap, AttrMapParts, AttrValue, CssProperty, EventOptions, HtmlAttr, HtmlEvent,
    ReactiveBoolFn, ReactiveStringFn, StyleStrategy, UserAttrs, data, escape_css_attribute_value,
    styles_to_nonce_css,
};
pub use error::ComponentError;
pub use i18n_registry::{I18nRegistries, MessagesRegistry, resolve_messages};
pub use message_fn::{ComponentMessages, MessageFn};
// ── Modality ────────────────────────────────────────────────────────
pub use modality::{
    DefaultModalityContext, KeyModifiers, KeyboardKey, ModalityContext, ModalitySnapshot,
    NullModalityContext, PointerType,
};
// ── Platform effects ────────────────────────────────────────────────
pub use platform::{
    MissingProviderEffects, NullPlatformEffects, PlatformEffects, Rect, TimerHandle,
};
// ── Provider ────────────────────────────────────────────────────────
pub use provider::{ArsContext, ColorMode};
pub use shared_flag::SharedFlag;
pub use shared_state::SharedState;
pub use url::{SafeUrl, UnsafeUrlError, is_safe_url, sanitize_url};
pub use weak_send::{StrongSend, WeakSend};

// ════════════════════════════════════════════════════════════════════
// Inline types — kept in lib.rs because the derive macros expand to
// `::ars_core::HasId` and `::ars_core::ComponentPart` which must
// resolve to the *trait* at crate root (Rust disambiguates derive
// macros from traits by usage context only when both are defined in
// the same scope).
// ════════════════════════════════════════════════════════════════════

// ────────────────────────────────────────────────────────────────────
// Effect cleanup
// ────────────────────────────────────────────────────────────────────

/// Type alias for the cleanup function returned by effect setup.
pub type CleanupFn = Box<dyn FnOnce()>;

/// No-op cleanup for effects that don't need teardown.
#[inline]
#[must_use]
pub fn no_cleanup() -> CleanupFn {
    Box::new(|| {})
}

// PendingEffect
// ────────────────────────────────────────────────────────────────────

/// Internal type alias for the effect setup closure.
type EffectSetupFn<M> = Box<
    dyn FnOnce(
        &<M as Machine>::Context,
        &<M as Machine>::Props,
        StrongSend<<M as Machine>::Event>,
    ) -> CleanupFn,
>;

/// A named side effect produced by a state transition.
///
/// Pending effects are returned from [`Service::send`] inside [`SendResult`].
/// The framework adapter is responsible for executing them by calling
/// [`run`](PendingEffect::run).
///
/// The setup function receives context, props, and a strong send handle.
/// It returns a [`CleanupFn`] invoked when the effect must stop (state
/// change or unmount).
pub struct PendingEffect<M: Machine> {
    /// The identifier for this effect, used by adapters to match and execute it.
    pub name: &'static str,

    /// The state after the transition that produced this effect.
    /// Set by [`Service::drain_queue`] before returning to the adapter.
    pub target_state: Option<M::State>,

    /// Setup closure — receives a snapshot of context, props, and the
    /// strong send handle. Returns a cleanup function.
    pub(crate) setup: EffectSetupFn<M>,
}

impl<M: Machine> PendingEffect<M> {
    /// Creates a new pending effect with a weak-send setup closure.
    ///
    /// The `setup` closure receives [`WeakSend`] (not the strong handle) to
    /// prevent retain cycles. `PendingEffect::new` bridges the strong→weak
    /// conversion internally.
    #[must_use]
    pub fn new(
        name: &'static str,
        setup: impl FnOnce(&M::Context, &M::Props, WeakSend<M::Event>) -> CleanupFn + 'static,
    ) -> Self {
        Self {
            name,
            target_state: None,
            setup: Box::new(move |ctx, props, send: StrongSend<M::Event>| {
                let weak_send = WeakSend::from(&send);
                setup(ctx, props, weak_send)
            }),
        }
    }

    /// Creates a marker-only effect with no-op setup.
    ///
    /// Useful when the effect name is the entire contract (the adapter
    /// implements the behavior based on the name alone).
    #[must_use]
    pub fn named(name: &'static str) -> Self {
        Self {
            name,
            target_state: None,
            setup: Box::new(|_ctx, _props, _send| no_cleanup()),
        }
    }

    /// Execute the effect setup, consuming it.
    ///
    /// Called by the adapter after `Service::send()` returns. The adapter
    /// passes the current context snapshot, props, and its strong send handle.
    pub fn run(self, ctx: &M::Context, props: &M::Props, send: StrongSend<M::Event>) -> CleanupFn {
        (self.setup)(ctx, props, send)
    }
}

impl<M: Machine> Debug for PendingEffect<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingEffect")
            .field("name", &self.name)
            .field("target_state", &self.target_state)
            .field("setup", &"<closure>")
            .finish()
    }
}

// ────────────────────────────────────────────────────────────────────
// TransitionPlan
// ────────────────────────────────────────────────────────────────────

type ApplyFn<M> = dyn FnOnce(&mut <M as Machine>::Context);

/// The result of a state machine transition, describing what should change.
///
/// Built using a fluent builder pattern. Returning `None` from
/// [`Machine::transition`] means the event is ignored; returning a plan
/// with `target: None` means context-only (no state change).
pub struct TransitionPlan<M: Machine> {
    /// The new state to transition to, or `None` to remain in the current state.
    pub target: Option<M::State>,

    /// Mutation to apply to the context after state change.
    pub(crate) apply: Option<Box<ApplyFn<M>>>,

    /// Human-readable description of the apply closure's purpose.
    pub(crate) apply_description: Option<&'static str>,

    /// Events to enqueue after this transition completes.
    pub then_send: Vec<M::Event>,

    /// Side effects for the adapter to set up.
    pub effects: Vec<PendingEffect<M>>,

    /// Named effects to cancel (cleanup runs immediately, no replacement).
    pub cancel_effects: Vec<&'static str>,
}

impl<M: Machine> Default for TransitionPlan<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: Machine> TransitionPlan<M> {
    /// Creates a plan that transitions to a new state.
    #[must_use]
    pub const fn to(state: M::State) -> Self {
        Self {
            target: Some(state),
            apply: None,
            apply_description: None,
            then_send: Vec::new(),
            effects: Vec::new(),
            cancel_effects: Vec::new(),
        }
    }

    /// Creates an empty plan with no state change.
    ///
    /// Useful as a builder starting point — chain `.apply()`, `.then()`,
    /// and `.with_effect()` to configure.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            target: None,
            apply: None,
            apply_description: None,
            then_send: Vec::new(),
            effects: Vec::new(),
            cancel_effects: Vec::new(),
        }
    }

    /// Adds a context mutation to this plan.
    ///
    /// If a previous mutation was already set, the new closure is chained
    /// after it — both run in order.
    #[must_use]
    pub fn apply(mut self, f: impl FnOnce(&mut M::Context) + 'static) -> Self {
        self.apply = match self.apply {
            Some(prev) => Some(Box::new(move |ctx: &mut M::Context| {
                prev(ctx);
                f(ctx);
            })),

            None => Some(Box::new(f)),
        };

        self
    }

    /// Creates a plan that only mutates context without changing state.
    #[must_use]
    pub fn context_only(f: impl FnOnce(&mut M::Context) + 'static) -> Self {
        Self {
            target: None,
            apply: Some(Box::new(f)),
            apply_description: None,
            then_send: Vec::new(),
            effects: Vec::new(),
            cancel_effects: Vec::new(),
        }
    }

    /// Enqueues a follow-up event after this transition.
    #[must_use]
    pub fn then(mut self, event: M::Event) -> Self {
        self.then_send.push(event);
        self
    }

    /// Attaches a side effect for the adapter to manage.
    #[must_use]
    pub fn with_effect(mut self, effect: PendingEffect<M>) -> Self {
        self.effects.push(effect);
        self
    }

    /// Convenience: build a [`PendingEffect`] inline from a name and closure.
    #[must_use]
    pub fn with_named_effect(
        self,
        name: &'static str,
        setup: impl FnOnce(&M::Context, &M::Props, WeakSend<M::Event>) -> CleanupFn + 'static,
    ) -> Self {
        self.with_effect(PendingEffect::new(name, setup))
    }

    /// Cancels a named effect without replacement.
    ///
    /// The adapter runs the effect's cleanup closure immediately. No-op if
    /// no effect with `name` is currently active.
    #[must_use]
    pub fn cancel_effect(mut self, name: &'static str) -> Self {
        self.cancel_effects.push(name);
        self
    }

    /// Returns a short string label for logging/debugging.
    #[must_use]
    pub const fn debug_summary(&self) -> &'static str {
        match (self.target.is_some(), self.apply.is_some()) {
            (true, _) => "to",
            (false, true) => "context_only",
            (false, false) => "none",
        }
    }
}

impl<M: Machine> Debug for TransitionPlan<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransitionPlan")
            .field("target", &self.target)
            .field(
                "apply",
                &if self.apply.is_some() {
                    "<closure>"
                } else {
                    "None"
                },
            )
            .field("apply_description", &self.apply_description)
            .field("then_send", &self.then_send)
            .field(
                "effects",
                &self.effects.iter().map(|e| e.name).collect::<Vec<_>>(),
            )
            .field("cancel_effects", &self.cancel_effects)
            .finish()
    }
}

// ────────────────────────────────────────────────────────────────────
// Bindable
// ────────────────────────────────────────────────────────────────────

/// Trait alias-style marker for values usable with [`Bindable`].
///
/// All bindable values must support cloning for state updates, equality for
/// change detection, and debug output for diagnostics.
pub trait BindableValue: Clone + PartialEq + Debug {}

impl<T: Clone + PartialEq + Debug> BindableValue for T {}

/// A value that may be controlled by the parent or managed internally.
///
/// Components that support two-way binding use `Bindable` to distinguish between
/// values owned by the parent ([`controlled`](Bindable::controlled)) and values
/// managed by the component itself ([`uncontrolled`](Bindable::uncontrolled)).
///
/// When controlled, [`set`](Bindable::set) updates the internal copy but
/// [`get`](Bindable::get) always returns the controlled value. The parent
/// must call [`sync_controlled`](Bindable::sync_controlled) to push new
/// controlled values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bindable<T: BindableValue> {
    /// The externally controlled value, or `None` if uncontrolled.
    controlled: Option<T>,

    /// The internal value managed by the component.
    internal: T,
}

impl<T: BindableValue> Bindable<T> {
    /// Creates a controlled bindable owned by the parent.
    ///
    /// Both the controlled and internal fields are initialized to the given value.
    /// The component reads the controlled value via [`get`](Self::get).
    #[must_use]
    pub fn controlled(value: T) -> Self {
        Self {
            internal: value.clone(),
            controlled: Some(value),
        }
    }

    /// Creates an uncontrolled bindable managed by the component.
    ///
    /// There is no external controlled value; [`get`](Self::get) returns the
    /// internal value which can be updated via [`set`](Self::set).
    #[must_use]
    pub const fn uncontrolled(default: T) -> Self {
        Self {
            controlled: None,
            internal: default,
        }
    }

    /// Returns a reference to the current value.
    ///
    /// Returns the controlled value if set, otherwise the internal value.
    #[must_use]
    pub fn get(&self) -> &T {
        self.controlled.as_ref().unwrap_or(&self.internal)
    }

    /// Returns `true` if this value is controlled by the parent.
    #[must_use]
    pub const fn is_controlled(&self) -> bool {
        self.controlled.is_some()
    }

    /// Updates the internal value.
    ///
    /// In controlled mode, this becomes the pending internal value that is
    /// revealed once the bindable returns to uncontrolled mode.
    pub fn set(&mut self, value: T) {
        self.internal = value;
    }

    /// Pushes a new controlled value from the parent.
    ///
    /// This should be called when the parent's controlled prop changes.
    pub fn sync_controlled(&mut self, value: Option<T>) {
        self.controlled = value;
    }

    /// Returns a mutable reference to the internal value.
    ///
    /// Use for in-place mutations on collection types to avoid cloning.
    /// **Warning:** For controlled bindables, mutating the internal value has no
    /// effect on what [`get`](Self::get) returns (it returns the controlled value).
    pub const fn get_mut_owned(&mut self) -> &mut T {
        &mut self.internal
    }
}

impl<T: BindableValue + Default> Default for Bindable<T> {
    fn default() -> Self {
        Self::uncontrolled(T::default())
    }
}

// ────────────────────────────────────────────────────────────────────
// HasId, ComponentPart, ConnectApi
// ────────────────────────────────────────────────────────────────────

/// Trait for props types that carry a framework-stable DOM ID.
///
/// Adapters use this contract to read and replace component IDs without knowing the
/// concrete props type. The `#[derive(HasId)]` macro implements this trait for any
/// struct with a `pub id: String` field.
pub trait HasId: Sized {
    /// Returns the current DOM ID.
    fn id(&self) -> &str;

    /// Returns a copy of `self` with the DOM ID replaced.
    #[must_use]
    fn with_id(self, id: String) -> Self;

    /// Updates the DOM ID in place.
    fn set_id(&mut self, id: String);
}

/// A named DOM part of a component (e.g. root, trigger, content, label).
///
/// Each component defines an enum of its parts that implements this trait,
/// typically via `#[derive(ComponentPart)]`. The connect API uses parts to
/// produce the correct [`AttrMap`] for each element in the component's DOM tree.
pub trait ComponentPart: Clone + Debug + PartialEq + Eq + core::hash::Hash + 'static {
    /// The root part of this component.
    const ROOT: Self;

    /// Returns the scope name used for `data-ars-scope`.
    fn scope() -> &'static str;

    /// Returns the string name of this part (e.g. `"root"`, `"trigger"`).
    fn name(&self) -> &'static str;

    /// Returns all parts defined for this component.
    fn all() -> Vec<Self>;

    /// Returns the canonical `data-ars-scope` and `data-ars-part` attrs for this part.
    fn data_attrs(&self) -> [(HtmlAttr, &'static str); 2] {
        [
            (HtmlAttr::Data("ars-scope"), Self::scope()),
            (HtmlAttr::Data("ars-part"), self.name()),
        ]
    }
}

/// Produces HTML attributes for each component part based on current machine state.
///
/// Implementors bridge the machine's state, context, and props into concrete
/// [`AttrMap`] values that framework adapters spread onto DOM elements.
pub trait ConnectApi {
    /// The component part enum this API produces attributes for.
    type Part: ComponentPart;

    /// Returns the attribute map for the given part.
    fn part_attrs(&self, part: Self::Part) -> AttrMap;
}

// ────────────────────────────────────────────────────────────────────
// Env — adapter-resolved environment context
// ────────────────────────────────────────────────────────────────────

/// Adapter-resolved environment context passed to [`Machine::init`].
///
/// The adapter reads these values from `ArsProvider` / `ArsContext` and passes
/// them to framework-agnostic core code. Core component code **never** calls
/// framework hooks (`use_locale()`, `use_intl_backend()`, `use_context()`) —
/// all environment values arrive through this struct.
///
/// Non-date-time components ignore `intl_backend` (it defaults to [`StubIntlBackend`]).
pub struct Env {
    /// The resolved locale from `ArsProvider`.
    pub locale: Locale,

    /// Calendar/locale data provider for date-time formatting.
    /// Defaults to [`StubIntlBackend`] (English-only, zero dependencies).
    pub intl_backend: Arc<dyn IntlBackend>,
}

impl Debug for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Env")
            .field("locale", &self.locale)
            .field("intl_backend", &"Arc(..)")
            .finish()
    }
}

impl Default for Env {
    fn default() -> Self {
        Self {
            locale: ars_i18n::locales::en_us(),
            intl_backend: Arc::new(StubIntlBackend),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Machine trait
// ────────────────────────────────────────────────────────────────────

/// Defines a component as a finite state machine.
///
/// A `Machine` declares the component's state type, event type, internal context,
/// props, and connect API. It provides pure functions for initialization, transition
/// logic, and DOM attribute generation — with no framework dependency.
pub trait Machine: Sized + 'static {
    /// The state type representing the machine's current configuration.
    type State: Clone + Debug + PartialEq;

    /// The event type that triggers state transitions.
    type Event: Clone + Debug;

    /// Internal context accumulated across transitions (e.g. focused index, scroll offset).
    type Context: Clone + Debug;

    /// External configuration passed in by the parent component.
    type Props: Clone + PartialEq + HasId;

    /// Per-component i18n messages type.
    type Messages: ComponentMessages + Clone + Default + 'static;

    /// The connect API type that produces attributes from current state.
    type Api<'a>: ConnectApi
    where
        Self: 'a;

    /// Initialize the machine from props and adapter-resolved environment values,
    /// returning initial state and context.
    ///
    /// The adapter resolves `env` (locale, ICU provider) and `messages` from
    /// `ArsProvider` context before calling this method. Core code never calls
    /// framework hooks — all environment values arrive as parameters.
    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context);

    /// Evaluates an event against the current state, context, and props.
    ///
    /// Returns `Some(plan)` to apply a transition or `None` to ignore the event.
    fn transition(
        state: &Self::State,
        event: &Self::Event,
        context: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>>;

    /// Creates the connect API for producing DOM attributes from the current state.
    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a>;

    /// Synchronizes prop changes by emitting events.
    ///
    /// Called by [`Service::set_props`] when props change. Returns events
    /// that are enqueued for processing (e.g., `SetValue`, `SetMode`).
    fn on_props_changed(_old: &Self::Props, _new: &Self::Props) -> Vec<Self::Event> {
        Vec::new()
    }
}

// ────────────────────────────────────────────────────────────────────
// SendResult and Service
// ────────────────────────────────────────────────────────────────────

/// Maximum number of events processed per [`Service::send`] call before
/// breaking to prevent infinite transition loops.
const MAX_DRAIN_ITERATIONS: usize = 100;

#[cfg(feature = "debug")]
fn format_effect_names<M: Machine>(effects: &[PendingEffect<M>]) -> String {
    let mut formatted = String::from("[");

    for (index, effect) in effects.iter().enumerate() {
        if index > 0 {
            formatted.push_str(", ");
        }

        formatted.push_str(effect.name);
    }

    formatted.push(']');

    formatted
}

#[cfg(feature = "debug")]
fn format_follow_up_events<Event: Debug>(events: &[Event]) -> String {
    format!("{events:?}")
}

#[cfg(feature = "debug")]
fn format_target_state<State: Debug>(target: Option<&State>) -> String {
    if let Some(next) = target {
        format!("{next:?}")
    } else {
        String::from("(same)")
    }
}

#[cfg(feature = "debug")]
fn format_apply_description(description: Option<&'static str>) -> String {
    if let Some(description) = description {
        format!("{description:?}")
    } else {
        String::from("\"none\"")
    }
}

/// Result of sending an event to the service.
///
/// Contains state/context change flags and pending effects for the adapter.
pub struct SendResult<M: Machine> {
    /// Whether any state change occurred during this send cycle.
    pub state_changed: bool,

    /// Whether any context mutation occurred (via `plan.apply`).
    ///
    /// Adapters should trigger re-render when `state_changed || context_changed`.
    pub context_changed: bool,

    /// Effects that the adapter must set up.
    pub pending_effects: Vec<PendingEffect<M>>,

    /// Named effects to cancel. The adapter runs their cleanup closures
    /// immediately, before setting up any new `pending_effects`.
    pub cancel_effects: Vec<&'static str>,

    /// Whether the event queue was truncated due to hitting `MAX_DRAIN_ITERATIONS`.
    pub truncated: bool,

    /// Number of consecutive context-only iterations at the end of drain.
    ///
    /// Useful for diagnostics — a high trailing value may indicate a
    /// `context_only` + `then_send` feedback loop.
    pub context_change_count: usize,
}

impl<M: Machine> Debug for SendResult<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SendResult")
            .field("state_changed", &self.state_changed)
            .field("context_changed", &self.context_changed)
            .field(
                "pending_effects",
                &self
                    .pending_effects
                    .iter()
                    .map(|e| e.name)
                    .collect::<Vec<_>>(),
            )
            .field("cancel_effects", &self.cancel_effects)
            .field("truncated", &self.truncated)
            .field("context_change_count", &self.context_change_count)
            .finish()
    }
}

/// Serializable snapshot of a machine's runtime state, used to round-trip
/// state across the SSR → client-hydration boundary.
///
/// The server builds the snapshot after rendering — typically
/// `HydrationSnapshot { state: svc.state().clone(), id: svc.props().id().into() }`
/// — writes it to the page (for example as a JSON payload alongside the SSR
/// HTML), and the client deserializes it before calling
/// [`Service::new_hydrated`] to restore the component without re-running
/// [`Machine::init`].
///
/// Only state and the component ID travel across the boundary. Context is
/// always recomputed from props by [`Service::new_hydrated`] so that
/// adapter-computed values (resolved locale, platform effects, derived
/// IDs) stay consistent with the client's environment.
///
/// The spec contract lives in `spec/testing/07-ssr-hydration.md` §2.
///
/// This type is gated behind the `ssr` and `serde` features because both
/// are required for its intended use: SSR builds provide the rendering
/// path, and serde provides the wire format.
#[cfg(all(feature = "ssr", feature = "serde"))]
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "M::State: serde::Serialize",
    deserialize = "M::State: serde::Deserialize<'de>"
))]
pub struct HydrationSnapshot<M: Machine> {
    /// The machine state captured on the server, to be restored on the client.
    pub state: M::State,

    /// The component ID that produced the snapshot. Equal to
    /// `svc.props().id()` on the server; used on the client to match the
    /// snapshot to the mounted component instance.
    pub id: String,
}

/// A running instance of a [`Machine`] that manages state, context, and props.
///
/// `Service` is the runtime counterpart to a `Machine` definition. It holds the
/// current state, applies transitions via [`send`](Service::send), and produces
/// connect APIs via [`connect`](Service::connect). Framework adapters wrap a
/// `Service` in reactive signals to drive re-renders on state changes.
pub struct Service<M: Machine> {
    state: M::State,
    context: M::Context,
    props: M::Props,
    event_queue: VecDeque<M::Event>,
    unmounted: bool,
}

impl<M: Machine> Debug for Service<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Service")
            .field("state", &self.state)
            .field("context", &self.context)
            .field("props_id", &self.props.id())
            .field("event_queue_len", &self.event_queue.len())
            .field("unmounted", &self.unmounted)
            .finish()
    }
}

impl<M: Machine> Service<M> {
    /// Creates a new service by initializing the machine with the given props,
    /// adapter-resolved environment, and i18n messages.
    #[must_use]
    pub fn new(props: M::Props, env: &Env, messages: &M::Messages) -> Self {
        debug_assert!(!props.id().is_empty(), "Props::id must not be empty");

        let (state, context) = M::init(&props, env, messages);

        Self {
            state,
            context,
            props,
            event_queue: VecDeque::new(),
            unmounted: false,
        }
    }

    /// Construct a Service from a hydrated state snapshot.
    ///
    /// Used during SSR hydration to restore server-rendered state on the client
    /// without re-running [`Machine::init`]. Calls `init` to derive a valid
    /// context from props, then replaces the initial state with the hydrated
    /// snapshot. This ensures context (which contains computed IDs, derived
    /// flags, etc.) is always correctly derived from props, while state is
    /// restored from the server snapshot.
    #[cfg(feature = "ssr")]
    #[must_use]
    pub fn new_hydrated(
        props: M::Props,
        state: M::State,
        env: &Env,
        messages: &M::Messages,
    ) -> Self {
        debug_assert!(!props.id().is_empty(), "Props::id must not be empty");

        let (_init_state, context) = M::init(&props, env, messages);

        Self {
            state,
            context,
            props,
            event_queue: VecDeque::new(),
            unmounted: false,
        }
    }

    /// Returns a reference to the current machine state.
    #[must_use]
    pub const fn state(&self) -> &M::State {
        &self.state
    }

    /// Returns a reference to the current machine context.
    #[must_use]
    pub const fn context(&self) -> &M::Context {
        &self.context
    }

    /// Returns a mutable reference to the current machine context.
    pub const fn context_mut(&mut self) -> &mut M::Context {
        &mut self.context
    }

    /// Returns a reference to the current props.
    #[must_use]
    pub const fn props(&self) -> &M::Props {
        &self.props
    }

    /// Returns a mutable reference to the current props.
    pub const fn props_mut(&mut self) -> &mut M::Props {
        &mut self.props
    }

    /// Sends an event to the machine, processing it and any chained events.
    ///
    /// Returns a [`SendResult`] with state/context change flags and
    /// pending effects for the adapter to execute.
    #[must_use]
    pub fn send(&mut self, event: M::Event) -> SendResult<M> {
        debug_assert!(!self.unmounted, "send() called after unmount()");

        if self.unmounted {
            #[cfg(feature = "debug")]
            log::debug!(
                "[ars:{}] dropped event after unmount: {:?}",
                self.props.id(),
                event
            );

            return SendResult {
                state_changed: false,
                context_changed: false,
                pending_effects: Vec::new(),
                cancel_effects: Vec::new(),
                truncated: false,
                context_change_count: 0,
            };
        }

        self.event_queue.push_back(event);

        self.drain_queue()
    }

    /// Processes all queued events iteratively with loop safety.
    fn drain_queue(&mut self) -> SendResult<M> {
        let mut pending_effects = Vec::new();

        let mut cancel_effects = Vec::new();

        let mut state_changed = false;

        let mut context_changed = false;

        #[cfg_attr(
            debug_assertions,
            expect(
                unused_mut,
                reason = "only mutated in release builds (debug panics first)"
            )
        )]
        let mut truncated = false;

        let mut iterations = 0;

        let mut context_change_count = 0;

        while let Some(event) = self.event_queue.pop_front() {
            iterations += 1;

            #[cfg(feature = "debug")]
            let state_before = format!("{:?}", self.state);

            #[cfg(feature = "debug")]
            let event_repr = format!("{event:?}");

            if iterations > MAX_DRAIN_ITERATIONS {
                #[cfg(debug_assertions)]
                panic!(
                    "Event queue exceeded {MAX_DRAIN_ITERATIONS} iterations — \
                     likely an infinite loop in transitions"
                );

                #[cfg(not(debug_assertions))]
                {
                    #[cfg(feature = "debug")]
                    log::warn!(
                        "drain_queue: event queue exceeded {MAX_DRAIN_ITERATIONS} iterations, \
                         truncating. This likely indicates an infinite loop in state machine \
                         transitions."
                    );

                    truncated = true;

                    break;
                }
            }

            if let Some(plan) = M::transition(&self.state, &event, &self.context, &self.props) {
                #[cfg(feature = "debug")]
                let target_state = format_target_state(plan.target.as_ref());

                #[cfg(feature = "debug")]
                let effect_names = format_effect_names(&plan.effects);

                #[cfg(feature = "debug")]
                let apply_description = format_apply_description(plan.apply_description);

                #[cfg(feature = "debug")]
                let follow_up_events = format_follow_up_events(&plan.then_send);

                // Apply context mutation.
                if let Some(apply) = plan.apply {
                    apply(&mut self.context);

                    context_changed = true;
                }

                // Track context-only iterations for diagnostics.
                if plan.target.is_none() {
                    context_change_count += 1;

                    if context_change_count >= MAX_DRAIN_ITERATIONS {
                        #[cfg(debug_assertions)]
                        panic!(
                            "drain_queue: {context_change_count} consecutive context_only \
                             iterations without a state change — likely an infinite \
                             context_only + then_send loop"
                        );

                        #[cfg(not(debug_assertions))]
                        {
                            #[cfg(feature = "debug")]
                            log::warn!(
                                "drain_queue: {context_change_count} context_only iterations \
                                 without state change — possible infinite loop, truncating."
                            );

                            truncated = true;

                            break;
                        }
                    }
                } else {
                    context_change_count = 0;
                }

                // Enqueue follow-up events.
                self.event_queue.extend(plan.then_send);

                #[cfg(feature = "debug")]
                let queue_depth = self.event_queue.len();

                // Apply state change.
                if let Some(next) = plan.target {
                    self.state = next;

                    state_changed = true;
                }

                // Collect effect cancellations.
                cancel_effects.extend(plan.cancel_effects);

                // Collect effects, tagged with the target state.
                let target = self.state.clone();

                pending_effects.extend(plan.effects.into_iter().map(|mut e| {
                    e.target_state = Some(target.clone());
                    e
                }));

                #[cfg(feature = "debug")]
                {
                    let component_id = self.props.id();

                    log::trace!(
                        "[ars:{component_id}] {state_before} + {event_repr} → {target_state} \
                         (guard: pass, effects: {effect_names})"
                    );

                    log::trace!(
                        "[ars:{component_id}]   apply: {apply_description}, then_send: \
                         {follow_up_events}, iteration: {iterations}, queue_depth: {queue_depth}"
                    );
                }
            } else {
                #[cfg(feature = "debug")]
                log::trace!(
                    "[ars:{}] {} + {} → (same) (guard: reject, effects: [])",
                    self.props.id(),
                    state_before,
                    event_repr
                );
            }
        }

        SendResult {
            state_changed,
            context_changed,
            pending_effects,
            cancel_effects,
            truncated,
            context_change_count,
        }
    }

    /// Updates props atomically and processes any resulting events.
    ///
    /// Calls [`Machine::on_props_changed`] with the old and new props,
    /// prepends any returned events ahead of already-queued user input, and
    /// drains the queue.
    pub fn set_props(&mut self, props: M::Props) -> SendResult<M> {
        let old_props = core::mem::replace(&mut self.props, props);

        let events = M::on_props_changed(&old_props, &self.props);

        for event in events.into_iter().rev() {
            self.event_queue.push_front(event);
        }

        self.drain_queue()
    }

    /// Unmounts the service, running all active effect cleanups.
    ///
    /// After calling this, no further [`send`](Self::send) calls are valid.
    /// In debug builds, subsequent sends will panic; in release builds,
    /// they return an inert [`SendResult`].
    pub fn unmount(&mut self, active_cleanups: Vec<CleanupFn>) {
        active_cleanups
            .into_iter()
            .rev()
            .for_each(|cleanup| cleanup());

        self.event_queue.clear();

        self.unmounted = true;
    }

    /// Returns `true` after [`unmount`](Self::unmount) has been called.
    #[must_use]
    pub const fn is_unmounted(&self) -> bool {
        self.unmounted
    }

    /// Creates a connect API snapshot for producing DOM attributes.
    ///
    /// The `send` closure is used by the API to wire event handlers. The returned
    /// API borrows from this service and must not outlive it.
    pub fn connect<'a>(&'a self, send: &'a dyn Fn(M::Event)) -> M::Api<'a> {
        M::connect(&self.state, &self.context, &self.props, send)
    }

    /// Test-only: force the service into a specific state.
    ///
    /// Re-derives context from the new state and current props via
    /// `Machine::init`, discarding the init state. Uses default env and
    /// messages. Used by keyboard matrix tests to start from arbitrary states.
    #[cfg(test)]
    pub fn set_state_for_test(&mut self, state: M::State) {
        let (_init_state, context) = M::init(&self.props, &Env::default(), &M::Messages::default());

        self.state = state;

        self.context = context;
    }
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloc::{format, vec};
    #[cfg(feature = "debug")]
    use std::sync::{Mutex, Once};

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    enum ToggleState {
        Off,
        On,
    }

    #[cfg(feature = "debug")]
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct CapturedLog {
        level: log::Level,
        message: String,
    }

    #[cfg(feature = "debug")]
    struct TestLogger {
        records: Mutex<Vec<CapturedLog>>,
    }

    #[cfg(feature = "debug")]
    static TEST_LOGGER: TestLogger = TestLogger {
        records: Mutex::new(Vec::new()),
    };

    #[cfg(feature = "debug")]
    static LOGGER_INIT: Once = Once::new();

    #[cfg(feature = "debug")]
    static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

    #[cfg(feature = "debug")]
    impl log::Log for TestLogger {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.level() <= log::Level::Trace
        }

        fn log(&self, record: &log::Record<'_>) {
            if self.enabled(record.metadata()) {
                self.records
                    .lock()
                    .expect("test logger mutex poisoned")
                    .push(CapturedLog {
                        level: record.level(),
                        message: format!("{}", record.args()),
                    });
            }
        }

        fn flush(&self) {}
    }

    #[cfg(feature = "debug")]
    fn init_test_logger() {
        LOGGER_INIT.call_once(|| {
            log::set_logger(&TEST_LOGGER).expect("test logger should install once");

            log::set_max_level(log::LevelFilter::Trace);
        });
    }

    #[cfg(feature = "debug")]
    fn capture_logs(run: impl FnOnce()) -> Vec<CapturedLog> {
        init_test_logger();

        let _capture_guard = CAPTURE_LOCK.lock().expect("capture mutex poisoned");

        TEST_LOGGER
            .records
            .lock()
            .expect("test logger mutex poisoned")
            .clear();

        run();

        let records = TEST_LOGGER
            .records
            .lock()
            .expect("test logger mutex poisoned")
            .clone();

        TEST_LOGGER
            .records
            .lock()
            .expect("test logger mutex poisoned")
            .clear();

        records
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ToggleEvent {
        Toggle,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ToggleContext;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct ToggleProps {
        id: String,
    }

    impl HasId for ToggleProps {
        fn id(&self) -> &str {
            &self.id
        }

        fn with_id(self, id: String) -> Self {
            Self { id }
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct TogglePart;

    impl ComponentPart for TogglePart {
        const ROOT: Self = Self;

        fn scope() -> &'static str {
            "toggle"
        }

        fn name(&self) -> &'static str {
            "root"
        }

        fn all() -> Vec<Self> {
            vec![Self]
        }
    }

    struct ToggleApi;

    impl ConnectApi for ToggleApi {
        type Part = TogglePart;

        fn part_attrs(&self, _part: Self::Part) -> AttrMap {
            AttrMap::new()
        }
    }

    struct ToggleMachine;

    impl Machine for ToggleMachine {
        type State = ToggleState;
        type Event = ToggleEvent;
        type Context = ToggleContext;
        type Props = ToggleProps;
        type Messages = ();
        type Api<'a> = ToggleApi;

        fn init(
            _props: &Self::Props,
            _env: &Env,
            _messages: &Self::Messages,
        ) -> (Self::State, Self::Context) {
            (ToggleState::Off, ToggleContext)
        }

        fn transition(
            state: &Self::State,
            event: &Self::Event,
            _context: &Self::Context,
            _props: &Self::Props,
        ) -> Option<TransitionPlan<Self>> {
            match (state, event) {
                (ToggleState::Off, ToggleEvent::Toggle) => {
                    Some(TransitionPlan::to(ToggleState::On))
                }

                (ToggleState::On, ToggleEvent::Toggle) => {
                    Some(TransitionPlan::to(ToggleState::Off))
                }
            }
        }

        fn connect<'a>(
            _state: &'a Self::State,
            _context: &'a Self::Context,
            _props: &'a Self::Props,
            _send: &'a dyn Fn(Self::Event),
        ) -> Self::Api<'a> {
            ToggleApi
        }
    }

    #[test]
    fn service_applies_transitions() {
        let mut service = Service::<ToggleMachine>::new(
            ToggleProps {
                id: String::from("toggle"),
            },
            &Env::default(),
            &(),
        );

        assert_eq!(service.state(), &ToggleState::Off);

        let result = service.send(ToggleEvent::Toggle);

        assert!(result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &ToggleState::On);
    }

    /// [`HydrationSnapshot`] serialises the state captured on the server and
    /// reconstructs the same logical [`Service`] on the client via
    /// [`Service::new_hydrated`] — without re-running [`Machine::init`]'s
    /// state derivation. This covers the `spec/testing/07-ssr-hydration.md`
    /// §2 round-trip contract.
    #[cfg(all(feature = "ssr", feature = "serde"))]
    #[test]
    fn hydration_snapshot_round_trips_through_serde_json() {
        let props = ToggleProps {
            id: String::from("toggle-1"),
        };

        // Server side: build a Service, advance to the "On" state, capture a snapshot.
        let mut server = Service::<ToggleMachine>::new(props.clone(), &Env::default(), &());

        drop(server.send(ToggleEvent::Toggle));

        assert_eq!(server.state(), &ToggleState::On);

        let snapshot: HydrationSnapshot<ToggleMachine> = HydrationSnapshot {
            state: *server.state(),
            id: server.props().id().into(),
        };

        let json = serde_json::to_string(&snapshot).expect("serialize snapshot");

        let restored: HydrationSnapshot<ToggleMachine> =
            serde_json::from_str(&json).expect("deserialize snapshot");

        assert_eq!(restored.state, ToggleState::On);
        assert_eq!(restored.id, "toggle-1");

        // Client side: hydrate the Service from the restored state. Context is
        // recomputed from props, state is injected from the snapshot.
        let client =
            Service::<ToggleMachine>::new_hydrated(props, restored.state, &Env::default(), &());

        assert_eq!(
            client.state(),
            &ToggleState::On,
            "hydrated client must skip init's initial state and adopt the snapshot",
        );
    }

    #[test]
    fn default_machine_on_props_changed_returns_no_events() {
        let old = ToggleProps {
            id: String::from("toggle-old"),
        };
        let new = ToggleProps {
            id: String::from("toggle-new"),
        };

        assert!(<ToggleMachine as Machine>::on_props_changed(&old, &new).is_empty());
    }

    #[test]
    fn service_accessors_debug_connect_and_unmount_behave_as_expected() {
        use alloc::{format, rc::Rc};
        use core::cell::RefCell;

        let mut service = Service::<ToggleMachine>::new(
            ToggleProps {
                id: String::from("toggle"),
            },
            &Env::default(),
            &(),
        );

        assert_eq!(service.props().id, "toggle");

        service.props_mut().id = String::from("toggle-updated");
        *service.context_mut() = ToggleContext;

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(TogglePart), AttrMap::new());
        assert_eq!(service.props().id, "toggle-updated");

        let debug = format!("{service:?}");

        assert!(debug.contains("Service"));
        assert!(debug.contains("props_id"));
        assert!(!service.is_unmounted());

        service.event_queue.push_back(ToggleEvent::Toggle);

        let cleanup_order = Rc::new(RefCell::new(Vec::new()));

        let first = {
            let cleanup_order = Rc::clone(&cleanup_order);

            Box::new(move || cleanup_order.borrow_mut().push("first")) as CleanupFn
        };
        let second = {
            let cleanup_order = Rc::clone(&cleanup_order);

            Box::new(move || cleanup_order.borrow_mut().push("second")) as CleanupFn
        };

        service.unmount(vec![first, second]);

        assert!(service.event_queue.is_empty());
        assert!(service.is_unmounted());
        assert_eq!(cleanup_order.borrow().as_slice(), ["second", "first"]);
    }

    #[test]
    fn send_result_reports_no_change_when_ignored() {
        let mut service = Service::<ToggleMachine>::new(
            ToggleProps {
                id: String::from("toggle"),
            },
            &Env::default(),
            &(),
        );

        // Toggle is exhaustive so nothing is ignored — toggle twice and check.
        let result = service.send(ToggleEvent::Toggle);

        assert!(result.state_changed);

        let result = service.send(ToggleEvent::Toggle);

        assert!(result.state_changed);
        assert_eq!(service.state(), &ToggleState::Off);
    }

    #[test]
    fn rejected_transition_returns_inert_send_result() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum RejectEvent {
            Ignore,
        }

        struct RejectMachine;

        impl Machine for RejectMachine {
            type State = ToggleState;
            type Event = RejectEvent;
            type Context = ToggleContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, ToggleContext)
            }

            fn transition(
                _state: &Self::State,
                _event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                None
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let mut service = Service::<RejectMachine>::new(
            ToggleProps {
                id: String::from("toggle"),
            },
            &Env::default(),
            &(),
        );

        let result = service.send(RejectEvent::Ignore);

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert!(result.pending_effects.is_empty());
        assert!(result.cancel_effects.is_empty());
        assert!(!result.truncated);
        assert_eq!(result.context_change_count, 0);
        assert_eq!(service.state(), &ToggleState::Off);
        assert_eq!(service.context(), &ToggleContext);
    }

    #[test]
    fn context_only_plan_does_not_change_state() {
        // Verify context_only plan reports context_changed but not state_changed.
        #[derive(Clone, Debug, PartialEq)]
        struct Ctx {
            count: u32,
        }

        struct CountMachine;

        impl Machine for CountMachine {
            type State = ToggleState;
            type Event = ToggleEvent;
            type Context = Ctx;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, Ctx { count: 0 })
            }

            fn transition(
                _state: &Self::State,
                _event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                Some(TransitionPlan::context_only(|ctx: &mut Ctx| ctx.count += 1))
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let mut service = Service::<CountMachine>::new(
            ToggleProps {
                id: String::from("test"),
            },
            &Env::default(),
            &(),
        );

        assert_eq!(
            service.connect(&|_| {}).part_attrs(TogglePart),
            AttrMap::new()
        );

        let result = service.send(ToggleEvent::Toggle);

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.context().count, 1);
    }

    #[test]
    fn cancel_effects_are_collected() {
        struct CancelMachine;

        impl Machine for CancelMachine {
            type State = ToggleState;
            type Event = ToggleEvent;
            type Context = ToggleContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, ToggleContext)
            }

            fn transition(
                _state: &Self::State,
                _event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                Some(
                    TransitionPlan::to(ToggleState::On)
                        .cancel_effect("timer")
                        .cancel_effect("polling"),
                )
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let mut service = Service::<CancelMachine>::new(
            ToggleProps {
                id: String::from("test"),
            },
            &Env::default(),
            &(),
        );

        assert_eq!(
            service.connect(&|_| {}).part_attrs(TogglePart),
            AttrMap::new()
        );

        let result = service.send(ToggleEvent::Toggle);

        assert_eq!(result.cancel_effects, vec!["timer", "polling"]);
    }

    #[test]
    fn effects_are_collected_with_target_state() {
        struct EffectMachine;

        impl Machine for EffectMachine {
            type State = ToggleState;
            type Event = ToggleEvent;
            type Context = ToggleContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, ToggleContext)
            }

            fn transition(
                _state: &Self::State,
                _event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                Some(TransitionPlan::to(ToggleState::On).with_effect(PendingEffect::named("focus")))
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let mut service = Service::<EffectMachine>::new(
            ToggleProps {
                id: String::from("test"),
            },
            &Env::default(),
            &(),
        );

        assert_eq!(
            service.connect(&|_| {}).part_attrs(TogglePart),
            AttrMap::new()
        );

        let result = service.send(ToggleEvent::Toggle);

        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, "focus");
        assert_eq!(
            result.pending_effects[0].target_state,
            Some(ToggleState::On)
        );
    }

    #[test]
    fn set_props_fires_on_props_changed() {
        struct PropsMachine;

        #[derive(Clone, Debug, PartialEq)]
        struct PropsCtx {
            mode: u8,
        }

        #[derive(Clone, Copy, Debug)]
        enum PropsEvent {
            SetMode(u8),
        }

        impl Machine for PropsMachine {
            type State = ToggleState;
            type Event = PropsEvent;
            type Context = PropsCtx;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, PropsCtx { mode: 0 })
            }

            fn transition(
                _state: &Self::State,
                event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                match event {
                    PropsEvent::SetMode(m) => {
                        let m = *m;

                        Some(TransitionPlan::context_only(move |ctx: &mut PropsCtx| {
                            ctx.mode = m;
                        }))
                    }
                }
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }

            fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
                if old.id == new.id {
                    vec![]
                } else {
                    vec![PropsEvent::SetMode(1)]
                }
            }
        }

        let mut service = Service::<PropsMachine>::new(
            ToggleProps {
                id: String::from("a"),
            },
            &Env::default(),
            &(),
        );

        assert_eq!(
            service.connect(&|_| {}).part_attrs(TogglePart),
            AttrMap::new()
        );
        assert_eq!(service.context().mode, 0);

        let unchanged = service.set_props(ToggleProps {
            id: String::from("a"),
        });

        assert!(!unchanged.state_changed);
        assert!(!unchanged.context_changed);
        assert_eq!(service.context().mode, 0);

        let result = service.set_props(ToggleProps {
            id: String::from("b"),
        });

        assert!(result.context_changed);
        assert_eq!(service.context().mode, 1);
    }

    #[test]
    fn set_props_prepends_prop_sync_events_ahead_of_queued_user_input() {
        struct QueueOrderMachine;

        #[derive(Clone, Debug, Default, PartialEq, Eq)]
        struct QueueOrderContext {
            events: Vec<&'static str>,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum QueueOrderEvent {
            PropSync,
            UserInput,
        }

        impl Machine for QueueOrderMachine {
            type State = ToggleState;
            type Event = QueueOrderEvent;
            type Context = QueueOrderContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, QueueOrderContext::default())
            }

            fn transition(
                _state: &Self::State,
                event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                let label = match event {
                    QueueOrderEvent::PropSync => "prop-sync",
                    QueueOrderEvent::UserInput => "user-input",
                };

                Some(TransitionPlan::context_only(
                    move |ctx: &mut QueueOrderContext| ctx.events.push(label),
                ))
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }

            fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
                if old.id == new.id {
                    Vec::new()
                } else {
                    vec![QueueOrderEvent::PropSync]
                }
            }
        }

        let mut service = Service::<QueueOrderMachine>::new(
            ToggleProps {
                id: String::from("a"),
            },
            &Env::default(),
            &(),
        );
        service.event_queue.push_back(QueueOrderEvent::UserInput);

        let result = service.set_props(ToggleProps {
            id: String::from("b"),
        });

        assert!(result.context_changed);
        assert_eq!(
            service.context().events,
            vec!["prop-sync", "user-input"],
            "prop sync events must be prepended so same-frame user input wins",
        );

        let unchanged = service.set_props(ToggleProps {
            id: String::from("b"),
        });

        assert!(!unchanged.state_changed);
        assert!(!unchanged.context_changed);
        assert_eq!(
            service.context().events,
            vec!["prop-sync", "user-input"],
            "unchanged props must not synthesize extra prop-sync work",
        );
    }

    #[test]
    fn set_state_for_test_overrides_state_and_re_derives_context() {
        let mut service = Service::<ToggleMachine>::new(
            ToggleProps {
                id: String::from("test"),
            },
            &Env::default(),
            &(),
        );

        assert_eq!(service.state(), &ToggleState::Off);

        service.set_state_for_test(ToggleState::On);

        assert_eq!(service.state(), &ToggleState::On);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Props::id must not be empty")]
    fn service_new_panics_on_empty_id_in_debug() {
        let _service =
            Service::<ToggleMachine>::new(ToggleProps { id: String::new() }, &Env::default(), &());
    }

    #[test]
    fn env_default_has_en_us_locale() {
        let env = Env::default();

        assert_eq!(env.locale.to_bcp47(), "en-US");
    }

    #[test]
    fn bindable_value_blanket_impl_covers_common_types() {
        fn assert_bindable_value<T: BindableValue>() {}

        assert_bindable_value::<String>();
        assert_bindable_value::<bool>();
        assert_bindable_value::<u32>();
        assert_bindable_value::<Vec<String>>();
    }

    #[test]
    fn bindable_default_uses_default_uncontrolled_value() {
        assert_eq!(
            Bindable::<String>::default(),
            Bindable::uncontrolled(String::default())
        );
        assert_eq!(Bindable::<bool>::default(), Bindable::uncontrolled(false));
    }

    #[cfg(feature = "debug")]
    #[test]
    fn send_emits_trace_log_for_processed_event() {
        let logs = capture_logs(|| {
            let mut service = Service::<ToggleMachine>::new(
                ToggleProps {
                    id: String::from("toggle#btn-1"),
                },
                &Env::default(),
                &(),
            );

            let _result = service.send(ToggleEvent::Toggle);
        });

        assert!(
            logs.iter().any(|record| record.level == log::Level::Trace),
            "expected at least one trace log, got {logs:?}"
        );
    }

    #[cfg(feature = "debug")]
    #[test]
    fn trace_log_formats_multiple_effect_names() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum MultiEffectEvent {
            Trigger,
        }

        struct MultiEffectMachine;

        impl Machine for MultiEffectMachine {
            type State = ToggleState;
            type Event = MultiEffectEvent;
            type Context = ToggleContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, ToggleContext)
            }

            fn transition(
                state: &Self::State,
                event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                match (state, event) {
                    (ToggleState::Off, MultiEffectEvent::Trigger) => Some(
                        TransitionPlan::to(ToggleState::On)
                            .with_effect(PendingEffect::named("focus"))
                            .with_effect(PendingEffect::named("announce")),
                    ),

                    _ => None,
                }
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let logs = capture_logs(|| {
            let mut service = Service::<MultiEffectMachine>::new(
                ToggleProps {
                    id: String::from("menu#item-1"),
                },
                &Env::default(),
                &(),
            );

            let _result = service.send(MultiEffectEvent::Trigger);
        });

        let transition = logs
            .iter()
            .find(|record| record.message.contains("[ars:menu#item-1]"))
            .expect("expected transition log");

        assert!(
            transition.message.contains("effects: [focus, announce]"),
            "expected comma-joined effect names: {transition:?}"
        );
    }

    #[cfg(feature = "debug")]
    #[test]
    fn trace_log_includes_required_structured_fields() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct DebugContext {
            pressed: bool,
        }

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum DebugEvent {
            Toggle,
            Notify,
        }

        struct DebugMachine;

        impl Machine for DebugMachine {
            type State = ToggleState;
            type Event = DebugEvent;
            type Context = DebugContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, DebugContext { pressed: false })
            }

            fn transition(
                state: &Self::State,
                event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                match (state, event) {
                    (ToggleState::Off, DebugEvent::Toggle) => {
                        let mut plan = TransitionPlan::to(ToggleState::On)
                            .apply(|ctx: &mut DebugContext| ctx.pressed = true)
                            .then(DebugEvent::Notify)
                            .with_effect(PendingEffect::named("notify_change"));

                        plan.apply_description = Some("set pressed = true");

                        Some(plan)
                    }

                    (ToggleState::On, DebugEvent::Notify) => Some(TransitionPlan::new()),

                    _ => None,
                }
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let logs = capture_logs(|| {
            let mut service = Service::<DebugMachine>::new(
                ToggleProps {
                    id: String::from("toggle#btn-1"),
                },
                &Env::default(),
                &(),
            );

            let _result = service.send(DebugEvent::Toggle);
        });

        let transition = logs
            .iter()
            .find(|record| {
                record
                    .message
                    .contains("[ars:toggle#btn-1] Off + Toggle → On")
            })
            .expect("expected transition log");

        assert_eq!(transition.level, log::Level::Trace);
        assert!(
            transition.message.contains("guard: pass"),
            "missing guard result: {transition:?}"
        );
        assert!(
            transition.message.contains("effects: [notify_change]"),
            "missing effect names: {transition:?}"
        );

        let detail = logs
            .iter()
            .find(|record| {
                record
                    .message
                    .contains("[ars:toggle#btn-1]   apply: \"set pressed = true\"")
            })
            .expect("expected detail log");

        assert!(
            detail.message.contains("then_send: [Notify]"),
            "missing follow-up events: {detail:?}"
        );
        assert!(
            detail.message.contains("iteration: 1"),
            "missing iteration counter: {detail:?}"
        );
        assert!(
            detail.message.contains("queue_depth: 1"),
            "missing queue depth: {detail:?}"
        );
    }

    #[cfg(feature = "debug")]
    #[test]
    fn rejected_event_logs_guard_reject() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum RejectEvent {
            Ignore,
        }

        struct RejectMachine;

        impl Machine for RejectMachine {
            type State = ToggleState;
            type Event = RejectEvent;
            type Context = ToggleContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, ToggleContext)
            }

            fn transition(
                _state: &Self::State,
                _event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                None
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let logs = capture_logs(|| {
            let mut service = Service::<RejectMachine>::new(
                ToggleProps {
                    id: String::from("combobox#cb-1"),
                },
                &Env::default(),
                &(),
            );

            let _result = service.send(RejectEvent::Ignore);
        });

        let rejected = logs
            .iter()
            .find(|record| record.message.contains("[ars:combobox#cb-1]"))
            .expect("expected rejected transition log");

        assert!(
            rejected.message.contains("guard: reject"),
            "missing guard reject marker: {rejected:?}"
        );
        assert!(
            rejected.message.contains("→ (same)"),
            "missing same-state marker: {rejected:?}"
        );
    }

    #[cfg(feature = "debug")]
    #[test]
    fn multi_step_drain_cycle_logs_each_iteration_with_queue_depth() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum ChainEvent {
            Start,
            Continue,
        }

        struct ChainMachine;

        impl Machine for ChainMachine {
            type State = ToggleState;
            type Event = ChainEvent;
            type Context = ToggleContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, ToggleContext)
            }

            fn transition(
                state: &Self::State,
                event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                match (state, event) {
                    (ToggleState::Off, ChainEvent::Start) => {
                        Some(TransitionPlan::to(ToggleState::On).then(ChainEvent::Continue))
                    }

                    (ToggleState::On, ChainEvent::Continue) => {
                        Some(TransitionPlan::to(ToggleState::Off))
                    }

                    _ => None,
                }
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let logs = capture_logs(|| {
            let mut service = Service::<ChainMachine>::new(
                ToggleProps {
                    id: String::from("dialog#dlg-1"),
                },
                &Env::default(),
                &(),
            );

            let _result = service.send(ChainEvent::Start);
        });

        let iteration_one = logs
            .iter()
            .find(|record| {
                record
                    .message
                    .contains("[ars:dialog#dlg-1]   apply: \"none\"")
                    && record.message.contains("iteration: 1")
            })
            .expect("expected first detail log");

        assert!(
            iteration_one.message.contains("queue_depth: 1"),
            "expected queued follow-up event after first iteration: {iteration_one:?}"
        );

        let iteration_two = logs
            .iter()
            .find(|record| {
                record
                    .message
                    .contains("[ars:dialog#dlg-1]   apply: \"none\"")
                    && record.message.contains("iteration: 2")
            })
            .expect("expected second detail log");

        assert!(
            iteration_two.message.contains("queue_depth: 0"),
            "expected queue to be drained after second iteration: {iteration_two:?}"
        );
    }

    #[cfg(all(feature = "debug", debug_assertions))]
    #[test]
    #[should_panic(expected = "Event queue exceeded 100 iterations")]
    fn drain_queue_panics_on_iteration_limit_in_debug_builds() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum LoopEvent {
            Continue,
        }

        struct IterationLoopMachine;

        impl Machine for IterationLoopMachine {
            type State = ToggleState;
            type Event = LoopEvent;
            type Context = ToggleContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, ToggleContext)
            }

            fn transition(
                state: &Self::State,
                event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                match (state, event) {
                    (ToggleState::Off, LoopEvent::Continue) => {
                        Some(TransitionPlan::to(ToggleState::On).then(LoopEvent::Continue))
                    }

                    (ToggleState::On, LoopEvent::Continue) => {
                        Some(TransitionPlan::to(ToggleState::Off).then(LoopEvent::Continue))
                    }
                }
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let mut service = Service::<IterationLoopMachine>::new(
            ToggleProps {
                id: String::from("loop#iteration"),
            },
            &Env::default(),
            &(),
        );

        let _result = service.send(LoopEvent::Continue);
    }

    #[cfg(all(feature = "debug", debug_assertions))]
    #[test]
    #[should_panic(expected = "consecutive context_only iterations")]
    fn drain_queue_panics_on_context_only_loop_in_debug_builds() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum LoopEvent {
            Continue,
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        struct LoopContext {
            ticks: usize,
        }

        struct ContextLoopMachine;

        impl Machine for ContextLoopMachine {
            type State = ToggleState;
            type Event = LoopEvent;
            type Context = LoopContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, LoopContext { ticks: 0 })
            }

            fn transition(
                _state: &Self::State,
                event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                match event {
                    LoopEvent::Continue => Some(
                        TransitionPlan::context_only(|ctx: &mut LoopContext| ctx.ticks += 1)
                            .then(LoopEvent::Continue),
                    ),
                }
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let mut service = Service::<ContextLoopMachine>::new(
            ToggleProps {
                id: String::from("loop#context"),
            },
            &Env::default(),
            &(),
        );

        let _result = service.send(LoopEvent::Continue);
    }

    #[cfg(all(feature = "debug", not(debug_assertions)))]
    #[test]
    fn send_after_unmount_emits_debug_log() {
        let mut service = Service::<ToggleMachine>::new(
            ToggleProps {
                id: String::from("toggle#btn-1"),
            },
            &Env::default(),
            &(),
        );

        service.unmount(Vec::new());

        let logs = capture_logs(|| {
            let result = service.send(ToggleEvent::Toggle);

            assert!(!result.state_changed);
            assert!(!result.context_changed);
            assert!(result.pending_effects.is_empty());
            assert!(result.cancel_effects.is_empty());
            assert!(!result.truncated);
            assert_eq!(result.context_change_count, 0);
        });

        let dropped = logs
            .iter()
            .find(|record| record.level == log::Level::Debug)
            .expect("expected dropped-event debug log");

        assert!(
            dropped
                .message
                .contains("[ars:toggle#btn-1] dropped event after unmount: Toggle"),
            "missing dropped-event diagnostic: {dropped:?}"
        );
    }

    #[cfg(all(feature = "debug", not(debug_assertions)))]
    #[test]
    fn iteration_limit_truncation_emits_warning_in_release_builds() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum LoopEvent {
            Continue,
        }

        struct IterationLoopMachine;

        impl Machine for IterationLoopMachine {
            type State = ToggleState;
            type Event = LoopEvent;
            type Context = ToggleContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, ToggleContext)
            }

            fn transition(
                state: &Self::State,
                event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                match (state, event) {
                    (ToggleState::Off, LoopEvent::Continue) => {
                        Some(TransitionPlan::to(ToggleState::On).then(LoopEvent::Continue))
                    }

                    (ToggleState::On, LoopEvent::Continue) => {
                        Some(TransitionPlan::to(ToggleState::Off).then(LoopEvent::Continue))
                    }
                }
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let mut service = Service::<IterationLoopMachine>::new(
            ToggleProps {
                id: String::from("loop#iteration"),
            },
            &Env::default(),
            &(),
        );

        let logs = capture_logs(|| {
            let result = service.send(LoopEvent::Continue);

            assert!(result.truncated);
            assert!(result.state_changed);
            assert!(!result.context_changed);
            assert_eq!(result.context_change_count, 0);
        });

        let warning = logs
            .iter()
            .find(|record| record.level == log::Level::Warn)
            .expect("expected truncation warning log");

        assert!(
            warning
                .message
                .contains("event queue exceeded 100 iterations, truncating"),
            "missing iteration-limit warning: {warning:?}"
        );
    }

    #[cfg(all(feature = "debug", not(debug_assertions)))]
    #[test]
    fn context_only_truncation_emits_warning_in_release_builds() {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum LoopEvent {
            Continue,
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        struct LoopContext {
            ticks: usize,
        }

        struct ContextLoopMachine;

        impl Machine for ContextLoopMachine {
            type State = ToggleState;
            type Event = LoopEvent;
            type Context = LoopContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, LoopContext { ticks: 0 })
            }

            fn transition(
                _state: &Self::State,
                event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                match event {
                    LoopEvent::Continue => Some(
                        TransitionPlan::context_only(|ctx: &mut LoopContext| ctx.ticks += 1)
                            .then(LoopEvent::Continue),
                    ),
                }
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let mut service = Service::<ContextLoopMachine>::new(
            ToggleProps {
                id: String::from("loop#context"),
            },
            &Env::default(),
            &(),
        );

        let logs = capture_logs(|| {
            let result = service.send(LoopEvent::Continue);

            assert!(result.truncated);
            assert!(!result.state_changed);
            assert!(result.context_changed);
            assert_eq!(result.context_change_count, 100);
        });

        let warning = logs
            .iter()
            .find(|record| record.level == log::Level::Warn)
            .expect("expected context_only truncation warning");

        assert!(
            warning
                .message
                .contains("100 context_only iterations without state change"),
            "missing context_only warning: {warning:?}"
        );
    }

    #[test]
    fn bindable_set_updates_internal_value_in_both_modes() {
        let mut uncontrolled = Bindable::uncontrolled(1_u8);

        uncontrolled.set(2);

        assert_eq!(uncontrolled.get(), &2);

        let mut controlled = Bindable::controlled(1_u8);

        controlled.set(2);

        assert_eq!(controlled.get(), &1);

        controlled.sync_controlled(None);

        assert_eq!(controlled.get(), &2);
    }

    #[test]
    fn bindable_sync_controlled_updates_only_the_controlled_value() {
        let mut b = Bindable::uncontrolled(10_u8);

        assert!(!b.is_controlled());

        b.sync_controlled(Some(20));

        assert!(b.is_controlled());
        assert_eq!(b.get(), &20);

        b.sync_controlled(None);

        assert!(!b.is_controlled());
        assert_eq!(b.get(), &10);
    }

    #[test]
    fn bindable_get_mut_owned_updates_uncontrolled_values_in_place() {
        let mut b = Bindable::uncontrolled(vec![String::from("a")]);

        b.get_mut_owned().push(String::from("b"));

        assert_eq!(b.get(), &vec![String::from("a"), String::from("b")]);
    }

    #[test]
    fn bindable_get_mut_owned_preserves_controlled_precedence_until_uncontrolled() {
        let mut b = Bindable::controlled(vec![String::from("controlled")]);

        b.get_mut_owned().push(String::from("pending"));

        assert_eq!(b.get(), &vec![String::from("controlled")]);

        b.sync_controlled(None);

        assert_eq!(
            b.get(),
            &vec![String::from("controlled"), String::from("pending")]
        );
    }

    #[test]
    fn pending_effect_new_bridges_strong_send_to_weak_send() {
        use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

        let observed = Arc::new(AtomicBool::new(false));

        let cleanup_calls = Arc::new(AtomicUsize::new(0));

        let effect = PendingEffect::<ToggleMachine>::new("notify", {
            let cleanup_calls = Arc::clone(&cleanup_calls);
            move |_ctx, _props, send| {
                send.call_if_alive(ToggleEvent::Toggle);

                let cleanup_calls = Arc::clone(&cleanup_calls);

                Box::new(move || {
                    cleanup_calls.fetch_add(1, Ordering::SeqCst);
                })
            }
        });

        let send: StrongSend<ToggleEvent> = {
            let observed = Arc::clone(&observed);

            Arc::new(move |_event| {
                observed.store(true, Ordering::SeqCst);
            })
        };

        let props = ToggleProps {
            id: String::from("toggle"),
        };

        let (_state, context) = ToggleMachine::init(&props, &Env::default(), &());

        let cleanup = effect.run(&context, &props, send);

        assert!(observed.load(Ordering::SeqCst));

        cleanup();

        assert_eq!(cleanup_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn pending_effect_named_is_noop_and_debuggable() {
        use alloc::format;

        let effect = PendingEffect::<ToggleMachine>::named("focus");

        assert_eq!(
            format!("{effect:?}"),
            "PendingEffect { name: \"focus\", target_state: None, setup: \"<closure>\" }"
        );

        let props = ToggleProps {
            id: String::from("toggle"),
        };

        let (_state, context) = ToggleMachine::init(&props, &Env::default(), &());

        let send: StrongSend<ToggleEvent> = Arc::new(|_| {});

        let cleanup = effect.run(&context, &props, send);

        cleanup();
    }

    #[test]
    fn transition_plan_helpers_record_effects_and_debug_state() {
        use alloc::format;

        let plan = TransitionPlan::<ToggleMachine>::new()
            .then(ToggleEvent::Toggle)
            .with_named_effect("announce", |_ctx, _props, _send| no_cleanup())
            .cancel_effect("timer");

        assert_eq!(plan.debug_summary(), "none");
        assert_eq!(plan.then_send, vec![ToggleEvent::Toggle]);
        assert_eq!(plan.effects.len(), 1);
        assert_eq!(plan.effects[0].name, "announce");
        assert_eq!(plan.cancel_effects, vec!["timer"]);
        assert_eq!(
            format!("{plan:?}"),
            "TransitionPlan { target: None, apply: \"None\", apply_description: None, then_send: [Toggle], effects: [\"announce\"], cancel_effects: [\"timer\"] }"
        );
    }

    #[test]
    fn transition_plan_apply_chains_mutations_in_order() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct ChainContext {
            values: Vec<u8>,
        }

        struct ApplyMachine;

        impl Machine for ApplyMachine {
            type State = ToggleState;
            type Event = ToggleEvent;
            type Context = ChainContext;
            type Props = ToggleProps;
            type Messages = ();
            type Api<'a> = ToggleApi;

            fn init(
                _props: &Self::Props,
                _env: &Env,
                _messages: &Self::Messages,
            ) -> (Self::State, Self::Context) {
                (ToggleState::Off, ChainContext { values: Vec::new() })
            }

            fn transition(
                _state: &Self::State,
                _event: &Self::Event,
                _context: &Self::Context,
                _props: &Self::Props,
            ) -> Option<TransitionPlan<Self>> {
                Some(
                    TransitionPlan::new()
                        .apply(|ctx: &mut ChainContext| ctx.values.push(1))
                        .apply(|ctx: &mut ChainContext| ctx.values.push(2)),
                )
            }

            fn connect<'a>(
                _state: &'a Self::State,
                _context: &'a Self::Context,
                _props: &'a Self::Props,
                _send: &'a dyn Fn(Self::Event),
            ) -> Self::Api<'a> {
                ToggleApi
            }
        }

        let mut service = Service::<ApplyMachine>::new(
            ToggleProps {
                id: String::from("apply"),
            },
            &Env::default(),
            &(),
        );

        assert_eq!(
            service.connect(&|_| {}).part_attrs(TogglePart),
            AttrMap::new()
        );

        let result = service.send(ToggleEvent::Toggle);

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.context().values, vec![1, 2]);
    }

    #[test]
    fn transition_plan_default_matches_new_and_formats_apply_closures() {
        use alloc::format;

        let default_plan = TransitionPlan::<ToggleMachine>::default();
        let new_plan = TransitionPlan::<ToggleMachine>::new();
        let apply_plan = TransitionPlan::<ToggleMachine>::context_only(|_ctx| {});

        assert_eq!(default_plan.debug_summary(), "none");
        assert_eq!(format!("{default_plan:?}"), format!("{new_plan:?}"));
        assert!(format!("{apply_plan:?}").contains("apply: \"<closure>\""));
    }

    #[test]
    fn transition_plan_debug_summary_distinguishes_plan_kinds() {
        assert_eq!(
            TransitionPlan::<ToggleMachine>::new().debug_summary(),
            "none"
        );
        assert_eq!(
            TransitionPlan::<ToggleMachine>::context_only(|_ctx| {}).debug_summary(),
            "context_only"
        );
        assert_eq!(
            TransitionPlan::<ToggleMachine>::to(ToggleState::On).debug_summary(),
            "to"
        );
    }

    #[test]
    fn companion_stylesheet_contains_required_utility_classes() {
        let css = include_str!("../ars-base.css");

        assert!(css.contains(".ars-visually-hidden"));
        assert!(
            css.contains("[data-ars-visually-hidden-focusable]:not(:focus):not(:focus-within)")
        );
        assert!(css.contains(".ars-sr-input"));
        assert!(css.contains(".ars-touch-none"));
    }

    #[test]
    fn toggle_props_has_id_helpers_replace_and_mutate_ids() {
        let props = ToggleProps {
            id: String::from("before"),
        };

        let replaced = props.clone().with_id(String::from("after"));

        assert_eq!(props.id(), "before");
        assert_eq!(replaced.id(), "after");

        let mut mutated = props;

        mutated.set_id(String::from("mutated"));

        assert_eq!(mutated.id(), "mutated");
    }

    #[test]
    fn component_part_data_attrs_include_scope_and_name() {
        assert_eq!(
            TogglePart::ROOT.data_attrs(),
            [
                (HtmlAttr::Data("ars-scope"), "toggle"),
                (HtmlAttr::Data("ars-part"), "root"),
            ]
        );
        assert_eq!(TogglePart::all(), vec![TogglePart]);
    }

    #[test]
    fn env_debug_includes_locale_and_backend_placeholder() {
        let debug = format!("{:?}", Env::default());

        assert!(debug.contains("Env"));
        assert!(debug.contains("en-US"));
        assert!(debug.contains("Arc(..)"));
    }

    #[test]
    fn send_result_debug_lists_effect_names() {
        let result = SendResult::<ToggleMachine> {
            state_changed: true,
            context_changed: false,
            pending_effects: vec![
                PendingEffect::named("focus"),
                PendingEffect::named("announce"),
            ],
            cancel_effects: vec!["timer"],
            truncated: false,
            context_change_count: 0,
        };

        let debug = format!("{result:?}");

        assert!(debug.contains("SendResult"));
        assert!(debug.contains("focus"));
        assert!(debug.contains("announce"));
        assert!(debug.contains("timer"));
    }

    #[cfg(feature = "embedded-css")]
    #[test]
    fn embedded_companion_stylesheet_matches_sidecar_file() {
        assert_eq!(companion_css::ARS_BASE_CSS, include_str!("../ars-base.css"));
    }

    #[test]
    fn no_cleanup_is_callable() {
        let cleanup = no_cleanup();

        cleanup();
    }
}
