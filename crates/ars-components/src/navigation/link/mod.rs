//! Link navigation component.
//!
//! This module owns the framework-agnostic link state machine: focus,
//! press, disabled guarding, current-item semantics, safe URL output, and
//! target/relationship repair for links that open a new tab.

use alloc::string::{String, ToString as _};
use core::{
    fmt::{self, Debug},
    hash::Hash,
};

use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, Locale, MessageFn, PendingEffect, SafeUrl, TransitionPlan, sanitize_url,
};

/// Describes the type of current-item indication for `aria-current`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AriaCurrent {
    /// The link represents the current page.
    Page,

    /// The link represents the current step.
    Step,

    /// The link represents the current location.
    Location,

    /// The link represents the current date.
    Date,

    /// The link represents the current time.
    Time,

    /// The link represents the current item using `aria-current="true"`.
    True,
}

impl AriaCurrent {
    /// Returns the token rendered into `aria-current`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Step => "step",
            Self::Location => "location",
            Self::Date => "date",
            Self::Time => "time",
            Self::True => "true",
        }
    }
}

/// Navigation target for a link.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    /// Browser-native URL navigation.
    Href(SafeUrl),

    /// Client-side route. Adapters may intercept activation, but the core
    /// still emits this string as an `href` for progressive enhancement.
    Route(String),
}

impl Default for Target {
    fn default() -> Self {
        Self::Href(SafeUrl::from_static(""))
    }
}

impl From<SafeUrl> for Target {
    fn from(value: SafeUrl) -> Self {
        Self::Href(value)
    }
}

impl Target {
    /// Returns the renderable href string for this target.
    #[must_use]
    pub fn href(&self) -> &str {
        match self {
            Self::Href(url) => url.as_str(),
            Self::Route(route) => route.as_str(),
        }
    }

    /// Returns `true` when this target is an absolute HTTP(S) URL.
    #[must_use]
    pub fn is_external(&self) -> bool {
        let href = self.href();

        href.starts_with("http://") || href.starts_with("https://")
    }
}

/// States for the [`Link`](self) component.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum State {
    /// Default resting state.
    #[default]
    Idle,

    /// The link has focus.
    Focused,

    /// The link is actively being pressed.
    Pressed,
}

impl State {
    /// Returns the token rendered into `data-ars-state`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Focused => "focused",
            Self::Pressed => "pressed",
        }
    }
}

/// Events accepted by the link state machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// The link received focus.
    Focus {
        /// Whether the focus came from keyboard modality.
        is_keyboard: bool,
    },

    /// Focus left the link.
    Blur,

    /// Pointer or keyboard press started.
    Press,

    /// Pointer or keyboard press ended.
    PressEnd,

    /// The link was activated.
    Navigate,

    /// Synchronize render props into context.
    SyncProps,
}

/// Typed effect intents emitted by the link machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// A non-disabled link was activated.
    Navigate,
}

/// Localized link messages.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Announcement text for links that open a new tab.
    pub external_link_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            external_link_label: MessageFn::static_str("opens in new tab"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Immutable configuration for a [`Link`](self) instance.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Navigation target.
    pub href: Target,

    /// Optional browsing context target.
    pub target: Option<String>,

    /// Optional relationship tokens.
    pub rel: Option<String>,

    /// Optional current-item semantic.
    pub is_current: Option<AriaCurrent>,

    /// Whether the link is non-interactive.
    pub disabled: bool,

    /// Callback fired by the navigate effect with the resolved target.
    pub on_navigate: Option<Callback<dyn Fn(Target) + Send + Sync>>,

    /// Callback fired by the navigate effect after `on_navigate`.
    pub on_press: Option<Callback<dyn Fn() + Send + Sync>>,
}

impl Props {
    /// Returns default link props.
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

    /// Sets [`href`](Self::href).
    #[must_use]
    pub fn href(mut self, href: impl Into<Target>) -> Self {
        self.href = href.into();
        self
    }

