//! SearchInput component state machine and connect API.
//!
//! This module implements the framework-agnostic `SearchInput` machine defined
//! in `spec/components/input/search-input.md`. The native `<input type="search">`
//! is the form participant; no hidden input is emitted. Optional `ClearTrigger`
//! and `SubmitTrigger` parts wire to the `Clear`/`Submit` events, and the
//! [`Effect::SearchDebounce`] named effect drives the search-as-you-type timer
//! that adapters implement on top of their wall-clock scheduler.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    fmt::{self, Debug, Display},
    time::Duration,
};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    CssProperty, Env, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan,
};

/// The states for the `SearchInput` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// The component is in an idle state — not focused and not loading.
    Idle,

    /// The component is currently focused.
    Focused,

    /// The component is performing a search (loading).
    Searching,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => "idle",
            Self::Focused => "focused",
            Self::Searching => "searching",
        })
    }
}

/// The events for the `SearchInput` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The component received focus.
    Focus {
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },

    /// The component lost focus.
    Blur,

    /// The input value changed.
    Change(String),

    /// The input was cleared (Escape key or clear button).
    Clear,

    /// The search was submitted (Enter key or submit button).
    Submit,

    /// Set the loading/searching state explicitly.
    SetSearching(bool),

    /// Fired by the debounce timer when the debounce period expires.
    DebounceExpired,

    /// Cancels any active debounce timer without firing the callback.
    CancelDebounce,

    /// IME composition started.
    CompositionStart,

    /// IME composition ended.
    CompositionEnd,

    /// Synchronize the externally controlled value prop.
    SetValue(Option<String>),

    /// Synchronize output-affecting props (disabled / readonly / invalid /
    /// placeholder / name) stored in [`Context`] when
    /// [`Service::set_props`] reports a change.
    SetProps,

    /// Track whether a [`Part::Description`] part is rendered (gates
    /// `aria-describedby`).
    SetHasDescription(bool),
}

/// The context for the `SearchInput` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The controlled/uncontrolled value of the component.
    pub value: Bindable<String>,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the component is focused.
    pub focused: bool,

    /// Whether the focus is visible (keyboard-initiated).
    pub focus_visible: bool,

    /// Whether the component is loading.
    pub loading: bool,

    /// The `name` attribute used for form submission.
    pub name: Option<String>,

    /// The placeholder text of the input.
    pub placeholder: Option<String>,

    /// True while an IME composition session is active.
    pub is_composing: bool,

    /// Whether a Description part is rendered (gates `aria-describedby`).
    pub has_description: bool,

    /// Resolved locale for i18n.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component IDs for part identification.
    pub ids: ComponentIds,
}

