//! Listbox selection component machine.

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_collections::{
    Collection, DisabledBehavior, Key, Node, StaticCollection,
    navigation::{first_enabled_key, last_enabled_key, next_enabled_key, prev_enabled_key},
    selection, typeahead,
};
use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, NoEffect, Orientation,
    TransitionPlan,
};
use ars_interactions::KeyboardEventData;

const PAGE_STEP: usize = 10;

/// User-facing payload for [`Listbox`](Machine) options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// Human-readable item label.
    pub label: String,
}

/// The state of the listbox component.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The listbox is idle.
    #[default]
    Idle,

    /// The listbox has focus.
    Focused,
}

/// Events accepted by the listbox state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The listbox received focus.
    Focus {
        /// Whether focus came from keyboard modality.
        is_keyboard: bool,
    },

    /// The listbox lost focus.
    Blur,

    /// Select an item by key.
    SelectItem(Key),

    /// Deselect an item by key.
    DeselectItem(Key),

    /// Toggle an item by key.
    ToggleItem(Key),

    /// Replace the highlighted item.
    HighlightItem(Option<Key>),

    /// Extend selection from the current anchor to the provided item key.
    ExtendSelection(Key),

    /// Highlight the next enabled item.
    HighlightNext,

    /// Highlight the previous enabled item.
    HighlightPrev,

    /// Highlight the first enabled item.
    HighlightFirst,

    /// Highlight the last enabled item.
    HighlightLast,

    /// Highlight an item one page before the current item.
    HighlightPageUp,

    /// Highlight an item one page after the current item.
    HighlightPageDown,

    /// Select every item.
    SelectAll,

    /// Clear the current selection.
    DeselectAll,

    /// Search options by type-ahead character and timestamp.
    TypeaheadSearch(char, u64),

    /// Clear the type-ahead buffer.
    ClearTypeahead,

    /// Mark IME composition as active.
    CompositionStart,

    /// Mark IME composition as inactive and clear stale type-ahead state.
    CompositionEnd,

    /// Synchronize context-backed fields from updated props.
    SyncProps,

    /// Replace the item collection dynamically.
    UpdateItems(StaticCollection<Item>),

    /// Notify that an item was activated without changing selection.
    ItemActivated(Key),
}

/// Context held by the listbox state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Resolved locale for message formatting.
    pub locale: Locale,

    /// Item collection, including structural section/header/separator nodes.
    pub items: StaticCollection<Item>,

    /// Controlled or uncontrolled selected-key binding.
    pub selection: Bindable<selection::Set>,

    /// Full collection selection state.
    pub selection_state: selection::State,

    /// Currently highlighted item key.
    pub highlighted_key: Option<Key>,

    /// Type-ahead buffer state.
    pub typeahead: typeahead::State,

    /// Whether the entire listbox is disabled.
    pub disabled: bool,

    /// Whether the field is required.
    pub required: bool,

    /// Whether the field is invalid.
    pub invalid: bool,

    /// Whether the content element currently has focus.
    pub focused: bool,

    /// Whether focus-visible styling should be emitted.
    pub focus_visible: bool,

    /// List orientation.
    pub orientation: Orientation,

    /// Resolved text direction for horizontal arrow-key behavior.
    pub dir: Direction,

    /// Whether keyboard navigation wraps at collection boundaries.
    pub loop_focus: bool,

    /// Whether IME composition is active.
    pub is_composing: bool,

    /// Whether a description element is rendered.
    pub has_description: bool,

    /// Whether a load-more request is in flight.
    pub loading: bool,

    /// Stable component ID derivation helper.
    pub ids: ComponentIds,

    /// Resolved localized message functions.
    pub messages: Messages,
}

/// Props for the listbox component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Controlled selected keys.
    pub value: Option<selection::Set>,

    /// Initial uncontrolled selected keys.
    pub default_value: selection::Set,

    /// Selection mode.
    pub selection_mode: selection::Mode,

    /// Selection behavior.
    pub selection_behavior: selection::Behavior,

    /// Disabled item behavior.
    pub disabled_behavior: DisabledBehavior,

    /// Whether the listbox is disabled.
    pub disabled: bool,

    /// Whether the listbox is required.
    pub required: bool,

    /// Whether the listbox is invalid.
    pub invalid: bool,

    /// List orientation.
    pub orientation: Orientation,

    /// Resolved text direction.
    pub dir: Direction,

    /// Whether keyboard navigation wraps at collection boundaries.
    pub loop_focus: bool,

    /// Whether deselecting the final selected item is blocked.
    pub disallow_empty_selection: bool,

    /// Item keys that should be disabled.
    pub disabled_keys: BTreeSet<Key>,

    /// Form field name associated with the listbox.
    pub name: Option<String>,

    /// Callback invoked when an item is activated.
    pub on_action: Option<Callback<dyn Fn(Key) + Send + Sync>>,

    /// Callback invoked by adapters when the loading sentinel is reached.
    pub on_load_more: Option<Callback<dyn Fn() + Send + Sync>>,

    /// Whether a load-more request is currently in flight.
    pub loading: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: selection::Set::Empty,
            selection_mode: selection::Mode::Single,
            selection_behavior: selection::Behavior::Toggle,
            disabled_behavior: DisabledBehavior::default(),
            disabled: false,
            required: false,
            invalid: false,
            orientation: Orientation::Vertical,
            dir: Direction::Ltr,
            loop_focus: true,
            disallow_empty_selection: false,
            disabled_keys: BTreeSet::new(),
            name: None,
            on_action: None,
            on_load_more: None,
            loading: false,
        }
    }
}

impl Props {
    /// Returns default listbox props.
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

    /// Sets controlled [`Self::value`].
    #[must_use]
    pub fn value(mut self, value: selection::Set) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets [`Self::default_value`].
    #[must_use]
    pub fn default_value(mut self, value: selection::Set) -> Self {
        self.default_value = value;
        self
    }

    /// Sets [`Self::selection_mode`].
    #[must_use]
    pub const fn selection_mode(mut self, mode: selection::Mode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Sets [`Self::selection_behavior`].
    #[must_use]
    pub const fn selection_behavior(mut self, behavior: selection::Behavior) -> Self {
        self.selection_behavior = behavior;
        self
    }

    /// Sets [`Self::disabled_behavior`].
    #[must_use]
    pub const fn disabled_behavior(mut self, behavior: DisabledBehavior) -> Self {
        self.disabled_behavior = behavior;
        self
    }

    /// Sets [`Self::disabled`].
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`Self::required`].
    #[must_use]
    pub const fn required(mut self, value: bool) -> Self {
        self.required = value;
        self
    }

    /// Sets [`Self::invalid`].
    #[must_use]
    pub const fn invalid(mut self, value: bool) -> Self {
        self.invalid = value;
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

    /// Sets [`Self::disallow_empty_selection`].
    #[must_use]
    pub const fn disallow_empty_selection(mut self, value: bool) -> Self {
        self.disallow_empty_selection = value;
        self
    }

    /// Sets [`Self::disabled_keys`].
    #[must_use]
    pub fn disabled_keys(mut self, value: BTreeSet<Key>) -> Self {
        self.disabled_keys = value;
        self
    }

    /// Sets [`Self::name`].
    #[must_use]
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Sets [`Self::on_action`].
    #[must_use]
    pub fn on_action(mut self, value: Callback<dyn Fn(Key) + Send + Sync>) -> Self {
        self.on_action = Some(value);
        self
    }

    /// Sets [`Self::on_load_more`].
    #[must_use]
    pub fn on_load_more(mut self, value: Callback<dyn Fn() + Send + Sync>) -> Self {
        self.on_load_more = Some(value);
        self
    }

    /// Sets [`Self::loading`].
    #[must_use]
    pub const fn loading(mut self, value: bool) -> Self {
        self.loading = value;
        self
    }
}

/// Message function used for listbox option-count announcements.
pub type CountAnnouncementMessage = dyn Fn(u64, &Locale) -> String + Send + Sync;

/// Localized messages for the listbox component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Text for the empty state when no options are available.
    pub empty_label: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,

