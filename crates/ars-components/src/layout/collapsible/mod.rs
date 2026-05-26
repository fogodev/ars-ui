//! Collapsible layout component.
//!
//! `Collapsible` owns DOM-free disclosure state, semantic relationships, and
//! adapter-consumable animation intent. Framework adapters keep live element
//! handles for measurement and transition listeners.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    CssProperty, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

/// States of the `Collapsible`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum State {
    /// The content region is visible.
    Open,

    /// The content region is hidden.
    #[default]
    Closed,
}

/// Events sent to the `Collapsible`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Event {
    /// Toggle between open and closed states.
    Toggle,

    /// Programmatically set the open state.
    SetOpen(bool),

    /// The trigger or content received focus.
    Focus {
        /// Whether the focus is received via keyboard.
        is_keyboard: bool,
    },

    /// The trigger or content lost focus.
    Blur,

    /// Synchronize prop-backed context fields.
    SyncProps,

    /// Synchronize the controlled open value.
    SyncControlledOpen(bool),
}

/// Runtime context for the `Collapsible` state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the collapsible is open.
    pub open: Bindable<bool>,

    /// Whether the trigger is non-interactive and toggle is suppressed.
    pub disabled: bool,

    /// Whether the trigger currently has focus.
    pub focused: bool,

    /// Whether focus was received via keyboard.
    pub focus_visible: bool,

    /// Component identifiers for ARIA attribute generation.
    pub ids: ComponentIds,

    /// CSS height to expose while closed for partial-content layouts.
    pub collapsed_height: Option<String>,

    /// CSS width to expose while closed for horizontal partial-content layouts.
    pub collapsed_width: Option<String>,

    /// Resolved locale for message formatting.
    pub locale: Locale,

    /// Resolved translatable messages.
    pub messages: Messages,
}

/// Configuration props for the `Collapsible` component.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,

    /// Controlled open state.
    pub open: Option<bool>,

    /// Initial open state for uncontrolled usage.
    pub default_open: bool,

    /// Disables interaction when `true`.
    pub disabled: bool,

    /// Adapter hint: content is not mounted until first opened.
    pub lazy_mount: bool,

    /// Adapter hint: content is removed from the DOM after collapsing.
    pub unmount_on_exit: bool,

    /// CSS height to expose when collapsed instead of fully hiding content.
    pub collapsed_height: Option<String>,

    /// CSS width to expose when collapsed instead of fully hiding content.
    pub collapsed_width: Option<String>,
}

impl Props {
    /// Returns fresh collapsible props with documented defaults.
    ///
    /// # Examples
    ///
    /// ```
    /// use ars_components::layout::collapsible::{Machine, Messages, Props};
    /// use ars_core::{Env, HtmlAttr, Service};
    ///
    /// let service = Service::<Machine>::new(
    ///     Props::new().id("details").default_open(true),
    ///     &Env::default(),
    ///     &Messages::default(),
    /// );
    /// let attrs = service.connect(&|_| {}).root_attrs();
    ///
    /// assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("open"));
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`open`](Self::open), making the collapsible controlled at mount.
    #[must_use]
    pub const fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Clears [`open`](Self::open), making the collapsible uncontrolled.
    #[must_use]
    pub const fn uncontrolled(mut self) -> Self {
        self.open = None;
        self
    }

    /// Sets [`default_open`](Self::default_open).
    #[must_use]
    pub const fn default_open(mut self, default_open: bool) -> Self {
        self.default_open = default_open;
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets [`lazy_mount`](Self::lazy_mount).
    #[must_use]
    pub const fn lazy_mount(mut self, lazy_mount: bool) -> Self {
        self.lazy_mount = lazy_mount;
        self
    }

    /// Sets [`unmount_on_exit`](Self::unmount_on_exit).
    #[must_use]
    pub const fn unmount_on_exit(mut self, unmount_on_exit: bool) -> Self {
        self.unmount_on_exit = unmount_on_exit;
        self
    }

    /// Sets [`collapsed_height`](Self::collapsed_height).
    #[must_use]
    pub fn collapsed_height(mut self, collapsed_height: impl Into<String>) -> Self {
        self.collapsed_height = Some(collapsed_height.into());
        self
    }

    /// Sets [`collapsed_width`](Self::collapsed_width).
    #[must_use]
    pub fn collapsed_width(mut self, collapsed_width: impl Into<String>) -> Self {
        self.collapsed_width = Some(collapsed_width.into());
        self
    }
}

