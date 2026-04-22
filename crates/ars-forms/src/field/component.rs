//! Field component state machine and connect API.
//!
//! This module implements the framework-agnostic `Field` machine defined in
//! `spec/foundation/07-forms.md` §13. The machine is intentionally stateless
//! at the state-enum level and instead tracks required, validation, and ARIA
//! wiring details in its context.

use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, ComponentIds, ComponentPart, ConnectApi, Direction, Env, HtmlAttr,
    TransitionPlan,
};

use crate::validation::Error;

/// Single state for the field component machine.
///
/// `Field` is effectively stateless. All meaningful changes are stored in
/// [`Context`] and applied through context-only transitions.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The field is mounted and ready to expose its current context.
    Idle,
}

/// Events that update field component context.
#[derive(Clone, Debug)]
pub enum Event {
    /// Replaces the current validation errors.
    SetErrors(Vec<Error>),

    /// Clears all current validation errors.
    ClearErrors,

    /// Tracks whether a description part is rendered.
    SetHasDescription(bool),

    /// Synchronizes the disabled state from props.
    SetDisabled(bool),

    /// Synchronizes the invalid state from props.
    SetInvalid(bool),

    /// Synchronizes the readonly state from props.
    SetReadonly(bool),

    /// Synchronizes the required state from props.
    SetRequired(bool),

    /// Synchronizes the layout direction from props.
    SetDir(Option<Direction>),

    /// Tracks whether async validation is currently running.
    SetValidating(bool),
}

/// Mutable machine context for the field component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the field is required.
    pub required: bool,

    /// Whether the field is disabled.
    pub disabled: bool,

    /// Whether the field is read-only.
    pub readonly: bool,

    /// Whether the field is currently invalid.
    pub invalid: bool,

    /// Whether an async validator is currently running.
    pub validating: bool,

    /// The configured text direction for RTL-aware rendering.
    pub dir: Option<Direction>,

    /// Field-level validation errors.
    pub errors: Vec<Error>,

    /// Whether a description part is rendered and should be referenced by ARIA.
    pub has_description: bool,

    /// Stable IDs derived from the adapter-provided base ID.
    pub ids: ComponentIds,
}

/// Immutable configuration for a field machine instance.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the field root.
    ///
    /// This ID is immutable for the lifetime of a machine instance because
    /// [`Context::ids`] caches the derived part IDs during initialization.
    pub id: String,

    /// Whether the field is required.
    pub required: bool,

    /// Whether the field is disabled.
    pub disabled: bool,

    /// Whether the field is read-only.
    pub readonly: bool,

    /// Whether the field is invalid before error-driven state is applied.
    pub invalid: bool,

    /// The configured text direction for RTL-aware rendering.
    pub dir: Option<Direction>,
}

/// Framework-agnostic field component state machine.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = ();
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            State::Idle,
            Context {
                required: props.required,
                disabled: props.disabled,
                readonly: props.readonly,
                invalid: props.invalid,
                validating: false,
                dir: props.dir,
                errors: Vec::new(),
                has_description: false,
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "field::component::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.disabled != new.disabled {
            events.push(Event::SetDisabled(new.disabled));
        }

        if old.invalid != new.invalid {
            events.push(Event::SetInvalid(new.invalid));
        }

        if old.readonly != new.readonly {
            events.push(Event::SetReadonly(new.readonly));
        }

        if old.required != new.required {
            events.push(Event::SetRequired(new.required));
        }

        if old.dir != new.dir {
            events.push(Event::SetDir(new.dir));
        }

        events
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::SetErrors(errors) => {
                let errors = errors.clone();
                let base_invalid = props.invalid;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.errors = errors;
                    ctx.invalid = base_invalid || !ctx.errors.is_empty();
                }))
            }

            Event::ClearErrors => {
                let base_invalid = props.invalid;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.errors.clear();
                    ctx.invalid = base_invalid;
                }))
            }

            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = has_description;
                }))
            }

            Event::SetDisabled(disabled) => {
                let disabled = *disabled;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = disabled;
                }))
            }

            Event::SetInvalid(invalid) => {
                let invalid = *invalid;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.invalid = invalid || !ctx.errors.is_empty();
                }))
            }

            Event::SetReadonly(readonly) => {
                let readonly = *readonly;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.readonly = readonly;
                }))
            }

            Event::SetRequired(required) => {
                let required = *required;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.required = required;
                }))
            }

            Event::SetDir(dir) => {
                let dir = *dir;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.dir = dir;
                }))
            }

            Event::SetValidating(validating) => {
                let validating = *validating;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.validating = validating;
                }))
            }
        }
    }

    fn connect<'a>(
        _state: &'a Self::State,
        ctx: &'a Self::Context,
        _props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { ctx }
    }
}

/// Snapshot connect API for deriving field DOM attributes.
pub struct Api<'a> {
    ctx: &'a Context,
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api").field("ctx", &self.ctx).finish()
    }
}

