//! TagGroup data-display component machine.
//!
//! Owns the tag collection, selection state, roving-focus intent, removal
//! state, and ARIA/data attributes. Adapters perform live DOM focus and
//! announce removal effects through their platform layer.

use alloc::{collections::BTreeSet, format, string::String, sync::Arc, vec, vec::Vec};
use core::fmt::{self, Debug};

use ars_collections::{Collection, CollectionItem, Key, StaticCollection, selection};
use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentMessages, ComponentPart, ConnectApi, Direction, Env,
    HasId, HtmlAttr, Locale, MessageFn, TransitionPlan, no_cleanup,
};
use ars_interactions::{KeyboardEventData, KeyboardKey};

type RemoveLabelFn = dyn Fn(&Locale) -> String + Send + Sync;
type RemovedAnnouncementFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Definition for a tag item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    /// Unique identifier for this tag.
    pub key: Key,

    /// Display label text.
    pub label: String,

    /// Whether this individual tag is disabled.
    pub disabled: bool,
}

impl CollectionItem for Tag {
    fn key(&self) -> &Key {
        &self.key
    }

    fn text_value(&self) -> &str {
        &self.label
    }
}

/// Props for the `TagGroup` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Tag items to display.
    pub items: StaticCollection<Tag>,

    /// Controlled selected keys.
    pub selected_keys: Option<BTreeSet<Key>>,

    /// Default selected keys for uncontrolled mode.
    pub default_selected_keys: BTreeSet<Key>,

    /// Selection mode.
    pub selection_mode: selection::Mode,

    /// Prevents deselecting the final selected tag.
    pub disallow_empty_selection: bool,

    /// Disable the entire tag group.
    pub disabled: bool,

    /// Accessible label for the tag group.
    pub label: Option<String>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            items: StaticCollection::default(),
            selected_keys: None,
            default_selected_keys: BTreeSet::new(),
            selection_mode: selection::Mode::None,
            disallow_empty_selection: false,
            disabled: false,
            label: None,
        }
    }
}

impl Props {
    /// Returns fresh `TagGroup` props with documented defaults.
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

    /// Sets the displayed tag collection.
    #[must_use]
    pub fn items(mut self, items: StaticCollection<Tag>) -> Self {
        self.items = items;
        self
    }

    /// Sets the controlled selected keys.
    #[must_use]
    pub fn selected_keys(mut self, selected_keys: BTreeSet<Key>) -> Self {
        self.selected_keys = Some(selected_keys);
        self
    }

    /// Sets the uncontrolled initial selected keys.
    #[must_use]
    pub fn default_selected_keys(mut self, selected_keys: BTreeSet<Key>) -> Self {
        self.default_selected_keys = selected_keys;
        self
    }

    /// Sets the selection mode.
    #[must_use]
    pub const fn selection_mode(mut self, selection_mode: selection::Mode) -> Self {
        self.selection_mode = selection_mode;
        self
    }

    /// Sets whether the last selected tag can be deselected.
    #[must_use]
    pub const fn disallow_empty_selection(mut self, disallow_empty_selection: bool) -> Self {
        self.disallow_empty_selection = disallow_empty_selection;
        self
    }

    /// Sets group disabled state.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the accessible label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// States for the `TagGroup` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// No tag is focused.
    Idle,

    /// A tag within the group has focus.
    Focused,
}

/// Events accepted by the `TagGroup` machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Focus entered the tag group or moved to a specific tag.
    Focus {
        /// Tag key to focus, or first enabled tag when `None`.
        item: Option<Key>,

        /// Whether focus was keyboard initiated.
        is_keyboard: bool,
    },

    /// Focus left the tag group.
    Blur,

    /// Remove the tag identified by key.
    RemoveTag(Key),

    /// Move focus to the next enabled tag.
    FocusNext,

    /// Move focus to the previous enabled tag.
    FocusPrevious,

    /// Move focus to the first enabled tag.
    FocusFirst,

    /// Move focus to the last enabled tag.
    FocusLast,

    /// Select the given tag.
    SelectTag(Key),

    /// Deselect the given tag.
    DeselectTag(Key),

    /// Toggle selection for the given tag.
    ToggleTag(Key),

    /// Synchronize props mirrored into context.
    SyncProps,
}

/// Context for the `TagGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The list of tag items currently displayed.
    pub items: StaticCollection<Tag>,

    /// Key of the currently focused tag.
    pub focused_key: Option<Key>,

    /// Whether the focus ring should be visible.
    pub focus_visible: bool,

    /// Group disabled state.
    pub disabled: bool,

    /// Selection mode for tags.
    pub selection_mode: selection::Mode,

    /// Currently selected tag keys.
    pub selected_keys: Bindable<BTreeSet<Key>>,

    /// Latest user-requested selected keys for adapter change notification.
    pub requested_selected_keys: Option<BTreeSet<Key>>,

    /// Keys removed by the core machine and hidden from prop-driven item sync.
    pub removed_keys: BTreeSet<Key>,

    /// Latest localized removal announcement for adapters to announce.
    pub removed_announcement: Option<String>,

    /// Active locale for message formatting.
    pub locale: Locale,

    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

/// Messages for the `TagGroup` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for tag remove buttons.
    pub remove_label: MessageFn<RemoveLabelFn>,

    /// Announcement emitted when a tag is removed.
    pub removed_announcement: MessageFn<RemovedAnnouncementFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            remove_label: MessageFn::static_str("Remove"),
            removed_announcement: MessageFn::new(Arc::new(|label: &str, _locale: &Locale| {
                format!("{label}, removed")
            }) as Arc<RemovedAnnouncementFn>),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed side-effect intents emitted by the `TagGroup` machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Announce that a tag was removed.
    AnnounceRemoved,

    /// Notify adapters that the user requested a selection change.
    SelectionChange,
}