    /// Live announcement for the number of available options.
    pub count_announcement: MessageFn<CountAnnouncementMessage>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            empty_label: MessageFn::static_str("No options available"),
            count_announcement: MessageFn::new(|count: u64, _locale: &Locale| match count {
                0 => "No options available".to_string(),
                1 => "1 option available".to_string(),
                count => format!("{count} options available"),
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// Listbox anatomy parts.
#[derive(ComponentPart)]
#[scope = "listbox"]
pub enum Part {
    /// Root container.
    Root,

    /// Visible label.
    Label,

    /// Focusable listbox content.
    Content,

    /// Structural item group.
    ItemGroup {
        /// Group key.
        key: Key,
    },

    /// Label for an item group.
    ItemGroupLabel {
        /// Group key.
        key: Key,
    },

    /// Option item.
    Item {
        /// Item key.
        key: Key,
    },

    /// Option text.
    ItemText {
        /// Item key.
        key: Key,
    },

    /// Option selection indicator.
    ItemIndicator {
        /// Item key.
        key: Key,
    },

    /// Help or description text.
    Description,

    /// Validation error message.
    ErrorMessage,

    /// Infinite-scroll loading sentinel.
    LoadingSentinel,
}

/// Machine for the listbox component.
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

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        let selected = props
            .value
            .clone()
            .unwrap_or_else(|| props.default_value.clone());
        let selected = normalize_selection_for_mode(selected, props.selection_mode);

        let mut selection_state =
            selection::State::new(props.selection_mode, props.selection_behavior);

        selection_state.disabled_behavior = props.disabled_behavior;
        selection_state.disabled_keys = props.disabled_keys.clone();
        selection_state.selected_keys = selected.clone();

        (
            State::Idle,
            Context {
                locale: env.locale.clone(),
                items: StaticCollection::default(),
                selection: match &props.value {
                    Some(_) => Bindable::controlled(selected.clone()),
                    None => Bindable::uncontrolled(selected.clone()),
                },
                selection_state,
                highlighted_key: None,
                typeahead: typeahead::State::default(),
                disabled: props.disabled,
                required: props.required,
                invalid: props.invalid,
                focused: false,
                focus_visible: false,
                orientation: props.orientation,
                dir: props.dir,
                loop_focus: props.loop_focus,
                is_composing: false,
                has_description: false,
                loading: props.loading,
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
        if ctx.disabled
            && matches!(
                event,
                Event::Focus { .. }
                    | Event::SelectItem(_)
                    | Event::DeselectItem(_)
                    | Event::ToggleItem(_)
                    | Event::SelectAll
                    | Event::DeselectAll
                    | Event::HighlightItem(_)
                    | Event::ExtendSelection(_)
                    | Event::HighlightNext
                    | Event::HighlightPrev
                    | Event::HighlightFirst
                    | Event::HighlightLast
                    | Event::HighlightPageUp
                    | Event::HighlightPageDown
                    | Event::TypeaheadSearch(_, _)
            )
        {
            return None;
        }

        match (state, event) {
            (State::Idle, Event::Focus { is_keyboard }) => {
                let focus_visible = *is_keyboard;
                let highlight = ctx.highlighted_key.clone().or_else(|| first_key(ctx));

                Some(
                    TransitionPlan::to(State::Focused).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = focus_visible;
                        ctx.highlighted_key = highlight;
                    }),
                )
            }

            (_, Event::Focus { is_keyboard }) => {
                let focus_visible = *is_keyboard;

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = focus_visible;
                }))
            }

            (State::Focused, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.typeahead = typeahead::State::default();
                }))
            }

            (_, Event::HighlightItem(key)) => {
                let key = key
                    .clone()
                    .filter(|key| ctx.items.contains_key(key) && is_focusable_key(ctx, key));

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Focused, Event::HighlightFirst) => {
                let key = first_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Focused, Event::HighlightLast) => {
                let key = last_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Focused, Event::HighlightNext) => {
                let key = next_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Focused, Event::HighlightPrev) => {
                let key = prev_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Focused, Event::HighlightPageDown) => {
                let key = page_key(ctx, true);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Focused, Event::HighlightPageUp) => {
                let key = page_key(ctx, false);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (_, Event::SelectItem(key)) => select_plan(ctx, props, key.clone(), false),

            (_, Event::DeselectItem(key)) => {
                if !ctx.items.contains_key(key) {
                    return None;
                }

                let next = normalize_selection_state(
                    ctx.selection_state.deselect_from_all(key, &ctx.items),
                );

                if props.disallow_empty_selection && selection_is_empty(&next.selected_keys) {
                    return None;
                }

                Some(apply_selection_plan(next))
            }

            (_, Event::ToggleItem(key)) => {
                if !ctx.items.contains_key(key) {
                    return None;
                }

                let next =
                    normalize_selection_state(ctx.selection_state.toggle(key.clone(), &ctx.items));

                if props.disallow_empty_selection && selection_is_empty(&next.selected_keys) {
                    return None;
                }

                Some(apply_selection_plan(next))
            }

            (_, Event::ExtendSelection(key)) => {
                let next = range_selection_state(ctx, key.clone());

                if props.disallow_empty_selection && selection_is_empty(&next.selected_keys) {
                    return None;
                }

                let highlighted_key = key.clone();

                Some(apply_selection_plan(next).apply(move |ctx: &mut Context| {
                    ctx.highlighted_key = Some(highlighted_key);
                }))
            }

            (_, Event::SelectAll) => {
                if ctx.selection_state.mode != selection::Mode::Multiple {
                    return None;
                }

                let mut next = ctx.selection_state.clone();

                next.selected_keys = set_from_keys(enabled_item_keys(ctx).collect());

                Some(apply_selection_plan(next))
            }

            (_, Event::DeselectAll) => {
                if props.disallow_empty_selection && !ctx.selection.get().is_empty() {
                    return None;
                }

                Some(apply_selection_plan(ctx.selection_state.clear()))
            }

            (State::Focused, Event::TypeaheadSearch(ch, now_ms)) => {
                if ctx.is_composing {
                    return None;
                }

                let (typeahead, found) = process_typeahead(ctx, *ch, *now_ms);

                let highlighted_key = found.or_else(|| ctx.highlighted_key.clone());

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.typeahead = typeahead;
                    ctx.highlighted_key = highlighted_key;
                }))
            }

            (_, Event::ClearTypeahead) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.typeahead = typeahead::State::default();
                }))
            }

            (_, Event::CompositionStart) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.is_composing = true;
                }))
            }

            (_, Event::CompositionEnd) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.is_composing = false;
                    ctx.typeahead = typeahead::State::default();
                }))
            }

            (_, Event::SyncProps) => Some(sync_props_plan(props)),

            (_, Event::UpdateItems(items)) => {
                let items = items.clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.items = items;
                    invalidate_collection_references(ctx);
                }))
            }

            (_, Event::ItemActivated(key)) => {
                if ctx.disabled
                    || !ctx.items.contains_key(key)
                    || ctx.selection_state.is_disabled(key)
                {
                    return None;
                }

                let key = key.clone();
                let on_action = props.on_action.clone();
                Some(TransitionPlan::context_only(move |_| {
                    if let Some(callback) = on_action {
                        callback(key);
                    }
                }))
            }

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

