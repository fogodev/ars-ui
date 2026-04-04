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
//! - [`TransitionPlan`] — declarative transition result with optional state, context, and effects

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt::Debug;

pub mod companion_css;
mod connect;

pub use connect::{
    AriaAttr, AttrMap, AttrMapParts, AttrValue, CssProperty, EventOptions, HtmlAttr, HtmlEvent,
    StyleStrategy, UserAttrs, data,
};

/// A named side effect produced by a state transition.
///
/// Pending effects are returned from [`Service::send`] after a transition is applied.
/// The framework adapter is responsible for executing them (e.g. focusing an element,
/// starting a timer, announcing to screen readers).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingEffect<E> {
    /// The identifier for this effect, used by adapters to match and execute it.
    pub name: &'static str,
    _event: core::marker::PhantomData<E>,
}

impl<E> PendingEffect<E> {
    /// Creates a new pending effect with the given name.
    #[must_use]
    pub fn named(name: &'static str) -> Self {
        Self {
            name,
            _event: core::marker::PhantomData,
        }
    }
}

/// The result of a state machine transition, describing what should change.
///
/// A transition plan may update the state, replace the context, and/or schedule
/// side effects. Returning `None` from [`Machine::transition`] means the event
/// is ignored; returning a plan with `target: None` means effects-only (no state change).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionPlan<S, E, C> {
    /// The new state to transition to, or `None` to remain in the current state.
    pub target: Option<S>,
    /// Replacement context, or `None` to keep the current context.
    pub context: Option<C>,
    /// Side effects to execute after the transition is applied.
    pub effects: Vec<PendingEffect<E>>,
}

impl<S, E, C> TransitionPlan<S, E, C> {
    /// Creates a transition plan that moves to `target` with no context change or effects.
    #[must_use]
    pub fn new(target: Option<S>) -> Self {
        Self {
            target,
            context: None,
            effects: Vec::new(),
        }
    }

    /// Adds a context replacement to this transition plan.
    #[must_use]
    pub fn with_context(mut self, context: C) -> Self {
        self.context = Some(context);
        self
    }

    /// Appends a side effect to this transition plan.
    #[must_use]
    pub fn with_effect(mut self, effect: PendingEffect<E>) -> Self {
        self.effects.push(effect);
        self
    }
}

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

/// A named DOM part of a component (e.g. root, trigger, content, label).
///
/// Each component defines an enum of its parts that implements this trait,
/// typically via `#[derive(ComponentPart)]`. The connect API uses parts to
/// produce the correct [`AttrMap`] for each element in the component's DOM tree.
pub trait ComponentPart: Clone + Debug + PartialEq + Eq + core::hash::Hash + 'static {
    /// Returns the root part of this component.
    fn root() -> Self;
    /// Returns the string name of this part (e.g. `"root"`, `"trigger"`).
    fn name(&self) -> &'static str;
    /// Returns all parts defined for this component.
    fn all() -> Vec<Self>;
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

/// Defines a component as a finite state machine.
///
/// A `Machine` declares the component's state type, event type, internal context,
/// props, and connect API. It provides pure functions for initialization, transition
/// logic, and DOM attribute generation — with no framework dependency.
pub trait Machine {
    /// The state type representing the machine's current configuration.
    type State: Clone + Debug + PartialEq;
    /// The event type that triggers state transitions.
    type Event: Clone + Debug;
    /// Internal context accumulated across transitions (e.g. focused index, scroll offset).
    type Context: Clone + Debug;
    /// External configuration passed in by the parent component.
    type Props: Clone + PartialEq;
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
    ) -> Option<TransitionPlan<Self::State, Self::Event, Self::Context>>;

    /// Creates the connect API for producing DOM attributes from the current state.
    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a>;
}

/// A running instance of a [`Machine`] that manages state, context, and props.
///
/// `Service` is the runtime counterpart to a `Machine` definition. It holds the
/// current state, applies transitions via [`send`](Service::send), and produces
/// connect APIs via [`connect`](Service::connect). Framework adapters wrap a
/// `Service` in reactive signals to drive re-renders on state changes.
#[derive(Debug)]
pub struct Service<M: Machine> {
    state: M::State,
    context: M::Context,
    props: M::Props,
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

    /// Returns a reference to the current props.
    #[must_use]
    pub fn props(&self) -> &M::Props {
        &self.props
    }

    /// Returns a mutable reference to the current props.
    pub fn props_mut(&mut self) -> &mut M::Props {
        &mut self.props
    }

    /// Sends an event to the machine, applying any resulting transition.
    ///
    /// Returns the list of pending side effects that the adapter should execute.
    /// If the event is ignored (transition returns `None`), the returned list is empty.
    #[must_use]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "spec-defined by-value signature; event ownership needed for future effect dispatch"
    )]
    pub fn send(&mut self, event: M::Event) -> Vec<PendingEffect<M::Event>> {
        let Some(plan) = M::transition(&self.state, &event, &self.context, &self.props) else {
            return Vec::new();
        };

        if let Some(target) = plan.target {
            self.state = target;
        }

        if let Some(context) = plan.context {
            self.context = context;
        }

        plan.effects
    }

    /// Creates a connect API snapshot for producing DOM attributes.
    ///
    /// The `send` closure is used by the API to wire event handlers. The returned
    /// API borrows from this service and must not outlive it.
    pub fn connect<'a>(&'a self, send: &'a dyn Fn(M::Event)) -> M::Api<'a> {
        M::connect(&self.state, &self.context, &self.props, send)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

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
    struct ToggleProps;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    struct TogglePart;

    impl ComponentPart for TogglePart {
        fn root() -> Self {
            Self
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
        ) -> Option<TransitionPlan<Self::State, Self::Event, Self::Context>> {
            match (state, event) {
                (ToggleState::Off, ToggleEvent::Toggle) => {
                    Some(TransitionPlan::new(Some(ToggleState::On)))
                }
                (ToggleState::On, ToggleEvent::Toggle) => {
                    Some(TransitionPlan::new(Some(ToggleState::Off)))
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
        let mut service = Service::<ToggleMachine>::new(ToggleProps);
        assert_eq!(service.state(), &ToggleState::Off);

        let effects = service.send(ToggleEvent::Toggle);
        assert!(effects.is_empty());
        assert_eq!(service.state(), &ToggleState::On);
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
}