/// Localized messages for [`Collapsible`](self).
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the trigger when collapsed.
    pub expand_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the trigger when expanded.
    pub collapse_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            expand_label: MessageFn::static_str("Show content"),
            collapse_label: MessageFn::static_str("Hide content"),
        }
    }
}

impl ComponentMessages for Messages {}

/// The machine for the `Collapsible` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = NoEffect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let initial_open = props.open.unwrap_or(props.default_open);
        let state = state_from_bool(initial_open);

        let open = if let Some(value) = props.open {
            Bindable::controlled(value)
        } else {
            Bindable::uncontrolled(initial_open)
        };

        (
            state,
            Context {
                open,
                disabled: props.disabled,
                focused: false,
                focus_visible: false,
                ids: ComponentIds::from_id(&props.id),
                collapsed_height: props.collapsed_height.clone(),
                collapsed_width: props.collapsed_width.clone(),
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::Toggle if !ctx.disabled => open_transition(ctx, !ctx.open.get()),

            Event::SetOpen(value) if !ctx.disabled => open_transition(ctx, *value),

            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;
                if ctx.focused && ctx.focus_visible == is_keyboard {
                    return None;
                }

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = is_keyboard;
                }))
            }

            Event::Blur => {
                if !ctx.focused && !ctx.focus_visible {
                    return None;
                }

                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                }))
            }

            Event::SyncProps => Some(sync_props_plan(ctx, props)),

            Event::SyncControlledOpen(value) => {
                let value = *value;
                let next = state_from_bool(value);

                let mut next_open = ctx.open.clone();
                next_open.set(value);
                next_open.sync_controlled(Some(value));

                if ctx.open == next_open && state == &next {
                    return None;
                }

                Some(TransitionPlan::to(next).apply(move |ctx: &mut Context| {
                    ctx.open.set(value);
                    ctx.open.sync_controlled(Some(value));
                }))
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
        assert_eq!(
            old.id, new.id,
            "collapsible::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        let prop_backed_context_changed = old.disabled != new.disabled
            || old.collapsed_height != new.collapsed_height
            || old.collapsed_width != new.collapsed_width
            || (old.open.is_some() && new.open.is_none());

        if prop_backed_context_changed {
            events.push(Event::SyncProps);
        }

        if old.open != new.open
            && let Some(open) = new.open
        {
            events.push(Event::SyncControlledOpen(open));
        }

        events
    }
}

/// Structural parts exposed by the `Collapsible` connect API.
#[derive(ComponentPart)]
#[scope = "collapsible"]
pub enum Part {
    /// The root collapsible container.
    Root,

    /// The button that toggles the content region.
    Trigger,

    /// Optional visual indicator for the current open state.
    Indicator,

    /// The expandable content region.
    Content,
}

/// Connected Collapsible API.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish()
    }
}

