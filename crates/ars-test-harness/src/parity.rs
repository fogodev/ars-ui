//! Shared adapter parity test case definitions.

use std::{
    any::{Any, type_name},
    collections::HashMap,
    fmt::{self, Debug},
    sync::Arc,
};

use crate::KeyboardKey;

#[derive(Clone)]
struct ErasedValue {
    value: Arc<dyn Any>,
    type_name: &'static str,
}

impl ErasedValue {
    fn new<T>(value: T) -> Self
    where
        T: 'static,
    {
        Self {
            value: Arc::new(value),
            type_name: type_name::<T>(),
        }
    }

    const fn type_name(&self) -> &'static str {
        self.type_name
    }

    fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.value.downcast_ref()
    }

    fn downcast_clone<T>(&self) -> Option<T>
    where
        T: Clone + 'static,
    {
        self.downcast_ref::<T>().cloned()
    }
}

/// Type-erased component props used by adapter parity test cases.
#[derive(Clone)]
pub struct Props {
    inner: ErasedValue,
}

impl Props {
    /// Wraps concrete component props for adapter-specific parity runners.
    #[must_use]
    pub fn from<T>(props: T) -> Self
    where
        T: 'static,
    {
        Self {
            inner: ErasedValue::new(props),
        }
    }

    /// Returns the concrete type name captured when the props were wrapped.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }

    /// Attempts to view the wrapped props as a concrete component props type.
    #[must_use]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.inner.downcast_ref()
    }

    /// Attempts to clone the wrapped props as a concrete component props type.
    #[must_use]
    pub fn downcast_clone<T>(&self) -> Option<T>
    where
        T: Clone + 'static,
    {
        self.inner.downcast_clone()
    }
}

impl Debug for Props {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Props")
            .field("type_name", &self.type_name())
            .finish_non_exhaustive()
    }
}

/// Type-erased machine event used by adapter parity test cases.
#[derive(Clone)]
pub struct Event {
    inner: ErasedValue,
}

impl Event {
    /// Wraps a concrete machine event for adapter-specific parity runners.
    #[must_use]
    pub fn from<T>(event: T) -> Self
    where
        T: 'static,
    {
        Self {
            inner: ErasedValue::new(event),
        }
    }

    /// Returns the concrete type name captured when the event was wrapped.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }

    /// Attempts to view the wrapped event as a concrete machine event type.
    #[must_use]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.inner.downcast_ref()
    }

    /// Attempts to clone the wrapped event as a concrete machine event type.
    #[must_use]
    pub fn downcast_clone<T>(&self) -> Option<T>
    where
        T: Clone + 'static,
    {
        self.inner.downcast_clone()
    }
}

impl Debug for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Event")
            .field("type_name", &self.type_name())
            .finish_non_exhaustive()
    }
}

/// A single cross-adapter parity test definition.
///
/// Both adapter test runners consume the same test case to verify static
/// attribute output after applying the same component props and machine events.
#[derive(Clone, Debug)]
pub struct ParityTestCase {
    /// Human-readable test name used in diagnostics.
    pub name: &'static str,

    /// Component family under test.
    pub component: ComponentType,

    /// Component props used to initialize the adapter component.
    pub props: Props,

    /// Machine events sent before checking the resulting attributes.
    pub events: Vec<Event>,

    /// Expected ARIA and HTML attributes on the root element after all events.
    pub expected_attrs: HashMap<&'static str, &'static str>,
}

/// Adapter-agnostic component family tag for parity test dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentType {
    /// Checkbox component parity target.
    Checkbox,

    /// Radio group component parity target.
    RadioGroup,

    /// Select component parity target.
    Select,

    /// Dialog component parity target.
    Dialog,

    /// Search input component parity target.
    SearchInput,

    /// Tabs component parity target.
    Tabs,
}

/// A DOM-level interaction parity test definition.
///
/// Adapter runners use this matrix form to execute equivalent click, keyboard,
/// focus, and text-entry sequences against Leptos and Dioxus components.
#[derive(Clone, Debug)]
pub struct InteractionTestCase {
    /// Human-readable test name used in diagnostics.
    pub name: &'static str,

    /// Component family under test.
    pub component: ComponentType,

    /// Ordered DOM interaction and assertion steps to execute.
    pub steps: Vec<TestStep>,

    /// Expected ARIA and HTML attributes after the interaction sequence.
    pub expected_attrs: HashMap<&'static str, &'static str>,
}