/// Structural parts exposed by the `TagGroup` connect API.
#[derive(ComponentPart)]
#[scope = "tag-group"]
pub enum Part {
    /// The root tag group element.
    Root,

    /// The visible label element.
    Label,

    /// The wrapper row containing tags.
    List,

    /// One tag item.
    Tag {
        /// Tag key.
        key: Key,
    },

    /// Remove control for one tag item.
    TagRemove {
        /// Tag key.
        key: Key,
    },
}

/// Machine for the `TagGroup` component.
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
                items: props.items.clone(),
                focused_key: None,
                focus_visible: false,
                disabled: props.disabled,
                selection_mode: props.selection_mode,
                selected_keys: match &props.selected_keys {
                    Some(keys) => Bindable::controlled(filter_selection(
                        keys,
                        &props.items,
                        props.selection_mode,
                    )),
                    None => Bindable::uncontrolled(filter_selection(
                        &props.default_selected_keys,
                        &props.items,
                        props.selection_mode,
                    )),
                },
                requested_selected_keys: None,
                removed_keys: BTreeSet::new(),
                removed_announcement: None,
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        context: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        if let Event::SyncProps = event {
            let removed_keys = context
                .removed_keys
                .iter()
                .filter(|key| props.items.contains_key(key))
                .cloned()
                .collect::<BTreeSet<_>>();
            let items = collection_without_keys(&props.items, &removed_keys);
            let disabled = props.disabled;
            let selection_mode = props.selection_mode;
            let controlled_keys = props
                .selected_keys
                .as_ref()
                .map(|keys| filter_selection(keys, &items, props.selection_mode));
            let focused_key = context
                .focused_key
                .as_ref()
                .filter(|key| !disabled && enabled_tag_in(&items, key).is_some())
                .cloned();
            let next_state = if focused_key.is_some() {
                State::Focused
            } else {
                State::Idle
            };

            return Some(
                TransitionPlan::to(next_state).apply(move |ctx: &mut Context| {
                    ctx.items = items;
                    ctx.removed_keys = removed_keys;
                    ctx.disabled = disabled;
                    ctx.selection_mode = selection_mode;
                    ctx.requested_selected_keys = None;
                    ctx.focused_key = focused_key;
                    ctx.focus_visible = ctx.focused_key.is_some() && ctx.focus_visible;
                    ctx.selected_keys.sync_controlled(controlled_keys.clone());

                    if controlled_keys.is_none() {
                        let selected_keys =
                            filter_selection(ctx.selected_keys.get(), &ctx.items, selection_mode);
                        ctx.selected_keys.set(selected_keys);
                    }
                }),
            );
        }

        if context.disabled {
            return match event {
                Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.focused_key = None;
                    ctx.focus_visible = false;
                })),

                Event::Focus { item, is_keyboard } => {
                    let key = item
                        .as_ref()
                        .filter(|key| context.items.contains_key(key))
                        .cloned();
                    let is_keyboard = *is_keyboard;
                    let target = if key.is_some() {
                        State::Focused
                    } else {
                        State::Idle
                    };

                    Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                        ctx.focused_key = key;
                        ctx.focus_visible = is_keyboard && ctx.focused_key.is_some();
                    }))
                }

                _ => None,
            };
        }

        match event {
            Event::Focus { item, is_keyboard } => {
                let key = match item {
                    Some(key) => {
                        enabled_tag(context, key)?;
                        Some(key.clone())
                    }
                    None => first_enabled_key(&context.items),
                };

                let is_keyboard = *is_keyboard;

                let target = if key.is_some() {
                    State::Focused
                } else {
                    State::Idle
                };

                Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    ctx.focused_key = key;
                    ctx.focus_visible = is_keyboard;
                }))
            }

            Event::Blur => Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                ctx.focused_key = None;
                ctx.focus_visible = false;
            })),

            Event::RemoveTag(key) => {
                let key = key.clone();
                let item = enabled_tag(context, &key)?;

                let label = item.label.clone();

                let next_key = next_focus_after_removal(&context.items, &key);

                let selection_changed = selected_keys_source(context).contains(&key);
                let mut plan = TransitionPlan::to(State::Focused)
                    .apply(move |ctx: &mut Context| {
                        ctx.items = collection_without(&ctx.items, &key);
                        ctx.removed_keys.insert(key.clone());
                        let mut selected = selected_keys_source(ctx).clone();

                        if selected.remove(&key) {
                            ctx.requested_selected_keys = Some(selected.clone());
                        }

                        sync_selection(&mut ctx.selected_keys, selected);

                        ctx.focused_key = next_key;
                        ctx.focus_visible = true;
                        ctx.removed_announcement =
                            Some((ctx.messages.removed_announcement)(&label, &ctx.locale));
                    })
                    .with_named_effect(Effect::AnnounceRemoved, |_ctx, _props, _send| no_cleanup());

                if selection_changed {
                    plan = plan
                        .with_named_effect(Effect::SelectionChange, |_ctx, _props, _send| {
                            no_cleanup()
                        });
                }

                Some(plan)
            }

            Event::FocusNext => {
                let current = context.focused_key.as_ref()?;
                let next =
                    next_enabled_key(&context.items, current).unwrap_or_else(|| current.clone());

                Some(focus_plan(next))
            }

            Event::FocusPrevious => {
                let current = context.focused_key.as_ref()?;
                let previous = previous_enabled_key(&context.items, current)
                    .unwrap_or_else(|| current.clone());

                Some(focus_plan(previous))
            }

            Event::FocusFirst => first_enabled_key(&context.items).map(focus_plan),

            Event::FocusLast => last_enabled_key(&context.items).map(focus_plan),

            Event::SelectTag(key) => {
                let key = key.clone();

                if enabled_tag(context, &key).is_none()
                    || context.selection_mode == selection::Mode::None
                {
                    return None;
                }

                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        let mut selected = selected_keys_source(ctx).clone();

                        match ctx.selection_mode {
                            selection::Mode::None => {}

                            selection::Mode::Single => {
                                selected.clear();
                                selected.insert(key);
                            }

                            selection::Mode::Multiple => {
                                selected.insert(key);
                            }
                        }

                        ctx.requested_selected_keys = Some(selected.clone());
                        sync_selection(&mut ctx.selected_keys, selected);
                    })
                    .with_named_effect(Effect::SelectionChange, |_ctx, _props, _send| no_cleanup()),
                )
            }

            Event::DeselectTag(key) => {
                let key = key.clone();
                let selected_source = selected_keys_source(context);

                if context.selection_mode == selection::Mode::None
                    || enabled_tag(context, &key).is_none()
                    || !selected_source.contains(&key)
                    || (props.disallow_empty_selection && selected_source.len() <= 1)
                {
                    return None;
                }

                Some(
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        let mut selected = selected_keys_source(ctx).clone();

                        selected.remove(&key);

                        ctx.requested_selected_keys = Some(selected.clone());
                        sync_selection(&mut ctx.selected_keys, selected);
                    })
                    .with_named_effect(Effect::SelectionChange, |_ctx, _props, _send| no_cleanup()),
                )
            }

            Event::ToggleTag(key) => {
                if selected_keys_source(context).contains(key) {
                    Self::transition(_state, &Event::DeselectTag(key.clone()), context, props)
                } else {
                    Self::transition(_state, &Event::SelectTag(key.clone()), context, props)
                }
            }

            Event::SyncProps => unreachable!("SyncProps handled before interactivity guards"),
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        context: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            context,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old.items != new.items
            || old.selected_keys != new.selected_keys
            || old.default_selected_keys != new.default_selected_keys
            || old.selection_mode != new.selection_mode
            || old.disabled != new.disabled
            || old.id != new.id
            || old.label != new.label
        {
            vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// API for the `TagGroup` component.
pub struct Api<'a> {
    state: &'a State,
    context: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("tag_group::Api")
            .field("state", self.state)
            .field("context", self.context)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs
            .set(HtmlAttr::Id, &self.props.id)
            .set(HtmlAttr::Role, "grid")
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "false")
            .set(HtmlAttr::Aria(AriaAttr::Relevant), "additions removals")
            .set(
                HtmlAttr::Data("ars-state"),
                match self.state {
                    State::Idle => "idle",
                    State::Focused => "focused",
                },
            );

        if let Some(label) = &self.props.label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }

        if self.context.selection_mode == selection::Mode::Multiple {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }

        if self.context.disabled {
            attrs
                .set_bool(HtmlAttr::Data("ars-disabled"), true)
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if !self.context.disabled
            && self.context.focused_key.is_none()
            && *self.state == State::Focused
        {
            attrs.set(HtmlAttr::TabIndex, "0");

            if self.context.focus_visible {
                attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
            }
        }

        attrs
    }

    /// Returns attributes for the visible label.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Label);

        attrs.set(HtmlAttr::Id, format!("{}-label", self.props.id));

        attrs
    }

    /// Returns attributes for the list row wrapper.
    #[must_use]
    pub fn list_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::List);

        attrs.set(HtmlAttr::Role, "row");

        if self.props.label.is_some() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                format!("{}-label", self.props.id),
            );
        }

        attrs
    }

    /// Returns attributes for a tag item.
    #[must_use]
    pub fn tag_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::Tag {
            key: Key::default(),
        });

        let item = self
            .context
            .items
            .get(key)
            .and_then(|node| node.value.as_ref());
        if item.is_none() {
            attrs
                .set(HtmlAttr::Data("ars-key"), key)
                .set_bool(HtmlAttr::Hidden, true)
                .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
            return attrs;
        }

        let is_disabled = self.context.disabled || item.is_some_and(|tag| tag.disabled);
        let is_focused = self.context.focused_key.as_ref() == Some(key);
        let is_selected = self.context.selected_keys.get().contains(key);

        let tabindex = if !self.context.disabled
            && (is_focused
                || (self.context.focused_key.is_none()
                    && first_enabled_key(&self.context.items).as_ref() == Some(key)))
        {
            "0"
        } else {
            "-1"
        };

        attrs
            .set(HtmlAttr::Data("ars-key"), key)
            .set(HtmlAttr::Role, "gridcell")
            .set(HtmlAttr::TabIndex, tabindex);

        if is_disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if is_selected {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Selected), "true")
                .set_bool(HtmlAttr::Data("ars-selected"), true);
        } else if self.context.selection_mode != selection::Mode::None {
            attrs.set(HtmlAttr::Aria(AriaAttr::Selected), "false");
        }

        if is_focused && self.context.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Returns attributes for a tag remove control.
    #[must_use]
    pub fn tag_remove_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::TagRemove {
            key: Key::default(),
        });

        let item = self
            .context
            .items
            .get(key)
            .and_then(|node| node.value.as_ref());
        if item.is_none() {
            attrs
                .set_bool(HtmlAttr::Hidden, true)
                .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
            return attrs;
        }

        let is_disabled = self.context.disabled || item.is_some_and(|tag| tag.disabled);

        attrs
            .set(HtmlAttr::Role, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.context.messages.remove_label)(&self.context.locale),
            )
            .set(HtmlAttr::TabIndex, "-1");

        if is_disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        attrs
    }

    /// Returns the latest localized removal announcement payload.
    #[must_use]
    pub fn removed_announcement(&self) -> Option<&str> {
        self.context.removed_announcement.as_deref()
    }

    /// Returns the latest user-requested selected keys.
    #[must_use]
    pub const fn requested_selected_keys(&self) -> Option<&BTreeSet<Key>> {
        self.context.requested_selected_keys.as_ref()
    }

    /// Returns the current tag collection after core removals and prop sync.
    #[must_use]
    pub const fn items(&self) -> &StaticCollection<Tag> {
        &self.context.items
    }

    /// Dispatches keyboard intent for a focused tag.
    pub fn on_tag_keydown(&self, key: &Key, data: &KeyboardEventData) {
        let is_rtl = Direction::from(self.context.locale.direction()) == Direction::Rtl;

        match data.key {
            KeyboardKey::Delete | KeyboardKey::Backspace => {
                (self.send)(Event::RemoveTag(key.clone()));
            }

            KeyboardKey::ArrowRight if is_rtl => (self.send)(Event::FocusPrevious),

            KeyboardKey::ArrowRight | KeyboardKey::ArrowDown => (self.send)(Event::FocusNext),

            KeyboardKey::ArrowLeft if is_rtl => (self.send)(Event::FocusNext),

            KeyboardKey::ArrowLeft | KeyboardKey::ArrowUp => (self.send)(Event::FocusPrevious),

            KeyboardKey::Home => (self.send)(Event::FocusFirst),

            KeyboardKey::End => (self.send)(Event::FocusLast),

            _ => {}
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::List => self.list_attrs(),
            Part::Tag { key } => self.tag_attrs(&key),
            Part::TagRemove { key } => self.tag_remove_attrs(&key),
        }
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_value), (part_attr, part_value)] = part.data_attrs();

    attrs
        .set(scope_attr, scope_value)
        .set(part_attr, part_value);

    attrs
}

