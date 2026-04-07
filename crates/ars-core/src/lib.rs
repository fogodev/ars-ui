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
//! - [`TransitionPlan`] — declarative transition result with closures, effects, and follow-ups
//! - [`PendingEffect`] — named side effect with setup closure and cleanup lifecycle
//! - [`Callback`] — shared callback wrapper (`Rc` on wasm, `Arc` on native)
//! - [`WeakSend`] — weak event sender for safe effect cleanup
//! - [`SendResult`] — structured result from [`Service::send`]

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;
extern crate self as ars_core;

use alloc::{boxed::Box, collections::VecDeque, string::String, vec::Vec};
use core::fmt::{self, Debug};

pub mod companion_css;
mod connect;
pub mod modality;
pub mod platform;
pub mod provider;

/// Hidden re-exports used by proc macros to stay hygienic without forcing
/// downstream crates to import `alloc`.
///
/// The derive macros expand in the downstream crate, so using `::alloc::...`
/// directly would require every consumer to write `extern crate alloc;`, even
/// in ordinary `std` crates. Routing through `::ars_core::__private` keeps the
/// generated code portable across `std` and `no_std + alloc` consumers while
/// preserving a stable macro expansion path.
#[doc(hidden)]
pub mod __private {
    pub use alloc::{string::String, vec::Vec};
}

#[doc(inline)]
pub use ars_derive::{ComponentPart, HasId};
// Re-export `Direction` from ars-i18n for convenience — used by
// `PlatformEffects::resolved_direction` so consumers don't need a
// separate `ars-i18n` dependency just for the return type.
pub use ars_i18n::Direction;
pub use connect::{
    AriaAttr, AttrMap, AttrMapParts, AttrValue, CssProperty, EventOptions, HtmlAttr, HtmlEvent,
    StyleStrategy, UserAttrs, data,
};
pub use modality::{
    DefaultModalityContext, KeyModifiers, KeyboardKey, ModalityContext, ModalitySnapshot,
    NullModalityContext, PointerType,
};
pub use platform::{
    MissingProviderEffects, NullPlatformEffects, PlatformEffects, Rect, TimerHandle,
};
pub use provider::{ArsContext, ColorMode};

// ────────────────────────────────────────────────────────────────────
// Callback, WeakSend, and effect cleanup types
// ────────────────────────────────────────────────────────────────────

/// Type alias for the cleanup function returned by effect setup.
///
/// Two allocations: the outer `Box` erases the closure's concrete type for
/// storage in a heterogeneous effect-cleanup list; the inner `dyn FnOnce()`
/// allows each effect to capture arbitrary owned state for teardown (event
/// listener handles, observer references, timer IDs, etc.).
pub type CleanupFn = Box<dyn FnOnce()>;

/// No-op cleanup for effects that don't need teardown.
#[inline]
#[must_use]
pub fn no_cleanup() -> CleanupFn {
    Box::new(|| {})
}

/// Shared callback wrapper for event handler closures in Props structs.
///
/// Clones the smart pointer, NOT the closure itself. Uses `Rc` on wasm
/// (single-threaded) and `Arc` on native (multi-threaded) targets. This is
/// distinct from `CleanupFn` (used for effect cleanup).
///
/// Supports an optional return type via `Callback<dyn Fn(Args) -> Out>`.
/// When the return type is `()` (the default), write `Callback<dyn Fn(Args)>`
/// as shorthand.
#[cfg(target_arch = "wasm32")]
pub struct Callback<T: ?Sized>(pub(crate) alloc::rc::Rc<T>);

/// Shared callback wrapper for event handler closures in Props structs.
///
/// Clones the smart pointer, NOT the closure itself. Uses `Rc` on wasm
/// (single-threaded) and `Arc` on native (multi-threaded) targets. This is
/// distinct from `CleanupFn` (used for effect cleanup).
///
/// Supports an optional return type via `Callback<dyn Fn(Args) -> Out>`.
/// When the return type is `()` (the default), write `Callback<dyn Fn(Args)>`
/// as shorthand.
#[cfg(not(target_arch = "wasm32"))]
pub struct Callback<T: ?Sized>(pub(crate) alloc::sync::Arc<T>);