impl Api<'_> {
    /// Whether the collapsible is currently open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.state == &State::Open
    }

    /// Attributes for the root container element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Data("ars-state"), state_token(self.is_open()));

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for the trigger button.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        let label = if self.is_open() {
            (self.ctx.messages.collapse_label)(&self.ctx.locale)
        } else {
            (self.ctx.messages.expand_label)(&self.ctx.locale)
        };

        attrs
            .set(HtmlAttr::Id, self.trigger_id())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Data("ars-state"), state_token(self.is_open()))
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                bool_token(self.is_open()),
            )
            .set(HtmlAttr::Aria(AriaAttr::Controls), self.content_id())
            .set(HtmlAttr::Aria(AriaAttr::Label), label);

        if self.ctx.disabled {
            attrs
                .set_bool(HtmlAttr::Disabled, true)
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.focused {
            attrs.set_bool(HtmlAttr::Data("ars-focus"), true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Attributes for an optional visual indicator.
    #[must_use]
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), state_token(self.is_open()))
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for the content region.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        let has_collapsed_size =
            self.ctx.collapsed_height.is_some() || self.ctx.collapsed_width.is_some();

        attrs
            .set(HtmlAttr::Id, self.content_id())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "region")
            .set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.trigger_id())
            .set(HtmlAttr::Data("ars-state"), state_token(self.is_open()));

        if !self.is_open() && !has_collapsed_size {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        if let Some(height) = &self.ctx.collapsed_height {
            attrs.set_style(
                CssProperty::Custom("ars-collapsible-collapsed-height"),
                height,
            );
        }

        if let Some(width) = &self.ctx.collapsed_width {
            attrs.set_style(
                CssProperty::Custom("ars-collapsible-collapsed-width"),
                width,
            );
        }

        if has_collapsed_size {
            attrs.set_bool(HtmlAttr::Data("ars-collapsed-size"), true);
        }

        attrs
    }

    /// Adapter handler: the trigger was clicked.
    pub fn on_trigger_click(&self) {
        if !self.ctx.disabled {
            (self.send)(Event::Toggle);
        }
    }

    /// Adapter handler: a key was pressed on the trigger.
    ///
    /// Returns `true` when the key was handled so adapters can prevent the
    /// native button activation click and avoid dispatching a duplicate toggle.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData) -> bool {
        if !self.ctx.disabled
            && !data.repeat
            && (data.key == KeyboardKey::Enter || data.key == KeyboardKey::Space)
        {
            (self.send)(Event::Toggle);
            return true;
        }

        false
    }

    /// Adapter handler: the trigger received focus.
    pub fn on_trigger_focus(&self, is_keyboard: bool) {
        if !self.ctx.disabled {
            (self.send)(Event::Focus { is_keyboard });
        }
    }

    /// Adapter handler: the trigger lost focus.
    pub fn on_trigger_blur(&self) {
        (self.send)(Event::Blur);
    }

    fn trigger_id(&self) -> String {
        self.ctx.ids.part("trigger")
    }

    fn content_id(&self) -> String {
        self.ctx.ids.part("content")
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Indicator => self.indicator_attrs(),
            Part::Content => self.content_attrs(),
        }
    }
}

fn open_transition(ctx: &Context, value: bool) -> Option<TransitionPlan<Machine>> {
    if ctx.open.is_controlled() {
        return Some(TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.open.set(value);
        }));
    }

    if ctx.open.get() == &value {
        return None;
    }

    Some(
        TransitionPlan::to(state_from_bool(value)).apply(move |ctx: &mut Context| {
            ctx.open.set(value);
        }),
    )
}

fn sync_props_plan(ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    let disabled = props.disabled;
    let collapsed_height = props.collapsed_height.clone();
    let collapsed_width = props.collapsed_width.clone();
    let was_controlled = ctx.open.is_controlled();
    let leaving_controlled = was_controlled && props.open.is_none();
    let visible_open = *ctx.open.get();

    let target_state = if leaving_controlled {
        Some(state_from_bool(visible_open))
    } else {
        None
    };

    let mut plan = if let Some(state) = target_state {
        TransitionPlan::to(state)
    } else {
        TransitionPlan::new()
    };

    plan = plan.apply(move |ctx: &mut Context| {
        ctx.disabled = disabled;
        ctx.collapsed_height = collapsed_height;
        ctx.collapsed_width = collapsed_width;

        if leaving_controlled {
            ctx.open.set(visible_open);
            ctx.open.sync_controlled(None);
        }

        if ctx.disabled {
            ctx.focused = false;
            ctx.focus_visible = false;
        }
    });

    plan
}