/// The props for the `SearchInput` component.
#[derive(Clone, Debug, Default, PartialEq, Eq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the search input root.
    pub id: String,

    /// Controlled value. When `Some`, component is controlled.
    pub value: Option<String>,

    /// Default value for uncontrolled mode.
    pub default_value: String,

    /// Whether the component is disabled.
    pub disabled: bool,

    /// Whether the component is readonly.
    pub readonly: bool,

    /// Whether the component is invalid.
    pub invalid: bool,

    /// Whether the input is required.
    pub required: bool,

    /// The placeholder text.
    pub placeholder: Option<String>,

    /// The `name` attribute for form submission.
    pub name: Option<String>,

    /// The ID of the form element the input is associated with.
    pub form: Option<String>,

    /// Optional debounce interval for search-as-you-type.
    ///
    /// When set, `Event::Change` schedules a [`Effect::SearchDebounce`] timer
    /// that must fire `Event::DebounceExpired` after `debounce` of quiescence.
    /// `None` disables debounce — adapters should propagate value changes
    /// immediately. A zero `Duration` should be treated as a one-millisecond
    /// debounce by adapters to avoid microtask-level race conditions.
    pub debounce: Option<Duration>,
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id), the adapter-provided base ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`value`](Self::value), switching to controlled mode.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Clears [`value`](Self::value), switching to uncontrolled mode.
    #[must_use]
    pub fn uncontrolled(mut self) -> Self {
        self.value = None;
        self
    }

    /// Sets [`default_value`](Self::default_value).
    #[must_use]
    pub fn default_value(mut self, value: impl Into<String>) -> Self {
        self.default_value = value.into();
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`readonly`](Self::readonly).
    #[must_use]
    pub const fn readonly(mut self, value: bool) -> Self {
        self.readonly = value;
        self
    }

    /// Sets [`invalid`](Self::invalid).
    #[must_use]
    pub const fn invalid(mut self, value: bool) -> Self {
        self.invalid = value;
        self
    }

    /// Sets [`required`](Self::required).
    #[must_use]
    pub const fn required(mut self, value: bool) -> Self {
        self.required = value;
        self
    }

    /// Sets [`placeholder`](Self::placeholder).
    #[must_use]
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = Some(value.into());
        self
    }

    /// Clears [`placeholder`](Self::placeholder).
    #[must_use]
    pub fn no_placeholder(mut self) -> Self {
        self.placeholder = None;
        self
    }

    /// Sets [`name`](Self::name).
    #[must_use]
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Clears [`name`](Self::name).
    #[must_use]
    pub fn no_name(mut self) -> Self {
        self.name = None;
        self
    }

    /// Sets [`form`](Self::form).
    #[must_use]
    pub fn form(mut self, value: impl Into<String>) -> Self {
        self.form = Some(value.into());
        self
    }

    /// Clears [`form`](Self::form).
    #[must_use]
    pub fn no_form(mut self) -> Self {
        self.form = None;
        self
    }

    /// Sets [`debounce`](Self::debounce).
    #[must_use]
    pub const fn debounce(mut self, value: Duration) -> Self {
        self.debounce = Some(value);
        self
    }

    /// Clears [`debounce`](Self::debounce).
    #[must_use]
    pub const fn no_debounce(mut self) -> Self {
        self.debounce = None;
        self
    }
}

/// Locale-specific labels for the `SearchInput` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the clear button.
    pub clear_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the submit button.
    pub submit_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            clear_label: MessageFn::static_str("Clear search"),
            submit_label: MessageFn::static_str("Submit search"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed identifier for the named effect intents the `search_input` machine emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts (or restarts) the debounce timer scheduled by
    /// [`Event::Change`].
    ///
    /// On expiration the adapter must send [`Event::DebounceExpired`] back to
    /// the service so the core machine can publish the debounced value.
    SearchDebounce,
}

/// The machine for the `SearchInput` component.
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

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        (
            State::Idle,
            Context {
                value: if let Some(value) = &props.value {
                    Bindable::controlled(value.clone())
                } else {
                    Bindable::uncontrolled(props.default_value.clone())
                },
                disabled: props.disabled,
                readonly: props.readonly,
                invalid: props.invalid,
                focused: false,
                focus_visible: false,
                loading: false,
                name: props.name.clone(),
                placeholder: props.placeholder.clone(),
                is_composing: false,
                has_description: false,
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn transition(
        _state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;
                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = is_keyboard;
                    }),
                )
            }

            Event::Blur => {
                let target = if ctx.loading {
                    State::Searching
                } else {
                    State::Idle
                };

                Some(TransitionPlan::to(target).apply(|ctx: &mut Context| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            Event::Change(value) => {
                if ctx.disabled || ctx.readonly {
                    return None;
                }

                let next_value = value.clone();

                let schedule_debounce = !ctx.is_composing && props.debounce.is_some();

                let plan = TransitionPlan::context_only(move |ctx: &mut Context| {
                    if !ctx.value.is_controlled() {
                        ctx.value.set(next_value);
                    }
                });

                let plan = plan.cancel_effect(Effect::SearchDebounce);

                Some(if schedule_debounce {
                    plan.with_effect(PendingEffect::named(Effect::SearchDebounce))
                } else {
                    plan
                })
            }

            Event::Clear => {
                if ctx.disabled || ctx.readonly {
                    return None;
                }

                Some(
                    TransitionPlan::context_only(|ctx: &mut Context| {
                        if !ctx.value.is_controlled() {
                            ctx.value.set(String::new());
                        }
                    })
                    .cancel_effect(Effect::SearchDebounce),
                )
            }

            Event::Submit => {
                if ctx.disabled {
                    return None;
                }

                Some(
                    TransitionPlan::to(State::Searching)
                        .apply(|ctx: &mut Context| {
                            ctx.loading = true;
                        })
                        .cancel_effect(Effect::SearchDebounce),
                )
            }

            Event::SetSearching(loading) => {
                let loading = *loading;

                let target = if loading {
                    State::Searching
                } else if ctx.focused {
                    State::Focused
                } else {
                    State::Idle
                };

                Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    ctx.loading = loading;
                }))
            }

            Event::CompositionStart => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.is_composing = true;
            })),

            Event::CompositionEnd => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.is_composing = false;
            })),

            Event::DebounceExpired | Event::CancelDebounce => None,

            Event::SetValue(value) => {
                let value = value.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if let Some(value) = value {
                        ctx.value.set(value.clone());
                        ctx.value.sync_controlled(Some(value));
                    } else {
                        ctx.value.sync_controlled(None);
                    }
                }))
            }

            Event::SetProps => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = props.disabled;
                    ctx.readonly = props.readonly;
                    ctx.invalid = props.invalid;
                    ctx.placeholder = props.placeholder;
                    ctx.name = props.name;
                }))
            }

            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = has_description;
                }))
            }
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "search_input::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.value != new.value {
            events.push(Event::SetValue(new.value.clone()));
        }

        if props_output_changed(old, new) {
            events.push(Event::SetProps);
        }

        events
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }
}