fn enabled_tag<'a>(context: &'a Context, key: &Key) -> Option<&'a Tag> {
    context
        .items
        .get(key)
        .and_then(|node| node.value.as_ref())
        .filter(|tag| !tag.disabled)
}

fn enabled_tag_in<'a>(items: &'a StaticCollection<Tag>, key: &Key) -> Option<&'a Tag> {
    items
        .get(key)
        .and_then(|node| node.value.as_ref())
        .filter(|tag| !tag.disabled)
}

fn first_enabled_key(items: &StaticCollection<Tag>) -> Option<Key> {
    items
        .nodes()
        .filter_map(|node| {
            node.value
                .as_ref()
                .filter(|tag| !tag.disabled)
                .map(|tag| tag.key.clone())
        })
        .next()
}

fn last_enabled_key(items: &StaticCollection<Tag>) -> Option<Key> {
    items
        .nodes()
        .filter_map(|node| {
            node.value
                .as_ref()
                .filter(|tag| !tag.disabled)
                .map(|tag| tag.key.clone())
        })
        .last()
}

fn next_enabled_key(items: &StaticCollection<Tag>, current: &Key) -> Option<Key> {
    let start = items.get(current)?.index + 1;

    items.nodes().skip(start).find_map(|node| {
        node.value
            .as_ref()
            .filter(|tag| !tag.disabled)
            .map(|tag| tag.key.clone())
    })
}