/// API for deriving listbox attributes and dispatching listbox events.
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

    /// Text for the empty state when no options are available.
    #[must_use]
    pub fn empty_label(&self) -> String {
        (self.ctx.messages.empty_label)(&self.ctx.locale)
    }

    /// Announcement text for the number of available options.
    #[must_use]
    pub fn count_announcement(&self, count: usize) -> String {
        (self.ctx.messages.count_announcement)(count as u64, &self.ctx.locale)
    }

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        attrs
    }

    /// Attributes for the label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Label);

        attrs.set(HtmlAttr::Id, self.ctx.ids.part("label"));

        attrs
    }

    /// Attributes for the focusable listbox content element.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Content);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, "listbox")
            .set(
                HtmlAttr::TabIndex,
                if self.ctx.disabled { "-1" } else { "0" },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                match self.ctx.orientation {
                    Orientation::Horizontal => "horizontal",
                    Orientation::Vertical => "vertical",
                },
            )
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        if self.ctx.selection_state.mode == selection::Mode::Multiple {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }

        if let Some(key) = valid_highlight(self.ctx) {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::ActiveDescendant),
                self.ctx.ids.item("item", key),
            );
        }

        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
        }

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        let mut described_by = Vec::new();

        if self.ctx.has_description {
            described_by.push(self.ctx.ids.part("description"));
        }

        if self.ctx.invalid {
            described_by.push(self.ctx.ids.part("error-message"));
        }

        if !described_by.is_empty() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                described_by.join(" "),
            );
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Attributes for one option item.
    #[must_use]
    pub fn item_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::Item {
            key: Key::default(),
        });

        let selected = self.ctx.selection.get().contains(key);
        let highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let disabled = self.ctx.disabled || self.ctx.selection_state.is_disabled(key);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("item", key))
            .set(HtmlAttr::Role, "option")
            .set(
                HtmlAttr::Aria(AriaAttr::Selected),
                if selected { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Data("ars-state"),
                if selected { "selected" } else { "unselected" },
            );

        if disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if highlighted {
            attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true);
        }

        if let Some(node) = self.ctx.items.get(key) {
            attrs.set(HtmlAttr::Data("ars-value"), node.text_value.as_str());
        }

        attrs
    }

    /// Attributes for an item group element.
    #[must_use]
    pub fn item_group_attrs(&self, section_key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemGroup {
            key: Key::default(),
        });

        attrs.set(HtmlAttr::Role, "group").set(
            HtmlAttr::Aria(AriaAttr::LabelledBy),
            self.ctx.ids.item_part("group", section_key, "label"),
        );

        attrs
    }

    /// Attributes for an item group label element.
    #[must_use]
    pub fn item_group_label_attrs(&self, section_key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemGroupLabel {
            key: Key::default(),
        });

        attrs.set(
            HtmlAttr::Id,
            self.ctx.ids.item_part("group", section_key, "label"),
        );

        attrs
    }

    /// Attributes for an item text element.
    #[must_use]
    pub fn item_text_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemText {
            key: Key::default(),
        });

        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", key, "text"));

        attrs
    }

    /// Attributes for an item indicator element.
    #[must_use]
    pub fn item_indicator_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemIndicator {
            key: Key::default(),
        });

        if self.ctx.selection.get().contains(key) {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        attrs
    }

    /// Attributes for the description element.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Description);

        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));

        attrs
    }

    /// Attributes for the error message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::ErrorMessage);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

        attrs
    }

    /// Attributes for the optional loading sentinel.
    #[must_use]
    pub fn loading_sentinel_attrs(&self) -> Option<AttrMap> {
        self.props.on_load_more.as_ref()?;

        let mut attrs = part_attrs(&Part::LoadingSentinel);

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true")
            .set(HtmlAttr::TabIndex, "-1");

        if self.ctx.loading {
            attrs.set_bool(HtmlAttr::Data("ars-loading"), true);
        }

        Some(attrs)
    }

    /// Iterates all collection nodes for rendering.
    pub fn items(&self) -> impl Iterator<Item = &Node<Item>> {
        self.ctx.items.nodes()
    }

    /// Dispatches a focus event.
    pub fn on_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches a blur event.
    pub fn on_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches an item click event.
    pub fn on_item_click(&self, key: Key) {
        (self.send)(Event::ToggleItem(key));
    }

    /// Dispatches an item hover event.
    pub fn on_item_hover(&self, key: Key) {
        (self.send)(Event::HighlightItem(Some(key)));
    }

    /// Dispatches an item leave event.
    pub fn on_item_leave(&self) {
        (self.send)(Event::HighlightItem(None));
    }

    /// Dispatches a keydown event.
    pub fn on_keydown(&self, data: &KeyboardEventData, shift: bool, ctrl: bool, meta: bool) {
        self.on_keydown_impl(data, shift, ctrl, meta, None);
    }

    /// Dispatches a keydown event with an adapter-provided monotonic timestamp.
    pub fn on_keydown_at(
        &self,
        data: &KeyboardEventData,
        shift: bool,
        ctrl: bool,
        meta: bool,
        now_ms: u64,
    ) {
        self.on_keydown_impl(data, shift, ctrl, meta, Some(now_ms));
    }

    fn on_keydown_impl(
        &self,
        data: &KeyboardEventData,
        shift: bool,
        ctrl: bool,
        meta: bool,
        now_ms: Option<u64>,
    ) {
        let key = resolved_arrow_key(data.key, self.ctx.orientation, self.ctx.dir);

        match key {
            KeyboardKey::ArrowDown | KeyboardKey::ArrowRight => {
                if shift {
                    self.extend_selection(true);
                } else {
                    (self.send)(Event::HighlightNext);
                }
            }

            KeyboardKey::ArrowUp | KeyboardKey::ArrowLeft => {
                if shift {
                    self.extend_selection(false);
                } else {
                    (self.send)(Event::HighlightPrev);
                }
            }

            KeyboardKey::Home => (self.send)(Event::HighlightFirst),

            KeyboardKey::End => (self.send)(Event::HighlightLast),

            KeyboardKey::PageUp => (self.send)(Event::HighlightPageUp),

            KeyboardKey::PageDown => (self.send)(Event::HighlightPageDown),

            KeyboardKey::Space | KeyboardKey::Enter => {
                if let Some(key) = &self.ctx.highlighted_key {
                    (self.send)(Event::ToggleItem(key.clone()));
                }
            }

            KeyboardKey::Escape => (self.send)(Event::DeselectAll),
            _ if data.character == Some('a') && (ctrl || meta) => {
                (self.send)(Event::SelectAll);
            }

            _ if data.character.is_some()
                && now_ms.is_some()
                && !ctrl
                && !meta
                && !data.is_composing =>
            {
                (self.send)(Event::TypeaheadSearch(
                    data.character.expect("checked"),
                    now_ms.expect("checked"),
                ));
            }

            _ => {}
        }
    }

    fn extend_selection(&self, forward: bool) {
        if let Some(current) = &self.ctx.highlighted_key {
            let next = if forward {
                next_enabled_key(
                    &self.ctx.items,
                    current,
                    &self.ctx.selection_state.disabled_keys,
                    self.ctx.selection_state.disabled_behavior,
                    self.ctx.loop_focus,
                )
            } else {
                prev_enabled_key(
                    &self.ctx.items,
                    current,
                    &self.ctx.selection_state.disabled_keys,
                    self.ctx.selection_state.disabled_behavior,
                    self.ctx.loop_focus,
                )
            };

            if let Some(next) = next {
                (self.send)(Event::ExtendSelection(next.clone()));
                (self.send)(Event::HighlightItem(Some(next)));
            }
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Content => self.content_attrs(),
            Part::ItemGroup { ref key } => self.item_group_attrs(key),
            Part::ItemGroupLabel { ref key } => self.item_group_label_attrs(key),
            Part::Item { ref key } => self.item_attrs(key),
            Part::ItemText { ref key } => self.item_text_attrs(key),
            Part::ItemIndicator { ref key } => self.item_indicator_attrs(key),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::LoadingSentinel => self.loading_sentinel_attrs().unwrap_or_default(),
        }
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

fn first_key(ctx: &Context) -> Option<Key> {
    first_enabled_key(
        &ctx.items,
        &ctx.selection_state.disabled_keys,
        ctx.selection_state.disabled_behavior,
    )
}

fn last_key(ctx: &Context) -> Option<Key> {
    last_enabled_key(
        &ctx.items,
        &ctx.selection_state.disabled_keys,
        ctx.selection_state.disabled_behavior,
    )
}

fn next_key(ctx: &Context) -> Option<Key> {
    ctx.highlighted_key
        .as_ref()
        .and_then(|key| {
            next_enabled_key(
                &ctx.items,
                key,
                &ctx.selection_state.disabled_keys,
                ctx.selection_state.disabled_behavior,
                ctx.loop_focus,
            )
        })
        .or_else(|| first_key(ctx))
}

fn prev_key(ctx: &Context) -> Option<Key> {
    ctx.highlighted_key
        .as_ref()
        .and_then(|key| {
            prev_enabled_key(
                &ctx.items,
                key,
                &ctx.selection_state.disabled_keys,
                ctx.selection_state.disabled_behavior,
                ctx.loop_focus,
            )
        })
        .or_else(|| last_key(ctx))
}

fn page_key(ctx: &Context, forward: bool) -> Option<Key> {
    let mut current = ctx.highlighted_key.clone().or_else(|| {
        if forward {
            first_key(ctx)
        } else {
            last_key(ctx)
        }
    })?;

    for _ in 0..PAGE_STEP {
        let next = if forward {
            next_enabled_key(
                &ctx.items,
                &current,
                &ctx.selection_state.disabled_keys,
                ctx.selection_state.disabled_behavior,
                false,
            )
        } else {
            prev_enabled_key(
                &ctx.items,
                &current,
                &ctx.selection_state.disabled_keys,
                ctx.selection_state.disabled_behavior,
                false,
            )
        };

        if let Some(next) = next {
            current = next;
        } else {
            break;
        }
    }

    Some(current)
}

fn select_plan(
    ctx: &Context,
    props: &Props,
    key: Key,
    toggle: bool,
) -> Option<TransitionPlan<Machine>> {
    if !ctx.items.contains_key(&key) || ctx.selection_state.is_disabled(&key) {
        return None;
    }
    if toggle
        && ctx.selection_state.is_selected(&key)
        && props.disallow_empty_selection
        && ctx.selection.get().len() <= 1
    {
        return None;
    }

    let next = if toggle {
        ctx.selection_state.toggle(key, &ctx.items)
    } else {
        ctx.selection_state.select(key)
    };
    let next = normalize_selection_state(next);

    if toggle && props.disallow_empty_selection && selection_is_empty(&next.selected_keys) {
        return None;
    }

    Some(apply_selection_plan(next))
}

fn apply_selection_plan(next: selection::State) -> TransitionPlan<Machine> {
    let next = normalize_selection_state(next);
    let selected = next.selected_keys.clone();
    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.selection.set(selected.clone());
        ctx.selection_state = next;
        ctx.selection_state.selected_keys = ctx.selection.get().clone();
    })
}

