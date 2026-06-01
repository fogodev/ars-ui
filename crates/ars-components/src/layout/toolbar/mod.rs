//! Toolbar layout component machine.
//!
//! `Toolbar` owns DOM-free item registration, disabled item skipping,
//! orientation-aware keyboard navigation, and roving tabindex attributes.
//! Framework adapters resolve the focused item index to a live element handle.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Direction, Env,
    HtmlAttr, KeyboardKey, NoEffect, Orientation, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

/// Toolbar is always idle; focus tracking is kept in [`Context`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum State {
    /// The idle state.
    #[default]
    Idle,
}

/// Events accepted by the `Toolbar` machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Focus a specific item by zero-based rendered index.
    FocusItem(usize),

    /// Move focus to the next enabled item, wrapping at the end.
    FocusNext,

    /// Move focus to the previous enabled item, wrapping at the start.
    FocusPrev,

    /// Move focus to the first enabled item.
    FocusFirst,

    /// Move focus to the last enabled item.
    FocusLast,

    /// Focus entered the toolbar.
    Focus {
        /// Whether focus entry was caused by keyboard interaction.
        is_keyboard: bool,
    },

    /// Focus left the toolbar.
    Blur,

    /// Replace the rendered item registry.
    SetItems {
        /// Number of rendered toolbar items.
        count: usize,

        /// Disabled item indices in the current rendered order.
        disabled_items: Vec<usize>,
    },

    /// Synchronize output-affecting props stored in context.
    SetProps {
        /// Latest props snapshot.
        props: Props,
    },
}

/// Runtime context for the `Toolbar` state machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    /// Index of the currently focused item, which receives `tabindex="0"`.
    pub focused_index: Option<usize>,

    /// Toolbar orientation for ARIA and arrow-key handling.
    pub orientation: Orientation,

    /// Text direction for RTL-aware horizontal navigation.
    pub dir: Direction,

    /// Whether the entire toolbar is disabled.
    pub disabled: bool,

    /// Number of rendered toolbar items.
    pub item_count: usize,

    /// Disabled item indices in rendered order.
    pub disabled_items: Vec<usize>,

    /// Component identifiers for derived part IDs.
    pub ids: ComponentIds,
}

/// Props for the `Toolbar` component.
#[derive(Clone, Debug, PartialEq, Eq, ars_core::HasId)]
pub struct Props {
    /// The id of the toolbar.
    pub id: String,

    /// Toolbar orientation.
    pub orientation: Orientation,

    /// Text direction for RTL support.
    pub dir: Direction,

    /// Accessible label for the toolbar.
    pub aria_label: Option<String>,

    /// Whether the toolbar and all child items are disabled.
    pub disabled: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            orientation: Orientation::Horizontal,
            dir: Direction::Ltr,
            aria_label: None,
            disabled: false,
        }
    }
}

impl Props {
    /// Returns fresh toolbar props with documented defaults.
    ///
    /// # Examples
    ///
    /// ```
    /// use ars_components::layout::toolbar::{Machine, Messages, Props};
    /// use ars_core::{Env, HtmlAttr, Service};
    ///
    /// let service = Service::<Machine>::new(
    ///     Props::new().id("formatting").aria_label("Formatting tools"),
    ///     &Env::default(),
    ///     &Messages::default(),
    /// );
    /// let attrs = service.connect(&|_| {}).root_attrs();
    ///
    /// assert_eq!(attrs.get(&HtmlAttr::Role), Some("toolbar"));
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

    /// Sets [`orientation`](Self::orientation).
    #[must_use]
    pub const fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Sets [`dir`](Self::dir).
    #[must_use]
    pub const fn dir(mut self, dir: Direction) -> Self {
        self.dir = dir;
        self
    }

    /// Sets [`aria_label`](Self::aria_label).
    #[must_use]
    pub fn aria_label(mut self, aria_label: impl Into<String>) -> Self {
        self.aria_label = Some(aria_label.into());
        self
    }

