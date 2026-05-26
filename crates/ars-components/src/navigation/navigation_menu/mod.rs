//! Navigation menu component machine.
//!
//! NavigationMenu owns framework-agnostic trigger registration, open-item
//! state, delayed hover intent, roving focus intent, localized landmark
//! labels, and ARIA/data attributes for the navigation menu anatomy. Adapters
//! own live element handles, DOM focus, timers, submenu positioning,
//! measurements, viewport animation styles, and z-index allocation.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Debug};

use ars_collections::Key;
use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, Orientation,
    PendingEffect, TransitionPlan, no_cleanup,
};
use ars_interactions::KeyboardEventData;

use super::key_token::dom_safe_key_token;

/// State of the `NavigationMenu` state machine.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// No submenu is open. The menu bar is idle.
    #[default]
    Idle,

    /// A submenu content panel is visible.
    Open {
        /// The key of the item whose content is currently shown.
        item: Key,
    },
}

/// Events accepted by the `NavigationMenu` state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open a specific item's content panel immediately.
    Open(Key),

    /// Close the currently open content panel and record a close timestamp.
    Close(u64),

    /// Pointer entered a trigger at the supplied adapter timestamp.
    PointerEnter(Key, u64),

    /// Pointer left a trigger.
    PointerLeave,

    /// A trigger received focus.
    FocusTrigger {
        /// The focused trigger key.
        item: Key,

        /// Whether focus originated from keyboard modality.
        is_keyboard: bool,
    },

    /// Move keyboard focus to the next trigger.
    FocusNext,

    /// Move keyboard focus to the previous trigger.
    FocusPrev,

    /// Move keyboard focus to the first trigger.
    FocusFirst,

    /// Move keyboard focus to the last trigger.
    FocusLast,

    /// A link inside the content was selected.
    SelectLink(u64),

    /// Escape requested submenu dismissal.
    EscapeKey(u64),

    /// The adapter-managed open-delay timer fired for the key.
    OpenTimerFired(Key),

    /// The adapter-managed close-delay timer fired at the timestamp.
    CloseTimerFired(u64),

    /// Pointer entered the content area.
    ContentPointerEnter,

    /// Pointer left the content area.
    ContentPointerLeave,

    /// Set the resolved text direction.
    SetDirection(Direction),

    /// Request that the adapter focus the element with the target id.
    RequestFocus {
        /// Adapter-resolvable target id.
        target_id: String,
    },

    /// Replace registered trigger keys in DOM order.
    SetItems(Vec<Key>),

    /// Synchronize props-backed context fields.
    SyncProps,

    /// Synchronize the externally controlled open item.
    SyncControlledValue(Option<Key>),

    /// Synchronize provider-backed locale and messages.
    SyncMessages {
        /// Active provider locale.
        locale: Locale,

        /// Localized `NavigationMenu` messages.
        messages: Messages,
    },
}

/// Typed effect intents emitted by the `NavigationMenu` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts or refreshes the open delay timer.
    OpenDelay,

    /// Adapter starts or refreshes the close delay timer.
    CloseDelay,

    /// Adapter moves DOM focus to [`Context::focused_trigger`].
    FocusTrigger,

    /// Adapter invokes [`Props::on_value_change`].
    ValueChange,
}

/// Localizable strings for the `NavigationMenu` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the navigation landmark.
    pub navigation_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            navigation_label: MessageFn::static_str("Main"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Props for the `NavigationMenu` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Unique component identifier.
    pub id: String,

    /// Controlled open item key.
    pub value: Option<Option<Key>>,

    /// Initial open item when uncontrolled.
    pub default_value: Option<Key>,

    /// Delay in milliseconds before a hovered trigger opens.
    pub delay_ms: u32,

    /// Window after closing during which hovering a new trigger skips delay.
    pub skip_delay_ms: u32,

    /// Layout orientation of the trigger list.
    pub orientation: Orientation,

    /// Text direction for root attrs and keyboard semantics.
    pub dir: Direction,

    /// Whether focus wraps from the last trigger to the first.
    pub loop_focus: bool,

    /// Callback invoked after the open item changes.
    pub on_value_change: Option<Callback<dyn Fn(Option<Key>) + Send + Sync>>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            delay_ms: 200,
            skip_delay_ms: 300,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            loop_focus: true,
            on_value_change: None,
        }
    }
}

impl Props {
    /// Returns default `NavigationMenu` props.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`Self::id`].
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`Self::value`], making the instance controlled at mount.
    #[must_use]
    pub fn value(mut self, value: Option<Key>) -> Self {
        self.value = Some(value);
        self
    }

    /// Clears [`Self::value`], making the instance uncontrolled at mount.
    #[must_use]
    pub fn uncontrolled(mut self) -> Self {
        self.value = None;
        self
    }

    /// Sets [`Self::default_value`].
    #[must_use]
    pub fn default_value(mut self, value: Key) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Clears [`Self::default_value`].
    #[must_use]
    pub fn no_default_value(mut self) -> Self {
        self.default_value = None;
        self
    }

    /// Sets [`Self::delay_ms`].
    #[must_use]
    pub const fn delay_ms(mut self, value: u32) -> Self {
        self.delay_ms = value;
        self
    }

    /// Sets [`Self::skip_delay_ms`].
    #[must_use]
    pub const fn skip_delay_ms(mut self, value: u32) -> Self {
        self.skip_delay_ms = value;
        self
    }

    /// Sets [`Self::orientation`].
    #[must_use]
    pub const fn orientation(mut self, value: Orientation) -> Self {
        self.orientation = value;
        self
    }

    /// Sets [`Self::dir`].
    #[must_use]
    pub const fn dir(mut self, value: Direction) -> Self {
        self.dir = value;
        self
    }

    /// Sets [`Self::loop_focus`].
    #[must_use]
    pub const fn loop_focus(mut self, value: bool) -> Self {
        self.loop_focus = value;
        self
    }

    /// Registers [`Self::on_value_change`].
    #[must_use]
    pub fn on_value_change(
        mut self,
        callback: Callback<dyn Fn(Option<Key>) + Send + Sync>,
    ) -> Self {
        self.on_value_change = Some(callback);
        self
    }
}

/// Props for a nested sub-navigation menu embedded inside a content panel.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SubProps {
    /// Controlled open item within this sub-menu.
    pub value: Option<Option<Key>>,

    /// Initial open item when uncontrolled.
    pub default_value: Option<Key>,

    /// Callback invoked after the sub-menu open item changes.
    pub on_value_change: Option<Callback<dyn Fn(Option<Key>) + Send + Sync>>,
}

impl SubProps {
    /// Returns default sub-menu props.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`Self::value`], making the sub-menu controlled at mount.
    #[must_use]
    pub fn value(mut self, value: Option<Key>) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets [`Self::default_value`].
    #[must_use]
    pub fn default_value(mut self, value: Key) -> Self {
        self.default_value = Some(value);
        self
    }

    /// Registers [`Self::on_value_change`].
    #[must_use]
    pub fn on_value_change(
        mut self,
        callback: Callback<dyn Fn(Option<Key>) + Send + Sync>,
    ) -> Self {
        self.on_value_change = Some(callback);
        self
    }
}