    /// Sets [`target`](Self::target).
    #[must_use]
    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Clears [`target`](Self::target).
    #[must_use]
    pub fn no_target(mut self) -> Self {
        self.target = None;
        self
    }

    /// Sets [`rel`](Self::rel).
    #[must_use]
    pub fn rel(mut self, rel: impl Into<String>) -> Self {
        self.rel = Some(rel.into());
        self
    }

    /// Clears [`rel`](Self::rel).
    #[must_use]
    pub fn no_rel(mut self) -> Self {
        self.rel = None;
        self
    }

    /// Sets [`is_current`](Self::is_current).
    #[must_use]
    pub const fn is_current(mut self, current: AriaCurrent) -> Self {
        self.is_current = Some(current);
        self
    }

    /// Clears [`is_current`](Self::is_current).
    #[must_use]
    pub const fn not_current(mut self) -> Self {
        self.is_current = None;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`on_navigate`](Self::on_navigate).
    #[must_use]
    pub fn on_navigate(
        mut self,
        callback: impl Into<Callback<dyn Fn(Target) + Send + Sync>>,
    ) -> Self {
        self.on_navigate = Some(callback.into());
        self
    }

    /// Sets [`on_press`](Self::on_press).
    #[must_use]
    pub fn on_press(mut self, callback: impl Into<Callback<dyn Fn() + Send + Sync>>) -> Self {
        self.on_press = Some(callback.into());
        self
    }
}

/// Runtime context for the link machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Current navigation target.
    pub href: Target,

    /// Current browsing context target.
    pub target: Option<String>,

    /// Current relationship tokens.
    pub rel: Option<String>,

    /// Current-item semantic.
    pub is_current: Option<AriaCurrent>,

    /// Disabled state.
    pub disabled: bool,

    /// Whether the link has focus.
    pub focused: bool,

    /// Whether focused styling should reflect keyboard modality.
    pub focus_visible: bool,

    /// Whether the link is actively pressed.
    pub pressed: bool,

    /// Stable ids derived from props.
    pub ids: ComponentIds,

    /// Active locale.
    pub locale: Locale,

    /// Localized messages.
    pub messages: Messages,
}

/// Anatomy parts exposed by the link connect API.
#[derive(ComponentPart)]
#[scope = "link"]
pub enum Part {
    /// Root anchor or repaired non-anchor host.
    Root,
}

