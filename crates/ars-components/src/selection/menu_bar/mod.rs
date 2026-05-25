//! Menu bar selection component machine.
//!
//! A menu bar owns the top-level trigger state for a horizontal or vertical
//! set of menus. Nested menu item navigation remains the responsibility of the
//! regular menu machine used inside each popup.

use alloc::{string::String, vec, vec::Vec};
use core::fmt::{self, Debug};

use ars_collections::{Collection, Key, StaticCollection};
use ars_core::{
    AriaAttr, AttrMap, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Direction, Env,
    HtmlAttr, KeyboardKey, Locale, NoEffect, Orientation, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

/// Payload for top-level menu bar entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    /// Human-readable menu label.
    pub label: String,
}

/// Menu bar machine states.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// No menu popup is active.
    #[default]
    Inactive,

    /// A top-level menu popup is active.
    Active {
        /// Active top-level menu key.
        menu: Key,
    },
}

/// Events accepted by the menu bar machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Focus a top-level menu trigger.
    FocusItem(Key),

    /// Activate a top-level menu popup.
    ActivateMenu(Key),

    /// Close the active menu while preserving trigger focus.
    DeactivateMenu,

    /// Move to the next top-level menu.
    MoveToNextMenu,

    /// Move to the previous top-level menu.
    MoveToPrevMenu,

    /// Close the active menu.
    Close,

    /// Mark focus state for the menu bar.
    Focus {
        /// Whether focus came from keyboard modality.
        is_keyboard: bool,
    },

    /// Mark the menu bar blurred.
    Blur,

    /// Replace the top-level menu collection dynamically.
    UpdateMenus(StaticCollection<Menu>),

    /// Synchronize context values derived from updated props.
    SyncProps,
}

/// Context for the menu bar machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Resolved locale for message formatting and fallback direction.
    pub locale: Locale,

    /// Top-level menu collection.
    pub menus: StaticCollection<Menu>,

    /// Active menu key.
    pub active_menu: Option<Key>,

    /// Focused top-level menu trigger key.
    pub focused_item: Option<Key>,

    /// Whether focus-visible styling should be emitted.
    pub focus_visible: bool,

    /// Stable component ID derivation helper.
    pub ids: ComponentIds,

    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

/// Props for the `MenuBar` component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Whether the menu bar is disabled.
    pub disabled: bool,

    /// Menu bar orientation.
    pub orientation: Orientation,

    /// Text direction for arrow-key handling.
    pub dir: Direction,

    /// Whether trigger focus wraps at collection boundaries.
    pub loop_focus: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            disabled: false,
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            loop_focus: true,
        }
    }
}

impl Props {
    /// Returns default menu bar props.
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

    /// Sets [`Self::disabled`].
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
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
}

/// Localized messages for the menu bar component.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// Menu bar anatomy parts.
#[derive(ComponentPart)]
#[scope = "menu-bar"]
pub enum Part {
    /// Root menubar container.
    Root,

    /// Top-level menu wrapper.
    Menu {
        /// Menu key.
        key: Key,
    },

    /// Top-level menu trigger.
    MenuTrigger {
        /// Menu key.
        key: Key,
    },

    /// Top-level menu positioner.
    MenuPositioner {
        /// Menu key.
        key: Key,
    },

    /// Top-level menu content panel.
    MenuContent {
        /// Menu key.
        key: Key,
    },
}