/// Context for the `NavigationMenu` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The key of the currently open item.
    pub value: Bindable<Option<Key>>,

    /// The trigger that currently has keyboard focus.
    pub focused_trigger: Option<Key>,

    /// Whether the focused trigger received focus via keyboard.
    pub focus_visible: bool,

    /// Layout orientation of the trigger list.
    pub orientation: Orientation,

    /// Text direction for keyboard semantics.
    pub dir: Direction,

    /// Delay before a hovered trigger opens.
    pub delay_ms: u32,

    /// Skip-delay window after close.
    pub skip_delay_ms: u32,

    /// Timestamp of the last close event.
    pub last_close_time: Option<u64>,

    /// Whether the pointer is currently inside the content area.
    pub pointer_in_content: bool,

    /// Registered trigger keys in DOM order.
    pub items: Vec<Key>,

    /// The key of the previously open item.
    pub previous_item: Option<Key>,

    /// ID of the list element.
    pub list_id: String,

    /// Component IDs for part identification.
    pub ids: ComponentIds,

    /// The resolved locale for this component instance.
    pub locale: Locale,

    /// Resolved messages for accessibility labels.
    pub messages: Messages,

    /// Pending hover-open key used by adapter timer effects.
    pub pending_open_item: Option<Key>,

    /// Last focus target id requested through [`Event::RequestFocus`].
    pub requested_focus_id: Option<String>,
}

/// Structural parts exposed by the `NavigationMenu` connect API.
#[derive(ComponentPart)]
#[scope = "navigation-menu"]
pub enum Part {
    /// The outer navigation landmark element.
    Root,

    /// The menubar list container.
    List,

    /// A top-level item wrapper.
    Item {
        /// Stable item key.
        item_key: Key,
    },

    /// A trigger that opens associated content.
    Trigger {
        /// Stable item key.
        item_key: Key,

        /// Associated content element id.
        content_id: String,
    },

    /// Dropdown content for a trigger.
    Content {
        /// Stable item key.
        item_key: Key,
    },

    /// A navigation link inside content.
    Link {
        /// Whether the link represents the current page.
        active: bool,
    },

    /// Visual active-trigger indicator.
    Indicator,

    /// Optional animated content viewport.
    Viewport,
}

/// Machine for the `NavigationMenu` component.
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

    fn init(props: &Props, env: &Env, messages: &Messages) -> (State, Context) {
        let initial_value = if let Some(value) = &props.value {
            value.clone()
        } else {
            props.default_value.clone()
        };

        let value = if let Some(value) = &props.value {
            Bindable::controlled(value.clone())
        } else {
            Bindable::uncontrolled(props.default_value.clone())
        };

        let ids = ComponentIds::from_id(&props.id);
        let list_id = ids.part("list");

        let state = if let Some(item) = initial_value {
            State::Open { item }
        } else {
            State::Idle
        };

        (
            state,
            Context {
                value,
                focused_trigger: None,
                focus_visible: false,
                orientation: props.orientation,
                dir: props.dir,
                delay_ms: props.delay_ms,
                skip_delay_ms: props.skip_delay_ms,
                last_close_time: None,
                pointer_in_content: false,
                items: Vec::new(),
                previous_item: None,
                list_id,
                ids,
                locale: env.locale.clone(),
                messages: messages.clone(),
                pending_open_item: None,
                requested_focus_id: None,
            },
        )
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (_, Event::Open(item)) => open_item_plan(state, ctx, item.clone()),

            (State::Open { .. }, Event::Close(now_ms) | Event::SelectLink(now_ms)) => {
                Some(close_plan(*now_ms))
            }

            (_, Event::PointerEnter(item, now_ms)) => {
                if matches!(state, State::Open { item: open } if open == item) {
                    return Some(
                        TransitionPlan::context_only(|ctx: &mut Context| {
                            ctx.pending_open_item = None;
                            ctx.pointer_in_content = false;
                        })
                        .cancel_effect(Effect::CloseDelay),
                    );
                }

                if matches!(state, State::Open { .. }) || in_skip_delay_window(ctx, *now_ms) {
                    open_item_plan(state, ctx, item.clone())
                        .map(|plan| plan.cancel_effect(Effect::CloseDelay))
                } else {
                    let item = item.clone();
                    Some(
                        TransitionPlan::context_only(move |ctx: &mut Context| {
                            ctx.pending_open_item = Some(item);
                        })
                        .with_effect(PendingEffect::named(Effect::OpenDelay)),
                    )
                }
            }

            (State::Open { .. }, Event::PointerLeave | Event::ContentPointerLeave) => Some(
                TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.pointer_in_content = false;
                })
                .with_effect(PendingEffect::named(Effect::CloseDelay)),
            ),

            (State::Idle, Event::PointerLeave) => Some(
                TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.pending_open_item = None;
                })
                .cancel_effect(Effect::OpenDelay),
            ),

            (State::Open { .. }, Event::ContentPointerEnter) => Some(
                TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.pointer_in_content = true;
                })
                .cancel_effect(Effect::CloseDelay),
            ),

            (State::Idle, Event::OpenTimerFired(item)) => {
                if ctx.pending_open_item.as_ref() != Some(item) {
                    return None;
                }

                Some(
                    open_to_plan(None, item.clone())
                        .apply(|ctx: &mut Context| {
                            ctx.pending_open_item = None;
                        })
                        .cancel_effect(Effect::OpenDelay),
                )
            }

            (State::Open { .. }, Event::CloseTimerFired(now_ms)) => {
                if ctx.pointer_in_content {
                    return None;
                }

                Some(close_plan(*now_ms).cancel_effect(Effect::CloseDelay))
            }

            (_, Event::FocusTrigger { item, is_keyboard }) => {
                let item = item.clone();
                let is_keyboard = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused_trigger = Some(item);
                    ctx.focus_visible = is_keyboard;
                }))
            }

            (_, Event::FocusNext) => focus_by_offset_plan(ctx, props, 1),

            (_, Event::FocusPrev) => focus_by_offset_plan(ctx, props, -1),

            (_, Event::FocusFirst) => focus_absolute_plan(ctx, 0),

            (_, Event::FocusLast) => {
                if ctx.items.is_empty() {
                    None
                } else {
                    focus_absolute_plan(ctx, ctx.items.len() - 1)
                }
            }

            (_, Event::EscapeKey(now_ms)) => {
                let item = effective_open_item(state, ctx)?.clone();
                Some(
                    close_plan(*now_ms)
                        .apply(move |ctx: &mut Context| {
                            ctx.focused_trigger = Some(item);
                            ctx.focus_visible = true;
                        })
                        .with_effect(PendingEffect::named(Effect::FocusTrigger)),
                )
            }

            (_, Event::SetDirection(dir)) if ctx.dir != *dir => {
                let dir = *dir;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.dir = dir;
                }))
            }

            (_, Event::RequestFocus { target_id }) => {
                let target_id = target_id.clone();
                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.requested_focus_id = Some(target_id);
                    })
                    .with_effect(PendingEffect::named(Effect::FocusTrigger)),
                )
            }

            (_, Event::SetItems(items)) => {
                let items = dedupe_keys(items);
                let open_removed = ctx
                    .value
                    .get()
                    .as_ref()
                    .is_some_and(|item| !items.iter().any(|candidate| candidate == item));

                let plan = if open_removed {
                    TransitionPlan::to(State::Idle)
                } else {
                    TransitionPlan::new()
                };

                Some(plan.apply(move |ctx: &mut Context| {
                    ctx.items = items;
                    if let Some(focused) = &ctx.focused_trigger
                        && !ctx.items.iter().any(|item| item == focused)
                    {
                        ctx.focused_trigger = None;
                        ctx.focus_visible = false;
                    }

                    if open_removed {
                        ctx.previous_item = ctx.value.get().clone();
                        ctx.value.set(None);
                        ctx.pending_open_item = None;
                        ctx.pointer_in_content = false;
                    }
                }))
            }

            (_, Event::SyncProps) => {
                let orientation = props.orientation;
                let delay_ms = props.delay_ms;
                let skip_delay_ms = props.skip_delay_ms;
                let value = props.value.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.orientation = orientation;
                    ctx.delay_ms = delay_ms;
                    ctx.skip_delay_ms = skip_delay_ms;
                    ctx.value.sync_controlled(value);
                }))
            }

            (_, Event::SyncControlledValue(value)) => {
                let value = value.clone();

                let next_state = if let Some(item) = &value {
                    State::Open { item: item.clone() }
                } else {
                    State::Idle
                };

                Some(
                    TransitionPlan::to(next_state).apply(move |ctx: &mut Context| {
                        ctx.previous_item = ctx.value.get().clone();
                        ctx.value.sync_controlled(Some(value));
                    }),
                )
            }

            (_, Event::SyncMessages { locale, messages }) => {
                let locale = locale.clone();
                let messages = messages.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.locale = locale;
                    ctx.messages = messages;
                }))
            }

            _ => None,
        }
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

    fn on_props_changed(old: &Props, new: &Props) -> Vec<Event> {
        assert_eq!(
            old.id, new.id,
            "NavigationMenu id cannot change after initialization"
        );

        let mut events = Vec::new();

        if old.dir != new.dir {
            events.push(Event::SetDirection(new.dir));
        }

        let needs_sync_props = old.orientation != new.orientation
            || old.delay_ms != new.delay_ms
            || old.skip_delay_ms != new.skip_delay_ms;

        let mut emitted_sync_props = false;

        if old.value != new.value {
            if let Some(value) = &new.value {
                events.push(Event::SyncControlledValue(value.clone()));
            } else {
                events.push(Event::SyncProps);
                emitted_sync_props = true;
            }
        }

        if needs_sync_props && !emitted_sync_props {
            events.push(Event::SyncProps);
        }

        events
    }
}

