//! Clipboard component state machine and connect API.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Debug, Display};

use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Env, HasId, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan, WeakSend,
    no_cleanup,
};

/// Why a clipboard copy operation failed.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CopyFailureReason {
    /// User denied clipboard access.
    PermissionDenied,

    /// Not HTTPS; the Clipboard API requires a secure context.
    NotSecureContext,

    /// Clipboard operation exceeded the platform timeout.
    Timeout,

    /// Neither `navigator.clipboard` nor a fallback copy API is available.
    ApiUnavailable,

    /// Unexpected error from the browser API.
    Unknown(String),
}

/// The state of the `Clipboard` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Waiting for user copy intent.
    Idle,

    /// A copy operation has been requested and is awaiting adapter completion.
    Copying,

    /// Copy succeeded and success feedback is visible.
    Copied,

    /// Copy failed and error feedback is visible.
    Error,
}

impl Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Idle => "idle",
            Self::Copying => "copying",
            Self::Copied => "copied",
            Self::Error => "error",
        })
    }
}

/// Events for the `Clipboard` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// User triggered a copy.
    Copy,

    /// Adapter reported a successful clipboard write.
    CopySuccess,

    /// Adapter reported a failed clipboard write.
    CopyError(CopyFailureReason),

    /// Feedback timeout expired; return to idle.
    ResetTimeout,

    /// Synchronize the externally controlled value prop.
    SetValue(Option<String>),

    /// Synchronize output-affecting props stored in context.
    SetProps,
}

/// Dynamic callable signature for [`Props::on_copy`].
pub type CopyRequestFn = dyn Fn((String, WeakSend<Event>)) + Send + Sync;

/// Context for the `Clipboard` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The text to copy, controlled by the parent or internally owned.
    pub value: Bindable<String>,

    /// How long copied/error feedback remains visible, in milliseconds.
    pub feedback_duration_ms: u32,

    /// Whether copy intent is disabled.
    pub disabled: bool,

    /// The reason the last copy failed, if the machine is in error feedback.
    pub error: Option<CopyFailureReason>,

    /// Active locale inherited from provider context.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,

    /// Component instance IDs.
    pub ids: ComponentIds,
}

/// Props for the `Clipboard` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Controlled text copied by the trigger.
    pub value: Option<String>,

    /// Default text copied by the trigger in uncontrolled mode.
    pub default_value: String,

    /// Duration to show copied/error feedback, in milliseconds.
    pub feedback_duration_ms: u32,

    /// Disabled state.
    pub disabled: bool,

    /// Callback invoked by the write-text effect with the current text and send handle.
    pub on_copy: Option<Callback<CopyRequestFn>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: String::new(),
            feedback_duration_ms: 2_000,
            disabled: false,
            on_copy: None,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
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

    /// Sets [`value`](Self::value), switching the copied text to controlled mode.
    #[must_use]
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Clears [`value`](Self::value), switching the copied text to uncontrolled mode.
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

    /// Sets [`feedback_duration_ms`](Self::feedback_duration_ms).
    #[must_use]
    pub const fn feedback_duration_ms(mut self, duration: u32) -> Self {
        self.feedback_duration_ms = duration;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`on_copy`](Self::on_copy).
    #[must_use]
    pub fn on_copy(mut self, callback: impl Into<Callback<CopyRequestFn>>) -> Self {
        self.on_copy = Some(callback.into());
        self
    }
}

/// Messages for the `Clipboard` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Label for the copy trigger button.
    pub trigger_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Label while copy is in progress.
    pub copying_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Feedback text when copy succeeds.
    pub copied_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Feedback text when copy fails.
    pub error_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Announcement when copy succeeds.
    pub copied_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Announcement when copy fails.
    pub error_announcement: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Copy to clipboard"),
            copying_label: MessageFn::static_str("Copying..."),
            copied_label: MessageFn::static_str("Copied!"),
            error_label: MessageFn::static_str("Copy failed, click to retry"),
            copied_announcement: MessageFn::static_str("Copied to clipboard"),
            error_announcement: MessageFn::static_str("Failed to copy to clipboard"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed effect intents emitted by the clipboard machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter writes the current text to the platform clipboard.
    WriteText,

    /// Adapter starts a feedback timer that dispatches [`Event::ResetTimeout`].
    FeedbackTimer,

    /// Adapter announces the copied feedback message.
    AnnounceCopied,

    /// Adapter announces the error feedback message.
    AnnounceError,
}