/// Link state machine.
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

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            State::Idle,
            Context {
                href: props.href.clone(),
                target: props.target.clone(),
                rel: props.rel.clone(),
                is_current: props.is_current,
                disabled: props.disabled,
                focused: false,
                focus_visible: false,
                pressed: false,
                ids: ComponentIds::from_id(&props.id),
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::Focus { is_keyboard } => {
                let focus_visible = *is_keyboard;
                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = focus_visible;
                    }),
                )
            }

            Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                ctx.focused = false;
                ctx.focus_visible = false;
                ctx.pressed = false;
            })),

            Event::Press if !ctx.disabled => Some(TransitionPlan::to(State::Pressed).apply(
                |ctx: &mut Context| {
                    ctx.pressed = true;
                },
            )),

            Event::PressEnd if ctx.pressed => Some(TransitionPlan::to(State::Focused).apply(
                |ctx: &mut Context| {
                    ctx.pressed = false;
                },
            )),

            Event::Navigate if !ctx.disabled => Some(
                TransitionPlan::context_only(|_| {}).with_effect(PendingEffect::new(
                    Effect::Navigate,
                    |ctx: &Context, props: &Props, _send| {
                        if let Some(callback) = &props.on_navigate {
                            (callback)(ctx.href.clone());
                        }

                        if let Some(callback) = &props.on_press {
                            (callback)();
                        }

                        ars_core::no_cleanup()
                    },
                )),
            ),

            Event::SyncProps => {
                let href = props.href.clone();
                let target = props.target.clone();
                let rel = props.rel.clone();
                let is_current = props.is_current;
                let disabled = props.disabled;

                if disabled {
                    Some(
                        TransitionPlan::to(State::Idle).apply(move |ctx: &mut Context| {
                            ctx.href = href;
                            ctx.target = target;
                            ctx.rel = rel;
                            ctx.is_current = is_current;
                            ctx.disabled = disabled;
                            ctx.focused = false;
                            ctx.focus_visible = false;
                            ctx.pressed = false;
                        }),
                    )
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.href = href;
                        ctx.target = target;
                        ctx.rel = rel;
                        ctx.is_current = is_current;
                        ctx.disabled = disabled;
                    }))
                }
            }

            _ => None,
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

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old.href != new.href
            || old.target != new.target
            || old.rel != new.rel
            || old.is_current != new.is_current
            || old.disabled != new.disabled
        {
            vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// Connected API for a [`Link`](self) service.
pub struct Api<'a> {
    /// Current state.
    state: &'a State,

    /// Current context.
    ctx: &'a Context,

    /// Current props.
    props: &'a Props,

    /// Event dispatcher.
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Attributes for the root link host.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_value), (part_attr, part_value)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_value)
            .set(part_attr, part_value)
            .set(HtmlAttr::Data("ars-state"), self.state.as_str());

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        } else {
            attrs.set(HtmlAttr::Href, sanitize_url(self.ctx.href.href()));

            let target = self.resolved_target();

            if let Some(target) = target.as_deref() {
                attrs.set(HtmlAttr::Target, target);
            }

            let rel = self.resolved_rel(target.as_deref());

            if let Some(rel) = rel {
                attrs.set(HtmlAttr::Rel, rel);
            }

            if target.as_deref() == Some("_blank") {
                attrs.set_bool(HtmlAttr::Data("ars-external"), true).set(
                    HtmlAttr::Aria(AriaAttr::Description),
                    (self.ctx.messages.external_link_label)(&self.ctx.locale),
                );
            }
        }

        if let Some(current) = self.ctx.is_current {
            attrs.set(HtmlAttr::Aria(AriaAttr::Current), current.as_str());
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.ctx.pressed {
            attrs.set_bool(HtmlAttr::Data("ars-pressed"), true);
        }

        attrs
    }

    /// Dispatches the link activation event.
    pub fn on_click(&self) {
        (self.send)(Event::Navigate);
    }

    /// Dispatches the focus event.
    pub fn on_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches the blur event.
    pub fn on_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches the press-start event.
    pub fn on_pointer_down(&self) {
        (self.send)(Event::Press);
    }

    /// Dispatches the press-end event.
    pub fn on_pointer_up(&self) {
        (self.send)(Event::PressEnd);
    }

    /// Returns `true` when the link is disabled.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.ctx.disabled
    }

    /// Returns `true` when the link has keyboard-visible focus.
    #[must_use]
    pub const fn is_focus_visible(&self) -> bool {
        self.ctx.focus_visible
    }

    /// Returns the target props used to create this API.
    #[must_use]
    pub const fn props(&self) -> &Props {
        self.props
    }

    fn resolved_target(&self) -> Option<String> {
        self.ctx
            .target
            .clone()
            .or_else(|| self.ctx.href.is_external().then(|| "_blank".to_string()))
    }

    fn resolved_rel(&self, target: Option<&str>) -> Option<String> {
        self.ctx
            .rel
            .clone()
            .or_else(|| (target == Some("_blank")).then(|| "noopener noreferrer".to_string()))
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::ToString as _, sync::Arc, vec::Vec};
    use std::sync::{Mutex, MutexGuard};

    use ars_core::{Service, StrongSend};

    use super::*;

    fn props() -> Props {
        Props::new().id("link").href(SafeUrl::from_static("/docs"))
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
        mutex.lock().expect("test mutex should not be poisoned")
    }

    #[test]
    fn root_attrs_emit_href_and_state() {
        let service = service(props());

        insta::assert_snapshot!(
            "link_root_default",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn external_href_gets_target_rel_and_announcement() {
        let service = service(
            props()
                .href(SafeUrl::from_static("https://example.com/docs"))
                .is_current(AriaCurrent::Page),
        );

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Target), Some("_blank"));
        assert_eq!(attrs.get(&HtmlAttr::Rel), Some("noopener noreferrer"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Current)), Some("page"));

        insta::assert_snapshot!("link_root_external", snapshot_attrs(&attrs));
    }

    #[test]
    fn explicit_blank_target_repairs_missing_rel() {
        let service = service(props().target("_blank"));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Rel), Some("noopener noreferrer"));
    }

    #[test]
    fn explicit_rel_wins_over_blank_target_repair() {
        let service = service(props().target("_blank").rel("external"));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Rel), Some("external"));
    }

    #[test]
    fn disabled_link_omits_href_and_sets_aria_disabled() {
        let service = service(props().disabled(true));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert!(attrs.get(&HtmlAttr::Href).is_none());

        insta::assert_snapshot!("link_root_disabled", snapshot_attrs(&attrs));
    }

    #[test]
    fn focus_and_press_states_update_attrs() {
        let mut service = service(props());

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert_eq!(service.state(), &State::Focused);
        assert_eq!(
            service
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-focus-visible")),
            Some("true")
        );

        drop(service.send(Event::Press));

        assert_eq!(service.state(), &State::Pressed);
        assert_eq!(
            service
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-pressed")),
            Some("true")
        );

        drop(service.send(Event::PressEnd));

        assert_eq!(service.state(), &State::Focused);

        drop(service.send(Event::Blur));

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().focus_visible);
        assert!(!service.context().pressed);
    }

    #[test]
    fn navigate_emits_effect_and_runs_callbacks() {
        let navigated = Arc::new(Mutex::new(Vec::new()));
        let pressed = Arc::new(Mutex::new(0usize));
        let navigated_clone = Arc::clone(&navigated);
        let pressed_clone = Arc::clone(&pressed);

        let mut service = service(
            props()
                .href(Target::Route("/settings".to_string()))
                .on_navigate(move |target| lock(&navigated_clone).push(target))
                .on_press(move || *lock(&pressed_clone) += 1),
        );

        let result = service.send(Event::Navigate);

        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::Navigate);

        let send: StrongSend<Event> = Arc::new(|_| {});

        for effect in result.pending_effects {
            drop(effect.run(service.context(), service.props(), Arc::clone(&send)));
        }

        assert_eq!(
            lock(&navigated).as_slice(),
            &[Target::Route("/settings".to_string())]
        );
        assert_eq!(*lock(&pressed), 1);
    }

    #[test]
    fn disabled_navigate_does_not_emit_effect() {
        let mut service = service(props().disabled(true));

        assert!(service.send(Event::Navigate).pending_effects.is_empty());
    }

    #[test]
    fn all_aria_current_variants_render_tokens() {
        for (variant, token) in [
            (AriaCurrent::Page, "page"),
            (AriaCurrent::Step, "step"),
            (AriaCurrent::Location, "location"),
            (AriaCurrent::Date, "date"),
            (AriaCurrent::Time, "time"),
            (AriaCurrent::True, "true"),
        ] {
            let service = service(props().is_current(variant));

            let attrs = service.connect(&|_| {}).root_attrs();

            assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Current)), Some(token));
        }
    }

    #[test]
    fn props_builder_clears_target_and_rel_without_resetting_other_fields() {
        let props = props()
            .href(Target::Route("/route".to_string()))
            .target("_blank")
            .rel("external")
            .is_current(AriaCurrent::Location)
            .disabled(true)
            .no_target()
            .no_rel();

        assert_eq!(props.id, "link");
        assert_eq!(props.href, Target::Route("/route".to_string()));
        assert_eq!(props.target, None);
        assert_eq!(props.rel, None);
        assert_eq!(props.is_current, Some(AriaCurrent::Location));
        assert!(props.disabled);
    }

    #[test]
    fn disabled_press_is_ignored() {
        let mut service = service(props().disabled(true));

        let result = service.send(Event::Press);

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().pressed);
    }

    #[test]
    fn press_end_without_active_press_is_ignored() {
        let mut service = service(props());

        let result = service.send(Event::PressEnd);

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().pressed);

        drop(service.send(Event::Press));
        drop(service.set_props(props().disabled(true)));

        let result = service.send(Event::PressEnd);

        assert!(!result.state_changed);
        assert!(!result.context_changed);
        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().pressed);
    }

    #[test]
    fn set_props_syncs_context_fields_and_clears_disabled_press() {
        let mut service = service(
            props()
                .target("_blank")
                .rel("external")
                .is_current(AriaCurrent::Page),
        );

        drop(service.send(Event::Press));

        assert!(service.context().pressed);

        drop(
            service.set_props(
                props()
                    .href(Target::Route("/updated".to_string()))
                    .disabled(true)
                    .is_current(AriaCurrent::Step),
            ),
        );

        assert_eq!(
            service.context().href,
            Target::Route("/updated".to_string())
        );
        assert_eq!(service.context().target, None);
        assert_eq!(service.context().rel, None);
        assert_eq!(service.context().is_current, Some(AriaCurrent::Step));
        assert!(service.context().disabled);
        assert!(!service.context().pressed);
    }

    #[test]
    fn disabling_focused_or_pressed_link_resets_interaction_state() {
        let mut service = service(props());

        drop(service.send(Event::Focus { is_keyboard: true }));
        drop(service.send(Event::Press));

        drop(service.set_props(props().disabled(true)));

        assert_eq!(service.state(), &State::Idle);
        assert!(service.context().disabled);
        assert!(!service.context().focused);
        assert!(!service.context().focus_visible);
        assert!(!service.context().pressed);

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("idle"));
        assert!(attrs.get(&HtmlAttr::Data("ars-focus-visible")).is_none());
        assert!(attrs.get(&HtmlAttr::Data("ars-pressed")).is_none());
    }

    #[test]
    fn on_props_changed_detects_each_context_field() {
        let old = props()
            .target("_blank")
            .rel("external")
            .is_current(AriaCurrent::Page);

        assert!(<Machine as ars_core::Machine>::on_props_changed(&old, &old).is_empty());

        for new in [
            old.clone().href(Target::Route("/next".to_string())),
            old.clone().target("_self"),
            old.clone().rel("bookmark"),
            old.clone().is_current(AriaCurrent::Step),
            old.clone().disabled(true),
        ] {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&old, &new),
                [Event::SyncProps],
                "expected SyncProps for {new:?}"
            );
        }
    }

    #[test]
    fn api_handlers_dispatch_events_and_accessors_report_context() {
        let mut focused = service(props());

        drop(focused.send(Event::Focus { is_keyboard: true }));

        let focused_api = focused.connect(&|_| {});

        assert!(!focused_api.is_disabled());
        assert!(focused_api.is_focus_visible());

        let disabled = service(props().disabled(true));
        let disabled_api = disabled.connect(&|_| {});

        assert!(disabled_api.is_disabled());
        assert!(!disabled_api.is_focus_visible());

        let sent = Mutex::new(Vec::new());
        let send = |event| lock(&sent).push(event);

        let api = disabled.connect(&send);

        api.on_click();
        api.on_focus(false);
        api.on_blur();
        api.on_pointer_down();
        api.on_pointer_up();

        assert_eq!(
            lock(&sent).as_slice(),
            &[
                Event::Navigate,
                Event::Focus { is_keyboard: false },
                Event::Blur,
                Event::Press,
                Event::PressEnd,
            ]
        );
    }

    #[test]
    fn part_attrs_dispatches_root_attrs() {
        let service = service(props());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }
}