/// Connected `NavigationMenu` API.
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

impl<'a> Api<'a> {
    /// Get the key of the currently open item, if any.
    #[must_use]
    pub fn open_item(&self) -> Option<&Key> {
        let item = self.ctx.value.get().as_ref()?;
        item_is_registered(self.ctx, item).then_some(item)
    }

    /// Check whether a specific item's content is currently showing.
    #[must_use]
    pub fn is_item_open(&self, item_key: &Key) -> bool {
        self.open_item() == Some(item_key)
    }

    /// Attrs for the outer navigation landmark element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "navigation")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.navigation_label)(&self.ctx.locale),
            )
            .set(
                HtmlAttr::Data("ars-orientation"),
                orientation_value(self.ctx.orientation),
            )
            .set(HtmlAttr::Dir, direction_value(self.ctx.dir));

        attrs
    }

    /// Attrs for the menubar list container.
    #[must_use]
    pub fn list_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::List.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.list_id.clone())
            .set(HtmlAttr::Role, "menubar")
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                orientation_value(self.ctx.orientation),
            );

        attrs
    }

    /// Attrs for an item wrapper.
    #[must_use]
    pub fn item_attrs(&self, _item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item {
            item_key: Key::default(),
        }
        .data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Attrs for a trigger button that opens or closes a content panel.
    #[must_use]
    pub fn trigger_attrs(&self, item_key: &Key, content_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Trigger {
            item_key: Key::default(),
            content_id: String::new(),
        }
        .data_attrs();

        let is_open = self.is_item_open(item_key);
        let is_focused = self.ctx.focused_trigger.as_ref() == Some(item_key);
        let trigger_id = trigger_dom_id(&self.ctx.ids, item_key);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, trigger_id)
            .set(HtmlAttr::Role, "menuitem")
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "true")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if is_open { "true" } else { "false" },
            );

        if is_open {
            attrs.set(HtmlAttr::Aria(AriaAttr::Controls), content_id);
        }

        attrs.set(
            HtmlAttr::Data("ars-state"),
            if is_open { "open" } else { "closed" },
        );

        if is_focused && self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs.set(HtmlAttr::TabIndex, self.trigger_tab_index(item_key));

        attrs
    }

    /// Attrs for a content panel revealed when its trigger is active.
    #[must_use]
    pub fn content_attrs(&self, item_key: &Key) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content {
            item_key: Key::default(),
        }
        .data_attrs();

        let is_open = self.is_item_open(item_key);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, content_dom_id(&self.ctx.ids, item_key))
            .set(
                HtmlAttr::Data("ars-state"),
                if is_open { "open" } else { "closed" },
            );

        if let Some(motion) = self.motion_direction(item_key) {
            attrs.set(HtmlAttr::Data("ars-motion"), motion);
        }

        if !is_open {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Attrs for a navigation link inside a content panel.
    #[must_use]
    pub fn link_attrs(&self, active: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] =
            Part::Link { active: false }.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if active {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Current), "page")
                .set_bool(HtmlAttr::Data("ars-active"), true);
        }

        attrs
    }

    /// Attrs for the visual indicator that tracks the active trigger.
    #[must_use]
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Indicator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(
                HtmlAttr::Data("ars-state"),
                if self.open_item().is_some() {
                    "visible"
                } else {
                    "hidden"
                },
            );

        attrs
    }

    /// Attrs for the optional viewport container.
    #[must_use]
    pub fn viewport_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Viewport.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-state"),
                if self.open_item().is_some() {
                    "open"
                } else {
                    "closed"
                },
            );

        attrs
    }

    /// Handle pointer enter on a trigger.
    pub fn on_trigger_pointer_enter(&self, item_key: &Key, now_ms: u64) {
        (self.send)(Event::PointerEnter(item_key.clone(), now_ms));
    }

    /// Handle pointer leave on a trigger.
    pub fn on_trigger_pointer_leave(&self) {
        (self.send)(Event::PointerLeave);
    }

    /// Handle focus on a trigger.
    pub fn on_trigger_focus(&self, item_key: &Key, is_keyboard: bool) {
        (self.send)(Event::FocusTrigger {
            item: item_key.clone(),
            is_keyboard,
        });
    }

    /// Handle keydown on a trigger.
    pub fn on_trigger_keydown(&self, item_key: &Key, data: &KeyboardEventData, now_ms: u64) {
        let (prev_key, next_key) = navigation_keys(self.ctx.orientation, self.ctx.dir);

        if data.key == next_key {
            (self.send)(Event::FocusNext);
        } else if data.key == prev_key {
            (self.send)(Event::FocusPrev);
        } else if data.key == KeyboardKey::Home {
            (self.send)(Event::FocusFirst);
        } else if data.key == KeyboardKey::End {
            (self.send)(Event::FocusLast);
        } else if data.key == KeyboardKey::Enter || data.key == KeyboardKey::Space {
            (self.send)(Event::Open(item_key.clone()));
        } else if data.key == KeyboardKey::Escape {
            (self.send)(Event::EscapeKey(now_ms));
        } else if self.ctx.orientation == Orientation::Horizontal
            && (data.key == KeyboardKey::ArrowDown || data.key == KeyboardKey::ArrowUp)
        {
            (self.send)(Event::Open(item_key.clone()));
        }
    }

    /// Handle pointer enter on the content area.
    pub fn on_content_pointer_enter(&self) {
        (self.send)(Event::ContentPointerEnter);
    }

    /// Handle pointer leave on the content area.
    pub fn on_content_pointer_leave(&self) {
        (self.send)(Event::ContentPointerLeave);
    }

    /// Handle keydown inside the content area.
    pub fn on_content_keydown(&self, data: &KeyboardEventData, now_ms: u64) {
        if data.key == KeyboardKey::Escape {
            (self.send)(Event::EscapeKey(now_ms));
        }
    }

    /// Handle click on a link inside the content.
    pub fn on_link_select(&self, now_ms: u64) {
        (self.send)(Event::SelectLink(now_ms));
    }

    fn trigger_tab_index(&self, item_key: &Key) -> &'static str {
        if self.ctx.focused_trigger.as_ref() == Some(item_key)
            || self.ctx.focused_trigger.is_none() && self.ctx.items.first() == Some(item_key)
        {
            "0"
        } else {
            "-1"
        }
    }

    fn motion_direction(&self, item_key: &Key) -> Option<&'static str> {
        let previous = self.ctx.previous_item.as_ref()?;

        let previous_index = self.ctx.items.iter().position(|item| item == previous)?;
        let current_index = self.ctx.items.iter().position(|item| item == item_key)?;

        if current_index > previous_index {
            Some("from-end")
        } else if current_index < previous_index {
            Some("from-start")
        } else {
            None
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            Part::Item { item_key } => self.item_attrs(&item_key),
            Part::Trigger {
                item_key,
                content_id,
            } => self.trigger_attrs(&item_key, &content_id),
            Part::Content { item_key } => self.content_attrs(&item_key),
            Part::Link { active } => self.link_attrs(active),
            Part::Indicator => self.indicator_attrs(),
            Part::Viewport => self.viewport_attrs(),
        }
    }
}