    /// Sets [`disabled`](Self::disabled).
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// This component has no translatable strings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// Machine for the `Toolbar` component.
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
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            State::Idle,
            Context {
                focused_index: None,
                orientation: props.orientation,
                dir: props.dir,
                disabled: props.disabled,
                item_count: 0,
                disabled_items: Vec::new(),
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::SetItems {
                count,
                disabled_items,
            } => set_items_plan(ctx, *count, disabled_items),

            Event::SetProps { props } => {
                let props = props.clone();

                if ctx.orientation == props.orientation
                    && ctx.dir == props.dir
                    && ctx.disabled == props.disabled
                {
                    return None;
                }

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.orientation = props.orientation;
                    ctx.dir = props.dir;
                    ctx.disabled = props.disabled;

                    if ctx.disabled {
                        ctx.focused_index = None;
                    }
                }))
            }

            _ if ctx.disabled => None,

            Event::FocusItem(index) => {
                let index = *index;

                if !can_focus_index(ctx.item_count, &ctx.disabled_items, index)
                    || ctx.focused_index == Some(index)
                {
                    return None;
                }

                Some(focus_plan(Some(index)))
            }

            Event::FocusNext => {
                let target = next_enabled_index(
                    ctx.focused_index.unwrap_or(0),
                    ctx.item_count,
                    &ctx.disabled_items,
                    true,
                );

                focus_if_changed(ctx, target)
            }

            Event::FocusPrev => {
                let target = next_enabled_index(
                    ctx.focused_index.unwrap_or(0),
                    ctx.item_count,
                    &ctx.disabled_items,
                    false,
                );

                focus_if_changed(ctx, target)
            }

            Event::FocusFirst => focus_if_changed(
                ctx,
                first_enabled_index(ctx.item_count, &ctx.disabled_items),
            ),

            Event::FocusLast => {
                focus_if_changed(ctx, last_enabled_index(ctx.item_count, &ctx.disabled_items))
            }

            Event::Focus { is_keyboard: _ } => {
                if ctx.focused_index.is_some() {
                    return None;
                }

                focus_if_changed(
                    ctx,
                    first_enabled_index(ctx.item_count, &ctx.disabled_items),
                )
            }

            Event::Blur => None,
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
        if old.orientation == new.orientation && old.dir == new.dir && old.disabled == new.disabled
        {
            Vec::new()
        } else {
            alloc::vec![Event::SetProps { props: new.clone() }]
        }
    }
}

fn set_items_plan(
    ctx: &Context,
    count: usize,
    disabled_items: &[usize],
) -> Option<TransitionPlan<Machine>> {
    let disabled_items = normalized_disabled_items(count, disabled_items);

    let focused_index = if ctx.disabled {
        None
    } else {
        ctx.focused_index
            .filter(|index| can_focus_index(count, &disabled_items, *index))
            .or_else(|| first_enabled_index(count, &disabled_items))
    };

    if ctx.item_count == count
        && ctx.disabled_items == disabled_items
        && ctx.focused_index == focused_index
    {
        return None;
    }

    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.item_count = count;
        ctx.disabled_items = disabled_items;
        ctx.focused_index = focused_index;
    }))
}

fn normalized_disabled_items(count: usize, disabled_items: &[usize]) -> Vec<usize> {
    let mut normalized = disabled_items
        .iter()
        .copied()
        .filter(|index| *index < count)
        .collect::<Vec<_>>();

    normalized.sort_unstable();
    normalized.dedup();

    normalized
}

fn can_focus_index(count: usize, disabled: &[usize], index: usize) -> bool {
    index < count && !disabled.contains(&index)
}

fn focus_if_changed(ctx: &Context, target: Option<usize>) -> Option<TransitionPlan<Machine>> {
    (ctx.focused_index != target).then(|| focus_plan(target))
}

fn focus_plan(target: Option<usize>) -> TransitionPlan<Machine> {
    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.focused_index = target;
    })
}