const fn state_from_bool(open: bool) -> State {
    if open { State::Open } else { State::Closed }
}

const fn bool_token(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

const fn state_token(open: bool) -> &'static str {
    if open { "open" } else { "closed" }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String, vec, vec::Vec};
    use core::cell::RefCell;

    use ars_core::{
        AriaAttr, AttrMap, ConnectApi, CssProperty, Env, HtmlAttr, KeyboardKey, Locale, MessageFn,
        Service,
    };
    use ars_interactions::KeyboardEventData;
    use insta::assert_snapshot;

    use super::*;

    fn props() -> Props {
        Props::new().id("collapsible")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn keyboard(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: key.as_w3c_str().to_owned(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        let mut entries = attrs.iter().collect::<Vec<_>>();

        entries.sort_by_key(|(attr, _)| attr.to_string());

        entries
            .into_iter()
            .map(|(attr, value)| format!("{}={}", attr, value.as_str().unwrap_or("<reactive>")))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn closed_to_open_on_toggle_event() {
        let mut service = service(props());

        assert_eq!(service.state(), &State::Closed);

        let result = service.send(Event::Toggle);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Open);
        assert_eq!(service.context().open.get(), &true);
    }

    #[test]
    fn trigger_attrs_include_aria_expanded_and_controls() {
        let service = service(props());

        let attrs = service.connect(&|_| {}).trigger_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("button"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("false")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Controls)),
            Some("collapsible-content")
        );
    }

    #[test]
    fn content_attrs_include_region_and_labelledby() {
        let service = service(props());

        let attrs = service.connect(&|_| {}).content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("region"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("collapsible-trigger")
        );
    }

    #[test]
    fn collapsed_height_width_emit_animation_intent_without_hidden() {
        let service = service(props().collapsed_height("80px").collapsed_width("120px"));

        let attrs = service.connect(&|_| {}).content_attrs();

        assert!(!attrs.contains(&HtmlAttr::Hidden));
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-collapsed-size")),
            Some("true")
        );
        assert!(attrs.styles().contains(&(
            CssProperty::Custom("ars-collapsible-collapsed-height"),
            String::from("80px")
        )));
        assert!(attrs.styles().contains(&(
            CssProperty::Custom("ars-collapsible-collapsed-width"),
            String::from("120px")
        )));
    }

    #[test]
    fn data_ars_state_reflects_open_closed() {
        let closed = service(props());

        let open = service(props().default_open(true));

        assert_eq!(
            closed
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("closed")
        );
        assert_eq!(
            open.connect(&|_| {})
                .content_attrs()
                .get(&HtmlAttr::Data("ars-state")),
            Some("open")
        );
    }

    #[test]
    fn disabled_state_blocks_toggle_and_marks_attrs() {
        let mut service = service(props().disabled(true));

        let result = service.send(Event::Toggle);
        let set_open = service.send(Event::SetOpen(true));

        assert!(!result.state_changed);
        assert!(!set_open.state_changed);
        assert!(!set_open.context_changed);
        assert_eq!(service.state(), &State::Closed);
        assert_eq!(service.context().open.get(), &false);

        let api = service.connect(&|_| {});

        assert_eq!(
            api.root_attrs().get(&HtmlAttr::Data("ars-disabled")),
            Some("true")
        );
        assert_eq!(api.trigger_attrs().get(&HtmlAttr::Disabled), Some("true"));
        assert_eq!(
            api.trigger_attrs().get(&HtmlAttr::Data("ars-disabled")),
            Some("true")
        );
    }

    #[test]
    fn default_open_initializes_open() {
        let service = service(props().default_open(true));

        assert_eq!(service.state(), &State::Open);
        assert!(service.connect(&|_| {}).is_open());
    }

    #[test]
    fn set_open_event_opens_and_closes_uncontrolled() {
        let mut service = service(props());

        let open = service.send(Event::SetOpen(true));

        assert!(open.state_changed);
        assert_eq!(service.state(), &State::Open);
        assert_eq!(service.context().open.get(), &true);

        let repeated_open = service.send(Event::SetOpen(true));

        assert!(!repeated_open.state_changed);
        assert!(!repeated_open.context_changed);

        let closed = service.send(Event::SetOpen(false));

        assert!(closed.state_changed);
        assert_eq!(service.state(), &State::Closed);
        assert_eq!(service.context().open.get(), &false);
    }

    #[test]
    fn controlled_and_uncontrolled_open_sync() {
        let mut service = service(props().open(false));

        drop(service.send(Event::Toggle));

        assert_eq!(service.state(), &State::Closed);
        assert_eq!(service.context().open.get(), &false);

        drop(service.set_props(props().open(true)));

        assert_eq!(service.state(), &State::Open);
        assert_eq!(service.context().open.get(), &true);

        drop(service.set_props(props().default_open(false)));
        drop(service.send(Event::Toggle));

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open.is_controlled());
    }

    #[test]
    fn controlled_to_uncontrolled_resumes_from_latest_controlled_value() {
        let mut service = service(props());

        drop(service.set_props(props().open(true)));

        assert_eq!(service.state(), &State::Open);
        assert_eq!(service.context().open.get(), &true);

        drop(service.set_props(props().uncontrolled()));

        assert_eq!(service.state(), &State::Open);
        assert_eq!(service.context().open.get(), &true);
        assert!(!service.context().open.is_controlled());
    }

    #[test]
    fn controlled_to_uncontrolled_ignores_uncommitted_internal_toggle() {
        let mut service = service(props().open(false));

        drop(service.send(Event::Toggle));

        assert_eq!(service.state(), &State::Closed);
        assert_eq!(service.context().open.get(), &false);

        drop(service.set_props(props().uncontrolled()));

        assert_eq!(service.state(), &State::Closed);
        assert_eq!(service.context().open.get(), &false);
        assert!(!service.context().open.is_controlled());
    }

    #[test]
    fn controlled_sync_transition_only_noops_for_matching_controlled_state() {
        let controlled = service(props().open(false));

        assert!(
            <Machine as ars_core::Machine>::transition(
                controlled.state(),
                &Event::SyncControlledOpen(false),
                controlled.context(),
                controlled.props(),
            )
            .is_none()
        );
        assert!(
            <Machine as ars_core::Machine>::transition(
                controlled.state(),
                &Event::SyncControlledOpen(true),
                controlled.context(),
                controlled.props(),
            )
            .is_some()
        );

        let uncontrolled = service(props());

        assert!(
            <Machine as ars_core::Machine>::transition(
                uncontrolled.state(),
                &Event::SyncControlledOpen(false),
                uncontrolled.context(),
                uncontrolled.props(),
            )
            .is_some()
        );
    }

    #[test]
    fn prop_sync_updates_each_prop_backed_context_branch() {
        let mut service = service(props().open(false).default_open(true));

        let controlled_noop = service.set_props(props().open(false).default_open(true));

        assert!(!controlled_noop.state_changed);
        assert!(!controlled_noop.context_changed);

        let disabled = service.set_props(props().open(false).disabled(true));

        assert!(disabled.context_changed);
        assert!(service.context().disabled);
        assert_eq!(service.state(), &State::Closed);
        assert!(service.context().open.is_controlled());

        let height = service.set_props(props().open(false).collapsed_height("44px"));

        assert!(height.context_changed);
        assert_eq!(service.context().collapsed_height.as_deref(), Some("44px"));
        assert_eq!(service.context().collapsed_width, None);
        assert!(!service.context().disabled);
        assert!(service.context().open.is_controlled());

        let width = service.set_props(
            props()
                .open(false)
                .collapsed_height("44px")
                .collapsed_width("12rem"),
        );

        assert!(width.context_changed);
        assert_eq!(service.context().collapsed_height.as_deref(), Some("44px"));
        assert_eq!(service.context().collapsed_width.as_deref(), Some("12rem"));
        assert!(service.context().open.is_controlled());

        drop(service.send(Event::Toggle));

        assert_eq!(service.state(), &State::Closed);

        let uncontrolled = service.set_props(props());

        assert!(uncontrolled.context_changed);
        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open.is_controlled());
    }

    #[test]
    fn on_props_changed_emits_precise_sync_events() {
        let base = props().open(false);

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &props().open(false).disabled(true)
            ),
            vec![Event::SyncProps]
        );
        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &props().open(false).collapsed_height("10px")
            ),
            vec![Event::SyncProps]
        );
        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &props().open(false).collapsed_width("20px")
            ),
            vec![Event::SyncProps]
        );
        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&base, &props()),
            vec![Event::SyncProps]
        );
        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&base, &props().open(true)),
            vec![Event::SyncControlledOpen(true)]
        );
        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &props().open(true).disabled(true)
            ),
            vec![Event::SyncProps, Event::SyncControlledOpen(true)]
        );
        assert!(<Machine as ars_core::Machine>::on_props_changed(&base, &base).is_empty());
    }

    #[test]
    fn keyboard_enter_and_space_toggle() {
        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let service = service(props());

        let api = service.connect(&send);

        assert!(api.on_trigger_keydown(&keyboard(KeyboardKey::Enter)));
        assert!(api.on_trigger_keydown(&keyboard(KeyboardKey::Space)));
        assert!(!api.on_trigger_keydown(&keyboard(KeyboardKey::Escape)));

        assert_eq!(events.into_inner(), vec![Event::Toggle, Event::Toggle]);
    }

    #[test]
    fn trigger_keydown_reports_handled_status_for_adapter_default_prevention() {
        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let enabled = service(props());

        let api = enabled.connect(&send);

        assert!(api.on_trigger_keydown(&keyboard(KeyboardKey::Enter)));
        assert!(api.on_trigger_keydown(&keyboard(KeyboardKey::Space)));
        assert!(!api.on_trigger_keydown(&keyboard(KeyboardKey::Escape)));

        assert_eq!(events.borrow().as_slice(), [Event::Toggle, Event::Toggle]);

        events.borrow_mut().clear();

        let mut repeated = keyboard(KeyboardKey::Space);
        repeated.repeat = true;

        assert!(!api.on_trigger_keydown(&repeated));
        assert!(events.borrow().is_empty());

        let disabled = service(props().disabled(true));

        let disabled_api = disabled.connect(&send);

        assert!(!disabled_api.on_trigger_keydown(&keyboard(KeyboardKey::Enter)));
        assert!(events.borrow().is_empty());
    }

    #[test]
    #[should_panic(expected = "collapsible::Props.id must remain stable after init")]
    fn collapsible_set_props_panics_when_id_changes() {
        let mut service = service(props());

        drop(service.set_props(props().id("next-collapsible")));
    }

    #[test]
    fn trigger_handlers_emit_precise_events_and_respect_disabled() {
        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let enabled = service(props());

        let api = enabled.connect(&send);

        api.on_trigger_click();
        api.on_trigger_keydown(&keyboard(KeyboardKey::Enter));
        api.on_trigger_keydown(&keyboard(KeyboardKey::Space));
        api.on_trigger_focus(false);
        api.on_trigger_blur();
        api.on_trigger_keydown(&keyboard(KeyboardKey::Escape));

        assert_eq!(
            events.borrow().as_slice(),
            [
                Event::Toggle,
                Event::Toggle,
                Event::Toggle,
                Event::Focus { is_keyboard: false },
                Event::Blur,
            ]
        );

        events.borrow_mut().clear();

        let disabled = service(props().disabled(true));

        let disabled_api = disabled.connect(&send);

        disabled_api.on_trigger_click();
        disabled_api.on_trigger_keydown(&keyboard(KeyboardKey::Enter));
        disabled_api.on_trigger_keydown(&keyboard(KeyboardKey::Space));
        disabled_api.on_trigger_focus(true);
        disabled_api.on_trigger_blur();

        assert_eq!(events.into_inner(), vec![Event::Blur]);
    }

    #[test]
    fn focus_and_blur_update_focus_attrs() {
        let mut service = service(props());

        drop(service.send(Event::Focus { is_keyboard: true }));

        let repeated = service.send(Event::Focus { is_keyboard: true });

        assert!(!repeated.context_changed);

        let pointer_focus = service.send(Event::Focus { is_keyboard: false });

        assert!(pointer_focus.context_changed);
        assert!(!service.context().focus_visible);

        let focused_attrs = service.connect(&|_| {}).trigger_attrs();

        assert_eq!(
            focused_attrs.get(&HtmlAttr::Data("ars-focus")),
            Some("true")
        );
        assert!(!focused_attrs.contains(&HtmlAttr::Data("ars-focus-visible")));

        drop(service.send(Event::Blur));

        assert!(
            !service
                .connect(&|_| {})
                .trigger_attrs()
                .contains(&HtmlAttr::Data("ars-focus"))
        );

        let idle_blur = service.send(Event::Blur);

        assert!(!idle_blur.context_changed);
    }

    #[test]
    fn part_attrs_matches_direct_methods() {
        let service = service(props().default_open(true));

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Trigger), api.trigger_attrs());
        assert_eq!(api.part_attrs(Part::Indicator), api.indicator_attrs());
        assert_eq!(api.part_attrs(Part::Content), api.content_attrs());
    }

    #[test]
    fn snapshot_all_output_affecting_branches() {
        let closed = service(props());
        let open = service(props().default_open(true));
        let disabled = service(props().disabled(true));

        let focused = {
            let mut service = service(props());
            drop(service.send(Event::Focus { is_keyboard: true }));
            service
        };

        let collapsed_size = service(props().collapsed_height("80px"));

        let custom_messages = Service::<Machine>::new(
            props(),
            &Env::default(),
            &Messages {
                expand_label: MessageFn::new(|_locale: &Locale| "Show details".to_string()),
                collapse_label: MessageFn::new(|_locale: &Locale| "Hide details".to_string()),
            },
        );

        assert_snapshot!(
            "collapsible_root_closed",
            snapshot_attrs(&closed.connect(&|_| {}).root_attrs())
        );
        assert_snapshot!(
            "collapsible_trigger_closed",
            snapshot_attrs(&closed.connect(&|_| {}).trigger_attrs())
        );
        assert_snapshot!(
            "collapsible_indicator_open",
            snapshot_attrs(&open.connect(&|_| {}).indicator_attrs())
        );
        assert_snapshot!(
            "collapsible_content_closed",
            snapshot_attrs(&closed.connect(&|_| {}).content_attrs())
        );
        assert_snapshot!(
            "collapsible_content_open",
            snapshot_attrs(&open.connect(&|_| {}).content_attrs())
        );
        assert_snapshot!(
            "collapsible_trigger_disabled",
            snapshot_attrs(&disabled.connect(&|_| {}).trigger_attrs())
        );
        assert_snapshot!(
            "collapsible_trigger_focus_visible",
            snapshot_attrs(&focused.connect(&|_| {}).trigger_attrs())
        );
        assert_snapshot!(
            "collapsible_content_collapsed_size",
            snapshot_attrs(&collapsed_size.connect(&|_| {}).content_attrs())
        );
        assert_snapshot!(
            "collapsible_trigger_custom_message",
            snapshot_attrs(&custom_messages.connect(&|_| {}).trigger_attrs())
        );
    }
}
