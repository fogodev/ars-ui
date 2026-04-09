//! Form submission state machine.
//!
//! This module implements the submission lifecycle as a proper [`Machine`]
//! (spec §8). It manages the flow from user-initiated submit through
//! client-side validation, optional async validation, server submission,
//! and error recovery.
//!
//! [`Machine`]: ars_core::Machine

use std::{collections::BTreeMap, fmt};

use ars_a11y::ComponentIds;
use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentPart, ConnectApi, HtmlAttr, PendingEffect,
    TransitionPlan, WeakSend, no_cleanup,
};

use crate::{
    form::{Context as FormContext, Mode},
    validation::BoxedAsyncValidator,
};

// ────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────

/// States of the form submission lifecycle.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// Form is ready for user input.
    Idle,
    /// Client-side validation is running.
    Validating,
    /// Validation failed — errors are shown.
    ValidationFailed,
    /// Submission is in progress (async).
    Submitting,
    /// Submission succeeded.
    Succeeded,
    /// Submission failed (server/network error).
    Failed,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            State::Idle => write!(f, "idle"),
            State::Validating => write!(f, "validating"),
            State::ValidationFailed => write!(f, "validation-failed"),
            State::Submitting => write!(f, "submitting"),
            State::Succeeded => write!(f, "succeeded"),
            State::Failed => write!(f, "failed"),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Event
// ────────────────────────────────────────────────────────────────────