fn in_skip_delay_window(ctx: &Context, now_ms: u64) -> bool {
    if let Some(last_close_time) = ctx.last_close_time {
        now_ms.saturating_sub(last_close_time) < u64::from(ctx.skip_delay_ms)
    } else {
        false
    }
}

const fn state_open_item(state: &State) -> Option<&Key> {
    match state {
        State::Open { item } => Some(item),
        State::Idle => None,
    }
}

fn item_is_registered(ctx: &Context, item: &Key) -> bool {
    ctx.items.is_empty() || ctx.items.iter().any(|candidate| candidate == item)
}

fn effective_open_item<'a>(state: &'a State, ctx: &'a Context) -> Option<&'a Key> {
    ctx.value
        .get()
        .as_ref()
        .filter(|item| item_is_registered(ctx, item))
        .or_else(|| state_open_item(state))
}

fn open_item_plan(state: &State, ctx: &Context, item: Key) -> Option<TransitionPlan<Machine>> {
    if ctx.value.get().as_ref() == Some(&item)
        && matches!(state, State::Open { item: open } if open == &item)
    {
        None
    } else {
        let previous = ctx
            .value
            .get()
            .clone()
            .or_else(|| state_open_item(state).cloned());

        Some(open_to_plan(previous, item))
    }
}

fn open_to_plan(previous: Option<Key>, item: Key) -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Open { item: item.clone() })
        .apply(move |ctx: &mut Context| {
            ctx.previous_item = previous;
            ctx.value.set(Some(item));
            ctx.pointer_in_content = false;
            ctx.pending_open_item = None;
        })
        .with_effect(value_change_effect())
}

fn close_plan(now_ms: u64) -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Idle)
        .apply(move |ctx: &mut Context| {
            ctx.previous_item = ctx.value.get().clone();
            ctx.value.set(None);
            ctx.pointer_in_content = false;
            ctx.pending_open_item = None;
            ctx.last_close_time = Some(now_ms);
        })
        .cancel_effect(Effect::OpenDelay)
        .cancel_effect(Effect::CloseDelay)
        .with_effect(value_change_effect())
}

fn value_change_effect() -> PendingEffect<Machine> {
    PendingEffect::new(
        Effect::ValueChange,
        |ctx: &Context, props: &Props, _send| {
            if let Some(callback) = &props.on_value_change {
                callback(ctx.value.get().clone());
            }

            no_cleanup()
        },
    )
}

fn focus_by_offset_plan(
    ctx: &Context,
    props: &Props,
    offset: isize,
) -> Option<TransitionPlan<Machine>> {
    if ctx.items.is_empty() {
        return None;
    }

    let current = ctx
        .focused_trigger
        .as_ref()
        .and_then(|focused| ctx.items.iter().position(|item| item == focused))
        .unwrap_or(0);

    let len = ctx.items.len();

    let next = if offset.is_positive() {
        if current + 1 >= len {
            if props.loop_focus { 0 } else { current }
        } else {
            current + 1
        }
    } else if current == 0 {
        if props.loop_focus { len - 1 } else { current }
    } else {
        current - 1
    };

    if next == current && !props.loop_focus {
        None
    } else {
        focus_absolute_plan(ctx, next)
    }
}

fn focus_absolute_plan(ctx: &Context, index: usize) -> Option<TransitionPlan<Machine>> {
    let item = ctx.items.get(index)?.clone();

    let target_id = trigger_dom_id(&ctx.ids, &item);

    Some(
        TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.focused_trigger = Some(item);
            ctx.focus_visible = true;
            ctx.requested_focus_id = Some(target_id);
        })
        .with_effect(PendingEffect::named(Effect::FocusTrigger)),
    )
}

fn dedupe_keys(items: &[Key]) -> Vec<Key> {
    let mut deduped = Vec::new();

    for item in items {
        if !deduped.iter().any(|existing| existing == item) {
            deduped.push(item.clone());
        }
    }

    deduped
}

const fn orientation_value(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

const fn direction_value(dir: Direction) -> &'static str {
    match dir {
        Direction::Ltr => "ltr",
        Direction::Rtl => "rtl",
        Direction::Auto => "auto",
    }
}

const fn navigation_keys(orientation: Orientation, dir: Direction) -> (KeyboardKey, KeyboardKey) {
    match (orientation, dir) {
        (Orientation::Horizontal, Direction::Rtl) => {
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        }

        (Orientation::Horizontal, _) => (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight),

        (Orientation::Vertical, _) => (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown),
    }
}

fn trigger_dom_id(ids: &ComponentIds, key: &Key) -> String {
    ids.item("trigger", &dom_safe_key_token(key))
}