/// The machine for the `Clipboard` component.
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
                feedback_duration_ms: props.feedback_duration_ms,
                disabled: props.disabled,
                error: None,
                locale: env.locale.clone(),
                messages: messages.clone(),
                ids: ComponentIds::from_id(&props.id),
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
            (_, Event::Copy) if props.disabled => None,

            (State::Idle, Event::Copy) => Some(copying_plan()),

            (State::Copied | State::Error, Event::Copy) => {
                Some(copying_plan().cancel_effect(Effect::FeedbackTimer))
            }

            (State::Copying, Event::CopySuccess) => Some(
                TransitionPlan::to(State::Copied)
                    .apply(|ctx: &mut Context| {
                        ctx.error = None;
                    })
                    .with_effect(PendingEffect::named(Effect::AnnounceCopied))
                    .with_effect(PendingEffect::named(Effect::FeedbackTimer)),
            ),

            (State::Copying, Event::CopyError(reason)) => {
                let reason = reason.clone();
                Some(
                    TransitionPlan::to(State::Error)
                        .apply(move |ctx: &mut Context| {
                            ctx.error = Some(reason);
                        })
                        .with_effect(PendingEffect::named(Effect::AnnounceError))
                        .with_effect(PendingEffect::named(Effect::FeedbackTimer)),
                )
            }

            (State::Copied | State::Error, Event::ResetTimeout) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.error = None;
                }))
            }

            (_, Event::SetValue(value)) => {
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

            (_, Event::SetProps) => Some(TransitionPlan::context_only({
                let feedback_duration_ms = props.feedback_duration_ms;
                let disabled = props.disabled;

                move |ctx: &mut Context| {
                    ctx.feedback_duration_ms = feedback_duration_ms;
                    ctx.disabled = disabled;
                }
            })),

            _ => None,
        }
    }

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "clipboard::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.value != new.value {
            events.push(Event::SetValue(new.value.clone()));
        }

        if old.feedback_duration_ms != new.feedback_duration_ms || old.disabled != new.disabled {
            events.push(Event::SetProps);
        }

        events
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

/// DOM parts of the `Clipboard` component.
#[derive(ComponentPart)]
#[scope = "clipboard"]
pub enum Part {
    /// Root wrapper element.
    Root,

    /// Label describing what will be copied.
    Label,

    /// Button that initiates copy.
    Trigger,

    /// Decorative visual state indicator.
    Indicator,

    /// Live region for copy feedback.
    Status,

    /// Text display for the value being copied.
    ValueText,
}

/// API for the `Clipboard` component.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("clipboard::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Whether copy succeeded and success feedback is visible.
    #[must_use]
    pub const fn is_copied(&self) -> bool {
        matches!(self.state, State::Copied)
    }

    /// Whether a copy operation is in progress.
    #[must_use]
    pub const fn is_copying(&self) -> bool {
        matches!(self.state, State::Copying)
    }

    /// Whether copy failed and error feedback is visible.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self.state, State::Error)
    }

    /// Returns the current copy error reason, if any.
    #[must_use]
    pub const fn error(&self) -> Option<&CopyFailureReason> {
        self.ctx.error.as_ref()
    }

    /// Returns the current text to copy.
    #[must_use]
    pub fn value(&self) -> &str {
        self.ctx.value.get()
    }

    /// Dispatches copy intent.
    pub fn copy(&self) {
        (self.send)(Event::Copy);
    }

    /// Root element attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Label element attributes.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Label.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Trigger button attributes.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("trigger"))
            .set(HtmlAttr::Type, "button")
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Label), self.trigger_label())
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        if self.ctx.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Indicator element attributes.
    #[must_use]
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        attrs
    }

    /// Status live-region attributes.
    #[must_use]
    pub fn status_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Status.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "status")
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "true");

        attrs
    }

    /// Value text attributes.
    #[must_use]
    pub fn value_text_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ValueText.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Dispatches trigger click intent.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Copy);
    }

    const fn state_token(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Copying => "copying",
            State::Copied => "copied",
            State::Error => "error",
        }
    }

    fn trigger_label(&self) -> String {
        match self.state {
            State::Idle => (self.ctx.messages.trigger_label)(&self.ctx.locale),
            State::Copying => (self.ctx.messages.copying_label)(&self.ctx.locale),
            State::Copied => (self.ctx.messages.copied_label)(&self.ctx.locale),
            State::Error => (self.ctx.messages.error_label)(&self.ctx.locale),
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Indicator => self.indicator_attrs(),
            Part::Status => self.status_attrs(),
            Part::ValueText => self.value_text_attrs(),
        }
    }
}