/// Structural parts exposed by the `SearchInput` connect API.
#[derive(ComponentPart)]
#[scope = "search-input"]
pub enum Part {
    /// The root container element (carries `role="search"`).
    Root,

    /// The visible label element.
    Label,

    /// The native search input element.
    Input,

    /// The optional clear button.
    ClearTrigger,

    /// The optional submit button.
    SubmitTrigger,

    /// The optional loading indicator shown during `Searching` state.
    LoadingIndicator,

    /// The optional descriptive help-text element.
    Description,

    /// The optional validation error message element.
    ErrorMessage,
}

/// The API for the `SearchInput` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .field("props", &self.props)
            .field("send", &"<callback>")
            .finish()
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Input => self.input_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::SubmitTrigger => self.submit_trigger_attrs(),
            Part::LoadingIndicator => self.loading_indicator_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
        }
    }
}

impl Api<'_> {
    /// Attributes for the root container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Role, "search")
            .set(HtmlAttr::Data("ars-state"), self.state.to_string());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        if self.ctx.loading {
            attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Attributes for the label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(HtmlAttr::For, self.ctx.ids.part("input"));

        attrs
    }

    /// Attributes for the native search input element.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Input.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("input"))
            .set(HtmlAttr::Type, "search")
            .set(HtmlAttr::Value, self.ctx.value.get().clone())
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        set_described_by(&mut attrs, self.ctx);

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
            attrs.set(
                HtmlAttr::Aria(AriaAttr::ErrorMessage),
                self.ctx.ids.part("error-message"),
            );
        }

        if self.props.required {
            attrs.set_bool(HtmlAttr::Required, true);
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }

        if let Some(placeholder) = &self.ctx.placeholder {
            attrs.set(HtmlAttr::Placeholder, placeholder.clone());
        }

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.clone());
        }

        if let Some(form) = &self.props.form {
            attrs.set(HtmlAttr::Form, form.clone());
        }

        attrs
    }

    /// Attributes for the clear trigger button.
    #[must_use]
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ClearTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::TabIndex, "-1")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.clear_label)(&self.ctx.locale),
            );

        if self.ctx.value.get().is_empty() {
            attrs.set_style(CssProperty::Display, "none");
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Attributes for the submit trigger button.
    #[must_use]
    pub fn submit_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::SubmitTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.submit_label)(&self.ctx.locale),
            );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Attributes for the loading indicator element.
    #[must_use]
    pub fn loading_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::LoadingIndicator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for the description/help text element.
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

    /// Attributes for the validation error message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

        attrs
    }

    /// Sends [`Event::Focus`] for input focus.
    pub fn on_input_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Sends [`Event::Blur`] for input blur.
    pub fn on_input_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Sends [`Event::Change`] for input value changes.
    pub fn on_input_change(&self, value: String) {
        (self.send)(Event::Change(value));
    }

    /// Sends [`Event::Clear`] for clear trigger activation.
    pub fn on_clear_click(&self) {
        (self.send)(Event::Clear);
    }

    /// Sends [`Event::Submit`] for submit trigger activation.
    pub fn on_submit_click(&self) {
        (self.send)(Event::Submit);
    }
}