fn content_dom_id(ids: &ComponentIds, key: &Key) -> String {
    ids.item("content", &dom_safe_key_token(key))
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, vec, vec::Vec};
    use core::cell::RefCell;

    use ars_collections::Key;
    use ars_core::{
        AriaAttr, AttrMap, Callback, ComponentIds, ConnectApi, Direction, Env, HtmlAttr,
        KeyboardKey, MessageFn, Orientation, SendResult, Service,
    };
    use ars_i18n::locales;
    use ars_interactions::KeyboardEventData;
    use insta::assert_snapshot;

    use super::{
        Api, Effect, Event, Machine, Messages, Part, Props, State, SubProps, content_dom_id,
        trigger_dom_id,
    };

    fn key(value: &str) -> Key {
        Key::str(value)
    }

    fn props() -> Props {
        Props::new().id("nav")
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn service_with_items(props: Props, items: &[Key]) -> Service<Machine> {
        let mut service = service(props);

        drop(service.send(Event::SetItems(items.to_vec())));

        service
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
        let mut attrs = attrs.iter().collect::<Vec<_>>();

        attrs.sort_by_key(|(attr, _)| attr.to_string());

        let mut out = String::new();

        for (attr, value) in attrs {
            out.push_str(&attr.to_string());
            out.push('=');
            out.push_str(value.as_str().unwrap_or("<reactive>"));
            out.push('\n');
        }

        out
    }

    fn effect_names(result: &SendResult<Machine>) -> Vec<Effect> {
        result
            .pending_effects
            .iter()
            .map(|effect| effect.name)
            .collect()
    }

    fn connect_noop(service: &Service<Machine>) -> Api<'_> {
        service.connect(&|_| {})
    }

    #[test]
    fn init_uses_controlled_value_when_present() {
        let service = service(Props::new().id("nav").value(Some(key("docs"))));

        assert_eq!(*service.state(), State::Open { item: key("docs") });
        assert_eq!(service.context().value.get(), &Some(key("docs")));
        assert!(service.context().value.is_controlled());
    }

    #[test]
    fn init_uses_default_value_when_uncontrolled() {
        let service = service(Props::new().id("nav").default_value(key("docs")));

        assert_eq!(*service.state(), State::Open { item: key("docs") });
        assert_eq!(service.context().value.get(), &Some(key("docs")));
        assert!(!service.context().value.is_controlled());
    }

    #[test]
    fn props_builder_clearing_keeps_unrelated_fields() {
        let props = Props::new()
            .id("nav")
            .value(Some(key("docs")))
            .default_value(key("blog"))
            .delay_ms(450)
            .uncontrolled()
            .no_default_value();

        assert_eq!(props.id, "nav");
        assert_eq!(props.value, None);
        assert_eq!(props.default_value, None);
        assert_eq!(props.delay_ms, 450);
    }

    #[test]
    fn open_event_opens_item_and_updates_previous_item() {
        let mut service = service(props().default_value(key("docs")));

        let result = service.send(Event::Open(key("blog")));

        assert_eq!(*service.state(), State::Open { item: key("blog") });
        assert_eq!(service.context().previous_item, Some(key("docs")));
        assert_eq!(service.context().value.get(), &Some(key("blog")));
        assert_eq!(effect_names(&result), vec![Effect::ValueChange]);
    }

    #[test]
    fn close_event_closes_and_records_last_close_time() {
        let mut service = service(props().default_value(key("docs")));

        let result = service.send(Event::Close(50));

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(service.context().previous_item, Some(key("docs")));
        assert_eq!(service.context().last_close_time, Some(50));
        assert_eq!(service.context().value.get(), &None);
        assert_eq!(effect_names(&result), vec![Effect::ValueChange]);
    }

    #[test]
    fn pointer_enter_idle_emits_open_delay_without_opening() {
        let mut service = service(props());

        let result = service.send(Event::PointerEnter(key("docs"), 10));

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(service.context().pending_open_item, Some(key("docs")));
        assert_eq!(effect_names(&result), vec![Effect::OpenDelay]);
    }

    #[test]
    fn pointer_leave_idle_clears_pending_open_delay() {
        let mut service = service(props());

        drop(service.send(Event::PointerEnter(key("docs"), 10)));

        let result = service.send(Event::PointerLeave);

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(service.context().pending_open_item, None);
        assert!(result.cancel_effects.contains(&Effect::OpenDelay));
    }

    #[test]
    fn open_timer_fired_opens_pending_item() {
        let mut service = service(props());

        drop(service.send(Event::PointerEnter(key("docs"), 10)));
        let result = service.send(Event::OpenTimerFired(key("docs")));

        assert_eq!(*service.state(), State::Open { item: key("docs") });
        assert_eq!(service.context().value.get(), &Some(key("docs")));
        assert_eq!(effect_names(&result), vec![Effect::ValueChange]);
    }

    #[test]
    fn stale_open_timer_fired_is_ignored() {
        let mut service = service(props());

        drop(service.send(Event::PointerEnter(key("docs"), 10)));
        let result = service.send(Event::OpenTimerFired(key("blog")));

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(service.context().pending_open_item, Some(key("docs")));
        assert!(result.pending_effects.is_empty());

        drop(service.send(Event::PointerLeave));

        let result = service.send(Event::OpenTimerFired(key("docs")));

        assert_eq!(*service.state(), State::Idle);
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn pointer_enter_during_open_switches_immediately() {
        let mut service = service(props().default_value(key("docs")));

        let result = service.send(Event::PointerEnter(key("blog"), 10));

        assert_eq!(*service.state(), State::Open { item: key("blog") });
        assert_eq!(service.context().previous_item, Some(key("docs")));
        assert!(result.cancel_effects.contains(&Effect::CloseDelay));
        assert_eq!(effect_names(&result), vec![Effect::ValueChange]);
    }

    #[test]
    fn pointer_enter_open_item_cancels_pending_close_delay() {
        let mut service = service(props().default_value(key("docs")));

        drop(service.send(Event::PointerLeave));
        let result = service.send(Event::PointerEnter(key("docs"), 50));

        assert_eq!(*service.state(), State::Open { item: key("docs") });
        assert!(result.cancel_effects.contains(&Effect::CloseDelay));
        assert!(result.pending_effects.is_empty());
    }

    #[test]
    fn pointer_enter_inside_skip_delay_opens_immediately() {
        let mut service = service(props());

        drop(service.send(Event::Open(key("docs"))));
        drop(service.send(Event::Close(100)));

        let result = service.send(Event::PointerEnter(key("blog"), 250));

        assert_eq!(*service.state(), State::Open { item: key("blog") });
        assert_eq!(effect_names(&result), vec![Effect::ValueChange]);
    }

    #[test]
    fn pointer_enter_at_skip_delay_boundary_waits_for_timer() {
        let mut service = service(props().skip_delay_ms(300));

        drop(service.send(Event::Open(key("docs"))));
        drop(service.send(Event::Close(100)));

        let result = service.send(Event::PointerEnter(key("blog"), 400));

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(service.context().pending_open_item, Some(key("blog")));
        assert_eq!(effect_names(&result), vec![Effect::OpenDelay]);
    }

    #[test]
    fn pointer_leave_emits_close_delay() {
        let mut service = service(props().default_value(key("docs")));

        let result = service.send(Event::PointerLeave);

        assert_eq!(*service.state(), State::Open { item: key("docs") });
        assert_eq!(effect_names(&result), vec![Effect::CloseDelay]);
    }

    #[test]
    fn content_pointer_enter_cancels_close_delay() {
        let mut service = service(props().default_value(key("docs")));

        let result = service.send(Event::ContentPointerEnter);

        assert!(service.context().pointer_in_content);
        assert!(result.cancel_effects.contains(&Effect::CloseDelay));
    }

    #[test]
    fn close_timer_ignored_while_pointer_in_content() {
        let mut service = service(props().default_value(key("docs")));

        drop(service.send(Event::ContentPointerEnter));

        let result = service.send(Event::CloseTimerFired(200));

        assert_eq!(*service.state(), State::Open { item: key("docs") });
        assert!(result.pending_effects.is_empty());
        assert_eq!(service.context().value.get(), &Some(key("docs")));
    }

    #[test]
    fn close_timer_fired_after_pointer_leaves_content_closes() {
        let mut service = service(props().default_value(key("docs")));

        drop(service.send(Event::ContentPointerEnter));
        drop(service.send(Event::ContentPointerLeave));

        let result = service.send(Event::CloseTimerFired(200));

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(service.context().last_close_time, Some(200));
        assert_eq!(effect_names(&result), vec![Effect::ValueChange]);
    }

    #[test]
    fn select_link_closes_menu() {
        let mut service = service(props().default_value(key("docs")));

        let result = service.send(Event::SelectLink(300));

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(service.context().last_close_time, Some(300));
        assert_eq!(effect_names(&result), vec![Effect::ValueChange]);
    }

    #[test]
    fn escape_key_closes_and_emits_focus_trigger() {
        let mut service = service(props().default_value(key("docs")));

        let result = service.send(Event::EscapeKey(400));

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(service.context().focused_trigger, Some(key("docs")));
        assert_eq!(
            effect_names(&result),
            vec![Effect::ValueChange, Effect::FocusTrigger]
        );
    }

    #[test]
    fn escape_key_focuses_rendered_controlled_item_when_state_lags() {
        let mut service = service_with_items(
            props().value(Some(key("docs"))),
            &[key("docs"), key("blog")],
        );

        drop(service.send(Event::Open(key("blog"))));
        let result = service.send(Event::EscapeKey(400));

        assert_eq!(service.context().focused_trigger, Some(key("docs")));
        assert_eq!(
            effect_names(&result),
            vec![Effect::ValueChange, Effect::FocusTrigger]
        );
    }

    #[test]
    fn focus_next_prev_first_last_follow_orientation_looping() {
        let mut service = service_with_items(props(), &[key("a"), key("b"), key("c")]);

        drop(service.send(Event::FocusTrigger {
            item: key("b"),
            is_keyboard: true,
        }));

        assert_eq!(
            effect_names(&service.send(Event::FocusNext)),
            vec![Effect::FocusTrigger]
        );
        assert_eq!(service.context().focused_trigger, Some(key("c")));

        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_trigger, Some(key("a")));

        drop(service.send(Event::FocusPrev));

        assert_eq!(service.context().focused_trigger, Some(key("c")));

        drop(service.send(Event::FocusFirst));

        assert_eq!(service.context().focused_trigger, Some(key("a")));

        drop(service.send(Event::FocusLast));

        assert_eq!(service.context().focused_trigger, Some(key("c")));
    }

    #[test]
    fn focus_prev_from_middle_moves_to_previous_item() {
        let mut service = service_with_items(props(), &[key("a"), key("b"), key("c")]);

        drop(service.send(Event::FocusTrigger {
            item: key("b"),
            is_keyboard: true,
        }));

        let result = service.send(Event::FocusPrev);

        assert_eq!(service.context().focused_trigger, Some(key("a")));
        assert_eq!(
            service.context().requested_focus_id,
            Some("nav-trigger-s-61".to_owned())
        );
        assert_eq!(effect_names(&result), vec![Effect::FocusTrigger]);
    }

    #[test]
    fn rtl_horizontal_arrow_semantics_are_reversed() {
        let recorder = RefCell::new(Vec::new());
        let send = |event| recorder.borrow_mut().push(event);

        let service = service_with_items(props().dir(Direction::Rtl), &[key("a"), key("b")]);

        let api = service.connect(&send);

        api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowRight), 0);
        api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowLeft), 0);

        assert_eq!(
            recorder.into_inner(),
            vec![Event::FocusPrev, Event::FocusNext]
        );
    }

    #[test]
    fn non_looping_focus_stops_at_edges() {
        let mut service =
            service_with_items(props().loop_focus(false), &[key("a"), key("b"), key("c")]);

        drop(service.send(Event::FocusTrigger {
            item: key("a"),
            is_keyboard: true,
        }));

        assert!(service.send(Event::FocusPrev).pending_effects.is_empty());
        assert_eq!(service.context().focused_trigger, Some(key("a")));

        drop(service.send(Event::FocusLast));

        assert!(service.send(Event::FocusNext).pending_effects.is_empty());
        assert_eq!(service.context().focused_trigger, Some(key("c")));
    }

    #[test]
    fn non_looping_focus_prev_from_middle_moves_backward() {
        let mut service =
            service_with_items(props().loop_focus(false), &[key("a"), key("b"), key("c")]);

        drop(service.send(Event::FocusTrigger {
            item: key("b"),
            is_keyboard: true,
        }));

        let result = service.send(Event::FocusPrev);

        assert_eq!(service.context().focused_trigger, Some(key("a")));
        assert_eq!(effect_names(&result), vec![Effect::FocusTrigger]);
    }

    #[test]
    fn request_focus_records_target_id_without_dom_access() {
        let mut service = service(props());

        let result = service.send(Event::RequestFocus {
            target_id: "nav-trigger-s-646f6373".to_owned(),
        });

        assert_eq!(
            service.context().requested_focus_id.as_deref(),
            Some("nav-trigger-s-646f6373")
        );
        assert_eq!(effect_names(&result), vec![Effect::FocusTrigger]);
    }

    #[test]
    fn set_direction_updates_context() {
        let mut service = service(props());

        drop(service.send(Event::SetDirection(Direction::Rtl)));

        assert_eq!(service.context().dir, Direction::Rtl);
    }

    #[test]
    fn set_direction_noops_when_unchanged() {
        let service = service(props().dir(Direction::Rtl));

        let plan = <Machine as ars_core::Machine>::transition(
            service.state(),
            &Event::SetDirection(Direction::Rtl),
            service.context(),
            service.props(),
        );

        assert!(plan.is_none());
    }

    #[test]
    fn set_items_replaces_dom_order() {
        let mut service = service(props());

        drop(service.send(Event::SetItems(vec![key("a"), key("b"), key("a")])));

        assert_eq!(service.context().items, vec![key("a"), key("b")]);
    }

    #[test]
    fn set_items_clears_only_stale_focus() {
        let mut service = service_with_items(props(), &[key("a"), key("b"), key("c")]);

        drop(service.send(Event::FocusTrigger {
            item: key("b"),
            is_keyboard: true,
        }));
        drop(service.send(Event::SetItems(vec![key("a"), key("b")])));

        assert_eq!(service.context().focused_trigger, Some(key("b")));
        assert!(service.context().focus_visible);

        drop(service.send(Event::SetItems(vec![key("a"), key("c")])));

        assert_eq!(service.context().focused_trigger, None);
        assert!(!service.context().focus_visible);
    }

    #[test]
    fn set_items_closes_when_open_item_is_removed() {
        let mut service =
            service_with_items(props().default_value(key("b")), &[key("a"), key("b")]);

        drop(service.send(Event::SetItems(vec![key("a")])));

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(service.context().value.get(), &None);
        assert_eq!(service.context().previous_item, Some(key("b")));
    }

    #[test]
    fn set_items_closes_when_registry_becomes_empty() {
        let mut service =
            service_with_items(props().default_value(key("b")), &[key("a"), key("b")]);

        drop(service.send(Event::SetItems(Vec::new())));

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(connect_noop(&service).open_item(), None);
        assert_eq!(service.context().value.get(), &None);
        assert_eq!(service.context().previous_item, Some(key("b")));
    }

    #[test]
    fn set_items_keeps_controlled_open_item_when_transient_state_item_is_removed() {
        let mut service = service_with_items(
            props().value(Some(key("docs"))),
            &[key("docs"), key("blog")],
        );

        drop(service.send(Event::Open(key("blog"))));
        drop(service.send(Event::SetItems(vec![key("docs")])));

        assert_eq!(*service.state(), State::Open { item: key("blog") });
        assert_eq!(connect_noop(&service).open_item(), Some(&key("docs")));
    }

    #[test]
    fn sync_controlled_value_updates_state_and_bindable_value() {
        let mut service = service(props().value(Some(key("docs"))));

        drop(service.send(Event::SyncControlledValue(Some(key("blog")))));

        assert_eq!(*service.state(), State::Open { item: key("blog") });
        assert_eq!(service.context().value.get(), &Some(key("blog")));
        assert_eq!(service.context().previous_item, Some(key("docs")));

        drop(service.send(Event::SyncControlledValue(None)));

        assert_eq!(*service.state(), State::Idle);
        assert_eq!(service.context().value.get(), &None);
        assert_eq!(service.context().previous_item, Some(key("blog")));
    }

    #[test]
    fn controlled_value_drives_public_open_item_and_attrs_until_parent_syncs() {
        let mut service = service_with_items(
            props().value(Some(key("docs"))),
            &[key("docs"), key("blog")],
        );

        drop(service.send(Event::Open(key("blog"))));

        let api = connect_noop(&service);

        assert_eq!(api.open_item(), Some(&key("docs")));
        assert!(api.is_item_open(&key("docs")));
        assert!(!api.is_item_open(&key("blog")));
        assert_eq!(
            api.trigger_attrs(&key("docs"), "docs-content")
                .get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );
        assert_eq!(
            api.trigger_attrs(&key("blog"), "blog-content")
                .get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("false")
        );
    }

    #[test]
    fn sync_props_can_switch_controlled_value_back_to_uncontrolled() {
        let mut service = service(props().value(Some(key("docs"))));

        drop(service.send(Event::Open(key("blog"))));
        service.set_props(props());

        assert!(!service.context().value.is_controlled());
        assert_eq!(service.context().value.get(), &Some(key("blog")));
        assert_eq!(connect_noop(&service).open_item(), Some(&key("blog")));
    }

    #[test]
    fn sync_props_preserves_resolved_auto_direction() {
        let mut service = service(props().dir(Direction::Auto));

        drop(service.send(Event::SetDirection(Direction::Rtl)));
        service.set_props(props().dir(Direction::Auto).delay_ms(500));

        assert_eq!(service.context().dir, Direction::Rtl);
        assert_eq!(service.context().delay_ms, 500);
    }

    #[test]
    fn controlled_sync_updates_motion_previous_item() {
        let mut service = service_with_items(
            props().value(Some(key("docs"))),
            &[key("docs"), key("blog")],
        );

        service.set_props(props().value(Some(key("blog"))));

        assert_eq!(
            connect_noop(&service)
                .content_attrs(&key("blog"))
                .get(&HtmlAttr::Data("ars-motion")),
            Some("from-end")
        );
    }

    #[test]
    fn open_event_reconverges_state_when_controlled_value_already_matches_target() {
        let mut service = service_with_items(
            props().value(Some(key("docs"))),
            &[key("docs"), key("blog")],
        );

        drop(service.send(Event::Open(key("blog"))));
        drop(service.send(Event::Open(key("docs"))));

        assert_eq!(*service.state(), State::Open { item: key("docs") });
        assert_eq!(connect_noop(&service).open_item(), Some(&key("docs")));
    }

    #[test]
    fn on_props_changed_reports_each_behavioral_change() {
        let base = props().value(Some(key("docs")));

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &Props {
                    value: Some(Some(key("blog"))),
                    ..base.clone()
                },
            ),
            vec![Event::SyncControlledValue(Some(key("blog")))]
        );

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &Props {
                    value: None,
                    ..base.clone()
                },
            ),
            vec![Event::SyncProps]
        );

        for changed in [
            Props {
                orientation: Orientation::Vertical,
                ..base.clone()
            },
            Props {
                delay_ms: 450,
                ..base.clone()
            },
            Props {
                skip_delay_ms: 900,
                ..base.clone()
            },
        ] {
            assert_eq!(
                <Machine as ars_core::Machine>::on_props_changed(&base, &changed),
                vec![Event::SyncProps]
            );
        }

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(
                &base,
                &Props {
                    dir: Direction::Rtl,
                    ..base.clone()
                },
            ),
            vec![Event::SetDirection(Direction::Rtl)]
        );

        assert!(<Machine as ars_core::Machine>::on_props_changed(&base, &base).is_empty());
    }

    #[test]
    fn sync_props_updates_orientation_dir_delay_loop() {
        let mut service = service(props());

        service.set_props(
            Props::new()
                .id("nav")
                .orientation(Orientation::Vertical)
                .dir(Direction::Rtl)
                .delay_ms(500)
                .skip_delay_ms(700)
                .loop_focus(false),
        );

        assert_eq!(service.context().orientation, Orientation::Vertical);
        assert_eq!(service.context().dir, Direction::Rtl);
        assert_eq!(service.context().delay_ms, 500);
        assert_eq!(service.context().skip_delay_ms, 700);
        assert!(!service.props().loop_focus);
    }

    #[test]
    fn sync_messages_updates_locale_messages() {
        let mut service = service(props());

        drop(service.send(Event::SyncMessages {
            locale: locales::en_us(),
            messages: Messages {
                navigation_label: MessageFn::static_str("Primary"),
            },
        }));

        assert_eq!(
            service
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Primary")
        );
    }

    #[test]
    fn sub_props_match_nested_menu_contract() {
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = std::sync::Arc::clone(&calls);

        let props = SubProps::new()
            .value(Some(key("child")))
            .default_value(key("fallback"))
            .on_value_change(Callback::new(move |value: Option<Key>| {
                captured.lock().expect("lock").push(value);
            }));

        assert_eq!(props.value, Some(Some(key("child"))));
        assert_eq!(props.default_value, Some(key("fallback")));

        (props.on_value_change.as_ref().expect("callback"))(None);

        assert_eq!(*calls.lock().expect("lock"), vec![None]);
    }

    #[test]
    fn root_list_and_item_snapshots() {
        let service = service(Props::new().id("nav").dir(Direction::Rtl));

        let api = connect_noop(&service);

        assert_snapshot!("navigation_menu_root", snapshot_attrs(&api.root_attrs()));
        assert_snapshot!("navigation_menu_list", snapshot_attrs(&api.list_attrs()));
        assert_snapshot!(
            "navigation_menu_item",
            snapshot_attrs(&api.item_attrs(&key("docs")))
        );
    }

    #[test]
    fn trigger_content_link_indicator_and_viewport_snapshots() {
        let mut menu_service = service_with_items(
            props().default_value(key("docs")),
            &[key("docs"), key("blog")],
        );

        drop(menu_service.send(Event::Open(key("blog"))));
        drop(menu_service.send(Event::FocusTrigger {
            item: key("blog"),
            is_keyboard: true,
        }));

        let api = connect_noop(&menu_service);

        let content_id = content_dom_id(&menu_service.context().ids, &key("blog"));

        assert_snapshot!(
            "navigation_menu_trigger_open_focus_visible",
            snapshot_attrs(&api.trigger_attrs(&key("blog"), &content_id))
        );
        assert_snapshot!(
            "navigation_menu_trigger_closed",
            snapshot_attrs(&api.trigger_attrs(&key("docs"), &content_id))
        );
        assert_snapshot!(
            "navigation_menu_content_open_motion",
            snapshot_attrs(&api.content_attrs(&key("blog")))
        );
        assert_snapshot!(
            "navigation_menu_content_closed",
            snapshot_attrs(&api.content_attrs(&key("docs")))
        );
        assert_snapshot!(
            "navigation_menu_link_active",
            snapshot_attrs(&api.link_attrs(true))
        );
        assert_snapshot!(
            "navigation_menu_link_inactive",
            snapshot_attrs(&api.link_attrs(false))
        );
        assert_snapshot!(
            "navigation_menu_indicator_visible",
            snapshot_attrs(&api.indicator_attrs())
        );
        assert_snapshot!(
            "navigation_menu_viewport_open",
            snapshot_attrs(&api.viewport_attrs())
        );

        let closed = service(props());

        let closed_api = connect_noop(&closed);

        assert_snapshot!(
            "navigation_menu_indicator_hidden",
            snapshot_attrs(&closed_api.indicator_attrs())
        );
        assert_snapshot!(
            "navigation_menu_viewport_closed",
            snapshot_attrs(&closed_api.viewport_attrs())
        );
    }

    #[test]
    fn trigger_attrs_tabstop_and_open_state_are_item_specific() {
        let mut service = service_with_items(
            props().default_value(key("docs")),
            &[key("docs"), key("blog")],
        );

        let api = connect_noop(&service);

        assert_eq!(
            api.trigger_attrs(&key("docs"), "content")
                .get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );
        assert_eq!(
            api.trigger_attrs(&key("blog"), "content")
                .get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("false")
        );
        assert_eq!(
            api.trigger_attrs(&key("docs"), "content")
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
        assert_eq!(
            api.trigger_attrs(&key("blog"), "content")
                .get(&HtmlAttr::TabIndex),
            Some("-1")
        );

        drop(service.send(Event::FocusTrigger {
            item: key("blog"),
            is_keyboard: false,
        }));

        let api = connect_noop(&service);

        assert_eq!(
            api.trigger_attrs(&key("docs"), "content")
                .get(&HtmlAttr::TabIndex),
            Some("-1")
        );
        assert_eq!(
            api.trigger_attrs(&key("blog"), "content")
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
    }

    #[test]
    fn content_motion_direction_reports_from_start_and_from_end() {
        let mut service = service_with_items(
            props().default_value(key("a")),
            &[key("a"), key("b"), key("c")],
        );

        drop(service.send(Event::Open(key("c"))));

        assert_eq!(
            connect_noop(&service)
                .content_attrs(&key("c"))
                .get(&HtmlAttr::Data("ars-motion")),
            Some("from-end")
        );

        drop(service.send(Event::Open(key("a"))));

        assert_eq!(
            connect_noop(&service)
                .content_attrs(&key("a"))
                .get(&HtmlAttr::Data("ars-motion")),
            Some("from-start")
        );
    }

    #[test]
    fn api_event_helpers_dispatch_typed_events() {
        let service = service_with_items(props(), &[key("a"), key("b")]);

        let recorder = RefCell::new(Vec::new());
        let send = |event| recorder.borrow_mut().push(event);

        {
            let api = service.connect(&send);

            api.on_trigger_pointer_enter(&key("a"), 10);
            api.on_trigger_pointer_leave();
            api.on_trigger_focus(&key("a"), true);
            api.on_content_pointer_enter();
            api.on_content_pointer_leave();
            api.on_link_select(20);
        }

        assert_eq!(
            recorder.into_inner(),
            vec![
                Event::PointerEnter(key("a"), 10),
                Event::PointerLeave,
                Event::FocusTrigger {
                    item: key("a"),
                    is_keyboard: true,
                },
                Event::ContentPointerEnter,
                Event::ContentPointerLeave,
                Event::SelectLink(20),
            ]
        );
    }

    #[test]
    fn trigger_keydown_dispatch_matrix_covers_handled_and_ignored_keys() {
        let service = service_with_items(props(), &[key("a"), key("b")]);

        let recorder = RefCell::new(Vec::new());
        let send = |event| recorder.borrow_mut().push(event);

        {
            let api = service.connect(&send);

            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowRight), 700);
            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowLeft), 700);
            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::Home), 700);
            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::End), 700);
            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::Enter), 700);
            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::Space), 700);
            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::Escape), 700);
            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowDown), 700);
            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::Backspace), 700);
        }

        assert_eq!(
            recorder.into_inner(),
            vec![
                Event::FocusNext,
                Event::FocusPrev,
                Event::FocusFirst,
                Event::FocusLast,
                Event::Open(key("a")),
                Event::Open(key("a")),
                Event::EscapeKey(700),
                Event::Open(key("a")),
            ]
        );
    }

    #[test]
    fn horizontal_trigger_arrow_down_opens_current_item() {
        let service = service_with_items(props(), &[key("a")]);

        let recorder = RefCell::new(Vec::new());
        let send = |event| recorder.borrow_mut().push(event);

        {
            let api = service.connect(&send);

            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowDown), 0);
        }

        assert_eq!(recorder.into_inner(), vec![Event::Open(key("a"))]);
    }

    #[test]
    fn vertical_trigger_keydown_uses_vertical_arrows_only() {
        let service = service_with_items(props().orientation(Orientation::Vertical), &[key("a")]);

        let recorder = RefCell::new(Vec::new());
        let send = |event| recorder.borrow_mut().push(event);

        {
            let api = service.connect(&send);

            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowDown), 0);
            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowUp), 0);
            api.on_trigger_keydown(&key("a"), &keyboard(KeyboardKey::ArrowLeft), 0);
        }

        assert_eq!(
            recorder.into_inner(),
            vec![Event::FocusNext, Event::FocusPrev]
        );
    }

    #[test]
    fn content_keydown_dispatches_only_escape() {
        let service = service(props());

        let recorder = RefCell::new(Vec::new());
        let send = |event| recorder.borrow_mut().push(event);

        {
            let api = service.connect(&send);

            api.on_content_keydown(&keyboard(KeyboardKey::Backspace), 900);

            assert!(recorder.borrow().is_empty());

            api.on_content_keydown(&keyboard(KeyboardKey::Escape), 900);
        }

        assert_eq!(recorder.into_inner(), vec![Event::EscapeKey(900)]);
    }

    #[test]
    fn part_attrs_parity_for_every_part() {
        let service = service_with_items(props().default_value(key("docs")), &[key("docs")]);

        let api = connect_noop(&service);

        let content_id = content_dom_id(&service.context().ids, &key("docs"));

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::List), api.list_attrs());
        assert_eq!(
            api.part_attrs(Part::Item {
                item_key: key("docs")
            }),
            api.item_attrs(&key("docs"))
        );
        assert_eq!(
            api.part_attrs(Part::Trigger {
                item_key: key("docs"),
                content_id: content_id.clone(),
            }),
            api.trigger_attrs(&key("docs"), &content_id)
        );
        assert_eq!(
            api.part_attrs(Part::Content {
                item_key: key("docs")
            }),
            api.content_attrs(&key("docs"))
        );
        assert_eq!(
            api.part_attrs(Part::Link { active: true }),
            api.link_attrs(true)
        );
        assert_eq!(api.part_attrs(Part::Indicator), api.indicator_attrs());
        assert_eq!(api.part_attrs(Part::Viewport), api.viewport_attrs());
    }

    #[test]
    fn trigger_and_content_ids_are_dom_safe() {
        let ids = ComponentIds::from_id("nav");

        let key = key("product docs");

        assert_eq!(
            trigger_dom_id(&ids, &key),
            "nav-trigger-s-70726f6475637420646f6373"
        );
        assert_eq!(
            content_dom_id(&ids, &key),
            "nav-content-s-70726f6475637420646f6373"
        );
    }

    #[test]
    fn no_core_z_index_surface_is_emitted() {
        let service = service(props().default_value(key("docs")));

        let attrs = service.connect(&|_| {}).content_attrs(&key("docs"));

        assert!(attrs.styles().is_empty());
        assert!(
            attrs
                .iter()
                .all(|(attr, _)| !attr.to_string().contains("z-index"))
        );
    }

    #[test]
    fn on_value_change_callback_is_invoked_by_value_change_effect() {
        let calls = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let captured = std::sync::Arc::clone(&calls);

        let mut service = service(props().on_value_change(Callback::new(
            move |value: Option<Key>| {
                captured.lock().expect("lock").push(value);
            },
        )));

        let result = service.send(Event::Open(key("docs")));

        for effect in result.pending_effects {
            let cleanup = effect.run(
                service.context(),
                service.props(),
                alloc::sync::Arc::new(|_| {}),
            );

            cleanup();
        }

        assert_eq!(*calls.lock().expect("lock"), vec![Some(key("docs"))]);
    }
}
