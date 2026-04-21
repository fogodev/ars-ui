//! Form component state machine and connect API.
//!
//! This module implements the framework-agnostic `Form` machine defined in
//! `spec/foundation/07-forms.md` §14. The machine models a simplified
//! submission lifecycle, server-error synchronization, and the structural
//! live-region wiring used by adapter-owned form components.

use std::{
    collections::BTreeMap,
    fmt::{self, Debug, Display},
};

use ars_core::{
    AriaAttr, AttrMap, ComponentIds, ComponentPart, ConnectApi, Env, HtmlAttr, TransitionPlan,
    sanitize_url,
};

/// Controls how validation errors are reported to the user.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ValidationBehavior {
    /// Use native HTML constraint validation.
    Native,

    /// Use ARIA-based validation display.
    #[default]
    Aria,
}

/// States of the form component lifecycle.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Form is idle, ready for input.
    Idle,

    /// Form submission is in progress.
    Submitting,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Submitting => write!(f, "submitting"),
        }
    }
}

/// Events that drive form component transitions.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Triggered when the form is submitted.
    Submit,

    /// Submission completed successfully or unsuccessfully.
    SubmitComplete {
        /// Whether the submission succeeded.
        success: bool,
    },

    /// Triggered when the form is reset.
    Reset,

    /// Replaces server-side validation errors keyed by field name.
    SetServerErrors(BTreeMap<String, Vec<String>>),

    /// Clears all server-side validation errors.
    ClearServerErrors,

    /// Synchronizes validation behavior from props.
    SetValidationBehavior(ValidationBehavior),

    /// Sets the status-region message announced by adapters.
    SetStatusMessage(Option<String>),
}

/// Mutable machine context for the form component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// How validation errors are displayed.
    pub validation_behavior: ValidationBehavior,

    /// Whether the form is currently submitting.
    pub is_submitting: bool,

    /// Server-side validation errors keyed by field name.
    pub server_errors: BTreeMap<String, Vec<String>>,

    /// Status message shown in the live region.
    pub status_message: Option<String>,

    /// Result of the last submission attempt.
    pub last_submit_succeeded: Option<bool>,

    /// Stable IDs derived from the adapter-provided base ID.
    pub ids: ComponentIds,
}

/// Immutable configuration for a form component machine instance.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the form root.
    ///
    /// This ID is immutable for the lifetime of a machine instance because
    /// [`Context::ids`] caches the derived part IDs during initialization.
    pub id: String,

    /// How validation errors are reported.
    pub validation_behavior: ValidationBehavior,

    /// Declarative server-side validation errors keyed by field name.
    pub validation_errors: BTreeMap<String, Vec<String>>,

    /// The URL to submit the form to.
    pub action: Option<String>,

    /// Optional explicit role override for the form root.
    pub role: Option<String>,
}

/// Framework-agnostic form component state machine.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    // Localized form strings live in `crate::form::Messages` and are resolved
    // by adapters/validation helpers, not by the core component machine.
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
                validation_behavior: props.validation_behavior,
                is_submitting: false,
                server_errors: props.validation_errors.clone(),
                status_message: None,
                last_submit_succeeded: None,
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "form::component::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.validation_behavior != new.validation_behavior {
            events.push(Event::SetValidationBehavior(new.validation_behavior));
        }

        if old.validation_errors != new.validation_errors {
            if new.validation_errors.is_empty() {
                events.push(Event::ClearServerErrors);
            } else {
                events.push(Event::SetServerErrors(new.validation_errors.clone()));
            }
        }

        events
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Idle, Event::Submit) => Some(TransitionPlan::to(State::Submitting).apply(
                |ctx: &mut Context| {
                    ctx.is_submitting = true;
                    ctx.status_message = None;
                },
            )),

            (State::Submitting, Event::SubmitComplete { success }) => {
                let success = *success;
                Some(
                    TransitionPlan::to(State::Idle).apply(move |ctx: &mut Context| {
                        ctx.is_submitting = false;
                        ctx.last_submit_succeeded = Some(success);
                        ctx.status_message = None;
                    }),
                )
            }

            (_, Event::Reset) => {
                let behavior = props.validation_behavior;
                Some(
                    TransitionPlan::to(State::Idle).apply(move |ctx: &mut Context| {
                        ctx.is_submitting = false;
                        ctx.last_submit_succeeded = None;
                        ctx.server_errors.clear();
                        ctx.status_message = None;
                        ctx.validation_behavior = behavior;
                    }),
                )
            }

            (_, Event::SetServerErrors(errors)) => {
                let errors = errors.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.server_errors = errors;
                }))
            }

            (_, Event::ClearServerErrors) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.server_errors.clear();
                }))
            }

            (_, Event::SetValidationBehavior(behavior)) => {
                let behavior = *behavior;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.validation_behavior = behavior;
                }))
            }

            (_, Event::SetStatusMessage(msg)) => {
                let msg = msg.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.status_message = msg;
                }))
            }

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props }
    }
}

