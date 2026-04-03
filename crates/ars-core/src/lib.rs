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

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::fmt::Debug;

/// Map of HTML attribute names to their string values.
///
/// Used by [`ConnectApi::part_attrs`] to produce data-only attributes (ARIA, `data-*`,
/// inline styles) for each component part. Does not carry event handlers.
pub type AttrMap = BTreeMap<String, String>;

/// A named side effect produced by a state transition.
///
/// Pending effects are returned from [`Service::send`] after a transition is applied.
/// The framework adapter is responsible for executing them (e.g. focusing an element,
/// starting a timer, announcing to screen readers).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingEffect<E> {
    /// The identifier for this effect, used by adapters to match and execute it.
    pub name: String,
    _event: core::marker::PhantomData<E>,
}

impl<E> PendingEffect<E> {
    /// Creates a new pending effect with the given name.
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
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
/// values owned by the parent ([`Controlled`](Bindable::Controlled)) and values
/// managed by the component itself ([`Uncontrolled`](Bindable::Uncontrolled)).
/// Calling [`set`](Bindable::set) on a controlled value is a no-op.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bindable<T> {
    /// The parent owns this value; internal [`set`](Bindable::set) calls are ignored.
    Controlled(T),
    /// The component owns this value; internal [`set`](Bindable::set) calls update it.
    Uncontrolled(T),
}

impl<T> Bindable<T> {
    /// Creates a controlled bindable owned by the parent.
    #[must_use]
    pub fn controlled(value: T) -> Self {
        Self::Controlled(value)
    }

    /// Creates an uncontrolled bindable managed by the component.
    #[must_use]
    pub fn uncontrolled(value: T) -> Self {
        Self::Uncontrolled(value)
    }

    /// Returns a reference to the current value regardless of control mode.
    #[must_use]
    pub fn get(&self) -> &T {
        match self {
            Self::Controlled(value) | Self::Uncontrolled(value) => value,
        }
    }

    /// Returns `true` if this value is controlled by the parent.
    #[must_use]
    pub fn is_controlled(&self) -> bool {
        matches!(self, Self::Controlled(_))
    }

    /// Updates the value if it is uncontrolled. Has no effect on controlled values.
    pub fn set(&mut self, value: T) {
        if let Self::Uncontrolled(current) = self {
            *current = value;
        }
    }
}

/// A named DOM part of a component (e.g. root, trigger, content, label).
///
/// Each component defines an enum of its parts that implements this trait,
/// typically via `#[derive(ComponentPart)]`. The connect API uses parts to
/// produce the correct [`AttrMap`] for each element in the component's DOM tree.
pub trait ComponentPart: Clone {
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
    type Event;
    /// Internal context accumulated across transitions (e.g. focused index, scroll offset).
    type Context: Clone + Default;
    /// External configuration passed in by the parent component.
    type Props;
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

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ToggleState {
        Off,
        On,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ToggleEvent {
        Toggle,
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct ToggleContext;

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct ToggleProps;

    #[derive(Clone, Debug)]
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
}