/// Machine for the menu bar component.
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

    fn init(props: &Props, env: &Env, messages: &Messages) -> (State, Context) {
        (
            State::Inactive,
            Context {
                locale: env.locale.clone(),
                menus: StaticCollection::default(),
                active_menu: None,
                focused_item: None,
                focus_visible: false,
                ids: ComponentIds::from_id(&props.id),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        if props.disabled
            && matches!(
                event,
                Event::FocusItem(_)
                    | Event::ActivateMenu(_)
                    | Event::MoveToNextMenu
                    | Event::MoveToPrevMenu
                    | Event::Focus { .. }
            )
        {
            return None;
        }

        match (state, event) {
            (_, Event::ActivateMenu(key)) => {
                if !ctx.menus.contains_key(key) {
                    return None;
                }

                let key = key.clone();

                Some(
                    TransitionPlan::to(State::Active { menu: key.clone() }).apply(
                        move |ctx: &mut Context| {
                            ctx.active_menu = Some(key.clone());
                            ctx.focused_item = Some(key.clone());
                        },
                    ),
                )
            }

            (State::Active { .. }, Event::DeactivateMenu | Event::Close) => Some(
                TransitionPlan::to(State::Inactive).apply(|ctx: &mut Context| {
                    ctx.active_menu = None;
                }),
            ),

            (_, Event::FocusItem(key)) => {
                if !ctx.menus.contains_key(key) {
                    return None;
                }

                let key = key.clone();
                let active = ctx.active_menu.is_some();

                if active {
                    Some(
                        TransitionPlan::to(State::Active { menu: key.clone() }).apply(
                            move |ctx: &mut Context| {
                                ctx.active_menu = Some(key.clone());
                                ctx.focused_item = Some(key.clone());
                            },
                        ),
                    )
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.focused_item = Some(key.clone());
                    }))
                }
            }

            (_, Event::Focus { is_keyboard }) => {
                let focus_visible = *is_keyboard;
                let first = ctx.menus.first_key().cloned();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focus_visible = focus_visible;
                    if ctx.focused_item.is_none() {
                        ctx.focused_item = first.clone();
                    }
                }))
            }

            (_, Event::Blur) => Some(TransitionPlan::to(State::Inactive).apply(
                |ctx: &mut Context| {
                    ctx.active_menu = None;
                    ctx.focused_item = None;
                    ctx.focus_visible = false;
                },
            )),

            (State::Active { .. }, Event::MoveToNextMenu) => {
                move_active_plan(ctx, props, DirectionIntent::Next)
            }

            (State::Active { .. }, Event::MoveToPrevMenu) => {
                move_active_plan(ctx, props, DirectionIntent::Prev)
            }

            (State::Inactive, Event::MoveToNextMenu) => {
                move_focus_plan(ctx, props, DirectionIntent::Next)
            }

            (State::Inactive, Event::MoveToPrevMenu) => {
                move_focus_plan(ctx, props, DirectionIntent::Prev)
            }

            (_, Event::UpdateMenus(menus)) => {
                let menus = menus.clone();
                let active_removed = ctx
                    .active_menu
                    .as_ref()
                    .is_some_and(|key| !menus.contains_key(key));
                let mut plan = if active_removed {
                    TransitionPlan::to(State::Inactive)
                } else {
                    TransitionPlan::new()
                };

                plan = plan.apply(move |ctx: &mut Context| {
                    ctx.menus = menus;
                    invalidate_menu_references(ctx);
                });

                Some(plan)
            }

            (_, Event::SyncProps) => Some(sync_props_plan(ctx, props)),

            _ => None,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps]
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
}