/// Snapshot connect API for deriving form DOM attributes.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("form::component::Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .finish()
    }
}

/// Structural parts exposed by the form connect API.
#[derive(ComponentPart)]
#[scope = "form"]
pub enum Part {
    /// The root `<form>` element.
    Root,

    /// The hidden live region announcing submission results.
    StatusRegion,
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::StatusRegion => self.status_region_attrs(),
        }
    }
}

impl<'a> Api<'a> {
    /// Returns attributes for the root `<form>` element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), self.state.to_string());

        if self.ctx.validation_behavior == ValidationBehavior::Aria {
            attrs.set_bool(HtmlAttr::NoValidate, true);
        }

        if self.ctx.is_submitting {
            attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        }

        if let Some(action) = &self.props.action {
            attrs.set(HtmlAttr::Action, sanitize_url(action));
        }

        if let Some(role) = &self.props.role {
            attrs.set(HtmlAttr::Role, role.as_str());
        }

        attrs
    }

    /// Returns attributes for the status-region element.
    #[must_use]
    pub fn status_region_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::StatusRegion.data_attrs();

        attrs.set(HtmlAttr::Role, "status");
        attrs.set(HtmlAttr::Aria(AriaAttr::Live), "polite");
        attrs.set(HtmlAttr::Aria(AriaAttr::Atomic), "true");
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);

        attrs
    }

    /// Returns whether the form is currently submitting.
    #[must_use]
    pub const fn is_submitting(&self) -> bool {
        self.ctx.is_submitting
    }

    /// Returns the current status-region message.
    #[must_use]
    pub fn status_message(&self) -> Option<&str> {
        self.ctx.status_message.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use ars_core::{ConnectApi, Machine as _, Service};

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "checkout".to_string(),
            ..Props::default()
        }
    }

    #[test]
    fn form_init_idle() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(
            service.context(),
            &Context {
                validation_behavior: ValidationBehavior::Aria,
                is_submitting: false,
                server_errors: BTreeMap::new(),
                status_message: None,
                last_submit_succeeded: None,
                ids: ComponentIds::from_id("checkout"),
            }
        );
    }

    #[test]
    fn form_submit_transitions_to_submitting() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let result = service.send(Event::Submit);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Submitting);
        assert!(service.context().is_submitting);
        assert!(service.context().status_message.is_none());
    }

    #[test]
    fn form_submit_from_submitting_ignored() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::Submit));

        let result = service.send(Event::Submit);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Submitting);
    }

    #[test]
    fn form_submit_complete_success() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::Submit));

        let result = service.send(Event::SubmitComplete { success: true });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().is_submitting);
        assert_eq!(service.context().last_submit_succeeded, Some(true));
    }

    #[test]
    fn form_submit_complete_failure() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::Submit));

        let result = service.send(Event::SubmitComplete { success: false });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().is_submitting);
        assert_eq!(service.context().last_submit_succeeded, Some(false));
    }

    #[test]
    fn form_reset_clears_state() {
        let mut service = Service::<Machine>::new(
            Props {
                validation_behavior: ValidationBehavior::Native,
                validation_errors: BTreeMap::from([(
                    "email".to_string(),
                    vec!["Taken".to_string()],
                )]),
                ..test_props()
            },
            &Env::default(),
            &(),
        );

        drop(service.send(Event::Submit));
        drop(service.send(Event::SetStatusMessage(Some("Working".to_string()))));

        let result = service.send(Event::Reset);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().is_submitting);
        assert!(service.context().server_errors.is_empty());
        assert!(service.context().status_message.is_none());
        assert_eq!(service.context().last_submit_succeeded, None);
        assert_eq!(
            service.context().validation_behavior,
            ValidationBehavior::Native
        );
    }

    #[test]
    fn form_set_server_errors() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let errors = BTreeMap::from([("email".to_string(), vec!["Taken".to_string()])]);

        let result = service.send(Event::SetServerErrors(errors.clone()));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.context().server_errors, errors);
    }

    #[test]
    fn form_clear_server_errors() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetServerErrors(BTreeMap::from([(
            "email".to_string(),
            vec!["Taken".to_string()],
        )]))));

        let result = service.send(Event::ClearServerErrors);

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert!(service.context().server_errors.is_empty());
    }

    #[test]
    fn form_set_validation_behavior_updates_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let result = service.send(Event::SetValidationBehavior(ValidationBehavior::Native));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert_eq!(
            service.context().validation_behavior,
            ValidationBehavior::Native
        );
    }

    #[test]
    fn form_root_attrs_novalidate_aria() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::NoValidate), Some("true"));
    }

    #[test]
    fn form_root_attrs_no_novalidate_native() {
        let service = Service::<Machine>::new(
            Props {
                validation_behavior: ValidationBehavior::Native,
                ..test_props()
            },
            &Env::default(),
            &(),
        );

        let attrs = service.connect(&|_| {}).root_attrs();

        assert!(!attrs.contains(&HtmlAttr::NoValidate));
    }

    #[test]
    fn form_root_attrs_aria_busy_submitting() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::Submit));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)), Some("true"));
    }

    #[test]
    fn form_status_region_attrs() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let attrs = service.connect(&|_| {}).status_region_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("status"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
    }

    #[test]
    fn form_on_props_changed() {
        let old = test_props();

        let new = Props {
            validation_behavior: ValidationBehavior::Native,
            validation_errors: BTreeMap::from([("email".to_string(), vec!["Taken".to_string()])]),
            ..test_props()
        };

        let events = Machine::on_props_changed(&old, &new);

        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0],
            Event::SetValidationBehavior(ValidationBehavior::Native)
        );
        assert_eq!(events[1], Event::SetServerErrors(new.validation_errors));
    }

    #[test]
    fn form_root_attrs_action_sanitizes_unsafe_url() {
        let service = Service::<Machine>::new(
            Props {
                action: Some("javascript:alert(1)".to_string()),
                ..test_props()
            },
            &Env::default(),
            &(),
        );

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Action), Some("#"));
    }

    #[test]
    fn form_root_attrs_action_preserves_safe_url() {
        let service = Service::<Machine>::new(
            Props {
                action: Some("https://example.com/submit".to_string()),
                ..test_props()
            },
            &Env::default(),
            &(),
        );

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Action),
            Some("https://example.com/submit")
        );
    }

    #[test]
    fn form_root_attrs_role_attribute() {
        let service = Service::<Machine>::new(
            Props {
                role: Some("search".to_string()),
                ..test_props()
            },
            &Env::default(),
            &(),
        );

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("search"));
    }

    #[test]
    fn form_root_attrs_state_attribute_reflects_state() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_eq!(
            service
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("idle")
        );

        drop(service.send(Event::Submit));

        assert_eq!(
            service
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("submitting")
        );
    }

    #[test]
    fn form_status_message_accessor_reflects_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetStatusMessage(Some("Saved".to_string()))));

        assert_eq!(service.connect(&|_| {}).status_message(), Some("Saved"));
    }

    #[test]
    fn form_is_submitting_accessor_reflects_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert!(!service.connect(&|_| {}).is_submitting());

        drop(service.send(Event::Submit));

        assert!(service.connect(&|_| {}).is_submitting());
    }

    #[test]
    fn form_last_submit_succeeded_tracks_result() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::Submit));
        drop(service.send(Event::SubmitComplete { success: false }));

        assert_eq!(service.context().last_submit_succeeded, Some(false));
    }

    #[test]
    #[should_panic(expected = "form::component::Props.id must remain stable after init")]
    fn form_set_props_panics_when_id_changes() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let mut next = test_props();

        next.id = "other".to_string();

        drop(service.set_props(next));
    }

    #[test]
    fn form_part_attrs_delegate_for_all_parts() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(
            api.part_attrs(Part::StatusRegion),
            api.status_region_attrs()
        );
    }

    #[test]
    fn form_api_debug_is_stable() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let debug = format!("{:?}", service.connect(&|_| {}));

        assert!(debug.contains("form::component::Api"));
        assert!(debug.contains("Idle"));
        assert!(debug.contains("checkout"));
    }

    #[test]
    fn form_on_props_changed_clears_server_errors_when_new_map_is_empty() {
        let old = Props {
            validation_errors: BTreeMap::from([("email".to_string(), vec!["Taken".to_string()])]),
            ..test_props()
        };

        let new = test_props();

        let events = Machine::on_props_changed(&old, &new);

        assert_eq!(events, vec![Event::ClearServerErrors]);
    }
}
