//! Alert dialog modal overlay machine.
//!
//! `AlertDialog` is the safety-oriented variant of [`dialog`]: it uses the
//! same binary open/closed lifecycle and adapter-resolvable focus, scroll, and
//! inertness intents, but defaults Escape and backdrop dismissal off and adds
//! cancel/action trigger semantics.
//!
//! The agnostic core owns semantic state, IDs, ARIA/data attributes, localized
//! action labels, and named effect intents only. It never traverses the DOM,
//! resolves elements by ID, attaches document listeners, or stores framework
//! element handles. Framework adapters resolve focus trap behavior, initial
//! focus on the cancel action, outside listeners, and live element references.

use alloc::{
    string::{String, ToString as _},
    vec,
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_a11y::FocusTarget;
use ars_core::{
    AriaAttr, AttrMap, Callback, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HasId, HtmlAttr, Locale, MessageFn, PendingEffect, TransitionPlan,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

use super::dialog;
use crate::utility::dismissable::DismissAttempt;

/// States for the [`AlertDialog`](self) component.
pub type State = dialog::State;

/// Events accepted by the [`AlertDialog`](self) state machine.
pub type Event = dialog::Event;

/// Adapter intent names emitted by the [`AlertDialog`](self) machine.
pub type Effect = dialog::Effect;

/// Runtime context for [`AlertDialog`](self).
///
/// The fields mirror [`dialog::Context`] so `AlertDialog` transitions can share
/// the same lifecycle semantics while retaining AlertDialog-specific
/// localized action messages.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the alert dialog is logically open.
    pub open: bool,

    /// Whether the alert dialog is modal.
    pub modal: bool,

    /// Whether backdrop clicks may close the alert dialog.
    pub close_on_backdrop: bool,

    /// Whether Escape may close the alert dialog.
    pub close_on_escape: bool,

    /// Whether body scroll should be locked while open.
    pub prevent_scroll: bool,

    /// Whether focus should be restored to the trigger when closed.
    pub restore_focus: bool,

    /// Initial focus target resolved by the adapter.
    pub initial_focus: Option<FocusTarget>,

    /// Final focus target resolved by the adapter.
    pub final_focus: Option<FocusTarget>,

    /// Semantic role applied to content.
    pub role: dialog::Role,

    /// Hydration-stable semantic IDs for ARIA wiring.
    pub ids: ComponentIds,

    /// Whether a title has been registered.
    pub has_title: bool,

    /// Whether a description has been registered.
    pub has_description: bool,

    /// Active locale used to resolve [`Messages`].
    pub locale: Locale,

    /// Localized `AlertDialog` action message bundle.
    pub messages: Messages,
}

/// Localizable action strings for [`AlertDialog`](self).
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the confirming action. Defaults to `"Confirm"`.
    pub confirm_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Accessible label for the safe cancel action. Defaults to `"Cancel"`.
    pub cancel_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            confirm_label: MessageFn::static_str("Confirm"),
            cancel_label: MessageFn::static_str("Cancel"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Immutable configuration for an [`AlertDialog`](self) instance.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Controlled open state.
    pub open: Option<bool>,

    /// Initial open state in uncontrolled mode.
    pub default_open: bool,

    /// Whether the alert dialog is modal. Defaults to `true`.
    pub modal: bool,

    /// Whether backdrop clicks may close the alert dialog. Defaults to `false`.
    pub close_on_backdrop: bool,

    /// Whether Escape may close the alert dialog. Defaults to `false`.
    pub close_on_escape: bool,

    /// Whether body scroll should be locked while open. Defaults to `true`.
    pub prevent_scroll: bool,

    /// Whether focus should be restored to the trigger on close.
    pub restore_focus: bool,

    /// Initial focus target resolved by the adapter.
    pub initial_focus: Option<FocusTarget>,

    /// Final focus target resolved by the adapter.
    pub final_focus: Option<FocusTarget>,

    /// Semantic role applied to content. Defaults to [`dialog::Role::AlertDialog`].
    pub role: dialog::Role,

    /// Heading level for the title, clamped to `1..=6`.
    pub title_level: u8,

    /// Whether adapters should defer mounting content until first open.
    pub lazy_mount: bool,

    /// Whether adapters should unmount content after exit.
    pub unmount_on_exit: bool,

    /// Whether the primary action is destructive.
    pub is_destructive: bool,

    /// Callback invoked after open state changes.
    pub on_open_change: Option<Callback<dyn Fn(bool) + Send + Sync>>,

    /// Callback invoked before Escape dismissal is dispatched.
    pub on_escape_key_down: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,

    /// Callback invoked before outside interaction dismissal is dispatched.
    pub on_interact_outside: Option<Callback<dyn Fn(DismissAttempt<()>) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            open: None,
            default_open: false,
            modal: true,
            close_on_backdrop: false,
            close_on_escape: false,
            prevent_scroll: true,
            restore_focus: true,
            initial_focus: None,
            final_focus: None,
            role: dialog::Role::AlertDialog,
            title_level: 2,
            lazy_mount: false,
            unmount_on_exit: false,
            is_destructive: false,
            on_open_change: None,
            on_escape_key_down: None,
            on_interact_outside: None,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with `AlertDialog` safety defaults.
    ///
    /// # Examples
    ///
    /// ```
    /// use ars_components::overlay::{
    ///     alert_dialog::Props,
    ///     dialog::Role,
    /// };
    ///
    /// let props = Props::new()
    ///     .id("delete-user")
    ///     .role(Role::AlertDialog)
    ///     .is_destructive(true);
    ///
    /// assert_eq!(props.id, "delete-user");
    /// assert_eq!(props.role, Role::AlertDialog);
    /// assert!(props.is_destructive);
    /// assert!(!props.close_on_escape);
    /// assert!(!props.close_on_backdrop);
    /// ```
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

    /// Sets [`open`](Self::open).
    #[must_use]
    pub const fn open(mut self, value: Option<bool>) -> Self {
        self.open = value;
        self
    }

    /// Sets [`default_open`](Self::default_open).
    #[must_use]
    pub const fn default_open(mut self, value: bool) -> Self {
        self.default_open = value;
        self
    }

    /// Sets [`modal`](Self::modal).
    #[must_use]
    pub const fn modal(mut self, value: bool) -> Self {
        self.modal = value;
        self
    }

    /// Sets [`close_on_backdrop`](Self::close_on_backdrop).
    #[must_use]
    pub const fn close_on_backdrop(mut self, value: bool) -> Self {
        self.close_on_backdrop = value;
        self
    }

    /// Sets [`close_on_escape`](Self::close_on_escape).
    #[must_use]
    pub const fn close_on_escape(mut self, value: bool) -> Self {
        self.close_on_escape = value;
        self
    }

    /// Sets [`prevent_scroll`](Self::prevent_scroll).
    #[must_use]
    pub const fn prevent_scroll(mut self, value: bool) -> Self {
        self.prevent_scroll = value;
        self
    }

    /// Sets [`restore_focus`](Self::restore_focus).
    #[must_use]
    pub const fn restore_focus(mut self, value: bool) -> Self {
        self.restore_focus = value;
        self
    }

    /// Sets [`initial_focus`](Self::initial_focus).
    #[must_use]
    pub const fn initial_focus(mut self, value: Option<FocusTarget>) -> Self {
        self.initial_focus = value;
        self
    }

    /// Sets [`final_focus`](Self::final_focus).
    #[must_use]
    pub const fn final_focus(mut self, value: Option<FocusTarget>) -> Self {
        self.final_focus = value;
        self
    }

    /// Sets [`role`](Self::role).
    #[must_use]
    pub const fn role(mut self, value: dialog::Role) -> Self {
        self.role = value;
        self
    }

    /// Sets [`title_level`](Self::title_level).
    #[must_use]
    pub const fn title_level(mut self, value: u8) -> Self {
        self.title_level = value;
        self
    }

    /// Sets [`lazy_mount`](Self::lazy_mount).
    #[must_use]
    pub const fn lazy_mount(mut self, value: bool) -> Self {
        self.lazy_mount = value;
        self
    }

    /// Sets [`unmount_on_exit`](Self::unmount_on_exit).
    #[must_use]
    pub const fn unmount_on_exit(mut self, value: bool) -> Self {
        self.unmount_on_exit = value;
        self
    }

    /// Sets [`is_destructive`](Self::is_destructive).
    #[must_use]
    pub const fn is_destructive(mut self, value: bool) -> Self {
        self.is_destructive = value;
        self
    }

    /// Registers [`on_open_change`](Self::on_open_change).
    #[must_use]
    pub fn on_open_change<F>(mut self, f: F) -> Self
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        self.on_open_change = Some(Callback::new(f));
        self
    }

    /// Registers [`on_escape_key_down`](Self::on_escape_key_down).
    #[must_use]
    pub fn on_escape_key_down<F>(mut self, f: F) -> Self
    where
        F: Fn(DismissAttempt<()>) + Send + Sync + 'static,
    {
        self.on_escape_key_down = Some(Callback::new(f));
        self
    }

    /// Registers [`on_interact_outside`](Self::on_interact_outside).
    #[must_use]
    pub fn on_interact_outside<F>(mut self, f: F) -> Self
    where
        F: Fn(DismissAttempt<()>) + Send + Sync + 'static,
    {
        self.on_interact_outside = Some(Callback::new(f));
        self
    }
}