fn copying_plan() -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Copying)
        .apply(|ctx: &mut Context| {
            ctx.error = None;
        })
        .with_effect(write_text_effect())
}

fn write_text_effect() -> PendingEffect<Machine> {
    PendingEffect::new(Effect::WriteText, |ctx: &Context, props: &Props, send| {
        if let Some(on_copy) = &props.on_copy {
            on_copy((ctx.value.get().clone(), send));
        } else {
            send.call_if_alive(Event::CopyError(CopyFailureReason::ApiUnavailable));
        }

        no_cleanup()
    })
}

#[cfg(test)]
mod tests {
    use alloc::{
        string::{String, ToString},
        sync::Arc,
        vec,
        vec::Vec,
    };
    use std::sync::Mutex;

    use ars_core::{
        AriaAttr, AttrMap, AttrValue, ConnectApi, Env, HtmlAttr, Machine as _, Service, StrongSend,
        WeakSend, callback,
    };
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "clip".into(),
            default_value: "copy me".into(),
            ..Props::default()
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn api_for_state(state: State) -> Api<'static> {
        let props = Box::leak(Box::new(test_props()));

        let messages = Messages::default();

        let (_, mut ctx) = Machine::init(props, &Env::default(), &messages);

        ctx.messages = messages;

        let ctx = Box::leak(Box::new(ctx));
        let state = Box::leak(Box::new(state));

        let send = Box::leak(Box::new(|_: Event| {}));

        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn api_for_error(reason: CopyFailureReason) -> Api<'static> {
        let props = Box::leak(Box::new(test_props()));

        let messages = Messages::default();

        let (_, mut ctx) = Machine::init(props, &Env::default(), &messages);

        ctx.error = Some(reason);