fn sync_props_plan(props: &Props) -> TransitionPlan<Machine> {
    let props = props.clone();

    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.disabled = props.disabled;
        ctx.required = props.required;
        ctx.invalid = props.invalid;
        ctx.orientation = props.orientation;
        ctx.dir = props.dir;
        ctx.loop_focus = props.loop_focus;
        ctx.loading = props.loading;
        ctx.ids = ComponentIds::from_id(&props.id);

        ctx.selection_state.mode = props.selection_mode;
        ctx.selection_state.behavior = props.selection_behavior;
        ctx.selection_state.disabled_behavior = props.disabled_behavior;
        ctx.selection_state.disabled_keys = props.disabled_keys.clone();

        if props.value.is_some() || ctx.selection.is_controlled() {
            ctx.selection.sync_controlled(
                props
                    .value
                    .clone()
                    .map(|value| normalize_selection_for_mode(value, props.selection_mode)),
            );
        }

        set_current_selection(
            ctx,
            normalize_selection_for_mode(ctx.selection.get().clone(), props.selection_mode),
        );
        ctx.selection_state = normalize_selection_state(ctx.selection_state.clone());
        invalidate_collection_references(ctx);
    })
}

fn range_selection_state(ctx: &Context, target: Key) -> selection::State {
    if ctx.selection_state.mode != selection::Mode::Multiple {
        return ctx.selection_state.select(target);
    }

    let anchor = match &ctx.selection_state.anchor_key {
        Some(anchor) if ctx.items.contains_key(anchor) => anchor.clone(),
        _ => return ctx.selection_state.select(target),
    };

    let mut in_range = false;
    let mut keys = BTreeSet::new();

    for node in ctx.items.nodes() {
        if !node.is_focusable() {
            continue;
        }

        let boundary = node.key == anchor || node.key == target;

        if boundary {
            if !ctx.selection_state.is_disabled(&node.key) {
                keys.insert(node.key.clone());
            }

            if anchor == target {
                break;
            }

            in_range = !in_range;
        } else if in_range && !ctx.selection_state.is_disabled(&node.key) {
            keys.insert(node.key.clone());
        }
    }

    selection::State {
        selected_keys: set_from_keys(keys),
        focused_key: Some(target),
        ..ctx.selection_state.clone()
    }
}

fn set_from_keys(keys: BTreeSet<Key>) -> selection::Set {
    if keys.is_empty() {
        selection::Set::Empty
    } else {
        selection::Set::Multiple(keys)
    }
}

fn normalize_selection_state(mut state: selection::State) -> selection::State {
    state.selected_keys = normalize_selection_for_mode(state.selected_keys, state.mode);

    state
}

fn normalize_selection_for_mode(selected: selection::Set, mode: selection::Mode) -> selection::Set {
    match mode {
        selection::Mode::None => selection::Set::Empty,
        selection::Mode::Single => match selected {
            selection::Set::Single(key) => selection::Set::Single(key),
            selection::Set::Multiple(keys) => keys
                .into_iter()
                .next()
                .map_or(selection::Set::Empty, selection::Set::Single),
            _ => selection::Set::Empty,
        },
        selection::Mode::Multiple => {
            if selection_is_empty(&selected) {
                selection::Set::Empty
            } else {
                selected
            }
        }
    }
}

fn set_current_selection(ctx: &mut Context, selected: selection::Set) {
    let selected = normalize_selection_for_mode(selected, ctx.selection_state.mode);

    if ctx.selection.is_controlled() {
        ctx.selection.sync_controlled(Some(selected.clone()));
    } else {
        ctx.selection.set(selected.clone());
    }

    ctx.selection_state.selected_keys = selected;
}

fn selection_is_empty(set: &selection::Set) -> bool {
    match set {
        selection::Set::Empty => true,
        selection::Set::Multiple(keys) => keys.is_empty(),
        _ => false,
    }
}

fn process_typeahead(ctx: &Context, ch: char, now_ms: u64) -> (typeahead::State, Option<Key>) {
    ctx.typeahead.process_char_with_locale(
        ch,
        now_ms,
        ctx.highlighted_key.as_ref(),
        &ctx.items,
        &ctx.locale,
        &ctx.selection_state.disabled_keys,
        ctx.selection_state.disabled_behavior,
    )
}

fn invalidate_collection_references(ctx: &mut Context) {
    if ctx
        .highlighted_key
        .as_ref()
        .is_some_and(|key| !ctx.items.contains_key(key) || !is_focusable_key(ctx, key))
    {
        ctx.highlighted_key = None;
    }

    let selection = retain_present_selection(ctx.selection.get(), &ctx.items);

    set_current_selection(ctx, selection);

    if ctx
        .selection_state
        .anchor_key
        .as_ref()
        .is_some_and(|key| !ctx.items.contains_key(key) || ctx.selection_state.is_disabled(key))
    {
        ctx.selection_state.anchor_key = None;
    }
}