fn props_output_changed(old: &Props, new: &Props) -> bool {
    old.disabled != new.disabled
        || old.readonly != new.readonly
        || old.invalid != new.invalid
        || old.required != new.required
        || old.placeholder != new.placeholder
        || old.name != new.name
        || old.form != new.form
        || old.debounce != new.debounce
}

fn set_described_by(attrs: &mut AttrMap, ctx: &Context) {
    let mut described_by = Vec::new();

    if ctx.has_description {
        described_by.push(ctx.ids.part("description"));
    }

    if ctx.invalid {
        described_by.push(ctx.ids.part("error-message"));
    }

    if !described_by.is_empty() {
        attrs.set(
            HtmlAttr::Aria(AriaAttr::DescribedBy),
            described_by.join(" "),
        );
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use ars_core::{ConnectApi, Env, HtmlAttr, Service};
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props::new().id("search")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn search_input_initial_state_is_idle() {
        let service = service(props().default_value("rust"));

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().value.get(), "rust");
        assert!(!service.context().loading);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn search_input_focus_transitions_to_focused() {
        let mut service = service(props());

        let result = service.send(Event::Focus { is_keyboard: true });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Focused);
        assert!(service.context().focused);
        assert!(service.context().focus_visible);
    }

    #[test]
    fn search_input_blur_returns_to_idle_or_searching() {
        let mut svc = service(props());

        drop(svc.send(Event::Focus { is_keyboard: false }));
        drop(svc.send(Event::Blur));

        assert_eq!(svc.state(), &State::Idle);
        assert!(!svc.context().focused);
        assert!(!svc.context().focus_visible);

        let mut svc = service(props());

        drop(svc.send(Event::Focus { is_keyboard: true }));
        drop(svc.send(Event::SetSearching(true)));
        drop(svc.send(Event::Blur));

        assert_eq!(svc.state(), &State::Searching);
        assert!(svc.context().loading);
    }

    #[test]
    fn search_input_change_updates_uncontrolled_value() {
        let mut service = service(props());

        drop(service.send(Event::Change("rust".to_string())));

        assert_eq!(service.context().value.get(), "rust");
    }

    #[test]
    fn search_input_change_schedules_debounce_effect() {
        let mut service = service(props().debounce(Duration::from_millis(200)));

        let result = service.send(Event::Change("r".to_string()));

        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::SearchDebounce);
    }

    #[test]
    fn search_input_change_cancels_previous_debounce_each_keystroke() {
        let mut service = service(props().debounce(Duration::from_millis(200)));

        drop(service.send(Event::Change("r".to_string())));

        let second = service.send(Event::Change("ru".to_string()));

        assert_eq!(second.cancel_effects, alloc::vec![Effect::SearchDebounce]);
        assert_eq!(second.pending_effects.len(), 1);
        assert_eq!(second.pending_effects[0].name, Effect::SearchDebounce);
    }

    #[test]
    fn search_input_change_without_debounce_emits_no_effect() {
        let mut service = service(props());

        let result = service.send(Event::Change("r".to_string()));

        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn search_input_change_during_composition_skips_debounce() {
        let mut service = service(props().debounce(Duration::from_millis(200)));

        drop(service.send(Event::CompositionStart));

        let result = service.send(Event::Change("ｒ".to_string()));

        assert!(result.pending_effects.is_empty());
        assert_eq!(service.context().value.get(), "ｒ");
    }

    #[test]
    fn search_input_clear_cancels_debounce_and_resets_value() {
        let mut service = service(
            props()
                .debounce(Duration::from_millis(200))
                .default_value("query"),
        );

        drop(service.send(Event::Change("q".to_string())));

        let result = service.send(Event::Clear);

        assert_eq!(result.cancel_effects, alloc::vec![Effect::SearchDebounce]);
        assert_eq!(service.context().value.get(), "");
    }

    #[test]
    fn search_input_submit_cancels_debounce_and_transitions_to_searching() {
        let mut service = service(props().debounce(Duration::from_millis(200)));

        drop(service.send(Event::Change("r".to_string())));

        let result = service.send(Event::Submit);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Searching);
        assert!(service.context().loading);
        assert_eq!(result.cancel_effects, alloc::vec![Effect::SearchDebounce]);
    }

    #[test]
    fn search_input_set_searching_toggles_aria_busy_via_state() {
        let mut service = service(props());

        drop(service.send(Event::SetSearching(true)));

        assert_eq!(service.state(), &State::Searching);
        assert!(service.context().loading);

        drop(service.send(Event::SetSearching(false)));

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().loading);
    }

    #[test]
    fn search_input_change_noops_when_disabled_or_readonly() {
        for props in [props().disabled(true), props().readonly(true)] {
            let mut service = service(props.default_value("before"));

            let result = service.send(Event::Change("after".to_string()));

            assert!(!result.context_changed);
            assert_eq!(service.context().value.get(), "before");
        }
    }

    #[test]
    fn search_input_submit_noops_when_disabled() {
        let mut service = service(props().disabled(true));

        let result = service.send(Event::Submit);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().loading);
    }

    #[test]
    fn search_input_set_props_syncs_controlled_value_and_output_props() {
        let mut svc = service(props().value("initial"));

        assert_eq!(svc.context().value.get(), "initial");

        drop(svc.set_props(props().value("updated").disabled(true)));

        assert_eq!(svc.context().value.get(), "updated");
        assert!(svc.context().disabled);

        drop(svc.set_props(props().uncontrolled().disabled(false).invalid(true)));

        assert!(!svc.context().value.is_controlled());
        assert!(!svc.context().disabled);
        assert!(svc.context().invalid);
    }

    #[test]
    fn search_input_set_has_description_flips_context_flag_and_describedby() {
        let mut svc = service(props());

        assert!(!svc.context().has_description);

        drop(svc.send(Event::SetHasDescription(true)));

        assert!(svc.context().has_description);
        assert_eq!(
            svc.connect(&|_| {})
                .input_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("search-description")
        );
    }

    #[test]
    fn search_input_composition_flag_tracks_lifecycle() {
        let mut service = service(props());

        drop(service.send(Event::CompositionStart));

        assert!(service.context().is_composing);

        drop(service.send(Event::CompositionEnd));

        assert!(!service.context().is_composing);
    }

    #[test]
    fn search_input_debounce_expired_and_cancel_are_no_ops_on_state() {
        let mut service = service(props().debounce(Duration::from_millis(200)));

        drop(service.send(Event::Change("r".to_string())));

        let expired = service.send(Event::DebounceExpired);
        let cancelled = service.send(Event::CancelDebounce);

        assert!(!expired.state_changed);
        assert!(!cancelled.state_changed);
    }

    #[test]
    fn search_input_input_attrs_marks_type_search_and_value() {
        let service = service(props().default_value("hello"));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("search"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("hello"));
    }

    #[test]
    fn search_input_root_aria_busy_present_when_loading() {
        let mut service = service(props());

        drop(service.send(Event::SetSearching(true)));

        let api = service.connect(&|_| {});
        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Role), Some("search"));
    }

    #[test]
    fn search_input_clear_trigger_hidden_when_empty_visible_when_populated() {
        let empty_svc = service(props());

        let api = empty_svc.connect(&|_| {});

        let hidden = api.clear_trigger_attrs();

        let display = hidden
            .iter_styles()
            .find(|(prop, _)| matches!(prop, CssProperty::Display))
            .map(|(_, value)| value.as_str());

        assert_eq!(display, Some("none"));

        let mut populated_svc = service(props());

        drop(populated_svc.send(Event::Change("q".to_string())));

        let api = populated_svc.connect(&|_| {});

        let visible = api.clear_trigger_attrs();

        let display = visible
            .iter_styles()
            .find(|(prop, _)| matches!(prop, CssProperty::Display));

        assert!(display.is_none());
    }

    #[test]
    fn search_input_required_sets_native_required_on_input() {
        let service = service(props().required(true));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert!(attrs.contains(&HtmlAttr::Required));
    }

    #[test]
    fn search_input_invalid_drives_describedby_error_message_and_aria_invalid() {
        let service = service(props().invalid(true));

        let api = service.connect(&|_| {});

        let attrs = api.input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ErrorMessage)),
            Some("search-error-message")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("search-error-message")
        );
    }

    #[test]
    fn search_input_part_attrs_delegates_to_each_part_method() {
        let service = service(props());

        let api = service.connect(&|_| {});

        for (part, snapshot) in [
            (Part::Root, snapshot_attrs(&api.root_attrs())),
            (Part::Label, snapshot_attrs(&api.label_attrs())),
            (Part::Input, snapshot_attrs(&api.input_attrs())),
            (
                Part::ClearTrigger,
                snapshot_attrs(&api.clear_trigger_attrs()),
            ),
            (
                Part::SubmitTrigger,
                snapshot_attrs(&api.submit_trigger_attrs()),
            ),
            (
                Part::LoadingIndicator,
                snapshot_attrs(&api.loading_indicator_attrs()),
            ),
            (Part::Description, snapshot_attrs(&api.description_attrs())),
            (
                Part::ErrorMessage,
                snapshot_attrs(&api.error_message_attrs()),
            ),
        ] {
            assert_eq!(
                snapshot_attrs(&api.part_attrs(part)),
                snapshot,
                "part_attrs disagrees with explicit accessor"
            );
        }
    }

    #[test]
    fn search_input_event_handlers_fan_out_through_send() {
        let received = core::cell::RefCell::new(Vec::<Event>::new());

        let service = service(props());

        let send = |event: Event| {
            received.borrow_mut().push(event);
        };

        let api = service.connect(&send);

        api.on_input_focus(true);
        api.on_input_blur();
        api.on_input_change("hi".to_string());
        api.on_clear_click();
        api.on_submit_click();

        let events = received.borrow();

        assert_eq!(events[0], Event::Focus { is_keyboard: true });
        assert_eq!(events[1], Event::Blur);
        assert_eq!(events[2], Event::Change("hi".to_string()));
        assert_eq!(events[3], Event::Clear);
        assert_eq!(events[4], Event::Submit);
    }

    #[test]
    fn search_input_root_idle_snapshot() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn search_input_root_focused_snapshot() {
        let mut service = service(props());

        drop(service.send(Event::Focus { is_keyboard: true }));

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn search_input_root_searching_snapshot() {
        let mut service = service(props());

        drop(service.send(Event::SetSearching(true)));

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn search_input_input_default_snapshot() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs()));
    }

    #[test]
    fn search_input_input_with_metadata_snapshot() {
        let service = service(
            props()
                .placeholder("Search...")
                .name("q")
                .form("results")
                .required(true)
                .invalid(true)
                .default_value("rust"),
        );

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.input_attrs()));
    }

    #[test]
    fn search_input_clear_trigger_hidden_snapshot() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.clear_trigger_attrs()));
    }

    #[test]
    fn search_input_clear_trigger_visible_snapshot() {
        let mut service = service(props());

        drop(service.send(Event::Change("rust".to_string())));

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.clear_trigger_attrs()));
    }

    #[test]
    fn search_input_submit_trigger_snapshot() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.submit_trigger_attrs()));
    }

    #[test]
    fn search_input_loading_indicator_snapshot() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.loading_indicator_attrs()));
    }

    #[test]
    fn search_input_description_snapshot() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.description_attrs()));
    }

    #[test]
    fn search_input_error_message_snapshot() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_snapshot!(snapshot_attrs(&api.error_message_attrs()));
    }
}