/// Events that drive form submission transitions.
#[derive(Clone, Debug)]
pub enum Event {
    /// User initiated submit (e.g., clicked submit button).
    Submit,
    /// All validators (sync + async) passed.
    ValidationPassed,
    /// One or more validators failed.
    ValidationFailed,
    /// Server submission succeeded.
    SubmitComplete,
    /// Server submission failed with an error message.
    SubmitError(String),
    /// Reset the form to initial state.
    Reset,
    /// Inject server-side validation errors into field state.
    SetServerErrors(BTreeMap<String, Vec<String>>),
    /// Update the validation mode without resetting form state.
    SetMode(Mode),
}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Internal context for the form submission state machine.
#[derive(Clone, Debug)]
pub struct Context {
    /// The embedded form context tracking all field state.
    pub form: FormContext,
    /// Component IDs for accessibility attributes.
    pub ids: ComponentIds,
    /// Error message from a failed submission (server/network error).
    pub submit_error: Option<String>,
    /// Whether synchronous validation passed (used by the async-validation effect).
    pub sync_valid: bool,
    // Server errors stored in `form.server_errors` (single source of truth).
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// External configuration for the form submission machine.
///
/// `id` must be set by the adapter layer (via `use_id`/`use_stable_id`);
/// adapters provide default no-op implementations for callback fields.
#[derive(Clone, ars_core::HasId)]
pub struct Props {
    /// DOM ID for the form element.
    pub id: String,
    /// Validation trigger mode.
    pub validation_mode: Mode,
    /// Adapter-provided async spawn for running async validators concurrently.
    ///
    /// Signature: `(validators, send) -> CleanupFn`.
    /// Leptos: wraps `spawn_local`; Dioxus: wraps `spawn`.
    #[expect(clippy::type_complexity, reason = "spec-defined callback signature")]
    pub spawn_async_validation: Callback<
        dyn Fn((Vec<(String, BoxedAsyncValidator)>, WeakSend<Event>)) -> Box<dyn FnOnce()>,
    >,
    /// Adapter-provided microtask scheduler for deferred event dispatch.
    ///
    /// WASM: wraps `queueMicrotask`; native: wraps `tokio::spawn` or equivalent.
    #[expect(clippy::type_complexity, reason = "spec-defined callback signature")]
    pub schedule_microtask: Callback<dyn Fn(Box<dyn FnOnce()>)>,
}

impl PartialEq for Props {
    fn eq(&self, other: &Self) -> bool {
        // Callback fields are not compared — identity determined by id + mode.
        self.id == other.id && self.validation_mode == other.validation_mode
    }
}

impl fmt::Debug for Props {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("form_submit::Props")
            .field("id", &self.id)
            .field("validation_mode", &self.validation_mode)
            .finish_non_exhaustive()
    }
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// The form submission state machine.
///
/// See spec `07-forms.md` §8 for the full lifecycle specification.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props) -> (Self::State, Self::Context) {
        (
            State::Idle,
            Context {
                form: FormContext::new(props.validation_mode),
                ids: ComponentIds::from_id(&props.id),
                submit_error: None,
                sync_valid: false,
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // Allow re-submission from any terminal state (including Succeeded)
            // without requiring an explicit Reset.
            (
                State::Idle | State::ValidationFailed | State::Failed | State::Succeeded,
                Event::Submit,
            ) => Some(
                TransitionPlan::to(State::Validating)
                    .apply(|ctx: &mut Context| {
                        ctx.submit_error = None;
                        ctx.form.touch_all();
                        ctx.sync_valid = ctx.form.validate_all();
                    })
                    .with_effect(PendingEffect::new(
                        "async-validation",
                        |ctx: &Context, props: &Props, send: WeakSend<Event>| {
                            if ctx.form.has_async_validators() {
                                let validators = ctx.form.collect_async_validators();
                                (props.spawn_async_validation)((validators, send))
                            } else {
                                let event = if ctx.sync_valid {
                                    Event::ValidationPassed
                                } else {
                                    Event::ValidationFailed
                                };
                                #[cfg(not(target_arch = "wasm32"))]
                                let cancelled =
                                    std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                                #[cfg(target_arch = "wasm32")]
                                let cancelled = std::rc::Rc::new(std::cell::Cell::new(false));
                                #[cfg(not(target_arch = "wasm32"))]
                                let cancelled_clone = std::sync::Arc::clone(&cancelled);
                                #[cfg(target_arch = "wasm32")]
                                let cancelled_clone = std::rc::Rc::clone(&cancelled);
                                (props.schedule_microtask)(Box::new(move || {
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let is_cancelled =
                                        cancelled_clone.load(std::sync::atomic::Ordering::Relaxed);
                                    #[cfg(target_arch = "wasm32")]
                                    let is_cancelled = cancelled_clone.get();
                                    if !is_cancelled {
                                        send.call_if_alive(event);
                                    }
                                }));
                                Box::new(move || {
                                    #[cfg(not(target_arch = "wasm32"))]
                                    cancelled.store(true, std::sync::atomic::Ordering::Relaxed);
                                    #[cfg(target_arch = "wasm32")]
                                    cancelled.set(true);
                                })
                            }
                        },
                    )),
            ),

            (State::Validating, Event::ValidationPassed) => Some(
                TransitionPlan::to(State::Submitting)
                    .apply(|ctx: &mut Context| {
                        ctx.form.is_submitting = true;
                    })
                    .with_effect(PendingEffect::new("submit", |_ctx, _props, _send| {
                        // Adapter observes Submitting state and invokes user on_submit.
                        // This effect exists so the adapter can register a cleanup
                        // function that cancels in-flight requests on state change.
                        no_cleanup()
                    })),
            ),

            (State::Validating, Event::ValidationFailed) => Some(
                TransitionPlan::to(State::ValidationFailed).apply(|ctx: &mut Context| {
                    ctx.sync_valid = false;
                }),
            ),

            (State::Submitting, Event::SubmitComplete) => Some(
                TransitionPlan::to(State::Succeeded).apply(|ctx: &mut Context| {
                    ctx.form.is_submitting = false;
                    ctx.submit_error = None;
                }),
            ),

            (State::Submitting, Event::SubmitError(msg)) => {
                let msg = msg.clone();
                Some(
                    TransitionPlan::to(State::Failed).apply(move |ctx: &mut Context| {
                        ctx.form.is_submitting = false;
                        ctx.submit_error = Some(msg);
                    }),
                )
            }

            // SetServerErrors can arrive from any state — intentional escape hatch.
            (_, Event::SetServerErrors(errors)) => {
                let errors = errors.clone();
                Some(
                    TransitionPlan::to(State::ValidationFailed).apply(move |ctx: &mut Context| {
                        ctx.form.is_submitting = false;
                        ctx.submit_error = None;
                        ctx.form.set_server_errors(errors);
                    }),
                )
            }

            (_, Event::Reset) => {
                let mode = props.validation_mode;
                Some(
                    TransitionPlan::to(State::Idle)
                        .cancel_effect("async-validation")
                        .cancel_effect("submit")
                        .apply(move |ctx: &mut Context| {
                            ctx.form.reset();
                            ctx.form.validation_mode = mode;
                            ctx.submit_error = None;
                        }),
                )
            }

            (_, Event::SetMode(mode)) => {
                let mode = *mode;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.form.validation_mode = mode;
                }))
            }

            // Submit during Validating/Submitting is intentionally dropped (debounce).
            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        if old.validation_mode == new.validation_mode {
            vec![]
        } else {
            vec![Event::SetMode(new.validation_mode)]
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
}