/// A single DOM-level interaction or assertion in an [`InteractionTestCase`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TestStep {
    /// Clicks the element matching the selector.
    Click(&'static str),

    /// Sends a keyboard press to the active or target element.
    Press(KeyboardKey),

    /// Asserts that the component is in its open state.
    AssertOpen,

    /// Asserts that the component is in its closed state.
    AssertClosed,

    /// Types text into the active text entry element.
    TypeText(&'static str),

    /// Moves focus to the element matching the selector.
    Focus(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct MockProps {
        id: &'static str,
    }

    #[derive(Clone, Debug, PartialEq)]
    enum MockEvent {
        Open,
    }

    #[test]
    fn parity_test_case_has_expected_fields() {
        let case = ParityTestCase {
            name: "select opens",
            component: ComponentType::Select,
            props: Props::from(MockProps { id: "select-1" }),
            events: vec![Event::from(MockEvent::Open)],
            expected_attrs: HashMap::from([("aria-expanded", "true")]),
        };

        assert_eq!(case.name, "select opens");
        assert_eq!(case.component, ComponentType::Select);
        assert_eq!(
            case.props.downcast_ref::<MockProps>(),
            Some(&MockProps { id: "select-1" })
        );
        assert_eq!(
            case.events[0].downcast_ref::<MockEvent>(),
            Some(&MockEvent::Open)
        );
        assert_eq!(case.expected_attrs.get("aria-expanded"), Some(&"true"));
    }

    #[test]
    fn component_type_has_initial_variants() {
        let variants = [
            ComponentType::Checkbox,
            ComponentType::RadioGroup,
            ComponentType::Select,
            ComponentType::Dialog,
            ComponentType::SearchInput,
            ComponentType::Tabs,
        ];

        assert_eq!(variants.len(), 6);
    }

    #[test]
    fn props_and_events_can_clone_concrete_values_for_runners() {
        let props = Props::from(MockProps { id: "select-1" });
        let event = Event::from(MockEvent::Open);

        assert!(props.type_name().ends_with("MockProps"));
        assert!(event.type_name().ends_with("MockEvent"));
        assert_eq!(
            props.downcast_clone::<MockProps>(),
            Some(MockProps { id: "select-1" })
        );
        assert_eq!(event.downcast_clone::<MockEvent>(), Some(MockEvent::Open));
        assert_eq!(props.downcast_clone::<String>(), None);
        assert_eq!(event.downcast_clone::<String>(), None);
    }

    #[test]
    fn props_and_events_debug_include_erased_type_name() {
        let props = Props::from(MockProps { id: "select-1" });
        let event = Event::from(MockEvent::Open);

        assert!(format!("{props:?}").contains("MockProps"));
        assert!(format!("{event:?}").contains("MockEvent"));
    }

    #[test]
    fn interaction_test_case_has_expected_fields() {
        let case = InteractionTestCase {
            name: "select opens with click",
            component: ComponentType::Select,
            steps: vec![
                TestStep::Click("[data-ars-part='trigger']"),
                TestStep::Press(KeyboardKey::Escape),
                TestStep::AssertClosed,
            ],
            expected_attrs: HashMap::from([("aria-expanded", "false")]),
        };

        assert_eq!(case.name, "select opens with click");
        assert_eq!(case.component, ComponentType::Select);
        assert_eq!(
            case.steps,
            vec![
                TestStep::Click("[data-ars-part='trigger']"),
                TestStep::Press(KeyboardKey::Escape),
                TestStep::AssertClosed,
            ]
        );
        assert_eq!(case.expected_attrs.get("aria-expanded"), Some(&"false"));
    }

    #[test]
    fn test_step_has_expected_variants() {
        let steps = [
            TestStep::Click("[data-ars-part='trigger']"),
            TestStep::Press(KeyboardKey::Enter),
            TestStep::AssertOpen,
            TestStep::AssertClosed,
            TestStep::TypeText("abc"),
            TestStep::Focus("[data-ars-part='input']"),
        ];

        assert_eq!(steps.len(), 6);
    }

    #[test]
    fn parity_types_are_clone_and_debug() {
        fn assert_clone_debug<T: Clone + Debug>() {}

        assert_clone_debug::<ParityTestCase>();
        assert_clone_debug::<InteractionTestCase>();
        assert_clone_debug::<ComponentType>();
        assert_clone_debug::<TestStep>();
    }
}