/// Structural parts exposed by the field connect API.
#[derive(ComponentPart)]
#[scope = "field"]
pub enum Part {
    /// The root container element.
    Root,

    /// The visible label element.
    Label,

    /// The input-like element receiving ARIA wiring.
    Input,

    /// The optional descriptive text element.
    Description,

    /// The field-level error message element.
    ErrorMessage,
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input => self.input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}

impl<'a> Api<'a> {
    /// Returns attributes for the root field container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        if let Some(dir) = self.ctx.dir {
            attrs.set(HtmlAttr::Dir, dir.as_html_attr());
        }

        attrs
    }

    /// Returns attributes for the visible label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));
        attrs.set(HtmlAttr::For, self.ctx.ids.part("input"));
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        attrs
    }

    /// Returns attributes to apply to the input-like child element.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();

        attrs.set(HtmlAttr::Id, self.ctx.ids.part("input"));
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(
            HtmlAttr::Aria(AriaAttr::LabelledBy),
            self.ctx.ids.part("label"),
        );

        let mut described_by = Vec::new();

        if self.ctx.has_description {
            described_by.push(self.ctx.ids.part("description"));
        }

        if !self.ctx.errors.is_empty() {
            described_by.push(self.ctx.ids.part("error-message"));
        }

        if !described_by.is_empty() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                described_by.join(" "),
            );
        }

        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");

            if !self.ctx.errors.is_empty() {
                attrs.set(
                    HtmlAttr::Aria(AriaAttr::ErrorMessage),
                    self.ctx.ids.part("error-message"),
                );
            }
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        if self.ctx.validating {
            attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        }

        attrs
    }

    /// Returns attributes for the description element.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();

        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the field error message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs.set(HtmlAttr::Id, self.ctx.ids.part("error-message"));
        attrs.set(HtmlAttr::Role, "alert");
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        if self.ctx.errors.is_empty() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }
}