fn retain_present_selection(
    selection: &selection::Set,
    items: &StaticCollection<Item>,
) -> selection::Set {
    match selection {
        selection::Set::Single(key) => {
            if items.contains_key(key) {
                selection::Set::Single(key.clone())
            } else {
                selection::Set::Empty
            }
        }

        selection::Set::Multiple(keys) => {
            let retained = keys
                .iter()
                .filter(|key| items.contains_key(key))
                .cloned()
                .collect::<BTreeSet<_>>();

            if retained.is_empty() {
                selection::Set::Empty
            } else {
                selection::Set::Multiple(retained)
            }
        }

        selection::Set::All => selection::Set::All,

        _ => selection::Set::Empty,
    }
}

fn enabled_item_keys(ctx: &Context) -> impl Iterator<Item = Key> + '_ {
    ctx.items
        .item_keys()
        .filter(|key| !ctx.selection_state.is_disabled(key))
        .cloned()
}

fn valid_highlight(ctx: &Context) -> Option<&Key> {
    ctx.highlighted_key
        .as_ref()
        .filter(|key| ctx.items.contains_key(key) && is_focusable_key(ctx, key))
}

fn is_focusable_key(ctx: &Context, key: &Key) -> bool {
    ctx.items.get(key).is_some_and(Node::is_focusable)
        && (!ctx.selection_state.is_disabled(key)
            || ctx.selection_state.disabled_behavior == DisabledBehavior::FocusOnly)
}