fn next_enabled_index(
    current: usize,
    count: usize,
    disabled: &[usize],
    forward: bool,
) -> Option<usize> {
    if count == 0 {
        return None;
    }

    for step in 1..=count {
        let index = if forward {
            (current + step) % count
        } else {
            (current + count - (step % count)) % count
        };

        if !disabled.contains(&index) {
            return Some(index);
        }
    }

    None
}

fn first_enabled_index(count: usize, disabled: &[usize]) -> Option<usize> {
    (0..count).find(|index| !disabled.contains(index))
}

fn last_enabled_index(count: usize, disabled: &[usize]) -> Option<usize> {
    (0..count).rev().find(|index| !disabled.contains(index))
}

/// Structural parts exposed by the `Toolbar` connect API.
#[derive(ComponentPart)]
#[scope = "toolbar"]
pub enum Part {
    /// The root toolbar container.
    Root,

    /// A toolbar item by rendered index.
    Item {
        /// Zero-based rendered item index.
        index: usize,
    },

    /// A visual and semantic separator between toolbar groups.
    Separator,
}

/// API for producing `Toolbar` attributes and dispatching typed events.
#[derive(Clone)]
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
            .finish_non_exhaustive()
    }
}

impl PartialEq for Api<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state && self.ctx == other.ctx && self.props == other.props
    }
}

impl Eq for Api<'_> {}

impl Api<'_> {
    /// Returns root toolbar attributes.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "toolbar")
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                orientation_attr(self.ctx.orientation),
            );

        if let Some(label) = &self.props.aria_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.as_str());
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs.set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());

        attrs
    }

    /// Returns attributes for the toolbar item at `index`.
    #[must_use]
    pub fn item_attrs(&self, index: usize) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Item { index: 0 }.data_attrs();

        let disabled = self.ctx.disabled || self.ctx.disabled_items.contains(&index);
        let focused = self.ctx.focused_index == Some(index);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::TabIndex, if focused { "0" } else { "-1" });

        if disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if focused {
            attrs.set_bool(HtmlAttr::Data("ars-focused"), true);
        }

        attrs
    }

    /// Returns separator attributes.
    #[must_use]
    pub fn separator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Separator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "separator")
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                match self.ctx.orientation {
                    Orientation::Horizontal => "vertical",
                    Orientation::Vertical => "horizontal",
                },
            );

        attrs
    }

    /// Dispatches roving-focus navigation for a root keydown event.
    pub fn on_root_keydown(&self, data: &KeyboardEventData) {
        let horizontal = self.ctx.orientation == Orientation::Horizontal;
        let rtl = self.ctx.dir == Direction::Rtl;

        match data.key {
            KeyboardKey::ArrowRight if horizontal && rtl => (self.send)(Event::FocusPrev),
            KeyboardKey::ArrowRight if horizontal => (self.send)(Event::FocusNext),
            KeyboardKey::ArrowLeft if horizontal && rtl => (self.send)(Event::FocusNext),
            KeyboardKey::ArrowLeft if horizontal => (self.send)(Event::FocusPrev),
            KeyboardKey::ArrowDown if !horizontal => (self.send)(Event::FocusNext),
            KeyboardKey::ArrowUp if !horizontal => (self.send)(Event::FocusPrev),
            KeyboardKey::Home => (self.send)(Event::FocusFirst),
            KeyboardKey::End => (self.send)(Event::FocusLast),
            _ => {}
        }
    }

    /// Dispatches focus for a rendered toolbar item.
    pub fn on_item_focus(&self, index: usize, _is_keyboard: bool) {
        (self.send)(Event::FocusItem(index));
    }

    /// Dispatches toolbar root blur.
    pub fn on_root_blur(&self) {
        (self.send)(Event::Blur);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Item { index } => self.item_attrs(index),
            Part::Separator => self.separator_attrs(),
        }
    }
}