/// API for deriving menu bar attributes and dispatching menu bar events.
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
    /// Returns the current machine state.
    #[must_use]
    pub const fn state(&self) -> &State {
        self.state
    }

    /// Attributes for the root menubar container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs.set(HtmlAttr::Role, "menubar").set(
            HtmlAttr::Aria(AriaAttr::Orientation),
            orientation_attr(self.props.orientation),
        );

        if self.props.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for a top-level menu wrapper.
    #[must_use]
    pub fn menu_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::Menu {
            key: Key::default(),
        });

        attrs.set(HtmlAttr::Id, self.ctx.ids.item("menu", key));

        attrs
    }

    /// Attributes for a top-level menu trigger.
    #[must_use]
    pub fn menu_trigger_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::MenuTrigger {
            key: Key::default(),
        });

        let active = self.ctx.active_menu.as_ref() == Some(key);
        let focused = self.ctx.focused_item.as_ref() == Some(key);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("trigger", key))
            .set(HtmlAttr::Role, "menuitem")
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "menu")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if active { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.item("menu-content", key),
            )
            .set(HtmlAttr::TabIndex, if focused { "0" } else { "-1" });

        if active {
            attrs.set_bool(HtmlAttr::Data("ars-active"), true);
        }

        if focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }

        if self.props.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        attrs
    }

    /// Attributes for a menu positioner wrapper.
    #[must_use]
    pub fn menu_positioner_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::MenuPositioner {
            key: Key::default(),
        });

        attrs.set(
            HtmlAttr::Id,
            self.ctx.ids.item_part("menu", key, "positioner"),
        );

        attrs
    }

    /// Attributes for a menu content panel.
    #[must_use]
    pub fn menu_content_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::MenuContent {
            key: Key::default(),
        });

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("menu-content", key))
            .set(HtmlAttr::Role, "menu")
            .set(HtmlAttr::TabIndex, "-1")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.item("trigger", key),
            );

        attrs
    }

    /// Iterates all top-level menu nodes for rendering.
    pub fn menus(&self) -> impl Iterator<Item = &ars_collections::Node<Menu>> {
        self.ctx.menus.nodes()
    }

    /// Dispatches a trigger click event.
    pub fn on_trigger_click(&self, key: &Key) {
        if self.ctx.active_menu.as_ref() == Some(key) {
            (self.send)(Event::DeactivateMenu);
        } else {
            (self.send)(Event::ActivateMenu(key.clone()));
        }
    }

    /// Dispatches trigger keydown events.
    pub fn on_trigger_keydown(&self, key: &Key, data: &KeyboardEventData) {
        match (self.props.orientation, data.key) {
            (_, KeyboardKey::Enter | KeyboardKey::Space)
            | (Orientation::Horizontal, KeyboardKey::ArrowDown) => {
                (self.send)(Event::ActivateMenu(key.clone()));
            }

            (Orientation::Vertical, KeyboardKey::ArrowRight)
                if self.props.dir == Direction::Ltr =>
            {
                (self.send)(Event::ActivateMenu(key.clone()));
            }

            (Orientation::Vertical, KeyboardKey::ArrowLeft) if self.props.dir == Direction::Rtl => {
                (self.send)(Event::ActivateMenu(key.clone()));
            }

            (Orientation::Horizontal, KeyboardKey::ArrowRight) => {
                if self.props.dir == Direction::Rtl {
                    (self.send)(Event::MoveToPrevMenu);
                } else {
                    (self.send)(Event::MoveToNextMenu);
                }
            }

            (Orientation::Horizontal, KeyboardKey::ArrowLeft) => {
                if self.props.dir == Direction::Rtl {
                    (self.send)(Event::MoveToNextMenu);
                } else {
                    (self.send)(Event::MoveToPrevMenu);
                }
            }

            (Orientation::Vertical, KeyboardKey::ArrowDown) => {
                (self.send)(Event::MoveToNextMenu);
            }

            (Orientation::Vertical, KeyboardKey::ArrowUp) => {
                (self.send)(Event::MoveToPrevMenu);
            }

            _ => {}
        }
    }

    /// Dispatches pointer enter on a top-level trigger.
    pub fn on_trigger_pointer_enter(&self, key: &Key) {
        if self.ctx.active_menu.is_some() {
            (self.send)(Event::FocusItem(key.clone()));
        }
    }

    /// Dispatches content keydown events handled by the menu bar shell.
    pub fn on_content_keydown(&self, data: &KeyboardEventData) {
        match (self.props.orientation, data.key) {
            (Orientation::Horizontal, KeyboardKey::ArrowRight) => {
                if self.props.dir == Direction::Rtl {
                    (self.send)(Event::MoveToPrevMenu);
                } else {
                    (self.send)(Event::MoveToNextMenu);
                }
            }

            (Orientation::Horizontal, KeyboardKey::ArrowLeft) => {
                if self.props.dir == Direction::Rtl {
                    (self.send)(Event::MoveToNextMenu);
                } else {
                    (self.send)(Event::MoveToPrevMenu);
                }
            }

            (Orientation::Vertical, KeyboardKey::ArrowDown) => {
                (self.send)(Event::MoveToNextMenu);
            }

            (Orientation::Vertical, KeyboardKey::ArrowUp) => {
                (self.send)(Event::MoveToPrevMenu);
            }

            (_, KeyboardKey::Escape) => (self.send)(Event::Close),

            _ => {}
        }
    }

    /// Dispatches root focus.
    pub fn on_root_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches root blur.
    pub fn on_root_blur(&self) {
        (self.send)(Event::Blur);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Menu { ref key } => self.menu_attrs(key),
            Part::MenuTrigger { ref key } => self.menu_trigger_attrs(key),
            Part::MenuPositioner { ref key } => self.menu_positioner_attrs(key),
            Part::MenuContent { ref key } => self.menu_content_attrs(key),
        }
    }
}