fn previous_enabled_key(items: &StaticCollection<Tag>, current: &Key) -> Option<Key> {
    let end = items.get(current)?.index;

    items
        .nodes()
        .take(end)
        .filter_map(|node| {
            node.value
                .as_ref()
                .filter(|tag| !tag.disabled)
                .map(|tag| tag.key.clone())
        })
        .last()
}

fn next_focus_after_removal(items: &StaticCollection<Tag>, key: &Key) -> Option<Key> {
    let current_index = items.get(key)?.index;

    items
        .nodes()
        .skip(current_index + 1)
        .filter_map(|node| {
            node.value
                .as_ref()
                .filter(|tag| !tag.disabled)
                .map(|tag| tag.key.clone())
        })
        .next()
        .or_else(|| {
            items
                .nodes()
                .take(current_index)
                .filter_map(|node| {
                    node.value
                        .as_ref()
                        .filter(|tag| !tag.disabled)
                        .map(|tag| tag.key.clone())
                })
                .last()
        })
}

fn collection_without(items: &StaticCollection<Tag>, removed: &Key) -> StaticCollection<Tag> {
    items
        .nodes()
        .filter_map(|node| node.value.as_ref())
        .filter(|tag| &tag.key != removed)
        .map(|tag| (tag.key.clone(), tag.label.clone(), tag.clone()))
        .collect()
}

fn collection_without_keys(
    items: &StaticCollection<Tag>,
    removed: &BTreeSet<Key>,
) -> StaticCollection<Tag> {
    items
        .nodes()
        .filter_map(|node| node.value.as_ref())
        .filter(|tag| !removed.contains(&tag.key))
        .map(|tag| (tag.key.clone(), tag.label.clone(), tag.clone()))
        .collect()
}

fn focus_plan(key: Key) -> TransitionPlan<Machine> {
    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
        ctx.focused_key = Some(key);
        ctx.focus_visible = true;
    })
}

fn filter_selection(
    keys: &BTreeSet<Key>,
    items: &StaticCollection<Tag>,
    mode: selection::Mode,
) -> BTreeSet<Key> {
    let mut selected = keys
        .iter()
        .filter(|key| enabled_tag_in(items, key).is_some())
        .cloned()
        .collect();

    normalize_selection(&mut selected, mode);

    selected
}

fn normalize_selection(selected: &mut BTreeSet<Key>, mode: selection::Mode) {
    match mode {
        selection::Mode::None => selected.clear(),
        selection::Mode::Multiple => {}
        selection::Mode::Single => {
            let first = selected.pop_first();

            selected.clear();

            if let Some(key) = first {
                selected.insert(key);
            }
        }
    }
}