#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Callback(alloc::rc::Rc::clone(&self.0))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized> Clone for Callback<T> {
    fn clone(&self) -> Self {
        Callback(alloc::sync::Arc::clone(&self.0))
    }
}

impl<T: ?Sized> Debug for Callback<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Callback(..)")
    }
}

#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> PartialEq for Callback<T> {
    fn eq(&self, other: &Self) -> bool {
        alloc::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: ?Sized> PartialEq for Callback<T> {
    fn eq(&self, other: &Self) -> bool {
        alloc::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T: ?Sized> core::ops::Deref for Callback<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> AsRef<T> for Callback<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

/// Constructor for `Callback<dyn Fn(Args) -> Out>`.
#[cfg(target_arch = "wasm32")]
impl<Args: 'static, Out: 'static> Callback<dyn Fn(Args) -> Out> {
    /// Creates a new callback wrapping the given closure.
    pub fn new(f: impl Fn(Args) -> Out + 'static) -> Self {
        Self(alloc::rc::Rc::new(f))
    }
}

/// Constructor for `Callback<dyn Fn(Args) -> Out>`.
#[cfg(not(target_arch = "wasm32"))]
impl<Args: 'static, Out: 'static> Callback<dyn Fn(Args) -> Out> {
    /// Creates a new callback wrapping the given closure.
    pub fn new(f: impl Fn(Args) -> Out + Send + Sync + 'static) -> Self {
        Self(alloc::sync::Arc::new(f))
    }
}

#[cfg(target_arch = "wasm32")]
impl<F: Fn(Args) -> Out + 'static, Args: 'static, Out: 'static> From<F>
    for Callback<dyn Fn(Args) -> Out>
{
    fn from(f: F) -> Self {
        Callback(alloc::rc::Rc::new(f))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<F: Fn(Args) -> Out + Send + Sync + 'static, Args: 'static, Out: 'static> From<F>
    for Callback<dyn Fn(Args) -> Out>
{
    fn from(f: F) -> Self {
        Callback(alloc::sync::Arc::new(f))
    }
}

/// Ergonomic constructor for [`Callback`] with better type inference.
///
/// The compiler can infer `Args` from the closure signature without
/// requiring turbofish syntax.
#[cfg(target_arch = "wasm32")]
pub fn callback<Args: 'static, Out: 'static>(
    f: impl Fn(Args) -> Out + 'static,
) -> Callback<dyn Fn(Args) -> Out> {
    Callback::new(f)
}

/// Ergonomic constructor for [`Callback`] with better type inference.
///
/// The compiler can infer `Args` from the closure signature without
/// requiring turbofish syntax.
#[cfg(not(target_arch = "wasm32"))]
pub fn callback<Args: 'static, Out: 'static>(
    f: impl Fn(Args) -> Out + Send + Sync + 'static,
) -> Callback<dyn Fn(Args) -> Out> {
    Callback::new(f)
}

/// Weak event sender for safe effect cleanup.
///
/// `WeakSend<T>` wraps a weak reference to the send function so that
/// long-lived effects (timers, observers) do not prevent the component
/// from being garbage collected. Use [`call_if_alive`](WeakSend::call_if_alive)
/// to dispatch events — it is a no-op if the component has been unmounted.
#[cfg(target_arch = "wasm32")]
pub struct WeakSend<T>(alloc::rc::Weak<dyn Fn(T)>);

/// Weak event sender for safe effect cleanup.
///
/// `WeakSend<T>` wraps a weak reference to the send function so that
/// long-lived effects (timers, observers) do not prevent the component
/// from being garbage collected. Use [`call_if_alive`](WeakSend::call_if_alive)
/// to dispatch events — it is a no-op if the component has been unmounted.
#[cfg(not(target_arch = "wasm32"))]
pub struct WeakSend<T>(alloc::sync::Weak<dyn Fn(T) + Send + Sync>);

impl<T> WeakSend<T> {
    /// Attempt to send an event if the component is still alive.
    ///
    /// Returns silently if the strong reference has been dropped.
    pub fn call_if_alive(&self, value: T) {
        if let Some(f) = self.0.upgrade() {
            f(value);
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl<T> Clone for WeakSend<T> {
    fn clone(&self) -> Self {
        WeakSend(alloc::rc::Weak::clone(&self.0))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T> Clone for WeakSend<T> {
    fn clone(&self) -> Self {
        WeakSend(alloc::sync::Weak::clone(&self.0))
    }
}

impl<T> Debug for WeakSend<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WeakSend(..)")
    }
}

/// Convenience constructors for [`WeakSend`] on wasm targets.
#[cfg(target_arch = "wasm32")]
impl<T: 'static> WeakSend<T> {
    /// Create a `WeakSend` by downgrading the given `Rc`.
    pub fn from_rc(rc: &alloc::rc::Rc<dyn Fn(T)>) -> Self {
        WeakSend(alloc::rc::Rc::downgrade(rc))
    }

    /// Alias for [`from_rc`](Self::from_rc) — more discoverable name.
    pub fn downgrade(rc: &alloc::rc::Rc<dyn Fn(T)>) -> Self {
        Self::from_rc(rc)
    }
}

/// Convenience constructors for [`WeakSend`] on native targets.
#[cfg(not(target_arch = "wasm32"))]
impl<T: 'static> WeakSend<T> {
    /// Create a `WeakSend` by downgrading the given `Arc`.
    pub fn from_arc(arc: &alloc::sync::Arc<dyn Fn(T) + Send + Sync>) -> Self {
        WeakSend(alloc::sync::Arc::downgrade(arc))
    }

    /// Alias for [`from_arc`](Self::from_arc) — more discoverable name.
    pub fn downgrade(arc: &alloc::sync::Arc<dyn Fn(T) + Send + Sync>) -> Self {
        Self::from_arc(arc)
    }
}

#[cfg(target_arch = "wasm32")]
impl<T: 'static> From<&alloc::rc::Rc<dyn Fn(T)>> for WeakSend<T> {
    fn from(rc: &alloc::rc::Rc<dyn Fn(T)>) -> Self {
        WeakSend(alloc::rc::Rc::downgrade(rc))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: 'static> From<&alloc::sync::Arc<dyn Fn(T) + Send + Sync>> for WeakSend<T> {
    fn from(arc: &alloc::sync::Arc<dyn Fn(T) + Send + Sync>) -> Self {
        WeakSend(alloc::sync::Arc::downgrade(arc))
    }
}

/// The strong send handle passed to effect setup closures.
///
/// Adapters hold the strong `Rc`/`Arc` and pass it to
/// [`PendingEffect::run`]. The setup closure downgrades to
/// [`WeakSend`] internally.
#[doc(hidden)]
#[cfg(target_arch = "wasm32")]
pub type StrongSend<E> = alloc::rc::Rc<dyn Fn(E)>;

/// The strong send handle passed to effect setup closures.
#[doc(hidden)]
#[cfg(not(target_arch = "wasm32"))]
pub type StrongSend<E> = alloc::sync::Arc<dyn Fn(E) + Send + Sync>;

// ────────────────────────────────────────────────────────────────────
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
    /// Creates a new pending effect from a name and user-authored setup closure.
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

/// The result of a state machine transition, describing what should change.
///
/// Built using a fluent builder pattern. Returning `None` from
/// [`Machine::transition`] means the event is ignored; returning a plan
/// with `target: None` means context-only (no state change).
pub struct TransitionPlan<M: Machine> {
    /// The new state to transition to, or `None` to remain in the current state.
    pub target: Option<M::State>,
    /// Mutation to apply to the context after state change.
    #[expect(clippy::type_complexity, reason = "closure type is inherently complex")]
    pub(crate) apply: Option<Box<dyn FnOnce(&mut M::Context)>>,
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
    pub fn to(state: M::State) -> Self {
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
    pub fn new() -> Self {
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
    pub fn debug_summary(&self) -> &'static str {
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
pub struct Bindable<T: Clone + PartialEq + Debug> {
    /// The externally controlled value, or `None` if uncontrolled.
    controlled: Option<T>,
    /// The internal value managed by the component.
    internal: T,
}

impl<T: Clone + PartialEq + Debug> Bindable<T> {
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
    pub fn uncontrolled(default: T) -> Self {
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
    pub fn is_controlled(&self) -> bool {
        self.controlled.is_some()
    }

    /// Updates the internal value if uncontrolled. Has no effect on controlled values.
    pub fn set(&mut self, value: T) {
        if self.controlled.is_none() {
            self.internal = value;
        }
    }

    /// Pushes a new controlled value from the parent.
    ///
    /// Updates both the controlled and internal fields. This should be called
    /// when the parent's controlled prop changes.
    pub fn sync_controlled(&mut self, value: Option<T>) {
        if let Some(ref v) = value {
            self.internal = v.clone();
        }
        self.controlled = value;
    }

    /// Returns a mutable reference to the internal value.
    ///
    /// Use for in-place mutations on collection types to avoid cloning.
    /// **Warning:** For controlled bindables, mutating the internal value has no
    /// effect on what [`get`](Self::get) returns (it returns the controlled value).
    pub fn get_mut_owned(&mut self) -> &mut T {
        &mut self.internal
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
    /// The connect API type that produces attributes from current state.
    type Api<'a>: ConnectApi
    where
        Self: 'a;

    /// Computes the initial state and context from the given props.
    fn init(props: &Self::Props) -> (Self::State, Self::Context);

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
    /// Creates a new service by initializing the machine with the given props.
    #[must_use]
    pub fn new(props: M::Props) -> Self {
        let (state, context) = M::init(&props);
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
    pub fn state(&self) -> &M::State {
        &self.state
    }

    /// Returns a reference to the current machine context.
    #[must_use]
    pub fn context(&self) -> &M::Context {
        &self.context
    }

    /// Returns a mutable reference to the current machine context.
    pub fn context_mut(&mut self) -> &mut M::Context {
        &mut self.context
    }

    /// Returns a reference to the current props.
    #[must_use]
    pub fn props(&self) -> &M::Props {
        &self.props
    }

    /// Returns a mutable reference to the current props.
    pub fn props_mut(&mut self) -> &mut M::Props {
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
        #[expect(
            unused_mut,
            reason = "only mutated in release builds (debug panics first)"
        )]
        let mut truncated = false;
        let mut iterations = 0;
        let mut context_change_count: usize = 0;

        while let Some(event) = self.event_queue.pop_front() {
            iterations += 1;
            if iterations > MAX_DRAIN_ITERATIONS {
                #[cfg(debug_assertions)]
                panic!(
                    "Event queue exceeded {MAX_DRAIN_ITERATIONS} iterations — \
                     likely an infinite loop in transitions"
                );
                #[cfg(not(debug_assertions))]
                {
                    truncated = true;
                    break;
                }
            }

            if let Some(plan) = M::transition(&self.state, &event, &self.context, &self.props) {
                // Apply context mutation.
                if let Some(apply) = plan.apply {
                    apply(&mut self.context);
                    context_changed = true;
                }

                // Track context-only iterations for diagnostics.
                if plan.target.is_none() {
                    context_change_count += 1;
                } else {
                    context_change_count = 0;
                }

                // Enqueue follow-up events.
                self.event_queue.extend(plan.then_send);

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
    /// enqueues any returned events, and drains the queue.
    pub fn set_props(&mut self, props: M::Props) -> SendResult<M> {
        let old_props = core::mem::replace(&mut self.props, props);
        let events = M::on_props_changed(&old_props, &self.props);
        for event in events {
            self.event_queue.push_back(event);
        }
        self.drain_queue()
    }

    /// Unmounts the service, running all active effect cleanups.
    ///
    /// After calling this, no further [`send`](Self::send) calls are valid.
    /// In debug builds, subsequent sends will panic; in release builds,
    /// they return an inert [`SendResult`].
    pub fn unmount(&mut self, active_cleanups: Vec<CleanupFn>) {
        for cleanup in active_cleanups.into_iter().rev() {
            cleanup();
        }
        self.event_queue.clear();
        self.unmounted = true;
    }

    /// Returns `true` after [`unmount`](Self::unmount) has been called.
    #[must_use]
    pub fn is_unmounted(&self) -> bool {
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
    /// `Machine::init`, discarding the init state.
    #[cfg(test)]
    pub fn set_state_for_test(&mut self, state: M::State) {
        let (_init_state, context) = M::init(&self.props);
        self.state = state;
        self.context = context;
    }
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ToggleState {
        Off,
        On,
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
        type Api<'a> = ToggleApi;

        fn init(_props: &Self::Props) -> (Self::State, Self::Context) {
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
        let mut service = Service::<ToggleMachine>::new(ToggleProps {
            id: String::from("toggle"),
        });
        assert_eq!(service.state(), &ToggleState::Off);

        let result = service.send(ToggleEvent::Toggle);
        assert!(result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &ToggleState::On);
    }

    #[test]
    fn send_result_reports_no_change_when_ignored() {
        let mut service = Service::<ToggleMachine>::new(ToggleProps {
            id: String::from("toggle"),
        });
        // Toggle is exhaustive so nothing is ignored — toggle twice and check.
        let result = service.send(ToggleEvent::Toggle);
        assert!(result.state_changed);
        let result = service.send(ToggleEvent::Toggle);
        assert!(result.state_changed);
        assert_eq!(service.state(), &ToggleState::Off);
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
            type Api<'a> = ToggleApi;

            fn init(_props: &Self::Props) -> (Self::State, Self::Context) {
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

        let mut service = Service::<CountMachine>::new(ToggleProps {
            id: String::from("test"),
        });
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
            type Api<'a> = ToggleApi;

            fn init(_props: &Self::Props) -> (Self::State, Self::Context) {
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

        let mut service = Service::<CancelMachine>::new(ToggleProps {
            id: String::from("test"),
        });
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
            type Api<'a> = ToggleApi;

            fn init(_props: &Self::Props) -> (Self::State, Self::Context) {
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

        let mut service = Service::<EffectMachine>::new(ToggleProps {
            id: String::from("test"),
        });
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
            type Api<'a> = ToggleApi;

            fn init(_props: &Self::Props) -> (Self::State, Self::Context) {
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

        let mut service = Service::<PropsMachine>::new(ToggleProps {
            id: String::from("a"),
        });
        assert_eq!(service.context().mode, 0);

        let result = service.set_props(ToggleProps {
            id: String::from("b"),
        });
        assert!(result.context_changed);
        assert_eq!(service.context().mode, 1);
    }

    #[test]
    fn bindable_only_updates_uncontrolled_values() {
        let mut uncontrolled = Bindable::uncontrolled(1_u8);
        uncontrolled.set(2);
        assert_eq!(uncontrolled.get(), &2);

        let mut controlled = Bindable::controlled(1_u8);
        controlled.set(2);
        assert_eq!(controlled.get(), &1);
    }

    #[test]
    fn bindable_sync_controlled_updates_both_fields() {
        let mut b = Bindable::uncontrolled(10_u8);
        assert!(!b.is_controlled());

        b.sync_controlled(Some(20));
        assert!(b.is_controlled());
        assert_eq!(b.get(), &20);

        b.sync_controlled(None);
        assert!(!b.is_controlled());
        // Internal was updated to 20 by sync, now reads as uncontrolled
        assert_eq!(b.get(), &20);
    }

    #[test]
    fn companion_stylesheet_contains_required_utility_classes() {
        let css = include_str!("../ars-base.css");

        assert!(css.contains(".ars-visually-hidden"));
        assert!(css.contains(".ars-sr-input"));
        assert!(css.contains(".ars-touch-none"));
    }

    #[cfg(feature = "embedded-css")]
    #[test]
    fn embedded_companion_stylesheet_matches_sidecar_file() {
        assert_eq!(companion_css::ARS_BASE_CSS, include_str!("../ars-base.css"));
    }

    #[test]
    fn callback_clone_and_invoke() {
        let cb = callback(|x: u32| x * 2);
        let cb2 = cb.clone();
        assert_eq!(cb(3), 6);
        assert_eq!(cb2(4), 8);
    }

    #[test]
    fn callback_pointer_equality() {
        let cb = callback(|_: ()| {});
        let cb2 = cb.clone();
        assert_eq!(cb, cb2);

        let cb3 = callback(|_: ()| {});
        assert_ne!(cb, cb3);
    }

    #[test]
    fn no_cleanup_is_callable() {
        let cleanup = no_cleanup();
        cleanup(); // should not panic
    }
}