const fn resolved_arrow_key(
    key: KeyboardKey,
    orientation: Orientation,
    dir: Direction,
) -> KeyboardKey {
    match (key, orientation, dir) {
        (KeyboardKey::ArrowRight, Orientation::Horizontal, Direction::Rtl) => {
            KeyboardKey::ArrowLeft
        }

        (KeyboardKey::ArrowLeft, Orientation::Horizontal, Direction::Rtl) => {
            KeyboardKey::ArrowRight
        }

        _ => key,
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, sync::Arc};
    use core::cell::RefCell;
    use std::sync::Mutex;

    use ars_collections::{CollectionBuilder, DisabledBehavior, Key, selection};
    use ars_core::{
        AriaAttr, Callback, ConnectApi, Direction, Env, HtmlAttr, KeyboardKey, Service,
    };
    use ars_interactions::KeyboardEventData;

    use super::{Event, Machine, Messages, Props};

    fn key(value: &'static str) -> Key {
        Key::str(value)
    }

    fn keyboard(key: KeyboardKey, character: Option<char>) -> KeyboardEventData {
        KeyboardEventData {
            key,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
            character,
        }
    }

    fn collection() -> ars_collections::StaticCollection<super::Item> {
        CollectionBuilder::new()
            .item(
                key("alpha"),
                "Alpha",
                super::Item {
                    label: "Alpha".into(),
                },
            )
            .item(
                key("bravo"),
                "Bravo",
                super::Item {
                    label: "Bravo".into(),
                },
            )
            .item(
                key("charlie"),
                "Charlie",
                super::Item {
                    label: "Charlie".into(),
                },
            )
            .build()
    }

    fn single_item_collection() -> ars_collections::StaticCollection<super::Item> {
        CollectionBuilder::new()
            .item(
                key("alpha"),
                "Alpha",
                super::Item {
                    label: "Alpha".into(),
                },
            )
            .build()
    }

    fn service(props: Props) -> Service<Machine> {
        let mut service = Service::<Machine>::new(props, &Env::default(), &Messages::default());

        drop(service.send(Event::UpdateItems(collection())));

        service
    }

    fn snapshot_attrs(attrs: &ars_core::AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn builder_methods_preserve_public_props_contract() {
        let props = Props::new()
            .id("lb")
            .value(selection::Set::Single(key("alpha")))
            .default_value(selection::Set::Single(key("bravo")))
            .name("choices");

        assert_eq!(props.value, Some(selection::Set::Single(key("alpha"))));
        assert_eq!(props.default_value, selection::Set::Single(key("bravo")));
        assert_eq!(props.name.as_deref(), Some("choices"));
    }

    #[test]
    fn content_and_item_attrs_emit_listbox_and_option_roles() {
        let mut listbox = service(Props::new().id("lb"));

        drop(listbox.send(Event::Focus { is_keyboard: true }));
        drop(listbox.send(Event::HighlightItem(Some(key("bravo")))));

        let api = listbox.connect(&|_| {});

        assert_eq!(api.content_attrs().get(&HtmlAttr::Role), Some("listbox"));
        assert_eq!(
            api.content_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant)),
            Some("lb-item-bravo")
        );
        assert_eq!(
            api.item_attrs(&key("bravo")).get(&HtmlAttr::Role),
            Some("option")
        );

        let disabled_service = service(Props::new().id("disabled").disabled(true));

        let disabled_api = disabled_service.connect(&|_| {});

        assert_eq!(
            disabled_api
                .item_attrs(&key("alpha"))
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
    }

    #[test]
    fn single_select_replaces_existing_selection() {
        let mut service = service(Props::new().id("lb"));

        drop(service.send(Event::SelectItem(key("alpha"))));
        drop(service.send(Event::SelectItem(key("bravo"))));

        assert!(service.context().selection.get().contains(&key("bravo")));
        assert!(!service.context().selection.get().contains(&key("alpha")));
    }

    #[test]
    fn multi_select_toggle_and_replace_modes_are_respected() {
        let mut toggle = service(
            Props::new()
                .id("toggle")
                .selection_mode(selection::Mode::Multiple)
                .selection_behavior(selection::Behavior::Toggle),
        );

        drop(toggle.send(Event::ToggleItem(key("alpha"))));
        drop(toggle.send(Event::ToggleItem(key("bravo"))));

        assert!(toggle.context().selection.get().contains(&key("alpha")));
        assert!(toggle.context().selection.get().contains(&key("bravo")));

        let mut replace = service(
            Props::new()
                .id("replace")
                .selection_mode(selection::Mode::Multiple)
                .selection_behavior(selection::Behavior::Replace),
        );

        drop(replace.send(Event::SelectItem(key("alpha"))));
        drop(replace.send(Event::SelectItem(key("bravo"))));

        assert!(!replace.context().selection.get().contains(&key("alpha")));
        assert!(replace.context().selection.get().contains(&key("bravo")));

        let mut allow_empty = service(
            Props::new()
                .id("allow-empty")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Single(key("alpha"))),
        );

        drop(allow_empty.send(Event::DeselectItem(key("alpha"))));

        assert!(allow_empty.context().selection.get().is_empty());

        drop(toggle.send(Event::DeselectItem(key("alpha"))));

        assert!(!toggle.context().selection.get().contains(&key("alpha")));

        drop(toggle.send(Event::SelectAll));

        assert!(toggle.context().selection.get().contains(&key("alpha")));
        assert!(toggle.context().selection.get().contains(&key("bravo")));
        assert!(toggle.context().selection.get().contains(&key("charlie")));

        drop(toggle.send(Event::DeselectAll));

        assert!(toggle.context().selection.get().is_empty());
    }

    #[test]
    fn typeahead_and_keyboard_navigation_skip_disabled_items() {
        let disabled = BTreeSet::from([key("bravo")]);

        let mut service = service(
            Props::new()
                .id("lb")
                .disabled_keys(disabled)
                .disabled_behavior(DisabledBehavior::Skip),
        );

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert_eq!(service.context().highlighted_key, Some(key("alpha")));

        drop(service.send(Event::HighlightNext));

        assert_eq!(service.context().highlighted_key, Some(key("charlie")));

        drop(service.send(Event::TypeaheadSearch('b', 100)));

        assert_eq!(service.context().highlighted_key, Some(key("charlie")));

        drop(service.send(Event::TypeaheadSearch('c', 700)));

        assert_eq!(service.context().highlighted_key, Some(key("charlie")));

        drop(service.send(Event::ClearTypeahead));

        assert_eq!(service.context().typeahead, Default::default());

        drop(service.send(Event::HighlightItem(Some(key("bravo")))));

        assert_eq!(service.context().highlighted_key, None);
    }

    #[test]
    fn disabled_listbox_ignores_highlight_and_typeahead_events() {
        let mut listbox = service(Props::new().id("lb").disabled(true));

        drop(listbox.send(Event::HighlightItem(Some(key("alpha")))));
        drop(listbox.send(Event::Focus { is_keyboard: true }));
        drop(listbox.send(Event::TypeaheadSearch('b', 100)));

        assert_eq!(listbox.context().highlighted_key, None);
        assert_eq!(listbox.context().typeahead.search, "");
    }

    #[test]
    fn select_all_with_no_enabled_items_canonicalizes_to_empty() {
        let disabled = BTreeSet::from([key("alpha"), key("bravo"), key("charlie")]);
        let mut listbox = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple)
                .disabled_keys(disabled),
        );

        drop(listbox.send(Event::SelectAll));

        assert_eq!(*listbox.context().selection.get(), selection::Set::Empty);
    }

    #[test]
    fn disallow_empty_selection_allows_deselect_from_all_when_items_remain() {
        let mut listbox = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::All)
                .disallow_empty_selection(true),
        );

        drop(listbox.send(Event::DeselectItem(key("alpha"))));

        assert_eq!(
            *listbox.context().selection.get(),
            selection::Set::Multiple(BTreeSet::from([key("bravo"), key("charlie")]))
        );
    }

    #[test]
    fn disallow_empty_selection_allows_toggle_from_all_when_items_remain() {
        let mut listbox = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::All)
                .disallow_empty_selection(true),
        );

        drop(listbox.send(Event::ToggleItem(key("alpha"))));

        assert_eq!(
            *listbox.context().selection.get(),
            selection::Set::Multiple(BTreeSet::from([key("bravo"), key("charlie")]))
        );
    }

    #[test]
    fn disallow_empty_selection_blocks_singleton_deselect_from_all() {
        let mut listbox = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::All)
                .disallow_empty_selection(true),
        );

        drop(listbox.send(Event::UpdateItems(single_item_collection())));
        drop(listbox.send(Event::DeselectItem(key("alpha"))));
        assert_eq!(*listbox.context().selection.get(), selection::Set::All);

        drop(listbox.send(Event::ToggleItem(key("alpha"))));
        assert_eq!(*listbox.context().selection.get(), selection::Set::All);
    }

    #[test]
    fn stale_item_selection_events_do_not_mutate_selection() {
        let mut listbox = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::All),
        );

        drop(listbox.send(Event::DeselectItem(key("missing"))));
        assert_eq!(*listbox.context().selection.get(), selection::Set::All);

        drop(listbox.send(Event::ToggleItem(key("missing"))));
        assert_eq!(*listbox.context().selection.get(), selection::Set::All);

        let mut empty = service(
            Props::new()
                .id("empty")
                .selection_mode(selection::Mode::Multiple),
        );

        drop(empty.send(Event::SelectItem(key("missing"))));
        assert_eq!(*empty.context().selection.get(), selection::Set::Empty);
    }

    #[test]
    fn focus_only_disabled_items_keep_active_descendant() {
        let mut listbox = service(
            Props::new()
                .id("lb")
                .disabled_keys(BTreeSet::from([key("bravo")]))
                .disabled_behavior(DisabledBehavior::FocusOnly),
        );

        drop(listbox.send(Event::Focus { is_keyboard: true }));
        drop(listbox.send(Event::HighlightItem(Some(key("bravo")))));

        assert_eq!(
            listbox
                .connect(&|_| {})
                .content_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant)),
            Some("lb-item-bravo")
        );
        assert_eq!(*listbox.context().selection.get(), selection::Set::Empty);
    }

    #[test]
    fn keydown_helpers_require_adapter_timestamps_for_typeahead() {
        let mut listbox = service(Props::new().id("lb"));

        drop(listbox.send(Event::Focus { is_keyboard: true }));
        let sent = RefCell::new(Vec::new());
        {
            let send = |event| {
                sent.borrow_mut().push(event);
            };
            let api = listbox.connect(&send);
            api.on_keydown(
                &keyboard(KeyboardKey::Unidentified, Some('b')),
                false,
                false,
                false,
            );
            api.on_keydown_at(
                &keyboard(KeyboardKey::Unidentified, Some('b')),
                false,
                false,
                false,
                100,
            );
        }
        assert_eq!(
            sent.borrow().as_slice(),
            &[Event::TypeaheadSearch('b', 100)]
        );
        for event in sent.take() {
            drop(listbox.send(event));
        }
        let first_time = listbox.context().typeahead.last_key_time_ms;
        let sent = RefCell::new(Vec::new());
        {
            let send = |event| {
                sent.borrow_mut().push(event);
            };
            let api = listbox.connect(&send);
            api.on_keydown_at(
                &keyboard(KeyboardKey::Unidentified, Some('r')),
                false,
                false,
                false,
                1_000,
            );
        }
        for event in sent.take() {
            drop(listbox.send(event));
        }

        assert_eq!(first_time, 100);
        assert_eq!(listbox.context().typeahead.last_key_time_ms, 1_000);
    }

    #[test]
    fn home_end_page_and_ime_behavior_match_contract() {
        let mut service = service(Props::new().id("lb"));

        drop(service.send(Event::Focus { is_keyboard: true }));
        drop(service.send(Event::HighlightLast));

        assert_eq!(service.context().highlighted_key, Some(key("charlie")));

        drop(service.send(Event::HighlightFirst));

        assert_eq!(service.context().highlighted_key, Some(key("alpha")));

        drop(service.send(Event::HighlightPageDown));

        assert_eq!(service.context().highlighted_key, Some(key("charlie")));

        drop(service.send(Event::HighlightPageUp));

        assert_eq!(service.context().highlighted_key, Some(key("alpha")));

        drop(service.send(Event::CompositionStart));
        drop(service.send(Event::TypeaheadSearch('c', 100)));

        assert_eq!(service.context().highlighted_key, Some(key("alpha")));

        drop(service.send(Event::CompositionEnd));
        drop(service.send(Event::TypeaheadSearch('c', 700)));

        assert_eq!(service.context().highlighted_key, Some(key("charlie")));
    }

    #[test]
    fn update_items_invalidates_stale_references() {
        let mut listbox = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple),
        );

        drop(listbox.send(Event::Focus { is_keyboard: true }));
        drop(listbox.send(Event::SelectItem(key("alpha"))));
        drop(listbox.send(Event::SelectItem(key("bravo"))));
        drop(listbox.send(Event::HighlightItem(Some(key("bravo")))));

        let new_items = CollectionBuilder::new()
            .item(
                key("charlie"),
                "Charlie",
                super::Item {
                    label: "Charlie".into(),
                },
            )
            .build();

        drop(listbox.send(Event::UpdateItems(new_items)));

        assert_eq!(listbox.context().highlighted_key, None);
        assert!(!listbox.context().selection.get().contains(&key("alpha")));
        assert!(!listbox.context().selection.get().contains(&key("bravo")));

        let mut retained = service(
            Props::new()
                .id("retained")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Multiple(BTreeSet::from([
                    key("alpha"),
                    key("bravo"),
                ]))),
        );

        let alpha_only = CollectionBuilder::new()
            .item(
                key("alpha"),
                "Alpha",
                super::Item {
                    label: "Alpha".into(),
                },
            )
            .build();

        drop(retained.send(Event::UpdateItems(alpha_only)));

        assert!(retained.context().selection.get().contains(&key("alpha")));
        assert!(!retained.context().selection.get().contains(&key("bravo")));

        let mut single = service(
            Props::new()
                .id("single")
                .default_value(selection::Set::Single(key("alpha"))),
        );

        drop(single.send(Event::UpdateItems(CollectionBuilder::new().build())));

        assert!(single.context().selection.get().is_empty());

        let mut all = service(
            Props::new()
                .id("all")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::All),
        );

        drop(all.send(Event::UpdateItems(CollectionBuilder::new().build())));

        assert_eq!(*all.context().selection.get(), selection::Set::All);

        let mut disabled_anchor = service(
            Props::new()
                .id("disabled-anchor")
                .disabled_keys(BTreeSet::from([key("bravo")])),
        );

        disabled_anchor.context_mut().highlighted_key = Some(key("bravo"));
        disabled_anchor.context_mut().selection_state.anchor_key = Some(key("bravo"));

        drop(disabled_anchor.send(Event::UpdateItems(collection())));

        assert_eq!(disabled_anchor.context().highlighted_key, None);
        assert_eq!(disabled_anchor.context().selection_state.anchor_key, None);

        let mut removed_refs = service(Props::new().id("removed-refs"));

        removed_refs.context_mut().highlighted_key = Some(key("ghost"));
        removed_refs.context_mut().selection_state.anchor_key = Some(key("ghost"));

        drop(removed_refs.send(Event::UpdateItems(collection())));

        assert_eq!(removed_refs.context().highlighted_key, None);
        assert_eq!(removed_refs.context().selection_state.anchor_key, None);

        let mut valid_refs = service(Props::new().id("valid-refs"));

        valid_refs.context_mut().highlighted_key = Some(key("alpha"));
        valid_refs.context_mut().selection_state.anchor_key = Some(key("alpha"));

        drop(valid_refs.send(Event::UpdateItems(collection())));

        assert_eq!(valid_refs.context().highlighted_key, Some(key("alpha")));
        assert_eq!(
            valid_refs.context().selection_state.anchor_key,
            Some(key("alpha"))
        );
    }

    #[test]
    fn keydown_routes_navigation_selection_and_rtl_arrows() {
        let captured = RefCell::new(Vec::new());

        let mut service = service(
            Props::new()
                .id("lb")
                .orientation(ars_core::Orientation::Horizontal)
                .dir(Direction::Rtl),
        );

        service.context_mut().highlighted_key = Some(key("alpha"));

        let send = |event| captured.borrow_mut().push(event);

        let api = service.connect(&send);

        api.on_focus(true);
        api.on_blur();
        api.on_item_click(key("alpha"));
        api.on_item_hover(key("bravo"));
        api.on_item_leave();
        api.on_keydown(
            &keyboard(KeyboardKey::ArrowRight, None),
            false,
            false,
            false,
        );
        api.on_keydown(&keyboard(KeyboardKey::ArrowLeft, None), false, false, false);
        api.on_keydown(&keyboard(KeyboardKey::ArrowDown, None), true, false, false);
        api.on_keydown(&keyboard(KeyboardKey::Home, None), false, false, false);
        api.on_keydown(&keyboard(KeyboardKey::End, None), false, false, false);
        api.on_keydown(&keyboard(KeyboardKey::PageUp, None), false, false, false);
        api.on_keydown(&keyboard(KeyboardKey::PageDown, None), false, false, false);
        api.on_keydown(&keyboard(KeyboardKey::Enter, None), false, false, false);
        api.on_keydown(&keyboard(KeyboardKey::Escape, None), false, false, false);
        api.on_keydown(
            &keyboard(KeyboardKey::Unidentified, Some('a')),
            false,
            false,
            false,
        );
        api.on_keydown(
            &keyboard(KeyboardKey::Unidentified, Some('a')),
            false,
            true,
            false,
        );
        api.on_keydown(
            &keyboard(KeyboardKey::Unidentified, Some('b')),
            false,
            false,
            false,
        );

        let mut composing = keyboard(KeyboardKey::Unidentified, Some('c'));

        composing.is_composing = true;

        api.on_keydown(&composing, false, false, false);

        assert_eq!(
            captured.into_inner(),
            vec![
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::ToggleItem(key("alpha")),
                Event::HighlightItem(Some(key("bravo"))),
                Event::HighlightItem(None),
                Event::HighlightPrev,
                Event::HighlightNext,
                Event::ExtendSelection(key("bravo")),
                Event::HighlightItem(Some(key("bravo"))),
                Event::HighlightFirst,
                Event::HighlightLast,
                Event::HighlightPageUp,
                Event::HighlightPageDown,
                Event::ToggleItem(key("alpha")),
                Event::DeselectAll,
                Event::SelectAll,
            ]
        );
    }

    #[test]
    fn set_props_syncs_context_backed_listbox_fields() {
        let mut listbox = service(Props::new().id("lb"));

        drop(
            listbox.set_props(
                Props::new()
                    .id("lb-next")
                    .disabled(true)
                    .required(true)
                    .invalid(true)
                    .orientation(ars_core::Orientation::Horizontal)
                    .dir(Direction::Rtl)
                    .loop_focus(false)
                    .loading(true)
                    .on_load_more(Callback::new_void(|| {}))
                    .selection_mode(selection::Mode::Multiple)
                    .disabled_behavior(DisabledBehavior::FocusOnly)
                    .disabled_keys(BTreeSet::from([key("bravo")]))
                    .value(selection::Set::Single(key("alpha"))),
            ),
        );

        assert!(listbox.context().disabled);
        assert!(listbox.context().required);
        assert!(listbox.context().invalid);
        assert_eq!(
            listbox.context().orientation,
            ars_core::Orientation::Horizontal
        );
        assert_eq!(listbox.context().dir, Direction::Rtl);
        assert!(!listbox.context().loop_focus);
        assert!(listbox.context().loading);
        assert_eq!(
            listbox.context().selection_state.mode,
            selection::Mode::Multiple
        );
        assert_eq!(
            listbox.context().selection_state.disabled_behavior,
            DisabledBehavior::FocusOnly
        );
        assert_eq!(
            *listbox.context().selection.get(),
            selection::Set::Single(key("alpha"))
        );

        assert_eq!(listbox.context().ids.id(), "lb-next");

        let attrs = listbox.connect(&|_| {}).content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("lb-next-content"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Required)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert!(listbox.connect(&|_| {}).loading_sentinel_attrs().is_some());
    }

    #[test]
    fn initial_selection_is_normalized_for_selection_mode() {
        let single = service(
            Props::new()
                .id("single")
                .selection_mode(selection::Mode::Single)
                .default_value(selection::Set::Multiple(BTreeSet::from([
                    key("alpha"),
                    key("bravo"),
                ]))),
        );

        assert_eq!(
            *single.context().selection.get(),
            selection::Set::Single(key("alpha"))
        );
        assert_eq!(
            single.context().selection_state.selected_keys,
            selection::Set::Single(key("alpha"))
        );

        let none = service(
            Props::new()
                .id("none")
                .selection_mode(selection::Mode::None)
                .value(selection::Set::Single(key("alpha"))),
        );

        assert_eq!(*none.context().selection.get(), selection::Set::Empty);
        assert_eq!(
            none.context().selection_state.selected_keys,
            selection::Set::Empty
        );
    }

    #[test]
    fn prop_sync_normalizes_selection_for_next_mode() {
        let mut listbox = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Multiple(BTreeSet::from([
                    key("alpha"),
                    key("bravo"),
                ]))),
        );

        drop(
            listbox.set_props(
                Props::new()
                    .id("lb")
                    .selection_mode(selection::Mode::Single),
            ),
        );

        assert_eq!(
            *listbox.context().selection.get(),
            selection::Set::Single(key("alpha"))
        );
        assert_eq!(
            listbox.context().selection_state.selected_keys,
            selection::Set::Single(key("alpha"))
        );

        drop(listbox.set_props(Props::new().id("lb").selection_mode(selection::Mode::None)));

        assert_eq!(*listbox.context().selection.get(), selection::Set::Empty);
        assert_eq!(
            listbox.context().selection_state.selected_keys,
            selection::Set::Empty
        );
    }

    #[test]
    fn controlled_selection_pruning_keeps_binding_and_state_in_sync() {
        let mut listbox = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple)
                .value(selection::Set::Multiple(BTreeSet::from([
                    key("alpha"),
                    key("bravo"),
                ]))),
        );

        drop(listbox.send(Event::UpdateItems(single_item_collection())));

        assert_eq!(
            *listbox.context().selection.get(),
            selection::Set::Multiple(BTreeSet::from([key("alpha")]))
        );
        assert_eq!(
            listbox.context().selection_state.selected_keys,
            *listbox.context().selection.get()
        );
    }

    #[test]
    fn focus_blur_disallow_empty_and_connect_api_edges_are_observable() {
        let mut listbox = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Single(key("alpha")))
                .disallow_empty_selection(true),
        );

        drop(listbox.send(Event::Focus { is_keyboard: true }));

        assert_eq!(*listbox.state(), super::State::Focused);
        assert!(listbox.context().focused);
        assert!(listbox.context().focus_visible);

        drop(listbox.send(Event::Focus { is_keyboard: false }));

        assert!(listbox.context().focused);
        assert!(!listbox.context().focus_visible);

        drop(listbox.send(Event::HighlightLast));

        assert_eq!(listbox.context().highlighted_key, Some(key("charlie")));

        drop(listbox.send(Event::HighlightPrev));

        assert_eq!(listbox.context().highlighted_key, Some(key("bravo")));

        drop(listbox.send(Event::DeselectItem(key("alpha"))));

        assert!(listbox.context().selection.get().contains(&key("alpha")));

        drop(listbox.send(Event::ToggleItem(key("alpha"))));

        assert!(listbox.context().selection.get().contains(&key("alpha")));

        drop(listbox.send(Event::SelectItem(key("alpha"))));

        assert!(listbox.context().selection.get().contains(&key("alpha")));

        drop(listbox.send(Event::DeselectAll));

        assert!(listbox.context().selection.get().contains(&key("alpha")));

        drop(listbox.send(Event::Blur));

        assert_eq!(*listbox.state(), super::State::Idle);
        assert!(!listbox.context().focused);
        assert!(!listbox.context().focus_visible);

        let api = listbox.connect(&|_| {});

        assert_eq!(api.empty_label(), "No options available");
        assert_eq!(api.count_announcement(3), "3 options available");
        assert_eq!(api.items().count(), 3);
        assert_eq!(
            api.part_attrs(super::Part::Content).get(&HtmlAttr::Role),
            Some("listbox")
        );

        let mut two_selected = service(
            Props::new()
                .id("two")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Multiple(BTreeSet::from([
                    key("alpha"),
                    key("bravo"),
                ])))
                .disallow_empty_selection(true),
        );

        drop(two_selected.send(Event::ToggleItem(key("alpha"))));

        assert!(
            !two_selected
                .context()
                .selection
                .get()
                .contains(&key("alpha"))
        );
        assert!(
            two_selected
                .context()
                .selection
                .get()
                .contains(&key("bravo"))
        );

        let mut active_disabled = service(
            Props::new()
                .id("active-disabled")
                .disabled_keys(BTreeSet::from([key("bravo")])),
        );

        active_disabled.context_mut().highlighted_key = Some(key("bravo"));

        let api = active_disabled.connect(&|_| {});

        assert!(
            !api.content_attrs()
                .contains(&HtmlAttr::Aria(AriaAttr::ActiveDescendant))
        );
    }

    #[test]
    fn item_activated_invokes_action_without_changing_selection() {
        let activated = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&activated);

        let mut service = service(Props::new().id("lb").on_action(Callback::new(
            move |key: Key| {
                captured
                    .lock()
                    .expect("activation capture poisoned")
                    .push(key);
            },
        )));

        drop(service.send(Event::ItemActivated(key("bravo"))));

        assert_eq!(
            activated
                .lock()
                .expect("activation capture poisoned")
                .as_slice(),
            &[key("bravo")]
        );
        assert!(service.context().selection.get().is_empty());
    }

    #[test]
    fn item_activated_ignores_disabled_or_stale_targets() {
        let activated = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&activated);

        let mut service = service(
            Props::new()
                .id("lb")
                .disabled_keys(BTreeSet::from([key("bravo")]))
                .on_action(Callback::new(move |key: Key| {
                    captured
                        .lock()
                        .expect("activation capture poisoned")
                        .push(key);
                })),
        );

        drop(service.send(Event::ItemActivated(key("bravo"))));
        drop(service.send(Event::ItemActivated(key("stale"))));
        drop(service.set_props(Props::new().id("lb").disabled(true)));
        drop(service.send(Event::ItemActivated(key("alpha"))));

        assert!(
            activated
                .lock()
                .expect("activation capture poisoned")
                .is_empty()
        );
    }

    #[test]
    fn extend_selection_recomputes_range_from_anchor() {
        let mut listbox = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple),
        );

        drop(listbox.send(Event::SelectItem(key("alpha"))));
        drop(listbox.send(Event::ExtendSelection(key("charlie"))));

        assert_eq!(
            *listbox.context().selection.get(),
            selection::Set::Multiple(BTreeSet::from([key("alpha"), key("bravo"), key("charlie")]))
        );

        drop(listbox.send(Event::ExtendSelection(key("bravo"))));

        assert_eq!(
            *listbox.context().selection.get(),
            selection::Set::Multiple(BTreeSet::from([key("alpha"), key("bravo")]))
        );
    }

    #[test]
    fn listbox_attrs_snapshot_all_parts_and_state_branches() {
        let mut service = service(
            Props::new()
                .id("lb")
                .selection_mode(selection::Mode::Multiple)
                .required(true)
                .invalid(true)
                .on_load_more(Callback::new_void(|| {}))
                .loading(true),
        );

        service.context_mut().has_description = true;

        drop(service.send(Event::Focus { is_keyboard: true }));
        drop(service.send(Event::SelectItem(key("bravo"))));
        drop(service.send(Event::HighlightItem(Some(key("bravo")))));

        let api = service.connect(&|_| {});

        insta::assert_snapshot!("listbox_root_invalid", snapshot_attrs(&api.root_attrs()));
        insta::assert_snapshot!("listbox_label", snapshot_attrs(&api.label_attrs()));
        insta::assert_snapshot!(
            "listbox_content_multi_invalid_active",
            snapshot_attrs(&api.content_attrs())
        );
        insta::assert_snapshot!(
            "listbox_item_selected_highlighted",
            snapshot_attrs(&api.item_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "listbox_item_text",
            snapshot_attrs(&api.item_text_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "listbox_item_indicator_selected",
            snapshot_attrs(&api.item_indicator_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "listbox_item_group",
            snapshot_attrs(&api.item_group_attrs(&key("group")))
        );
        insta::assert_snapshot!(
            "listbox_item_group_label",
            snapshot_attrs(&api.item_group_label_attrs(&key("group")))
        );
        insta::assert_snapshot!(
            "listbox_description",
            snapshot_attrs(&api.description_attrs())
        );
        insta::assert_snapshot!(
            "listbox_error_message",
            snapshot_attrs(&api.error_message_attrs())
        );
        insta::assert_snapshot!(
            "listbox_loading_sentinel_loading",
            snapshot_attrs(&api.loading_sentinel_attrs().expect("sentinel attrs"))
        );
    }
}