// ────────────────────────────────────────────────────────────────────
// Part and Connect API
// ────────────────────────────────────────────────────────────────────

/// DOM parts of the form submission component.
#[derive(ars_core::ComponentPart)]
#[scope = "form-submit"]
pub enum Part {
    /// The root `<form>` element.
    Root,
    /// The submit button element.
    SubmitButton,
}

/// Connect API for the form submission machine.
///
/// Produces attributes and accessor methods from the current state.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    #[expect(dead_code, reason = "reserved for future prop-derived attributes")]
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl fmt::Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("form_submit::Api")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::SubmitButton => self.submit_button_attrs(),
        }
    }
}

impl<'a> Api<'a> {
    /// Whether the form is currently being submitted.
    #[must_use]
    pub fn is_submitting(&self) -> bool {
        *self.state == State::Submitting
    }

    /// Whether the form is currently valid (no fields have errors).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.ctx.form.is_valid()
    }

    /// The error message from a failed submission, if any.
    #[must_use]
    pub fn submit_error(&self) -> Option<&str> {
        self.ctx.submit_error.as_deref()
    }

    /// Attributes for the root `<form>` element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ctx.ids.id());
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-state"), self.state.to_string());
        attrs
    }

    /// Typed handler: call from the adapter's `on:submit` handler.
    pub fn on_form_submit(&self) {
        (self.send)(Event::Submit);
    }

    /// Attributes for the submit button.
    #[must_use]
    pub fn submit_button_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SubmitButton.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        if self.is_submitting() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Disabled, true);
        }
        attrs
    }
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // Import Machine trait for on_props_changed.
    use ars_core::{Machine as _, Service, callback};

    use super::*;

    /// Helper to construct Props with no-op callbacks for testing.
    fn test_props() -> Props {
        Props {
            id: "test-form".into(),
            validation_mode: Mode::on_submit(),
            spawn_async_validation: callback(
                |_: (Vec<(String, BoxedAsyncValidator)>, WeakSend<Event>)| -> Box<dyn FnOnce()> {
                    Box::new(|| {})
                },
            ),
            schedule_microtask: callback(|_: Box<dyn FnOnce()>| {}),
        }
    }

    // --- State transition tests ---

    #[test]
    fn init_state_is_idle() {
        let service = Service::<Machine>::new(test_props());
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().submit_error.is_none());
        assert!(!service.context().sync_valid);
    }

    #[test]
    fn submit_from_idle_transitions_to_validating() {
        let mut service = Service::<Machine>::new(test_props());
        let result = service.send(Event::Submit);
        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Validating);
        // Should have "async-validation" effect
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, "async-validation");
    }

    #[test]
    fn submit_from_validation_failed_transitions_to_validating() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationFailed));
        assert_eq!(service.state(), &State::ValidationFailed);

        let result = service.send(Event::Submit);
        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Validating);
    }

    #[test]
    fn submit_from_failed_transitions_to_validating() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));
        drop(service.send(Event::SubmitError("server error".into())));
        assert_eq!(service.state(), &State::Failed);

        let result = service.send(Event::Submit);
        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Validating);
    }

    #[test]
    fn submit_from_succeeded_transitions_to_validating() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));
        drop(service.send(Event::SubmitComplete));
        assert_eq!(service.state(), &State::Succeeded);

        let result = service.send(Event::Submit);
        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Validating);
    }

    #[test]
    fn submit_from_validating_is_ignored() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        assert_eq!(service.state(), &State::Validating);

        let result = service.send(Event::Submit);
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Validating);
    }

    #[test]
    fn submit_from_submitting_is_ignored() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));
        assert_eq!(service.state(), &State::Submitting);

        let result = service.send(Event::Submit);
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Submitting);
    }

    #[test]
    fn validation_passed_transitions_to_submitting() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));

        let result = service.send(Event::ValidationPassed);
        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Submitting);
        assert!(service.context().form.is_submitting);
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, "submit");
    }

    #[test]
    fn validation_failed_transitions_to_validation_failed() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));

        let result = service.send(Event::ValidationFailed);
        assert!(result.state_changed);
        assert_eq!(service.state(), &State::ValidationFailed);
        assert!(!service.context().sync_valid);
    }

    #[test]
    fn submit_complete_transitions_to_succeeded() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));

        let result = service.send(Event::SubmitComplete);
        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Succeeded);
        assert!(!service.context().form.is_submitting);
        assert!(service.context().submit_error.is_none());
    }

    #[test]
    fn submit_error_transitions_to_failed() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));

        let result = service.send(Event::SubmitError("server down".into()));
        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Failed);
        assert!(!service.context().form.is_submitting);
        assert_eq!(
            service.context().submit_error.as_deref(),
            Some("server down")
        );
    }

    // --- SetServerErrors tests ---

    #[test]
    fn set_server_errors_from_idle_transitions_to_validation_failed() {
        let mut service = Service::<Machine>::new(test_props());
        // Register a field so set_server_errors has something to inject into.
        service.context_mut().form.register(
            "email",
            crate::field::Value::Text(String::new()),
            None,
            None,
        );

        let errors = BTreeMap::from([("email".into(), vec!["Already registered".into()])]);
        let result = service.send(Event::SetServerErrors(errors));

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::ValidationFailed);
        assert!(!service.context().form.server_errors.is_empty());
    }

    #[test]
    fn set_server_errors_from_submitting_clears_is_submitting() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));
        assert!(service.context().form.is_submitting);

        let errors = BTreeMap::from([("email".into(), vec!["Taken".into()])]);
        drop(service.send(Event::SetServerErrors(errors)));

        assert_eq!(service.state(), &State::ValidationFailed);
        assert!(!service.context().form.is_submitting);
    }

    // --- Reset tests ---

    #[test]
    fn reset_transitions_to_idle_and_cancels_effects() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));
        drop(service.send(Event::SubmitError("err".into())));
        assert_eq!(service.state(), &State::Failed);
        assert!(service.context().submit_error.is_some());

        let result = service.send(Event::Reset);
        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().submit_error.is_none());
        assert_eq!(result.cancel_effects, vec!["async-validation", "submit"]);
    }

    // --- SetMode tests ---

    #[test]
    fn set_mode_updates_context_only() {
        let mut service = Service::<Machine>::new(test_props());
        let result = service.send(Event::SetMode(Mode::on_change()));
        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.context().form.validation_mode, Mode::on_change());
    }

    // --- on_props_changed tests ---

    #[test]
    fn on_props_changed_emits_set_mode_when_mode_differs() {
        let events = Machine::on_props_changed(
            &test_props(),
            &Props {
                validation_mode: Mode::on_change(),
                ..test_props()
            },
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::SetMode(_)));
    }

    #[test]
    fn on_props_changed_emits_nothing_when_mode_same() {
        let events = Machine::on_props_changed(&test_props(), &test_props());
        assert!(events.is_empty());
    }

    // --- Connect API tests ---

    #[test]
    fn api_is_submitting_reflects_state() {
        let mut service = Service::<Machine>::new(test_props());
        let noop = |_: Event| {};

        let api = service.connect(&noop);
        assert!(!api.is_submitting());

        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));

        let api = service.connect(&noop);
        assert!(api.is_submitting());
    }

    #[test]
    fn api_submit_error_reflects_context() {
        let mut service = Service::<Machine>::new(test_props());
        let noop = |_: Event| {};

        let api = service.connect(&noop);
        assert!(api.submit_error().is_none());

        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));
        drop(service.send(Event::SubmitError("oops".into())));

        let api = service.connect(&noop);
        assert_eq!(api.submit_error(), Some("oops"));
    }

    #[test]
    fn api_root_attrs_contain_required_fields() {
        let service = Service::<Machine>::new(test_props());
        let noop = |_: Event| {};
        let api = service.connect(&noop);
        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id).unwrap(), "test-form");
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")).unwrap(),
            "form-submit"
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")).unwrap(), "root");
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")).unwrap(), "idle");
    }

    #[test]
    fn api_submit_button_attrs_disabled_when_submitting() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));

        let noop = |_: Event| {};
        let api = service.connect(&noop);
        let attrs = api.submit_button_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)).unwrap(), "true");
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)).unwrap(),
            "true"
        );
        assert!(attrs.get(&HtmlAttr::Disabled).is_some());
    }

    #[test]
    fn api_submit_button_attrs_not_disabled_when_idle() {
        let service = Service::<Machine>::new(test_props());
        let noop = |_: Event| {};
        let api = service.connect(&noop);
        let attrs = api.submit_button_attrs();

        assert!(attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)).is_none());
        assert!(attrs.get(&HtmlAttr::Disabled).is_none());
    }

    // --- Context mutation verification ---

    #[test]
    fn submit_touches_all_fields_and_runs_validation() {
        let mut service = Service::<Machine>::new(test_props());
        // Register two fields — neither is touched initially.
        service.context_mut().form.register(
            "email",
            crate::field::Value::Text(String::new()),
            None,
            None,
        );
        service.context_mut().form.register(
            "name",
            crate::field::Value::Text(String::new()),
            None,
            None,
        );
        assert!(!service.context().form.fields.get("email").unwrap().touched);
        assert!(!service.context().form.fields.get("name").unwrap().touched);

        drop(service.send(Event::Submit));

        // Both fields should be touched after submit.
        assert!(service.context().form.fields.get("email").unwrap().touched);
        assert!(service.context().form.fields.get("name").unwrap().touched);
        // sync_valid should be true (no validators registered → all pass).
        assert!(service.context().sync_valid);
    }

    #[test]
    fn submit_sets_sync_valid_false_when_validation_fails() {
        use crate::validation::{Error, ErrorCode, Errors, Validator, boxed_validator};

        struct AlwaysFail;
        impl Validator for AlwaysFail {
            fn validate(
                &self,
                _value: &crate::field::Value,
                _ctx: &crate::validation::Context<'_>,
            ) -> crate::validation::Result {
                Err(Errors(vec![Error {
                    code: ErrorCode::Required,
                    message: "required".into(),
                }]))
            }
        }

        let mut service = Service::<Machine>::new(test_props());
        service.context_mut().form.register(
            "email",
            crate::field::Value::Text(String::new()),
            Some(boxed_validator(AlwaysFail)),
            None,
        );

        drop(service.send(Event::Submit));

        assert!(!service.context().sync_valid);
    }

    #[test]
    fn submit_from_failed_clears_stale_error() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));
        drop(service.send(Event::SubmitError("old error".into())));
        assert_eq!(service.context().submit_error.as_deref(), Some("old error"));

        // Re-submit should clear the stale error.
        drop(service.send(Event::Submit));
        assert!(service.context().submit_error.is_none());
    }

    #[test]
    fn set_server_errors_clears_submit_error() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));
        drop(service.send(Event::SubmitError("network failure".into())));
        assert!(service.context().submit_error.is_some());

        let errors = BTreeMap::from([("email".into(), vec!["Taken".into()])]);
        drop(service.send(Event::SetServerErrors(errors)));
        assert!(service.context().submit_error.is_none());
    }

    // --- Events in wrong states (wildcard coverage) ---

    #[test]
    fn validation_passed_ignored_outside_validating() {
        let mut service = Service::<Machine>::new(test_props());
        // From Idle — should be ignored.
        let result = service.send(Event::ValidationPassed);
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn submit_complete_ignored_outside_submitting() {
        let mut service = Service::<Machine>::new(test_props());
        let result = service.send(Event::SubmitComplete);
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn submit_error_ignored_outside_submitting() {
        let mut service = Service::<Machine>::new(test_props());
        let result = service.send(Event::SubmitError("err".into()));
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);
    }

    // --- Display / data-ars-state coverage ---

    #[test]
    fn state_display_produces_kebab_case() {
        assert_eq!(State::Idle.to_string(), "idle");
        assert_eq!(State::Validating.to_string(), "validating");
        assert_eq!(State::ValidationFailed.to_string(), "validation-failed");
        assert_eq!(State::Submitting.to_string(), "submitting");
        assert_eq!(State::Succeeded.to_string(), "succeeded");
        assert_eq!(State::Failed.to_string(), "failed");
    }

    #[test]
    fn root_attrs_state_reflects_current_state() {
        let mut service = Service::<Machine>::new(test_props());
        let noop = |_: Event| {};

        drop(service.send(Event::Submit));
        let attrs = service.connect(&noop).root_attrs();
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-state")).unwrap(),
            "validating"
        );

        drop(service.send(Event::ValidationPassed));
        let attrs = service.connect(&noop).root_attrs();
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-state")).unwrap(),
            "submitting"
        );
    }

    #[test]
    fn submit_button_attrs_contain_scope_and_part() {
        let service = Service::<Machine>::new(test_props());
        let noop = |_: Event| {};
        let attrs = service.connect(&noop).submit_button_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")).unwrap(),
            "form-submit"
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-part")).unwrap(),
            "submit-button"
        );
    }

    // --- Lifecycle integration ---

    #[test]
    fn full_happy_path_lifecycle() {
        let mut service = Service::<Machine>::new(test_props());
        assert_eq!(service.state(), &State::Idle);

        drop(service.send(Event::Submit));
        assert_eq!(service.state(), &State::Validating);

        drop(service.send(Event::ValidationPassed));
        assert_eq!(service.state(), &State::Submitting);
        assert!(service.context().form.is_submitting);

        drop(service.send(Event::SubmitComplete));
        assert_eq!(service.state(), &State::Succeeded);
        assert!(!service.context().form.is_submitting);
    }

    #[test]
    fn error_retry_lifecycle() {
        let mut service = Service::<Machine>::new(test_props());

        // First attempt fails.
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));
        drop(service.send(Event::SubmitError("timeout".into())));
        assert_eq!(service.state(), &State::Failed);
        assert_eq!(service.context().submit_error.as_deref(), Some("timeout"));

        // Retry succeeds.
        drop(service.send(Event::Submit));
        assert!(service.context().submit_error.is_none()); // cleared on re-submit
        drop(service.send(Event::ValidationPassed));
        drop(service.send(Event::SubmitComplete));
        assert_eq!(service.state(), &State::Succeeded);
    }

    #[test]
    fn set_props_updates_mode_through_service() {
        let mut service = Service::<Machine>::new(test_props());
        assert_eq!(service.context().form.validation_mode, Mode::on_submit());

        let result = service.set_props(Props {
            validation_mode: Mode::on_change(),
            ..test_props()
        });
        assert!(result.context_changed);
        assert_eq!(service.context().form.validation_mode, Mode::on_change());
    }

    // --- Edge cases ---

    #[test]
    fn reset_from_validating_cancels_async_validation() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        assert_eq!(service.state(), &State::Validating);

        let result = service.send(Event::Reset);
        assert_eq!(service.state(), &State::Idle);
        assert!(result.cancel_effects.contains(&"async-validation"));
    }

    #[test]
    fn reset_from_submitting_cancels_submit() {
        let mut service = Service::<Machine>::new(test_props());
        drop(service.send(Event::Submit));
        drop(service.send(Event::ValidationPassed));
        assert_eq!(service.state(), &State::Submitting);

        let result = service.send(Event::Reset);
        assert_eq!(service.state(), &State::Idle);
        assert!(result.cancel_effects.contains(&"submit"));
        assert!(!service.context().form.is_submitting);
    }

    #[test]
    fn on_form_submit_dispatches_submit_event() {
        let service = Service::<Machine>::new(test_props());
        let called = std::cell::Cell::new(false);
        let send_fn = |e: Event| {
            assert!(matches!(e, Event::Submit));
            called.set(true);
        };
        let api = service.connect(&send_fn);
        api.on_form_submit();
        assert!(called.get());
    }

    #[test]
    fn init_context_has_correct_component_ids() {
        let service = Service::<Machine>::new(test_props());
        assert_eq!(service.context().ids.id(), "test-form");
    }
}