        let ctx = Box::leak(Box::new(ctx));
        let state = Box::leak(Box::new(State::Error));
        let send = Box::leak(Box::new(|_: Event| {}));

        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn state_after_success() -> Service<Machine> {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::Copy));
        drop(service.send(Event::CopySuccess));

        service
    }

    fn state_after_error() -> Service<Machine> {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::Copy));
        drop(service.send(Event::CopyError(CopyFailureReason::PermissionDenied)));

        service
    }

    #[test]
    fn clipboard_props_default_matches_spec() {
        let props = Props::default();

        assert_eq!(props.id, "");
        assert_eq!(props.value, None);
        assert_eq!(props.default_value, "");
        assert_eq!(props.feedback_duration_ms, 2_000);
        assert!(!props.disabled);
        assert!(props.on_copy.is_none());
    }

    #[test]
    fn clipboard_props_builder_sets_expected_fields() {
        let props = Props::new()
            .id("clip")
            .value("controlled")
            .uncontrolled()
            .default_value("fallback")
            .feedback_duration_ms(750)
            .disabled(true)
            .on_copy(callback(|_: (String, WeakSend<Event>)| {}));

        assert_eq!(props.id, "clip");
        assert_eq!(props.value, None);
        assert_eq!(props.default_value, "fallback");
        assert_eq!(props.feedback_duration_ms, 750);
        assert!(props.disabled);
        assert!(props.on_copy.is_some());
    }

    #[test]
    fn clipboard_state_display_matches_data_state_tokens() {
        assert_eq!(State::Idle.to_string(), "idle");
        assert_eq!(State::Copying.to_string(), "copying");
        assert_eq!(State::Copied.to_string(), "copied");
        assert_eq!(State::Error.to_string(), "error");
    }

    #[test]
    fn clipboard_initial_state_uses_default_value() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().value.get(), "copy me");
        assert!(!service.context().value.is_controlled());
        assert_eq!(service.context().feedback_duration_ms, 2_000);
        assert!(!service.context().disabled);
        assert_eq!(service.context().error, None);
        assert_eq!(service.context().ids.id(), "clip");
    }

    #[test]
    fn clipboard_initial_state_uses_controlled_value() {
        let service = Service::<Machine>::new(
            Props::new().id("clip").value("controlled"),
            &Env::default(),
            &Messages::default(),
        );

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().value.get(), "controlled");
        assert!(service.context().value.is_controlled());
    }

    #[test]
    fn clipboard_idle_copy_enters_copying_and_emits_write_effect() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let result = service.send(Event::Copy);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Copying);
        assert_eq!(service.context().error, None);
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::WriteText);
    }

    #[test]
    fn clipboard_copy_success_enters_copied_announces_and_starts_timer() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::Copy));

        let result = service.send(Event::CopySuccess);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Copied);
        assert_eq!(service.context().error, None);
        assert_eq!(
            result
                .pending_effects
                .iter()
                .map(|effect| effect.name)
                .collect::<Vec<_>>(),
            vec![Effect::AnnounceCopied, Effect::FeedbackTimer]
        );
    }

    #[test]
    fn clipboard_copy_error_enters_error_announces_and_starts_timer() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(service.send(Event::Copy));

        let result = service.send(Event::CopyError(CopyFailureReason::Timeout));

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Error);
        assert_eq!(service.context().error, Some(CopyFailureReason::Timeout));
        assert_eq!(
            result
                .pending_effects
                .iter()
                .map(|effect| effect.name)
                .collect::<Vec<_>>(),
            vec![Effect::AnnounceError, Effect::FeedbackTimer]
        );
    }

    #[test]
    fn clipboard_reset_timeout_returns_to_idle_from_copied() {
        let mut service = state_after_success();

        let result = service.send(Event::ResetTimeout);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().error, None);
    }

    #[test]
    fn clipboard_reset_timeout_returns_to_idle_from_error() {
        let mut service = state_after_error();

        let result = service.send(Event::ResetTimeout);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().error, None);
    }

    #[test]
    fn clipboard_recopy_from_copied_cancels_timer_and_writes_again() {
        let mut service = state_after_success();

        let result = service.send(Event::Copy);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Copying);
        assert_eq!(service.context().error, None);
        assert_eq!(result.cancel_effects, vec![Effect::FeedbackTimer]);
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::WriteText);
    }

    #[test]
    fn clipboard_disabled_blocks_copy_intent() {
        let mut service = Service::<Machine>::new(
            Props {
                disabled: true,
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        let result = service.send(Event::Copy);

        assert!(!result.state_changed);
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.state(), &State::Idle);
    }

    #[test]
    fn clipboard_set_props_syncs_value_feedback_duration_and_disabled() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let result = service.set_props(
            Props::new()
                .id("clip")
                .value("updated")
                .feedback_duration_ms(300)
                .disabled(true),
        );

        assert!(result.context_changed);
        assert_eq!(service.context().value.get(), "updated");
        assert!(service.context().value.is_controlled());
        assert_eq!(service.context().feedback_duration_ms, 300);
        assert!(service.context().disabled);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn clipboard_on_props_changed_detects_each_behavioral_prop() {
        let base = test_props();

        assert!(<Machine as ars_core::Machine>::on_props_changed(&base, &base).is_empty());

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &Props {
                    value: Some("updated".into()),
                    ..base.clone()
                },
            ),
            vec![Event::SetValue(Some("updated".into()))]
        );

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &Props {
                    feedback_duration_ms: 750,
                    ..base.clone()
                },
            ),
            vec![Event::SetProps]
        );

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &Props {
                    disabled: true,
                    ..base.clone()
                },
            ),
            vec![Event::SetProps]
        );
    }

    #[test]
    fn clipboard_api_accessors_report_each_state_error_and_value() {
        let idle = api_for_state(State::Idle);

        assert!(!idle.is_copied());
        assert!(!idle.is_copying());
        assert!(!idle.is_error());
        assert_eq!(idle.error(), None);
        assert_eq!(idle.value(), "copy me");

        let copying = api_for_state(State::Copying);

        assert!(!copying.is_copied());
        assert!(copying.is_copying());
        assert!(!copying.is_error());

        let copied = api_for_state(State::Copied);

        assert!(copied.is_copied());
        assert!(!copied.is_copying());
        assert!(!copied.is_error());

        let error = api_for_error(CopyFailureReason::NotSecureContext);

        assert!(!error.is_copied());
        assert!(!error.is_copying());
        assert!(error.is_error());
        assert_eq!(error.error(), Some(&CopyFailureReason::NotSecureContext));
    }

    #[test]
    fn clipboard_api_copy_sends_copy_event() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let send_event = move |event| captured.lock().unwrap().push(event);

        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let api = service.connect(&send_event);

        api.copy();

        assert_eq!(*events.lock().unwrap(), vec![Event::Copy]);
    }

    #[test]
    fn clipboard_trigger_click_sends_copy_event() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&events);
        let send_event = move |event| captured.lock().unwrap().push(event);

        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let api = service.connect(&send_event);

        api.on_trigger_click();

        assert_eq!(*events.lock().unwrap(), vec![Event::Copy]);
    }

    #[test]
    fn clipboard_trigger_attrs_use_state_specific_aria_labels() {
        assert_eq!(
            api_for_state(State::Idle)
                .trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Copy to clipboard")
        );
        assert_eq!(
            api_for_state(State::Copying)
                .trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Copying...")
        );
        assert_eq!(
            api_for_state(State::Copied)
                .trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Copied!")
        );
        assert_eq!(
            api_for_state(State::Error)
                .trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Copy failed, click to retry")
        );
    }

    #[test]
    fn clipboard_trigger_attrs_do_not_combine_accessible_name_sources() {
        let attrs = api_for_state(State::Copied).trigger_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Copied!"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::LabelledBy)));
    }

    #[test]
    fn clipboard_indicator_tracks_data_state() {
        assert_eq!(
            api_for_state(State::Copying)
                .indicator_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("copying")
        );
        assert_eq!(
            api_for_state(State::Copied)
                .indicator_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("copied")
        );
    }

    #[test]
    fn clipboard_status_attrs_are_live_region() {
        let attrs = api_for_state(State::Idle).status_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("status"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
    }

    #[test]
    fn clipboard_connect_api_dispatch_matches_inherent_attrs() {
        let api = api_for_state(State::Idle);

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Label), api.label_attrs());
        assert_eq!(api.part_attrs(Part::Trigger), api.trigger_attrs());
        assert_eq!(api.part_attrs(Part::Indicator), api.indicator_attrs());
        assert_eq!(api.part_attrs(Part::Status), api.status_attrs());
        assert_eq!(api.part_attrs(Part::ValueText), api.value_text_attrs());
    }

    #[test]
    fn clipboard_write_effect_invokes_copy_callback_with_current_value() {
        let copied = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&copied);
        let mut service = Service::<Machine>::new(
            Props {
                on_copy: Some(callback(move |(value, send): (String, WeakSend<Event>)| {
                    captured.lock().unwrap().push(value);
                    send.call_if_alive(Event::CopySuccess);
                })),
                ..test_props()
            },
            &Env::default(),
            &Messages::default(),
        );

        let result = service.send(Event::Copy);

        let sent = Arc::new(Mutex::new(Vec::new()));
        let captured_sent = Arc::clone(&sent);
        let send: StrongSend<Event> = Arc::new(move |event| {
            captured_sent.lock().unwrap().push(event);
        });

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(*copied.lock().unwrap(), vec!["copy me".to_string()]);
        assert_eq!(*sent.lock().unwrap(), vec![Event::CopySuccess]);
    }

    #[test]
    fn clipboard_write_effect_without_copy_callback_reports_api_unavailable() {
        let mut service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        let result = service.send(Event::Copy);

        let sent = Arc::new(Mutex::new(Vec::new()));
        let captured_sent = Arc::clone(&sent);
        let send: StrongSend<Event> = Arc::new(move |event| {
            captured_sent.lock().unwrap().push(event);
        });

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(
            *sent.lock().unwrap(),
            vec![Event::CopyError(CopyFailureReason::ApiUnavailable)]
        );
    }

    #[test]
    fn clipboard_timer_effect_target_state_is_copied_or_error() {
        let mut copied_service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(copied_service.send(Event::Copy));

        let copied_result = copied_service.send(Event::CopySuccess);

        assert!(
            copied_result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::FeedbackTimer
                    && effect.target_state == Some(State::Copied))
        );

        let mut error_service =
            Service::<Machine>::new(test_props(), &Env::default(), &Messages::default());

        drop(error_service.send(Event::Copy));

        let error_result = error_service.send(Event::CopyError(CopyFailureReason::ApiUnavailable));

        assert!(
            error_result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::FeedbackTimer
                    && effect.target_state == Some(State::Error))
        );
    }

    #[test]
    fn clipboard_root_idle_snapshot() {
        assert_snapshot!(snapshot_attrs(&api_for_state(State::Idle).root_attrs()));
    }

    #[test]
    fn clipboard_root_disabled_snapshot() {
        let props = Box::leak(Box::new(Props {
            disabled: true,
            ..test_props()
        }));

        let messages = Messages::default();

        let (state, ctx) = Machine::init(props, &Env::default(), &messages);

        let state = Box::leak(Box::new(state));
        let ctx = Box::leak(Box::new(ctx));
        let send = Box::leak(Box::new(|_: Event| {}));

        let api = Api {
            state,
            ctx,
            props,
            send,
        };

        assert_snapshot!(snapshot_attrs(&api.root_attrs()));
    }

    #[test]
    fn clipboard_trigger_idle_snapshot() {
        assert_snapshot!(snapshot_attrs(&api_for_state(State::Idle).trigger_attrs()));
    }

    #[test]
    fn clipboard_trigger_copying_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api_for_state(State::Copying).trigger_attrs()
        ));
    }

    #[test]
    fn clipboard_trigger_copied_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api_for_state(State::Copied).trigger_attrs()
        ));
    }

    #[test]
    fn clipboard_trigger_error_snapshot() {
        assert_snapshot!(snapshot_attrs(&api_for_state(State::Error).trigger_attrs()));
    }

    #[test]
    fn clipboard_indicator_copying_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api_for_state(State::Copying).indicator_attrs()
        ));
    }

    #[test]
    fn clipboard_indicator_copied_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api_for_state(State::Copied).indicator_attrs()
        ));
    }

    #[test]
    fn clipboard_indicator_error_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api_for_state(State::Error).indicator_attrs()
        ));
    }

    #[test]
    fn clipboard_status_snapshot() {
        assert_snapshot!(snapshot_attrs(&api_for_state(State::Idle).status_attrs()));
    }

    #[test]
    fn clipboard_label_snapshot() {
        assert_snapshot!(snapshot_attrs(&api_for_state(State::Idle).label_attrs()));
    }

    #[test]
    fn clipboard_value_text_snapshot() {
        assert_snapshot!(snapshot_attrs(
            &api_for_state(State::Idle).value_text_attrs()
        ));
    }

    #[test]
    fn clipboard_disabled_trigger_sets_bool_data_attr() {
        let props = Box::leak(Box::new(Props {
            disabled: true,
            ..test_props()
        }));

        let messages = Messages::default();

        let (state, ctx) = Machine::init(props, &Env::default(), &messages);

        let state = Box::leak(Box::new(state));
        let ctx = Box::leak(Box::new(ctx));
        let send = Box::leak(Box::new(|_: Event| {}));

        let api = Api {
            state,
            ctx,
            props,
            send,
        };

        let attrs = api.trigger_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-disabled")),
            Some(&AttrValue::Bool(true))
        );
    }
}