/// Anatomy parts exposed by the [`AlertDialog`](self) connect API.
#[derive(ComponentPart)]
#[scope = "alert-dialog"]
pub enum Part {
    /// The root container element.
    Root,

    /// The trigger button that opens the alert dialog.
    Trigger,

    /// The modal backdrop behind the content.
    Backdrop,

    /// The wrapper element that positions the content.
    Positioner,

    /// The alert dialog content element.
    Content,

    /// The required title element.
    Title,

    /// The required description element.
    Description,

    /// The safe cancel action trigger.
    CancelTrigger,

    /// The confirming action trigger.
    ActionTrigger,

    /// Optional explicit close trigger.
    CloseTrigger,
}

/// State machine for the [`AlertDialog`](self) component.
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
        let open = props.open.unwrap_or(props.default_open);

        let state = if open { State::Open } else { State::Closed };

        (
            state,
            Context {
                open,
                modal: props.modal,
                close_on_backdrop: props.close_on_backdrop,
                close_on_escape: props.close_on_escape,
                prevent_scroll: props.prevent_scroll,
                restore_focus: props.restore_focus,
                initial_focus: props.initial_focus,
                final_focus: props.final_focus,
                role: props.role,
                ids: ComponentIds::from_id(&props.id),
                has_title: false,
                has_description: false,
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
        match (state, event) {
            (State::Closed, Event::Open | Event::Toggle) => {
                let mut plan = TransitionPlan::to(State::Open)
                    .apply(|ctx: &mut Context| {
                        ctx.open = true;
                    })
                    .with_effect(PendingEffect::named(Effect::OpenChange))
                    .with_effect(PendingEffect::named(Effect::FocusInitial))
                    .with_effect(PendingEffect::named(Effect::FocusFirstTabbable));

                if ctx.prevent_scroll {
                    plan = plan.with_effect(PendingEffect::named(Effect::ScrollLockAcquire));
                }

                if ctx.modal {
                    plan = plan.with_effect(PendingEffect::named(Effect::SetBackgroundInert));
                }

                Some(plan)
            }

            (State::Open, Event::Close | Event::Toggle) => {
                let mut plan = TransitionPlan::to(State::Closed)
                    .apply(|ctx: &mut Context| {
                        ctx.open = false;
                    })
                    .with_effect(PendingEffect::named(Effect::OpenChange));

                if ctx.prevent_scroll {
                    plan = plan.with_effect(PendingEffect::named(Effect::ScrollLockRelease));
                }

                if ctx.modal {
                    plan = plan.with_effect(PendingEffect::named(Effect::RemoveBackgroundInert));
                }

                if ctx.restore_focus {
                    plan = plan.with_effect(PendingEffect::named(Effect::RestoreFocus));
                }

                Some(plan)
            }

            (State::Open, Event::CloseOnBackdropClick) if ctx.close_on_backdrop => Some(
                TransitionPlan::to(State::Closed)
                    .apply(|ctx: &mut Context| {
                        ctx.open = false;
                    })
                    .with_effect(PendingEffect::named(Effect::OpenChange)),
            ),

            (State::Open, Event::CloseOnEscape) if ctx.close_on_escape => Some(
                TransitionPlan::to(State::Closed)
                    .apply(|ctx: &mut Context| {
                        ctx.open = false;
                    })
                    .with_effect(PendingEffect::named(Effect::OpenChange)),
            ),

            (_, Event::RegisterTitle) if !ctx.has_title => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.has_title = true;
                }))
            }

            (_, Event::RegisterDescription) if !ctx.has_description => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.has_description = true;
                }))
            }

            (_, Event::SyncProps) => {
                let modal = props.modal;
                let close_on_backdrop = props.close_on_backdrop;
                let close_on_escape = props.close_on_escape;
                let prevent_scroll = props.prevent_scroll;
                let restore_focus = props.restore_focus;
                let initial_focus = props.initial_focus;
                let final_focus = props.final_focus;
                let role = props.role;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.modal = modal;
                    ctx.close_on_backdrop = close_on_backdrop;
                    ctx.close_on_escape = close_on_escape;
                    ctx.prevent_scroll = prevent_scroll;
                    ctx.restore_focus = restore_focus;
                    ctx.initial_focus = initial_focus;
                    ctx.final_focus = final_focus;
                    ctx.role = role;
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
        let mut events = Vec::new();

        if let (was, Some(now)) = (old.open, new.open)
            && was != Some(now)
        {
            events.push(if now { Event::Open } else { Event::Close });
        }

        if context_relevant_props_changed(old, new) {
            events.push(Event::SyncProps);
        }

        events
    }

    fn initial_effects(
        state: &Self::State,
        context: &Self::Context,
        _props: &Self::Props,
    ) -> Vec<PendingEffect<Self>> {
        if !matches!(state, State::Open) {
            return Vec::new();
        }

        let mut effects = vec![
            PendingEffect::named(Effect::OpenChange),
            PendingEffect::named(Effect::FocusInitial),
            PendingEffect::named(Effect::FocusFirstTabbable),
        ];

        if context.prevent_scroll {
            effects.push(PendingEffect::named(Effect::ScrollLockAcquire));
        }

        if context.modal {
            effects.push(PendingEffect::named(Effect::SetBackgroundInert));
        }

        effects
    }
}

fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.modal != new.modal
        || old.close_on_backdrop != new.close_on_backdrop
        || old.close_on_escape != new.close_on_escape
        || old.prevent_scroll != new.prevent_scroll
        || old.restore_focus != new.restore_focus
        || old.initial_focus != new.initial_focus
        || old.final_focus != new.final_focus
        || old.role != new.role
}

/// Connected API surface for the [`AlertDialog`](self) component.
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
    /// Returns `true` when the alert dialog is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.state, State::Open)
    }

    /// Returns `true` when configured as modal.
    #[must_use]
    pub const fn is_modal(&self) -> bool {
        self.ctx.modal
    }

    /// Returns the content role.
    #[must_use]
    pub const fn role(&self) -> dialog::Role {
        self.ctx.role
    }

    /// Returns whether adapters should lazy-mount content.
    #[must_use]
    pub const fn lazy_mount(&self) -> bool {
        self.props.lazy_mount
    }

    /// Returns whether adapters should unmount content on exit.
    #[must_use]
    pub const fn unmount_on_exit(&self) -> bool {
        self.props.unmount_on_exit
    }

    const fn state_token(&self) -> &'static str {
        if self.is_open() { "open" } else { "closed" }
    }

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token());

        attrs
    }

    /// Attributes for the trigger button.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("trigger"))
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "dialog")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.is_open() { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            );

        attrs
    }

    /// Adapter handler: the trigger was activated.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Attributes for the backdrop element.
    #[must_use]
    pub fn backdrop_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Backdrop.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::Inert, "");

        attrs
    }

    /// Adapter handler: a backdrop click attempted to close the alert dialog.
    pub fn on_backdrop_click(&self) {
        (self.send)(Event::CloseOnBackdropClick);
    }

    /// Attributes for the positioner element.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Positioner.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Attributes for the alert dialog content element.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, self.ctx.role.as_aria_role())
            .set(HtmlAttr::Data("ars-state"), self.state_token())
            .set(HtmlAttr::TabIndex, "-1");

        if self.ctx.modal {
            attrs.set(HtmlAttr::Aria(AriaAttr::Modal), "true");
        }

        if self.ctx.has_title {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("title"),
            );
        }

        if self.ctx.has_description {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                self.ctx.ids.part("description"),
            );
        }

        attrs
    }

    /// Adapter handler: keydown on the content element.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::CloseOnEscape);
        }
    }

    /// Attributes for the title element.
    #[must_use]
    pub fn title_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Title.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("title"))
            .set(
                HtmlAttr::Data("ars-heading-level"),
                self.props.title_level.clamp(1, 6).to_string(),
            );

        attrs
    }

    /// Attributes for the description element.
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

    /// Attributes for the safe cancel trigger.
    #[must_use]
    pub fn cancel_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CancelTrigger.data_attrs();
        let label = (self.ctx.messages.cancel_label)(&self.ctx.locale);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label);

        attrs
    }

    /// Adapter handler: the cancel trigger was activated.
    pub fn on_cancel_trigger_click(&self) {
        (self.send)(Event::Close);
    }

    /// Attributes for the confirming action trigger.
    #[must_use]
    pub fn action_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ActionTrigger.data_attrs();
        let label = (self.ctx.messages.confirm_label)(&self.ctx.locale);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label);

        if self.props.is_destructive {
            attrs.set_bool(HtmlAttr::Data("ars-destructive"), true);
        }

        attrs
    }

    /// Adapter handler: the action trigger was activated.
    pub fn on_action_trigger_click(&self) {
        (self.send)(Event::Close);
    }

    /// Attributes for the optional explicit close trigger.
    #[must_use]
    pub fn close_trigger_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button");

        attrs
    }

    /// Adapter handler: the close trigger was activated.
    pub fn on_close_trigger_click(&self) {
        (self.send)(Event::Close);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::Backdrop => self.backdrop_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::Title => self.title_attrs(),
            Part::Description => self.description_attrs(),
            Part::CancelTrigger => self.cancel_trigger_attrs(),
            Part::ActionTrigger => self.action_trigger_attrs(),
            Part::CloseTrigger => self.close_trigger_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        rc::Rc,
        string::{String, ToString},
        vec::Vec,
    };
    use core::cell::RefCell;

    use ars_a11y::FocusTarget;
    use ars_core::{
        AriaAttr, AttrMap, ConnectApi as _, Env, HtmlAttr, Locale, Machine as MachineTrait,
        MessageFn, SendResult, Service,
    };
    use ars_interactions::{KeyboardEventData, KeyboardKey};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "alert".to_string(),
            ..Props::default()
        }
    }

    fn fresh_service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn open_service(props: Props) -> Service<Machine> {
        let mut props = props;

        props.default_open = true;

        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn effect_names(result: &SendResult<Machine>) -> Vec<Effect> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn keyboard_data(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn default_props_are_alert_dialog_safe_defaults() {
        let props = Props::default();

        assert_eq!(props.role, dialog::Role::AlertDialog);
        assert!(props.modal);
        assert!(!props.close_on_backdrop);
        assert!(!props.close_on_escape);
        assert!(props.prevent_scroll);
        assert!(props.restore_focus);
        assert_eq!(props.initial_focus, None);
        assert!(!props.is_destructive);
    }

    #[test]
    fn init_default_open_false_starts_closed() {
        let service = fresh_service(test_props());

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(service.context().role, dialog::Role::AlertDialog);
        assert_eq!(service.context().ids.part("trigger"), "alert-trigger");
    }

    #[test]
    fn init_default_open_true_starts_open() {
        let service = fresh_service(Props {
            default_open: true,
            ..test_props()
        });

        assert_eq!(service.state(), &State::Open);
        assert!(service.context().open);
    }

    #[test]
    fn init_controlled_open_overrides_default_open() {
        let service = fresh_service(Props {
            open: Some(false),
            default_open: true,
            ..test_props()
        });

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
    }

    #[test]
    fn escape_does_not_close_by_default() {
        let mut service = open_service(test_props());

        let result = service.send(Event::CloseOnEscape);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Open);
    }

    #[test]
    fn backdrop_click_does_not_close_by_default() {
        let mut service = open_service(test_props());

        let result = service.send(Event::CloseOnBackdropClick);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Open);
    }

    #[test]
    fn close_cancel_action_and_close_trigger_explicitly_close() {
        let activators: &[fn(&Api<'_>)] = &[
            |api| api.on_cancel_trigger_click(),
            |api| api.on_action_trigger_click(),
            |api| api.on_close_trigger_click(),
        ];

        for activate in activators {
            assert_explicit_close_handler_closes(*activate);
        }
    }

    fn assert_explicit_close_handler_closes(activate: fn(&Api<'_>)) {
        let mut service = open_service(test_props());
        let captured = Rc::new(RefCell::new(Vec::new()));
        let captured_for_send = Rc::clone(&captured);
        let send = move |event: Event| captured_for_send.borrow_mut().push(event);

        activate(&service.connect(&send));

        assert_eq!(captured.borrow().as_slice(), &[Event::Close]);

        drop(service.send(Event::Close));

        assert_eq!(service.state(), &State::Closed);
    }

    #[test]
    fn overriding_escape_and_backdrop_guards_allows_dismissal() {
        let mut escape = open_service(Props {
            close_on_escape: true,
            ..test_props()
        });

        assert!(escape.send(Event::CloseOnEscape).state_changed);
        assert_eq!(escape.state(), &State::Closed);

        let mut backdrop = open_service(Props {
            close_on_backdrop: true,
            ..test_props()
        });

        assert!(backdrop.send(Event::CloseOnBackdropClick).state_changed);
        assert_eq!(backdrop.state(), &State::Closed);
    }

    #[test]
    fn open_emits_dialog_adapter_intents_without_metadata() {
        let mut service = fresh_service(test_props());

        let result = service.send(Event::Open);

        let names = effect_names(&result);

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::FocusInitial));
        assert!(names.contains(&Effect::FocusFirstTabbable));
        assert!(names.contains(&Effect::ScrollLockAcquire));
        assert!(names.contains(&Effect::SetBackgroundInert));
        assert!(
            result
                .pending_effects
                .iter()
                .all(|effect| effect.metadata.is_none())
        );
    }

    #[test]
    fn content_attrs_produce_alertdialog_role_and_modal_state() {
        let service = open_service(test_props());

        let attrs = service.connect(&|_| {}).content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("alertdialog"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Modal)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Id), Some("alert-content"));
    }

    #[test]
    fn title_and_description_registration_wire_content_aria() {
        let mut service = open_service(test_props());

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        let attrs = service.connect(&|_| {}).content_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("alert-title")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("alert-description")
        );
    }

    #[test]
    fn register_title_and_description_are_idempotent_after_first_registration() {
        let mut service = open_service(test_props());

        let first_title = service.send(Event::RegisterTitle);
        let second_title = service.send(Event::RegisterTitle);
        let first_description = service.send(Event::RegisterDescription);
        let second_description = service.send(Event::RegisterDescription);

        assert!(first_title.context_changed);
        assert!(first_description.context_changed);
        assert!(!second_title.state_changed);
        assert!(!second_title.context_changed);
        assert!(!second_description.state_changed);
        assert!(!second_description.context_changed);
        assert!(service.context().has_title);
        assert!(service.context().has_description);
    }

    #[test]
    fn event_sync_props_replays_context_backed_fields_without_effects() {
        let mut service = open_service(test_props());

        *service.props_mut() = Props {
            id: "alert".to_string(),
            modal: false,
            close_on_backdrop: true,
            close_on_escape: true,
            prevent_scroll: false,
            restore_focus: false,
            initial_focus: Some(FocusTarget::First),
            final_focus: Some(FocusTarget::Last),
            role: dialog::Role::Dialog,
            default_open: true,
            ..Props::default()
        };

        let result = service.send(Event::SyncProps);

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert!(result.pending_effects.is_empty());

        let ctx = service.context();

        assert!(!ctx.modal);
        assert!(ctx.close_on_backdrop);
        assert!(ctx.close_on_escape);
        assert!(!ctx.prevent_scroll);
        assert!(!ctx.restore_focus);
        assert_eq!(ctx.initial_focus, Some(FocusTarget::First));
        assert_eq!(ctx.final_focus, Some(FocusTarget::Last));
        assert_eq!(ctx.role, dialog::Role::Dialog);
    }

    #[test]
    fn on_props_changed_emits_open_close_and_sync_props_only_when_relevant() {
        let base = test_props();

        assert_eq!(
            Machine::on_props_changed(
                &base,
                &Props {
                    open: Some(true),
                    ..test_props()
                },
            ),
            [Event::Open]
        );

        assert_eq!(
            Machine::on_props_changed(
                &Props {
                    open: Some(true),
                    ..test_props()
                },
                &Props {
                    open: Some(false),
                    ..test_props()
                },
            ),
            [Event::Close]
        );

        assert!(Machine::on_props_changed(&base, &base).is_empty());
        assert!(
            Machine::on_props_changed(
                &base,
                &Props {
                    id: "different-alert".to_string(),
                    ..test_props()
                },
            )
            .is_empty()
        );

        for new in [
            Props {
                modal: false,
                ..test_props()
            },
            Props {
                close_on_backdrop: true,
                ..test_props()
            },
            Props {
                close_on_escape: true,
                ..test_props()
            },
            Props {
                prevent_scroll: false,
                ..test_props()
            },
            Props {
                restore_focus: false,
                ..test_props()
            },
            Props {
                initial_focus: Some(FocusTarget::First),
                ..test_props()
            },
            Props {
                final_focus: Some(FocusTarget::Last),
                ..test_props()
            },
            Props {
                role: dialog::Role::Dialog,
                ..test_props()
            },
        ] {
            assert_eq!(
                Machine::on_props_changed(&base, &new),
                [Event::SyncProps],
                "expected SyncProps for {new:?}",
            );
        }

        assert_eq!(
            Machine::on_props_changed(
                &base,
                &Props {
                    open: Some(true),
                    modal: false,
                    ..test_props()
                },
            ),
            [Event::Open, Event::SyncProps]
        );
    }

    #[test]
    fn initial_effects_match_open_initial_state_only() {
        let mut closed = fresh_service(test_props());

        assert!(closed.take_initial_effects().is_empty());

        let mut open = open_service(test_props());

        let names = open
            .take_initial_effects()
            .into_iter()
            .map(|effect| effect.name)
            .collect::<Vec<_>>();

        assert!(names.contains(&Effect::OpenChange));
        assert!(names.contains(&Effect::FocusInitial));
        assert!(names.contains(&Effect::FocusFirstTabbable));
        assert!(names.contains(&Effect::ScrollLockAcquire));
        assert!(names.contains(&Effect::SetBackgroundInert));
    }

    #[test]
    fn api_accessors_reflect_modal_and_mount_props() {
        let defaults = fresh_service(test_props());
        let default_api = defaults.connect(&|_| {});

        assert!(default_api.is_modal());
        assert!(!default_api.lazy_mount());
        assert!(!default_api.unmount_on_exit());

        let custom = fresh_service(Props {
            modal: false,
            lazy_mount: true,
            unmount_on_exit: true,
            ..test_props()
        });

        let custom_api = custom.connect(&|_| {});

        assert!(!custom_api.is_modal());
        assert!(custom_api.lazy_mount());
        assert!(custom_api.unmount_on_exit());
    }

    #[test]
    fn cancel_and_action_trigger_labels_are_localized() {
        let messages = Messages {
            cancel_label: MessageFn::new(|locale: &Locale| {
                if locale.to_bcp47().starts_with("es") {
                    "Cancelar".to_string()
                } else {
                    "Cancel".to_string()
                }
            }),
            confirm_label: MessageFn::static_str("Confirmar"),
        };

        let env = Env {
            locale: Locale::parse("es").expect("valid locale"),
            ..Env::default()
        };

        let service = Service::<Machine>::new(test_props(), &env, &messages);
        let api = service.connect(&|_| {});

        assert_eq!(
            api.cancel_trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Cancelar")
        );
        assert_eq!(
            api.action_trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Confirmar")
        );
    }

    #[test]
    fn destructive_action_trigger_sets_data_attribute() {
        let service = fresh_service(Props {
            is_destructive: true,
            ..test_props()
        });

        let attrs = service.connect(&|_| {}).action_trigger_attrs();

        assert!(attrs.contains(&HtmlAttr::Data("ars-destructive")));
    }

    #[test]
    fn connect_api_part_attrs_match_direct_methods() {
        let mut service = open_service(test_props());

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Trigger), api.trigger_attrs());
        assert_eq!(api.part_attrs(Part::Backdrop), api.backdrop_attrs());
        assert_eq!(api.part_attrs(Part::Positioner), api.positioner_attrs());
        assert_eq!(api.part_attrs(Part::Content), api.content_attrs());
        assert_eq!(api.part_attrs(Part::Title), api.title_attrs());
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::CancelTrigger),
            api.cancel_trigger_attrs()
        );
        assert_eq!(
            api.part_attrs(Part::ActionTrigger),
            api.action_trigger_attrs()
        );
        assert_eq!(
            api.part_attrs(Part::CloseTrigger),
            api.close_trigger_attrs()
        );
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new()
            .id("builder")
            .open(Some(true))
            .default_open(true)
            .modal(false)
            .close_on_backdrop(true)
            .close_on_escape(true)
            .prevent_scroll(false)
            .restore_focus(false)
            .initial_focus(Some(FocusTarget::First))
            .final_focus(Some(FocusTarget::Last))
            .role(dialog::Role::Dialog)
            .title_level(4)
            .lazy_mount(true)
            .unmount_on_exit(true)
            .is_destructive(true)
            .on_open_change(|_| {})
            .on_escape_key_down(|_| {})
            .on_interact_outside(|_| {});

        assert_eq!(props.id, "builder");
        assert_eq!(props.open, Some(true));
        assert!(props.default_open);
        assert!(!props.modal);
        assert!(props.close_on_backdrop);
        assert!(props.close_on_escape);
        assert!(!props.prevent_scroll);
        assert!(!props.restore_focus);
        assert_eq!(props.initial_focus, Some(FocusTarget::First));
        assert_eq!(props.final_focus, Some(FocusTarget::Last));
        assert_eq!(props.role, dialog::Role::Dialog);
        assert_eq!(props.title_level, 4);
        assert!(props.lazy_mount);
        assert!(props.unmount_on_exit);
        assert!(props.is_destructive);
        assert!(props.on_open_change.is_some());
        assert!(props.on_escape_key_down.is_some());
        assert!(props.on_interact_outside.is_some());
    }

    #[test]
    fn props_debug_redacts_callback_closures() {
        let props = Props::new()
            .id("dbg")
            .on_open_change(|_| {})
            .on_escape_key_down(|_| {})
            .on_interact_outside(|_| {});

        let rendered = format!("{props:?}");

        assert!(rendered.contains("Props"));
        assert!(rendered.contains("Callback(..)"));
    }

    #[test]
    fn api_event_handlers_send_expected_events() {
        let service = open_service(test_props());
        let captured = Rc::new(RefCell::new(Vec::new()));
        let captured_for_send = Rc::clone(&captured);
        let send = move |event: Event| captured_for_send.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_trigger_click();
        api.on_backdrop_click();
        api.on_content_keydown(&keyboard_data(KeyboardKey::Escape));
        api.on_content_keydown(&keyboard_data(KeyboardKey::Tab));

        assert_eq!(
            captured.borrow().as_slice(),
            &[
                Event::Toggle,
                Event::CloseOnBackdropClick,
                Event::CloseOnEscape
            ]
        );
    }

    #[test]
    fn content_keydown_non_escape_does_not_send_event() {
        let service = open_service(test_props());
        let captured = Rc::new(RefCell::new(Vec::new()));
        let captured_for_send = Rc::clone(&captured);
        let send = move |event: Event| captured_for_send.borrow_mut().push(event);

        service
            .connect(&send)
            .on_content_keydown(&keyboard_data(KeyboardKey::Tab));

        assert!(captured.borrow().is_empty());
    }

    #[test]
    fn snapshot_root_closed() {
        assert_snapshot!(
            "alert_dialog_root_closed",
            snapshot_attrs(&fresh_service(test_props()).connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_open() {
        assert_snapshot!(
            "alert_dialog_root_open",
            snapshot_attrs(&open_service(test_props()).connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_trigger_closed() {
        assert_snapshot!(
            "alert_dialog_trigger_closed",
            snapshot_attrs(&fresh_service(test_props()).connect(&|_| {}).trigger_attrs())
        );
    }

    #[test]
    fn snapshot_trigger_open() {
        assert_snapshot!(
            "alert_dialog_trigger_open",
            snapshot_attrs(&open_service(test_props()).connect(&|_| {}).trigger_attrs())
        );
    }

    #[test]
    fn snapshot_backdrop_closed() {
        assert_snapshot!(
            "alert_dialog_backdrop_closed",
            snapshot_attrs(
                &fresh_service(test_props())
                    .connect(&|_| {})
                    .backdrop_attrs()
            )
        );
    }

    #[test]
    fn snapshot_backdrop_open() {
        assert_snapshot!(
            "alert_dialog_backdrop_open",
            snapshot_attrs(&open_service(test_props()).connect(&|_| {}).backdrop_attrs())
        );
    }

    #[test]
    fn snapshot_positioner() {
        assert_snapshot!(
            "alert_dialog_positioner",
            snapshot_attrs(
                &fresh_service(test_props())
                    .connect(&|_| {})
                    .positioner_attrs()
            )
        );
    }

    #[test]
    fn snapshot_content_open_default() {
        assert_snapshot!(
            "alert_dialog_content_open_default",
            snapshot_attrs(&open_service(test_props()).connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_closed() {
        assert_snapshot!(
            "alert_dialog_content_closed",
            snapshot_attrs(&fresh_service(test_props()).connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_with_title() {
        let mut service = open_service(test_props());

        drop(service.send(Event::RegisterTitle));

        assert_snapshot!(
            "alert_dialog_content_with_title",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_with_description() {
        let mut service = open_service(test_props());

        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "alert_dialog_content_with_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_content_with_title_and_description() {
        let mut service = open_service(test_props());

        drop(service.send(Event::RegisterTitle));
        drop(service.send(Event::RegisterDescription));

        assert_snapshot!(
            "alert_dialog_content_with_title_and_description",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn snapshot_title_default_h2() {
        assert_snapshot!(
            "alert_dialog_title_default_h2",
            snapshot_attrs(&fresh_service(test_props()).connect(&|_| {}).title_attrs())
        );
    }

    #[test]
    fn snapshot_title_clamped_above_six() {
        assert_snapshot!(
            "alert_dialog_title_clamped_above_six",
            snapshot_attrs(
                &fresh_service(Props {
                    title_level: 10,
                    ..test_props()
                })
                .connect(&|_| {})
                .title_attrs(),
            )
        );
    }

    #[test]
    fn snapshot_title_clamped_below_one() {
        assert_snapshot!(
            "alert_dialog_title_clamped_below_one",
            snapshot_attrs(
                &fresh_service(Props {
                    title_level: 0,
                    ..test_props()
                })
                .connect(&|_| {})
                .title_attrs(),
            )
        );
    }

    #[test]
    fn snapshot_description() {
        assert_snapshot!(
            "alert_dialog_description",
            snapshot_attrs(
                &fresh_service(test_props())
                    .connect(&|_| {})
                    .description_attrs()
            )
        );
    }

    #[test]
    fn snapshot_cancel_trigger_default_label() {
        assert_snapshot!(
            "alert_dialog_cancel_trigger_default_label",
            snapshot_attrs(
                &fresh_service(test_props())
                    .connect(&|_| {})
                    .cancel_trigger_attrs()
            )
        );
    }

    #[test]
    fn snapshot_cancel_trigger_localized_label() {
        let messages = Messages {
            cancel_label: MessageFn::static_str("Cancelar"),
            ..Messages::default()
        };

        let service = Service::<Machine>::new(test_props(), &Env::default(), &messages);

        assert_snapshot!(
            "alert_dialog_cancel_trigger_localized_label",
            snapshot_attrs(&service.connect(&|_| {}).cancel_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_action_trigger_default_label() {
        assert_snapshot!(
            "alert_dialog_action_trigger_default_label",
            snapshot_attrs(
                &fresh_service(test_props())
                    .connect(&|_| {})
                    .action_trigger_attrs()
            )
        );
    }

    #[test]
    fn snapshot_action_trigger_localized_label() {
        let messages = Messages {
            confirm_label: MessageFn::static_str("Confirmar"),
            ..Messages::default()
        };

        let service = Service::<Machine>::new(test_props(), &Env::default(), &messages);

        assert_snapshot!(
            "alert_dialog_action_trigger_localized_label",
            snapshot_attrs(&service.connect(&|_| {}).action_trigger_attrs())
        );
    }

    #[test]
    fn snapshot_action_trigger_destructive() {
        assert_snapshot!(
            "alert_dialog_action_trigger_destructive",
            snapshot_attrs(
                &fresh_service(Props {
                    is_destructive: true,
                    ..test_props()
                })
                .connect(&|_| {})
                .action_trigger_attrs(),
            )
        );
    }

    #[test]
    fn snapshot_close_trigger() {
        assert_snapshot!(
            "alert_dialog_close_trigger",
            snapshot_attrs(
                &fresh_service(test_props())
                    .connect(&|_| {})
                    .close_trigger_attrs()
            )
        );
    }
}