const fn orientation_attr(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::String, vec};

    use ars_core::{
        AriaAttr, AttrMap, ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Orientation, Service,
    };
    use ars_interactions::KeyboardEventData;
    use insta::assert_snapshot;

    use super::{Event, Machine, Messages, Part, Props};

    fn service(props: Props) -> Service<Machine> {
        let props = if props.id.is_empty() {
            props.id("toolbar")
        } else {
            props
        };

        let mut service = Service::<Machine>::new(props, &Env::default(), &Messages);

        drop(service.send(Event::SetItems {
            count: 4,
            disabled_items: vec![2],
        }));

        service
    }

    fn keyboard(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character: None,
            code: String::new(),
            ctrl_key: false,
            shift_key: false,
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
    fn root_attrs_emit_toolbar_role_orientation_label_disabled_and_dir() {
        let service = service(
            Props::new()
                .id("tools")
                .aria_label("Formatting tools")
                .orientation(Orientation::Vertical)
                .dir(Direction::Rtl)
                .disabled(true),
        );

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("toolbar"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("vertical")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Formatting tools")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
    }

    #[test]
    fn roving_tabindex_tracks_focused_item_and_skips_disabled_items() {
        let mut service = service(Props::new().id("tools"));

        assert_eq!(
            service
                .connect(&|_| {})
                .item_attrs(0)
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );

        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_index, Some(1));

        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_index, Some(3));

        let api = service.connect(&|_| {});

        assert_eq!(api.item_attrs(1).get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(api.item_attrs(2).get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(
            api.item_attrs(2).get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
        assert_eq!(api.item_attrs(3).get(&HtmlAttr::TabIndex), Some("0"));
    }

    #[test]
    fn set_items_seeds_first_enabled_roving_target() {
        let mut service =
            Service::<Machine>::new(Props::new().id("toolbar"), &Env::default(), &Messages);

        drop(service.send(Event::SetItems {
            count: 4,
            disabled_items: vec![0, 2],
        }));

        assert_eq!(service.context().focused_index, Some(1));

        let api = service.connect(&|_| {});

        assert_eq!(api.item_attrs(0).get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(api.item_attrs(1).get(&HtmlAttr::TabIndex), Some("0"));
    }

    #[test]
    fn keyboard_navigation_respects_orientation_and_rtl() {
        let captured = alloc::rc::Rc::new(core::cell::RefCell::new(Vec::new()));

        let toolbar = service(Props::new().dir(Direction::Rtl));

        let send = {
            let captured = alloc::rc::Rc::clone(&captured);
            move |event| captured.borrow_mut().push(event)
        };

        toolbar
            .connect(&send)
            .on_root_keydown(&keyboard(KeyboardKey::ArrowRight));
        toolbar
            .connect(&send)
            .on_root_keydown(&keyboard(KeyboardKey::ArrowLeft));

        assert_eq!(
            captured.borrow().as_slice(),
            &[Event::FocusPrev, Event::FocusNext]
        );

        captured.borrow_mut().clear();

        service(Props::new().orientation(Orientation::Vertical))
            .connect(&send)
            .on_root_keydown(&keyboard(KeyboardKey::ArrowDown));

        service(Props::new().orientation(Orientation::Vertical))
            .connect(&send)
            .on_root_keydown(&keyboard(KeyboardKey::ArrowUp));

        assert_eq!(
            captured.borrow().as_slice(),
            &[Event::FocusNext, Event::FocusPrev]
        );
    }

    #[test]
    fn home_end_focus_first_and_last_enabled_items() {
        let mut service = service(Props::new());

        drop(service.send(Event::FocusLast));

        assert_eq!(service.context().focused_index, Some(3));

        drop(service.send(Event::FocusFirst));

        assert_eq!(service.context().focused_index, Some(0));
    }

    #[test]
    fn previous_focus_wraps_and_skips_disabled_items() {
        let mut service = service(Props::new());

        drop(service.send(Event::FocusFirst));
        drop(service.send(Event::FocusPrev));

        assert_eq!(service.context().focused_index, Some(3));

        drop(service.send(Event::FocusPrev));

        assert_eq!(service.context().focused_index, Some(1));
    }

    #[test]
    fn focus_entry_targets_first_enabled_item_and_blur_preserves_focus() {
        let mut service = service(Props::new());

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert_eq!(service.context().focused_index, Some(0));

        drop(service.send(Event::Focus { is_keyboard: false }));

        assert_eq!(service.context().focused_index, Some(0));

        drop(service.send(Event::Blur));

        assert_eq!(service.context().focused_index, Some(0));
    }

    #[test]
    fn zero_all_disabled_and_out_of_range_focus_requests_are_ignored() {
        let mut service =
            Service::<Machine>::new(Props::new().id("toolbar"), &Env::default(), &Messages);

        drop(service.send(Event::FocusNext));

        assert_eq!(service.context().focused_index, None);

        drop(service.send(Event::SetItems {
            count: 2,
            disabled_items: vec![0, 1],
        }));
        drop(service.send(Event::FocusFirst));

        assert_eq!(service.context().focused_index, None);

        drop(service.send(Event::FocusItem(4)));

        assert_eq!(service.context().focused_index, None);
    }

    #[test]
    fn set_items_normalizes_disabled_items_and_prunes_invalid_focus() {
        let mut service = service(Props::new());

        drop(service.send(Event::FocusLast));

        assert_eq!(service.context().focused_index, Some(3));

        drop(service.send(Event::SetItems {
            count: 3,
            disabled_items: vec![1, 1, 9],
        }));

        assert_eq!(service.context().item_count, 3);
        assert_eq!(service.context().disabled_items, vec![1]);
        assert_eq!(service.context().focused_index, Some(0));

        let before = service.context().clone();

        drop(service.send(Event::SetItems {
            count: 3,
            disabled_items: vec![1],
        }));

        assert_eq!(*service.context(), before);
    }

    #[test]
    fn disabled_toolbar_suppresses_navigation() {
        let mut service = service(Props::new().disabled(true));

        drop(service.send(Event::FocusFirst));

        assert_eq!(service.context().focused_index, None);
    }

    #[test]
    fn set_props_syncs_output_affecting_context() {
        let mut service = service(Props::new().id("toolbar"));

        drop(service.send(Event::FocusFirst));
        drop(
            service.set_props(
                Props::new()
                    .id("toolbar")
                    .orientation(Orientation::Vertical)
                    .dir(Direction::Rtl)
                    .disabled(true),
            ),
        );

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(service.context().orientation, Orientation::Vertical);
        assert_eq!(service.context().dir, Direction::Rtl);
        assert!(service.context().disabled);
        assert_eq!(service.context().focused_index, None);
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("vertical")
        );
        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
    }

    #[test]
    fn unchanged_output_props_do_not_emit_sync_events() {
        let mut service = service(Props::new().id("toolbar"));

        let before = service.context().clone();

        drop(service.set_props(Props::new().id("toolbar")));

        assert_eq!(*service.context(), before);

        drop(
            service.set_props(
                Props::new()
                    .id("toolbar")
                    .orientation(Orientation::Vertical)
                    .dir(Direction::Rtl),
            ),
        );

        assert_eq!(service.context().orientation, Orientation::Vertical);
        assert_eq!(service.context().dir, Direction::Rtl);
        assert!(!service.context().disabled);
    }

    #[test]
    fn separator_attrs_use_perpendicular_orientation() {
        let service = service(Props::new().orientation(Orientation::Horizontal));

        let attrs = service.connect(&|_| {}).separator_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("separator"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("vertical")
        );
    }

    #[test]
    fn vertical_separator_attrs_use_horizontal_orientation() {
        let service = service(Props::new().orientation(Orientation::Vertical));

        let attrs = service.connect(&|_| {}).separator_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("horizontal")
        );
    }

    #[test]
    fn typed_handlers_dispatch_focus_and_blur_events() {
        let captured = alloc::rc::Rc::new(core::cell::RefCell::new(Vec::new()));
        let send = {
            let captured = alloc::rc::Rc::clone(&captured);
            move |event| captured.borrow_mut().push(event)
        };

        let service = service(Props::new());

        let api = service.connect(&send);

        api.on_item_focus(1, true);
        api.on_root_blur();

        assert_eq!(
            captured.borrow().as_slice(),
            &[Event::FocusItem(1), Event::Blur]
        );
    }

    #[test]
    fn keydown_handler_dispatches_ltr_home_end_and_ignores_unhandled_keys() {
        let captured = alloc::rc::Rc::new(core::cell::RefCell::new(Vec::new()));
        let send = {
            let captured = alloc::rc::Rc::clone(&captured);
            move |event| captured.borrow_mut().push(event)
        };

        let service = service(Props::new());

        let api = service.connect(&send);

        api.on_root_keydown(&keyboard(KeyboardKey::ArrowRight));
        api.on_root_keydown(&keyboard(KeyboardKey::ArrowLeft));
        api.on_root_keydown(&keyboard(KeyboardKey::Home));
        api.on_root_keydown(&keyboard(KeyboardKey::End));
        api.on_root_keydown(&keyboard(KeyboardKey::Tab));

        assert_eq!(
            captured.borrow().as_slice(),
            &[
                Event::FocusNext,
                Event::FocusPrev,
                Event::FocusFirst,
                Event::FocusLast,
            ]
        );
    }

    #[test]
    fn part_attrs_delegate_to_specific_attr_methods() {
        let service = service(Props::new());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Item { index: 1 }), api.item_attrs(1));
        assert_eq!(api.part_attrs(Part::Separator), api.separator_attrs());
    }

    #[test]
    fn api_debug_and_equality_reflect_state_context_and_props() {
        let toolbar = service(Props::new().aria_label("Tools"));

        let api = toolbar.connect(&|_| {});

        let same = toolbar.connect(&|_| {});

        let other_service = service(Props::new().orientation(Orientation::Vertical));

        let other = other_service.connect(&|_| {});

        assert_eq!(api, same);
        assert_ne!(api, other);
        assert!(format!("{api:?}").contains("Api"));
    }

    #[test]
    fn toolbar_root_snapshot() {
        assert_snapshot!(
            "toolbar_root",
            snapshot_attrs(
                &service(Props::new().id("toolbar").aria_label("Formatting tools"))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
    }

    #[test]
    fn toolbar_item_focused_snapshot() {
        let mut service = service(Props::new());

        drop(service.send(Event::FocusFirst));

        assert_snapshot!(
            "toolbar_item_focused",
            snapshot_attrs(&service.connect(&|_| {}).item_attrs(0))
        );
    }

    #[test]
    fn toolbar_item_disabled_snapshot() {
        assert_snapshot!(
            "toolbar_item_disabled",
            snapshot_attrs(&service(Props::new()).connect(&|_| {}).item_attrs(2))
        );
    }

    #[test]
    fn toolbar_root_disabled_snapshot() {
        assert_snapshot!(
            "toolbar_root_disabled",
            snapshot_attrs(
                &service(Props::new().disabled(true))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
    }

    #[test]
    fn toolbar_separator_snapshot() {
        assert_snapshot!(
            "toolbar_separator",
            snapshot_attrs(&service(Props::new()).connect(&|_| {}).separator_attrs())
        );
    }

    #[test]
    fn toolbar_vertical_root_snapshot() {
        assert_snapshot!(
            "toolbar_vertical_root",
            snapshot_attrs(
                &service(Props::new().orientation(Orientation::Vertical))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
    }

    #[test]
    fn toolbar_rtl_root_snapshot() {
        assert_snapshot!(
            "toolbar_rtl_root",
            snapshot_attrs(
                &service(Props::new().dir(Direction::Rtl))
                    .connect(&|_| {})
                    .root_attrs()
            )
        );
    }
}