#[cfg(test)]
mod tests {
    use ars_core::{ConnectApi, Service};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "email".to_string(),
            ..Props::default()
        }
    }

    fn test_props_with_invalid() -> Props {
        Props {
            invalid: true,
            ..test_props()
        }
    }

    fn custom_error() -> Error {
        Error::custom("required", "Field is invalid")
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn field_init_default_props() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().required);
        assert!(!service.context().disabled);
        assert!(!service.context().readonly);
        assert!(!service.context().invalid);
        assert!(!service.context().validating);
        assert_eq!(service.context().dir, None);
        assert!(service.context().errors.is_empty());
        assert!(!service.context().has_description);
        assert_eq!(service.context().ids.id(), "email");
        assert_eq!(service.context().ids.part("input"), "email-input");
    }

    #[test]
    fn field_set_required_updates_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let result = service.send(Event::SetRequired(true));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert!(service.context().required);
    }

    #[test]
    fn field_set_disabled_updates_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let result = service.send(Event::SetDisabled(true));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert!(service.context().disabled);
    }

    #[test]
    fn field_set_readonly_updates_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let result = service.send(Event::SetReadonly(true));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert!(service.context().readonly);
    }

    #[test]
    fn field_on_props_changed_emits_events() {
        let old = Props {
            id: "email".to_string(),
            required: false,
            disabled: false,
            readonly: false,
            invalid: false,
            dir: None,
        };

        let new = Props {
            id: "email".to_string(),
            required: true,
            disabled: true,
            readonly: true,
            invalid: true,
            dir: Some(Direction::Rtl),
        };

        let events = <Machine as ars_core::Machine>::on_props_changed(&old, &new);

        assert_eq!(events.len(), 5);
        assert!(matches!(events[0], Event::SetDisabled(true)));
        assert!(matches!(events[1], Event::SetInvalid(true)));
        assert!(matches!(events[2], Event::SetReadonly(true)));
        assert!(matches!(events[3], Event::SetRequired(true)));
        assert!(matches!(events[4], Event::SetDir(Some(Direction::Rtl))));
    }

    #[test]
    fn field_on_props_changed_no_changes_emits_no_events() {
        let old = test_props();
        let new = test_props();

        let events = <Machine as ars_core::Machine>::on_props_changed(&old, &new);

        assert!(events.is_empty());
    }

    #[test]
    #[should_panic(expected = "field::component::Props.id must remain stable after init")]
    fn field_set_props_panics_when_id_changes() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let mut next = test_props();

        next.id = "other".to_string();

        drop(service.set_props(next));
    }

    #[test]
    fn field_label_attrs_for_attribute() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        let attrs = api.label_attrs();

        assert_eq!(attrs.get(&HtmlAttr::For), Some("email-input"));
    }

    #[test]
    fn field_input_attrs_aria_required() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetRequired(true)));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Required)), Some("true"));
    }

    #[test]
    fn field_input_attrs_aria_invalid() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetInvalid(true)));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
    }

    #[test]
    fn field_input_attrs_invalid_without_errors_omits_aria_errormessage() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetInvalid(true)));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::ErrorMessage)));
    }

    #[test]
    fn field_input_attrs_aria_disabled() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetDisabled(true)));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
    }

    #[test]
    fn field_input_attrs_aria_readonly() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetReadonly(true)));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ReadOnly)), Some("true"));
    }

    #[test]
    fn field_error_message_hidden_when_no_errors() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        let attrs = api.error_message_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Hidden), Some("true"));
    }

    #[test]
    fn field_error_message_visible_when_errors_present() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));

        let api = service.connect(&|_| {});

        let attrs = api.error_message_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("alert"));
        assert!(!attrs.contains(&HtmlAttr::Hidden));
    }

    #[test]
    fn field_input_attrs_aria_busy_when_validating() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetValidating(true)));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)), Some("true"));
    }

    #[test]
    fn field_set_invalid_preserves_error_driven_invalidity() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));
        drop(service.send(Event::SetInvalid(false)));

        assert!(service.context().invalid);
    }

    #[test]
    fn field_clear_errors_restores_prop_invalid() {
        let mut service = Service::<Machine>::new(test_props_with_invalid(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));
        drop(service.send(Event::ClearErrors));

        assert!(service.context().errors.is_empty());
        assert!(service.context().invalid);
    }

    #[test]
    fn field_set_errors_empty_preserves_prop_invalid() {
        let mut service = Service::<Machine>::new(test_props_with_invalid(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));
        drop(service.send(Event::SetErrors(vec![])));

        assert!(service.context().errors.is_empty());
        assert!(service.context().invalid);
    }

    #[test]
    fn field_input_attrs_sets_aria_errormessage_when_errors_present() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetInvalid(true)));
        drop(service.send(Event::SetErrors(vec![custom_error()])));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ErrorMessage)),
            Some("email-error-message")
        );
    }

    #[test]
    fn field_input_attrs_describedby_orders_description_before_error() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetHasDescription(true)));
        drop(service.send(Event::SetErrors(vec![custom_error()])));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("email-description email-error-message")
        );
    }

    #[test]
    fn field_root_attrs_sets_dir() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetDir(Some(Direction::Rtl))));

        let api = service.connect(&|_| {});

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
    }

    #[test]
    fn field_root_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_snapshot!(
            "field_root_default",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn field_root_rtl_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetDir(Some(Direction::Rtl))));

        assert_snapshot!(
            "field_root_rtl",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn field_label_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_snapshot!(
            "field_label_default",
            snapshot_attrs(&service.connect(&|_| {}).label_attrs())
        );
    }

    #[test]
    fn field_input_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_snapshot!(
            "field_input_default",
            snapshot_attrs(&service.connect(&|_| {}).input_attrs())
        );
    }

    #[test]
    fn field_input_required_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetRequired(true)));

        assert_snapshot!(
            "field_input_required",
            snapshot_attrs(&service.connect(&|_| {}).input_attrs())
        );
    }

    #[test]
    fn field_input_invalid_no_errors_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetInvalid(true)));

        assert_snapshot!(
            "field_input_invalid_no_errors",
            snapshot_attrs(&service.connect(&|_| {}).input_attrs())
        );
    }

    #[test]
    fn field_input_invalid_with_description_and_error_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetHasDescription(true)));
        drop(service.send(Event::SetInvalid(true)));
        drop(service.send(Event::SetErrors(vec![custom_error()])));

        assert_snapshot!(
            "field_input_invalid_with_description_and_error",
            snapshot_attrs(&service.connect(&|_| {}).input_attrs())
        );
    }

    #[test]
    fn field_input_disabled_readonly_validating_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetDisabled(true)));
        drop(service.send(Event::SetReadonly(true)));
        drop(service.send(Event::SetValidating(true)));

        assert_snapshot!(
            "field_input_disabled_readonly_validating",
            snapshot_attrs(&service.connect(&|_| {}).input_attrs())
        );
    }

    #[test]
    fn field_description_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_snapshot!(
            "field_description_default",
            snapshot_attrs(&service.connect(&|_| {}).description_attrs())
        );
    }

    #[test]
    fn field_error_message_hidden_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_snapshot!(
            "field_error_message_hidden",
            snapshot_attrs(&service.connect(&|_| {}).error_message_attrs())
        );
    }

    #[test]
    fn field_error_message_visible_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));

        assert_snapshot!(
            "field_error_message_visible",
            snapshot_attrs(&service.connect(&|_| {}).error_message_attrs())
        );
    }

    #[test]
    fn field_part_attrs_delegate_for_all_parts() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Label), api.label_attrs());
        assert_eq!(api.part_attrs(Part::Input), api.input_attrs());
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::ErrorMessage),
            api.error_message_attrs()
        );
    }

    #[test]
    fn field_api_debug_is_stable() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        let debug = format!("{api:?}");

        assert!(debug.contains("Api"));
        assert!(debug.contains("email"));
        assert!(debug.contains("Context"));
    }
}