#[derive(Clone, Copy)]
enum DirectionIntent {
    Next,
    Prev,
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

const fn orientation_attr(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

fn move_active_plan(
    ctx: &Context,
    props: &Props,
    direction: DirectionIntent,
) -> Option<TransitionPlan<Machine>> {
    let key = adjacent_key(ctx, props, direction)?;

    Some(
        TransitionPlan::to(State::Active { menu: key.clone() }).apply(move |ctx: &mut Context| {
            ctx.active_menu = Some(key.clone());
            ctx.focused_item = Some(key.clone());
        }),
    )
}

fn move_focus_plan(
    ctx: &Context,
    props: &Props,
    direction: DirectionIntent,
) -> Option<TransitionPlan<Machine>> {
    let key = adjacent_key(ctx, props, direction)?;

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.focused_item = Some(key.clone());
    }))
}

fn adjacent_key(ctx: &Context, props: &Props, direction: DirectionIntent) -> Option<Key> {
    let current = ctx.focused_item.as_ref();

    let next = match (current, direction) {
        (Some(key), DirectionIntent::Next) => ctx.menus.key_after(key).cloned(),
        (Some(key), DirectionIntent::Prev) => ctx.menus.key_before(key).cloned(),
        (None, DirectionIntent::Next) => ctx.menus.first_key().cloned(),
        (None, DirectionIntent::Prev) => ctx.menus.last_key().cloned(),
    };

    next.or_else(|| {
        if props.loop_focus {
            match direction {
                DirectionIntent::Next => ctx.menus.first_key().cloned(),
                DirectionIntent::Prev => ctx.menus.last_key().cloned(),
            }
        } else {
            None
        }
    })
}

fn sync_props_plan(ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    let props = props.clone();
    let mut plan = if props.disabled && ctx.active_menu.is_some() {
        TransitionPlan::to(State::Inactive)
    } else {
        TransitionPlan::new()
    };

    plan = plan.apply(move |ctx: &mut Context| {
        ctx.ids = ComponentIds::from_id(&props.id);

        if props.disabled {
            ctx.active_menu = None;
            ctx.focused_item = None;
            ctx.focus_visible = false;
        }
    });

    plan
}