fn sync_selection(bindable: &mut Bindable<BTreeSet<Key>>, selected: BTreeSet<Key>) {
    bindable.set(selected);
}

fn selected_keys_source(context: &Context) -> &BTreeSet<Key> {
    context
        .requested_selected_keys
        .as_ref()
        .unwrap_or_else(|| context.selected_keys.get())
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, string::String, vec::Vec};
    use core::cell::RefCell;

    use ars_collections::CollectionItem;
    use ars_core::{AttrMap, Env, Machine as _, Service};
    use ars_interactions::{KeyboardEventData, KeyboardKey};
    use insta::assert_snapshot;

    use super::*;

    fn key(value: &str) -> Key {
        Key::str(value)
    }

    fn item(key_value: &str, label: &str, disabled: bool) -> Tag {
        Tag {
            key: key(key_value),
            label: label.into(),
            disabled,
        }
    }

    fn items() -> StaticCollection<Tag> {
        StaticCollection::new([
            (key("alpha"), "Alpha".into(), item("alpha", "Alpha", false)),
            (key("beta"), "Beta".into(), item("beta", "Beta", true)),
            (key("gamma"), "Gamma".into(), item("gamma", "Gamma", false)),
        ])
    }

    fn selected(keys: &[&str]) -> BTreeSet<Key> {
        keys.iter().map(|key_value| key(key_value)).collect()
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    const fn keyboard(key: KeyboardKey) -> KeyboardEventData {
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

    #[test]
    fn attrs_navigation_removal_and_selection() {
        let mut service = service(
            Props::new()
                .id("tags")
                .items(items())
                .label("Selected tags")
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["alpha"]))
                .disallow_empty_selection(true),
        );

        drop(service.send(Event::Focus {
            item: None,
            is_keyboard: true,
        }));

        assert_eq!(service.context().focused_key, Some(key("alpha")));

        drop(service.send(Event::FocusNext));

        assert_eq!(
            service.context().focused_key,
            Some(key("gamma")),
            "disabled beta is skipped"
        );

        drop(service.send(Event::ToggleTag(key("gamma"))));

        assert!(
            service
                .context()
                .selected_keys
                .get()
                .contains(&key("gamma"))
        );

        drop(service.send(Event::DeselectTag(key("gamma"))));

        assert!(
            !service
                .context()
                .selected_keys
                .get()
                .contains(&key("gamma")),
            "disallow_empty_selection still allows removing one key from a multi-selection"
        );

        drop(service.send(Event::DeselectTag(key("alpha"))));

        assert!(
            service
                .context()
                .selected_keys
                .get()
                .contains(&key("alpha")),
            "disallow_empty_selection keeps the last selected tag"
        );

        let api = service.connect(&|_| {});

        let root = api.root_attrs();

        assert_eq!(root.get(&HtmlAttr::Role), Some("grid"));
        assert_eq!(root.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(
            root.get(&HtmlAttr::Aria(AriaAttr::Label)),
            Some("Selected tags")
        );
        assert_eq!(
            root.get(&HtmlAttr::Aria(AriaAttr::Relevant)),
            Some("additions removals")
        );

        let tag = api.tag_attrs(&key("gamma"));

        assert_eq!(tag.get(&HtmlAttr::Role), Some("gridcell"));
        assert_eq!(tag.get(&HtmlAttr::TabIndex), Some("0"));
        assert_eq!(tag.get(&HtmlAttr::Data("ars-focus-visible")), Some("true"));

        let remove = api.tag_remove_attrs(&key("gamma"));

        assert_eq!(remove.get(&HtmlAttr::Role), Some("button"));
        assert_eq!(remove.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Remove"));
        assert_eq!(remove.get(&HtmlAttr::TabIndex), Some("-1"));

        drop(service.send(Event::RemoveTag(key("gamma"))));

        let api = service.connect(&|_| {});

        assert_eq!(service.context().focused_key, Some(key("alpha")));
        assert!(
            !service
                .context()
                .items
                .keys()
                .any(|item_key| item_key == &key("gamma"))
        );
        assert!(
            !api.items().keys().any(|item_key| item_key == &key("gamma")),
            "connect API exposes the current post-removal collection"
        );
        assert_eq!(
            api.tag_attrs(&key("gamma")).get(&HtmlAttr::Hidden),
            Some("true")
        );
        assert_eq!(
            api.tag_attrs(&key("gamma"))
                .get(&HtmlAttr::Aria(AriaAttr::Hidden)),
            Some("true")
        );
        assert_eq!(api.tag_attrs(&key("gamma")).get(&HtmlAttr::Role), None);
        assert_eq!(
            api.tag_remove_attrs(&key("gamma")).get(&HtmlAttr::Hidden),
            Some("true")
        );
    }

    #[test]
    fn prop_changes_sync_items_focus_disabled_and_controlled_selection() {
        let mut service = service(
            Props::new()
                .id("tags")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["alpha"])),
        );

        drop(service.send(Event::Focus {
            item: Some(key("alpha")),
            is_keyboard: true,
        }));
        drop(
            service.set_props(
                Props::new()
                    .id("tags")
                    .items(StaticCollection::new([(
                        key("gamma"),
                        "Gamma".into(),
                        item("gamma", "Gamma", false),
                    )]))
                    .selection_mode(selection::Mode::Single)
                    .selected_keys(selected(&["gamma"]))
                    .disabled(true),
            ),
        );

        assert_eq!(service.state(), &State::Idle);
        assert_eq!(service.context().focused_key, None);
        assert!(!service.context().focus_visible);
        assert!(service.context().disabled);
        assert_eq!(service.context().selection_mode, selection::Mode::Single);
        assert!(service.context().items.contains_key(&key("gamma")));
        assert!(!service.context().items.contains_key(&key("alpha")));
        assert_eq!(service.context().selected_keys.get(), &selected(&["gamma"]));
        assert!(service.context().selected_keys.is_controlled());
    }

    #[test]
    fn on_props_changed_emits_sync_props_for_context_changes() {
        let old = Props::new().id("tags").items(items());

        assert!(Machine::on_props_changed(&old, &old).is_empty());
        assert_eq!(
            Machine::on_props_changed(&old, &old.clone().selected_keys(selected(&["gamma"]))),
            vec![Event::SyncProps]
        );
        assert_eq!(
            Machine::on_props_changed(&old, &old.clone().disabled(true)),
            vec![Event::SyncProps]
        );
        assert_eq!(
            Machine::on_props_changed(&old, &old.clone().label("Updated")),
            vec![Event::SyncProps]
        );
    }

    #[test]
    fn removal_effect_exposes_announcement_payload() {
        let mut service = service(Props::new().id("tags").items(items()));

        let result = service.send(Event::RemoveTag(key("alpha")));

        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::AnnounceRemoved);
        assert_eq!(
            service.context().removed_announcement.as_deref(),
            Some("Alpha, removed")
        );
        assert_eq!(
            service.connect(&|_| {}).removed_announcement(),
            Some("Alpha, removed")
        );
    }

    #[test]
    fn selection_sync_normalizes_mode_and_removal_filters_controlled_keys() {
        let mut single = service(
            Props::new()
                .id("tags")
                .items(items())
                .selection_mode(selection::Mode::Single)
                .selected_keys(selected(&["alpha", "gamma"])),
        );

        assert_eq!(single.context().selected_keys.get(), &selected(&["alpha"]));

        drop(
            single.set_props(
                Props::new()
                    .id("tags")
                    .items(items())
                    .selection_mode(selection::Mode::Multiple)
                    .selected_keys(selected(&["alpha", "gamma"])),
            ),
        );

        assert_eq!(
            single.context().selected_keys.get(),
            &selected(&["alpha", "gamma"])
        );

        drop(single.send(Event::RemoveTag(key("alpha"))));

        assert_eq!(
            single.context().selected_keys.get(),
            &selected(&["alpha", "gamma"]),
            "controlled selection keeps the last prop value until props sync"
        );
        assert_eq!(
            single.context().requested_selected_keys.as_ref(),
            Some(&selected(&["gamma"])),
            "removing a selected tag exposes the requested selection"
        );
        assert!(!single.context().items.contains_key(&key("alpha")));

        drop(
            single.set_props(
                Props::new()
                    .id("tags")
                    .items(items())
                    .selection_mode(selection::Mode::Multiple)
                    .selected_keys(selected(&["gamma"])),
            ),
        );

        assert!(
            !single.context().items.contains_key(&key("alpha")),
            "prop sync preserves tags removed by the machine"
        );
        assert_eq!(
            single.context().selected_keys.get(),
            &selected(&["gamma"]),
            "removed controlled keys stay filtered after prop sync"
        );
        assert!(single.context().removed_keys.contains(&key("alpha")));

        let items_without_alpha = StaticCollection::new([
            (key("beta"), "Beta".into(), item("beta", "Beta", true)),
            (key("gamma"), "Gamma".into(), item("gamma", "Gamma", false)),
        ]);

        drop(
            single.set_props(
                Props::new()
                    .id("tags")
                    .items(items_without_alpha)
                    .selection_mode(selection::Mode::Multiple)
                    .selected_keys(selected(&["alpha", "gamma"])),
            ),
        );

        assert!(
            !single.context().removed_keys.contains(&key("alpha")),
            "parent omission clears the removed-key tombstone"
        );

        drop(
            single.set_props(
                Props::new()
                    .id("tags")
                    .items(items())
                    .selection_mode(selection::Mode::Multiple)
                    .selected_keys(selected(&["alpha", "gamma"])),
            ),
        );

        assert!(
            single.context().items.contains_key(&key("alpha")),
            "later parent re-add restores a previously removed tag"
        );
        assert_eq!(
            single.context().selected_keys.get(),
            &selected(&["alpha", "gamma"])
        );

        let disabled_selection = service(
            Props::new()
                .id("tags")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["beta"])),
        );

        assert!(
            disabled_selection.context().selected_keys.get().is_empty(),
            "initial selection drops disabled tag keys"
        );

        let none = service(
            Props::new()
                .id("tags")
                .items(items())
                .selection_mode(selection::Mode::None)
                .default_selected_keys(selected(&["alpha"])),
        );

        assert!(none.context().selected_keys.get().is_empty());
    }

    #[test]
    fn removing_last_selected_tag_focuses_root_and_emits_selection_change() {
        let mut service = service(
            Props::new()
                .id("tags")
                .items(StaticCollection::new([(
                    key("alpha"),
                    "Alpha".into(),
                    item("alpha", "Alpha", false),
                )]))
                .selection_mode(selection::Mode::Single)
                .default_selected_keys(selected(&["alpha"])),
        );

        let result = service.send(Event::RemoveTag(key("alpha")));

        assert_eq!(service.state(), &State::Focused);
        assert_eq!(service.context().focused_key, None);
        assert_eq!(
            service
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::TabIndex),
            Some("0")
        );
        assert_eq!(
            service.context().requested_selected_keys.as_ref(),
            Some(&BTreeSet::new())
        );
        assert_eq!(result.pending_effects.len(), 2);
        assert!(
            result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::AnnounceRemoved)
        );
        assert!(
            result
                .pending_effects
                .iter()
                .any(|effect| effect.name == Effect::SelectionChange)
        );
    }

    #[test]
    fn controlled_selection_requests_are_exposed() {
        let mut service = service(
            Props::new()
                .id("tags")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .selected_keys(selected(&["alpha"])),
        );

        let result = service.send(Event::SelectTag(key("gamma")));

        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::SelectionChange);
        assert_eq!(
            service.context().requested_selected_keys.as_ref(),
            Some(&selected(&["alpha", "gamma"]))
        );
        assert_eq!(
            service.connect(&|_| {}).requested_selected_keys(),
            Some(&selected(&["alpha", "gamma"]))
        );
        assert_eq!(
            service.context().selected_keys.get(),
            &selected(&["alpha"]),
            "controlled selection remains on the last prop value until the parent accepts the request"
        );

        drop(
            service.set_props(
                Props::new()
                    .id("tags")
                    .items(items())
                    .selection_mode(selection::Mode::Multiple),
            ),
        );

        assert!(!service.context().selected_keys.is_controlled());
        assert_eq!(
            service.context().selected_keys.get(),
            &selected(&["alpha", "gamma"]),
            "releasing controlled selection preserves the pending requested value"
        );

        drop(
            service.set_props(
                Props::new()
                    .id("tags")
                    .items(items())
                    .selection_mode(selection::Mode::Multiple)
                    .selected_keys(selected(&["alpha", "gamma"])),
            ),
        );

        assert_eq!(
            service.context().selected_keys.get(),
            &selected(&["alpha", "gamma"])
        );
    }

    #[test]
    fn controlled_selection_changes_build_from_pending_request() {
        let mut service = service(
            Props::new()
                .id("tags")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .selected_keys(selected(&["alpha"])),
        );

        drop(service.send(Event::RemoveTag(key("alpha"))));

        assert_eq!(
            service.context().requested_selected_keys.as_ref(),
            Some(&BTreeSet::new())
        );

        drop(service.send(Event::SelectTag(key("gamma"))));

        assert_eq!(
            service.context().requested_selected_keys.as_ref(),
            Some(&selected(&["gamma"])),
            "pending controlled removals are the base for the next selection request"
        );
        assert_eq!(
            service.context().selected_keys.get(),
            &selected(&["alpha"]),
            "controlled bindable state remains on the prop value until parent sync"
        );
    }

    #[test]
    fn disabled_group_removes_roving_tab_stop() {
        let service = service(Props::new().id("tags").items(items()).disabled(true));
        let api = service.connect(&|_| {});

        assert_eq!(
            api.tag_attrs(&key("alpha")).get(&HtmlAttr::TabIndex),
            Some("-1")
        );
        assert_eq!(
            api.tag_attrs(&key("gamma")).get(&HtmlAttr::TabIndex),
            Some("-1")
        );
    }

    #[test]
    fn builders_collection_item_part_attrs_and_guards_are_observable() {
        let selected_keys = selected(&["alpha"]);

        let props = Props::new()
            .id("tags")
            .items(items())
            .selected_keys(selected_keys.clone());

        assert_eq!(props.selected_keys, Some(selected_keys));

        let tag = item("delta", "Delta", false);

        assert_eq!(<Tag as CollectionItem>::key(&tag), &key("delta"));
        assert_eq!(<Tag as CollectionItem>::text_value(&tag), "Delta");

        let mut none = service(
            Props::new()
                .id("tags")
                .items(items())
                .default_selected_keys(selected(&["alpha"]))
                .selection_mode(selection::Mode::None),
        );

        drop(none.send(Event::SelectTag(key("alpha"))));
        drop(none.send(Event::DeselectTag(key("alpha"))));

        assert!(none.context().selected_keys.get().is_empty());

        let selectable = service(
            Props::new()
                .id("tags")
                .items(items())
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["alpha"])),
        );

        assert_eq!(
            selectable
                .connect(&|_| {})
                .root_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::MultiSelectable)),
            Some("true"),
            "multiple-selection grids expose the aria multiselectable state"
        );
        assert_eq!(
            selectable
                .connect(&|_| {})
                .tag_attrs(&key("gamma"))
                .get(&HtmlAttr::Aria(AriaAttr::Selected)),
            Some("false"),
            "selectable but unselected tags expose aria-selected=false"
        );

        let mut disabled = service(
            Props::new()
                .id("tags")
                .items(items())
                .selection_mode(selection::Mode::Multiple),
        );

        disabled
            .context_mut()
            .selected_keys
            .sync_controlled(Some(selected(&["beta"])));

        drop(disabled.send(Event::SelectTag(key("beta"))));
        drop(disabled.send(Event::DeselectTag(key("beta"))));
        drop(disabled.send(Event::ToggleTag(key("beta"))));

        assert!(
            disabled
                .context()
                .selected_keys
                .get()
                .contains(&key("beta"))
        );

        let mut removable = service(
            Props::new()
                .id("tags")
                .items(items())
                .selection_mode(selection::Mode::Single)
                .default_selected_keys(selected(&["alpha"])),
        );

        drop(removable.send(Event::DeselectTag(key("alpha"))));

        assert!(removable.context().selected_keys.get().is_empty());

        drop(removable.send(Event::Focus {
            item: Some(key("alpha")),
            is_keyboard: true,
        }));
        drop(removable.send(Event::RemoveTag(key("alpha"))));

        assert_eq!(removable.context().focused_key, Some(key("gamma")));

        let api = removable.connect(&|_| {});

        assert_eq!(
            api.part_attrs(Part::Tag { key: key("gamma") })
                .get(&HtmlAttr::Role),
            Some("gridcell")
        );
    }

    #[test]
    fn navigation_edges_and_disabled_focus_are_observable() {
        let mut disabled = service(Props::new().id("tags").items(items()).disabled(true));

        drop(disabled.send(Event::Focus {
            item: Some(key("alpha")),
            is_keyboard: true,
        }));

        assert_eq!(disabled.context().focused_key, Some(key("alpha")));
        assert!(disabled.context().focus_visible);
        assert_eq!(
            disabled
                .connect(&|_| {})
                .tag_attrs(&key("alpha"))
                .get(&HtmlAttr::Data("ars-focus-visible")),
            Some("true")
        );

        drop(disabled.send(Event::Blur));

        assert_eq!(disabled.context().focused_key, None);
        assert!(!disabled.context().focus_visible);

        let mut service = service(Props::new().id("tags").items(items()));

        let api = service.connect(&|_| {});

        assert_eq!(
            api.tag_attrs(&key("alpha")).get(&HtmlAttr::TabIndex),
            Some("0")
        );
        assert_eq!(
            api.tag_attrs(&key("gamma")).get(&HtmlAttr::TabIndex),
            Some("-1")
        );

        drop(service.send(Event::FocusLast));

        assert_eq!(service.context().focused_key, Some(key("gamma")));

        drop(service.send(Event::Focus {
            item: Some(key("beta")),
            is_keyboard: true,
        }));

        assert_eq!(
            service.context().focused_key,
            Some(key("gamma")),
            "explicit focus requests for disabled tags are no-ops"
        );

        drop(service.send(Event::FocusPrevious));

        assert_eq!(service.context().focused_key, Some(key("alpha")));

        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::ArrowLeft));
        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::ArrowUp));

        assert_eq!(
            events.into_inner(),
            vec![Event::FocusPrevious, Event::FocusPrevious]
        );
    }

    #[test]
    fn keyboard_helper_emits_remove_for_focused_tag() {
        let service = service(
            Props::new()
                .id("tags")
                .items(items())
                .selection_mode(selection::Mode::Single),
        );

        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::Delete));
        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::Backspace));
        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::ArrowRight));
        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::Home));
        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::End));

        assert_eq!(
            events.into_inner(),
            vec![
                Event::RemoveTag(key("alpha")),
                Event::RemoveTag(key("alpha")),
                Event::FocusNext,
                Event::FocusFirst,
                Event::FocusLast,
            ]
        );
    }

    #[test]
    fn keyboard_helper_swaps_horizontal_arrows_for_rtl_locales() {
        let env = Env {
            locale: Locale::parse("ar").expect("ar is a valid RTL locale"),
            ..Env::default()
        };
        let service = Service::<Machine>::new(
            Props::new().id("tags").items(items()),
            &env,
            &Messages::default(),
        );

        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::ArrowRight));
        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::ArrowLeft));
        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::ArrowDown));
        api.on_tag_keydown(&key("alpha"), &keyboard(KeyboardKey::ArrowUp));

        assert_eq!(
            events.into_inner(),
            vec![
                Event::FocusPrevious,
                Event::FocusNext,
                Event::FocusNext,
                Event::FocusPrevious,
            ]
        );
    }

    #[test]
    fn remove_ignores_disabled_and_empty_state_is_stable() {
        let mut disabled_item = service(Props::new().id("tags").items(items()));

        drop(disabled_item.send(Event::Focus {
            item: Some(key("beta")),
            is_keyboard: true,
        }));
        drop(disabled_item.send(Event::RemoveTag(key("beta"))));

        assert!(
            disabled_item
                .context()
                .items
                .keys()
                .any(|item_key| item_key == &key("beta"))
        );
        assert_eq!(
            disabled_item.context().focused_key,
            None,
            "focusing a disabled tag from idle does not redirect to another tag"
        );

        let empty = service(Props::new().id("tags"));

        let api = empty.connect(&|_| {});

        assert_eq!(empty.context().focused_key, None);
        assert_eq!(api.root_attrs().get(&HtmlAttr::Role), Some("grid"));
        assert_eq!(api.list_attrs().get(&HtmlAttr::Role), Some("row"));
        assert_eq!(
            api.list_attrs().get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            None,
            "unlabelled groups do not reference a missing label element"
        );
    }

    #[test]
    fn tag_group_snapshots_cover_output_branches() {
        let mut group = service(
            Props::new()
                .id("tags")
                .items(items())
                .label("Selected tags")
                .selection_mode(selection::Mode::Multiple)
                .default_selected_keys(selected(&["alpha"])),
        );

        drop(group.send(Event::Focus {
            item: Some(key("alpha")),
            is_keyboard: true,
        }));

        let api = group.connect(&|_| {});

        assert_snapshot!("tag_group_root_focused", snapshot_attrs(&api.root_attrs()));
        assert_snapshot!("tag_group_label", snapshot_attrs(&api.label_attrs()));
        assert_snapshot!("tag_group_list", snapshot_attrs(&api.list_attrs()));
        assert_snapshot!(
            "tag_group_tag_focused_selected",
            snapshot_attrs(&api.tag_attrs(&key("alpha")))
        );
        assert_snapshot!(
            "tag_group_tag_disabled",
            snapshot_attrs(&api.tag_attrs(&key("beta")))
        );
        assert_snapshot!(
            "tag_group_remove_enabled",
            snapshot_attrs(&api.tag_remove_attrs(&key("alpha")))
        );
        assert_snapshot!(
            "tag_group_remove_disabled",
            snapshot_attrs(&api.tag_remove_attrs(&key("beta")))
        );

        let disabled = service(Props::new().id("tags").items(items()).disabled(true));

        assert_snapshot!(
            "tag_group_root_disabled",
            snapshot_attrs(&disabled.connect(&|_| {}).root_attrs())
        );
    }
}