fn invalidate_menu_references(ctx: &mut Context) {
    if ctx
        .focused_item
        .as_ref()
        .is_some_and(|key| !ctx.menus.contains_key(key))
    {
        ctx.focused_item = None;
    }

    if ctx
        .active_menu
        .as_ref()
        .is_some_and(|key| !ctx.menus.contains_key(key))
    {
        ctx.active_menu = None;
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::String};
    use core::cell::RefCell;

    use ars_collections::{CollectionBuilder, Key};
    use ars_core::{
        AriaAttr, ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Orientation, Service,
    };
    use ars_interactions::KeyboardEventData;

    use super::{Event, Machine, Menu, Messages, Part, Props, State};

    fn key(value: &'static str) -> Key {
        Key::str(value)
    }

    fn keyboard(key: KeyboardKey) -> KeyboardEventData {
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

    fn collection() -> ars_collections::StaticCollection<Menu> {
        CollectionBuilder::new()
            .item(
                key("file"),
                "File",
                Menu {
                    label: "File".into(),
                },
            )
            .item(
                key("edit"),
                "Edit",
                Menu {
                    label: "Edit".into(),
                },
            )
            .item(
                key("view"),
                "View",
                Menu {
                    label: "View".into(),
                },
            )
            .build()
    }

    fn service(props: Props) -> Service<Machine> {
        let mut service = Service::<Machine>::new(props, &Env::default(), &Messages);

        drop(service.send(Event::UpdateMenus(collection())));

        service
    }

    fn snapshot_attrs(attrs: &ars_core::AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn captured_events(
        menu_bar: &Service<Machine>,
        dispatch: impl FnOnce(&super::Api<'_>),
    ) -> Vec<Event> {
        let captured = RefCell::new(Vec::new());

        let send = |event| captured.borrow_mut().push(event);

        let api = menu_bar.connect(&send);

        dispatch(&api);

        captured.into_inner()
    }

    fn dispatch_root_blur(api: &super::Api<'_>) {
        api.on_root_blur();
    }

    #[test]
    fn root_trigger_and_content_attrs_emit_menu_roles() {
        let mut menu_bar = service(Props::new().id("menu-bar"));

        drop(menu_bar.send(Event::Focus { is_keyboard: true }));

        let api = menu_bar.connect(&|_| {});

        assert_eq!(api.root_attrs().get(&HtmlAttr::Role), Some("menubar"));
        assert_eq!(
            api.menu_trigger_attrs(&key("file")).get(&HtmlAttr::Role),
            Some("menuitem")
        );
        assert_eq!(
            api.menu_content_attrs(&key("file")).get(&HtmlAttr::Role),
            Some("menu")
        );
    }

    #[test]
    fn horizontal_arrows_move_focused_trigger() {
        let mut menu_bar = service(Props::new().id("menu-bar"));

        drop(menu_bar.send(Event::Focus { is_keyboard: true }));
        drop(menu_bar.send(Event::MoveToNextMenu));

        assert_eq!(menu_bar.context().focused_item, Some(key("edit")));

        drop(menu_bar.send(Event::MoveToPrevMenu));

        assert_eq!(menu_bar.context().focused_item, Some(key("file")));
    }

    #[test]
    fn rtl_reverses_left_and_right_trigger_keys() {
        let menu_bar = service(Props::new().id("menu-bar").dir(Direction::Rtl));

        assert_eq!(
            captured_events(&menu_bar, |api| api
                .on_trigger_keydown(&key("file"), &keyboard(KeyboardKey::ArrowRight))),
            vec![Event::MoveToPrevMenu]
        );
        assert_eq!(
            captured_events(&menu_bar, |api| api
                .on_trigger_keydown(&key("file"), &keyboard(KeyboardKey::ArrowLeft))),
            vec![Event::MoveToNextMenu]
        );
    }

    #[test]
    fn vertical_activation_keys_open_menu() {
        let menu_bar = service(Props::new().id("menu-bar"));
        let rtl_menu_bar = service(Props::new().id("menu-bar").dir(Direction::Rtl));

        assert_eq!(
            captured_events(&menu_bar, |api| api
                .on_trigger_keydown(&key("file"), &keyboard(KeyboardKey::ArrowDown))),
            vec![Event::ActivateMenu(key("file"))]
        );
        assert_eq!(
            captured_events(&rtl_menu_bar, |api| api
                .on_trigger_keydown(&key("file"), &keyboard(KeyboardKey::ArrowDown))),
            vec![Event::ActivateMenu(key("file"))]
        );
        assert_eq!(
            captured_events(&menu_bar, |api| api
                .on_trigger_keydown(&key("file"), &keyboard(KeyboardKey::Enter))),
            vec![Event::ActivateMenu(key("file"))]
        );
        assert_eq!(
            captured_events(&menu_bar, |api| api
                .on_trigger_keydown(&key("file"), &keyboard(KeyboardKey::Space))),
            vec![Event::ActivateMenu(key("file"))]
        );
    }

    #[test]
    fn vertical_arrows_move_focused_trigger_and_open_with_right_arrow() {
        let vertical = service(
            Props::new()
                .id("menu-bar")
                .orientation(Orientation::Vertical),
        );

        assert_eq!(
            captured_events(&vertical, |api| api
                .on_trigger_keydown(&key("file"), &keyboard(KeyboardKey::ArrowDown))),
            vec![Event::MoveToNextMenu]
        );
        assert_eq!(
            captured_events(&vertical, |api| api
                .on_trigger_keydown(&key("file"), &keyboard(KeyboardKey::ArrowUp))),
            vec![Event::MoveToPrevMenu]
        );
        assert_eq!(
            captured_events(&vertical, |api| api
                .on_trigger_keydown(&key("file"), &keyboard(KeyboardKey::ArrowRight))),
            vec![Event::ActivateMenu(key("file"))]
        );

        let mut active_vertical = service(
            Props::new()
                .id("menu-bar")
                .orientation(Orientation::Vertical),
        );

        drop(active_vertical.send(Event::ActivateMenu(key("file"))));

        assert_eq!(
            captured_events(&active_vertical, |api| api
                .on_content_keydown(&keyboard(KeyboardKey::ArrowDown))),
            vec![Event::MoveToNextMenu]
        );
        assert_eq!(
            captured_events(&active_vertical, |api| api
                .on_content_keydown(&keyboard(KeyboardKey::ArrowUp))),
            vec![Event::MoveToPrevMenu]
        );
    }

    #[test]
    fn pointer_hover_switches_only_when_active() {
        let mut menu_bar = service(Props::new().id("menu-bar"));

        assert!(
            captured_events(&menu_bar, |api| api.on_trigger_pointer_enter(&key("edit"))).is_empty()
        );

        drop(menu_bar.send(Event::ActivateMenu(key("file"))));

        assert_eq!(
            captured_events(&menu_bar, |api| api.on_trigger_pointer_enter(&key("edit"))),
            vec![Event::FocusItem(key("edit"))]
        );
    }

    #[test]
    fn escape_closes_active_menu() {
        let menu_bar = service(Props::new().id("menu-bar"));

        assert_eq!(
            captured_events(&menu_bar, |api| api
                .on_content_keydown(&keyboard(KeyboardKey::Escape))),
            vec![Event::Close]
        );
    }

    #[test]
    fn trigger_click_toggles_active_menu_and_menus_iterates_collection() {
        let mut menu_bar = service(Props::new().id("menu-bar"));

        let api = menu_bar.connect(&|_| {});

        let keys = api.menus().map(|node| node.key.clone()).collect::<Vec<_>>();

        assert_eq!(keys, vec![key("file"), key("edit"), key("view")]);
        assert_eq!(
            captured_events(&menu_bar, |api| api.on_trigger_click(&key("file"))),
            vec![Event::ActivateMenu(key("file"))]
        );

        drop(menu_bar.send(Event::ActivateMenu(key("file"))));

        assert_eq!(
            captured_events(&menu_bar, |api| api.on_trigger_click(&key("file"))),
            vec![Event::DeactivateMenu]
        );
        assert_eq!(
            captured_events(&menu_bar, |api| api.on_trigger_click(&key("edit"))),
            vec![Event::ActivateMenu(key("edit"))]
        );
    }

    #[test]
    fn transition_events_cover_focus_close_blur_and_active_movement() {
        let mut menu_bar = service(Props::new().id("menu-bar"));

        drop(menu_bar.send(Event::Focus { is_keyboard: true }));

        assert_eq!(menu_bar.context().focused_item, Some(key("file")));
        assert!(menu_bar.context().focus_visible);

        drop(menu_bar.send(Event::FocusItem(key("edit"))));

        assert_eq!(menu_bar.context().focused_item, Some(key("edit")));
        assert_eq!(menu_bar.state(), &State::Inactive);

        drop(menu_bar.send(Event::ActivateMenu(key("edit"))));
        drop(menu_bar.send(Event::MoveToNextMenu));

        assert_eq!(menu_bar.state(), &State::Active { menu: key("view") });
        assert_eq!(menu_bar.context().active_menu, Some(key("view")));
        assert_eq!(menu_bar.context().focused_item, Some(key("view")));

        drop(menu_bar.send(Event::MoveToPrevMenu));

        assert_eq!(menu_bar.state(), &State::Active { menu: key("edit") });

        drop(menu_bar.send(Event::DeactivateMenu));

        assert_eq!(menu_bar.state(), &State::Inactive);
        assert_eq!(menu_bar.context().active_menu, None);
        assert_eq!(menu_bar.context().focused_item, Some(key("edit")));

        drop(menu_bar.send(Event::ActivateMenu(key("file"))));
        drop(menu_bar.send(Event::Close));

        assert_eq!(menu_bar.state(), &State::Inactive);
        assert_eq!(menu_bar.context().active_menu, None);

        drop(menu_bar.send(Event::Blur));

        assert_eq!(menu_bar.context().focused_item, None);
        assert!(!menu_bar.context().focus_visible);
    }

    #[test]
    fn invalid_focus_item_and_disabled_sync_clear_interactive_state() {
        let old_props = Props::new().id("old");
        let new_props = Props::new().id("new").disabled(true);

        let mut menu_bar = service(old_props.clone());

        assert!(
            <Machine as ars_core::Machine>::on_props_changed(&old_props, &old_props).is_empty()
        );
        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&old_props, &new_props),
            vec![Event::SyncProps]
        );

        drop(menu_bar.send(Event::FocusItem(key("missing"))));

        assert_eq!(menu_bar.context().focused_item, None);

        drop(menu_bar.send(Event::ActivateMenu(key("file"))));
        drop(menu_bar.set_props(new_props));

        assert_eq!(menu_bar.state(), &State::Inactive);
        assert_eq!(
            menu_bar.context().ids,
            ars_core::ComponentIds::from_id("new")
        );
        assert_eq!(menu_bar.context().active_menu, None);
        assert_eq!(menu_bar.context().focused_item, None);
        assert!(!menu_bar.context().focus_visible);
    }

    #[test]
    fn content_arrow_keys_switch_top_level_menus_with_direction() {
        let ltr = service(Props::new().id("menu-bar"));

        assert_eq!(
            captured_events(&ltr, |api| api
                .on_content_keydown(&keyboard(KeyboardKey::ArrowRight))),
            vec![Event::MoveToNextMenu]
        );
        assert_eq!(
            captured_events(&ltr, |api| api
                .on_content_keydown(&keyboard(KeyboardKey::ArrowLeft))),
            vec![Event::MoveToPrevMenu]
        );

        let rtl = service(Props::new().id("menu-bar").dir(Direction::Rtl));

        assert_eq!(
            captured_events(&rtl, |api| api
                .on_content_keydown(&keyboard(KeyboardKey::ArrowRight))),
            vec![Event::MoveToPrevMenu]
        );
        assert_eq!(
            captured_events(&rtl, |api| api
                .on_content_keydown(&keyboard(KeyboardKey::ArrowLeft))),
            vec![Event::MoveToNextMenu]
        );
    }

    #[test]
    fn root_focus_and_blur_helpers_dispatch_events() {
        let menu_bar = service(Props::new().id("menu-bar"));

        assert_eq!(
            captured_events(&menu_bar, |api| api.on_root_focus(true)),
            vec![Event::Focus { is_keyboard: true }]
        );
        assert_eq!(
            captured_events(&menu_bar, dispatch_root_blur),
            vec![Event::Blur]
        );
    }

    #[test]
    fn update_menus_invalidates_only_stale_focus_and_active_keys() {
        let mut menu_bar = service(Props::new().id("menu-bar"));

        drop(menu_bar.send(Event::ActivateMenu(key("edit"))));
        drop(menu_bar.send(Event::UpdateMenus(collection())));

        assert_eq!(menu_bar.context().active_menu, Some(key("edit")));
        assert_eq!(menu_bar.context().focused_item, Some(key("edit")));

        let replacement = CollectionBuilder::new()
            .item(
                key("file"),
                "File",
                Menu {
                    label: "File".into(),
                },
            )
            .item(
                key("view"),
                "View",
                Menu {
                    label: "View".into(),
                },
            )
            .build();

        drop(menu_bar.send(Event::UpdateMenus(replacement)));

        assert_eq!(menu_bar.state(), &State::Inactive);
        assert_eq!(menu_bar.context().active_menu, None);
        assert_eq!(menu_bar.context().focused_item, None);
    }

    #[test]
    fn disabled_menu_bar_suppresses_activation() {
        let mut menu_bar = service(Props::new().id("menu-bar").disabled(true));

        drop(menu_bar.send(Event::ActivateMenu(key("file"))));

        assert_eq!(menu_bar.state(), &State::Inactive);
        assert_eq!(menu_bar.context().active_menu, None);
    }

    #[test]
    fn connect_attrs_snapshot_all_parts_and_branches() {
        insta::assert_snapshot!(
            "menu_bar_root_default",
            snapshot_attrs(
                &service(Props::new().id("menu-bar"))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );

        let mut menu_bar = service(
            Props::new()
                .id("menu-bar")
                .orientation(Orientation::Vertical),
        );

        drop(menu_bar.send(Event::ActivateMenu(key("file"))));

        let api = menu_bar.connect(&|_| {});

        insta::assert_snapshot!("menu_bar_root_vertical", snapshot_attrs(&api.root_attrs()));
        insta::assert_snapshot!(
            "menu_bar_menu",
            snapshot_attrs(&api.menu_attrs(&key("file")))
        );
        insta::assert_snapshot!(
            "menu_bar_trigger_active",
            snapshot_attrs(&api.menu_trigger_attrs(&key("file")))
        );
        insta::assert_snapshot!(
            "menu_bar_positioner",
            snapshot_attrs(&api.menu_positioner_attrs(&key("file")))
        );
        insta::assert_snapshot!(
            "menu_bar_content",
            snapshot_attrs(&api.menu_content_attrs(&key("file")))
        );

        assert_eq!(
            api.part_attrs(Part::MenuContent { key: key("file") })
                .get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("menu-bar-trigger-file")
        );
    }
}
