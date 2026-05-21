//! Select selection component machine.

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
    ConnectApi, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

use crate::overlay::positioning::PositioningOptions;

/// Message function used by Select single-locale messages.
pub type LocaleMessage = dyn Fn(&Locale) -> String + Send + Sync;

/// Message function used by Select selected-count announcements.
pub type SelectedCountMessage = dyn Fn(usize, &Locale) -> String + Send + Sync;

/// Open-state change callback.
pub type OpenChangeCallback = dyn Fn(bool) + Send + Sync;

/// User-facing payload for Select options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// Human-readable option label.
    pub label: String,
}

/// Select machine states.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The dropdown is closed.
    #[default]
    Closed,

    /// The dropdown is open.
    Open,
}

/// Events accepted by the Select machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Open the dropdown.
    Open,

    /// Close the dropdown.
    Close,

    /// Toggle the dropdown.
    Toggle,

    /// Select an item by key.
    SelectItem(Key),

    /// Deselect an item by key.
    DeselectItem(Key),

    /// Highlight an item by key.
    HighlightItem(Option<Key>),

    /// Highlight the first enabled item.
    HighlightFirst,

    /// Highlight the last enabled item.
    HighlightLast,

    /// Highlight the next enabled item.
    HighlightNext,

    /// Highlight the previous enabled item.
    HighlightPrev,

    /// Search by type-ahead character and timestamp.
    TypeaheadSearch(char, u64),

    /// Mark IME composition active.
    CompositionStart,

    /// Mark IME composition inactive.
    CompositionEnd,

    /// Trigger focus state.
    Focus {
        /// Whether focus came from keyboard modality.
        is_keyboard: bool,
    },

    /// Blur the component.
    Blur,

    /// Close due to an outside click.
    ClickOutside,

    /// Clear selection.
    Clear,

    /// Clear the type-ahead buffer.
    ClearTypeahead,

    /// Synchronize context-backed fields from updated props.
    SyncProps,

    /// Replace the item collection dynamically.
    UpdateItems(StaticCollection<Item>),
}

/// Select machine context.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Resolved locale for message formatting.
    pub locale: Locale,

    /// Item collection.
    pub items: StaticCollection<Item>,

    /// Controlled or uncontrolled selected-key binding.
    pub selection: Bindable<selection::Set>,

    /// Full collection selection state.
    pub selection_state: selection::State,

    /// Highlighted item key.
    pub highlighted_key: Option<Key>,

    /// Type-ahead buffer state.
    pub typeahead: typeahead::State,

    /// Whether the dropdown is open.
    pub open: bool,

    /// Whether the select is disabled.
    pub disabled: bool,

    /// Whether the select is readonly.
    pub readonly: bool,

    /// Whether the select is required.
    pub required: bool,

    /// Whether the select is invalid.
    pub invalid: bool,

    /// Whether multiple selections are allowed.
    pub multiple: bool,

    /// Whether the trigger has focus.
    pub focused: bool,

    /// Whether focus-visible styling should be emitted.
    pub focus_visible: bool,

    /// Form field name.
    pub name: Option<String>,

    /// Whether keyboard navigation wraps at boundaries.
    pub loop_focus: bool,

    /// Whether IME composition is active.
    pub is_composing: bool,

    /// Whether a description element is rendered.
    pub has_description: bool,

    /// Stable component ID derivation helper.
    pub ids: ComponentIds,

    /// Resolved localized messages.
    pub messages: Messages,
}

/// Props for the Select component.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Controlled selected keys.
    pub value: Option<selection::Set>,

    /// Initial uncontrolled selected keys.
    pub default_value: selection::Set,

    /// Whether multiple items can be selected.
    pub multiple: bool,

    /// Selection mode.
    pub selection_mode: selection::Mode,

    /// Selection behavior.
    pub selection_behavior: selection::Behavior,

    /// Disabled-item behavior.
    pub disabled_behavior: DisabledBehavior,

    /// Whether the select is disabled.
    pub disabled: bool,

    /// Whether the select is readonly.
    pub readonly: bool,

    /// Whether the select is required.
    pub required: bool,

    /// Whether the select is invalid.
    pub invalid: bool,

    /// Item keys that should be disabled.
    pub disabled_keys: BTreeSet<Key>,

    /// Form field name.
    pub name: Option<String>,

    /// Associated form id.
    pub form: Option<String>,

    /// Placeholder text override.
    pub placeholder: Option<String>,

    /// Whether selecting an item closes the dropdown.
    pub close_on_select: Option<bool>,

    /// Whether keyboard navigation wraps at boundaries.
    pub loop_focus: bool,

    /// Positioning options for adapter-resolved dropdown placement.
    pub positioning: PositioningOptions,

    /// Hidden input autocomplete value.
    pub autocomplete: Option<String>,

    /// Whether deselecting the final selected item is blocked.
    pub disallow_empty_selection: bool,

    /// Whether a multi-select trigger should use a multi-line layout.
    pub multi_line_trigger: bool,

    /// Callback invoked when the dropdown open state changes.
    pub on_open_change: Option<Callback<OpenChangeCallback>>,

    /// Whether adapters should virtualize option rendering.
    pub virtualized: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: selection::Set::Empty,
            multiple: false,
            selection_mode: selection::Mode::Single,
            selection_behavior: selection::Behavior::Toggle,
            disabled_behavior: DisabledBehavior::default(),
            disabled: false,
            readonly: false,
            required: false,
            invalid: false,
            disabled_keys: BTreeSet::new(),
            name: None,
            form: None,
            placeholder: None,
            close_on_select: None,
            loop_focus: true,
            positioning: PositioningOptions::default(),
            autocomplete: None,
            disallow_empty_selection: false,
            multi_line_trigger: false,
            on_open_change: None,
            virtualized: false,
        }
    }
}

impl Props {
    /// Returns default Select props.
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

    /// Sets [`Self::value`].
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

    /// Sets [`Self::multiple`].
    #[must_use]
    pub const fn multiple(mut self, value: bool) -> Self {
        self.multiple = value;
        self
    }

    /// Sets [`Self::selection_mode`].
    #[must_use]
    pub const fn selection_mode(mut self, value: selection::Mode) -> Self {
        self.selection_mode = value;
        self
    }

    /// Sets [`Self::selection_behavior`].
    #[must_use]
    pub const fn selection_behavior(mut self, value: selection::Behavior) -> Self {
        self.selection_behavior = value;
        self
    }

    /// Sets [`Self::disabled_behavior`].
    #[must_use]
    pub const fn disabled_behavior(mut self, value: DisabledBehavior) -> Self {
        self.disabled_behavior = value;
        self
    }

    /// Sets [`Self::disabled`].
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`Self::readonly`].
    #[must_use]
    pub const fn readonly(mut self, value: bool) -> Self {
        self.readonly = value;
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

    /// Sets [`Self::form`].
    #[must_use]
    pub fn form(mut self, value: impl Into<String>) -> Self {
        self.form = Some(value.into());
        self
    }

    /// Sets [`Self::placeholder`].
    #[must_use]
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = Some(value.into());
        self
    }

    /// Sets [`Self::close_on_select`].
    #[must_use]
    pub const fn close_on_select(mut self, value: bool) -> Self {
        self.close_on_select = Some(value);
        self
    }

    /// Sets [`Self::loop_focus`].
    #[must_use]
    pub const fn loop_focus(mut self, value: bool) -> Self {
        self.loop_focus = value;
        self
    }

    /// Sets [`Self::positioning`].
    #[must_use]
    pub fn positioning(mut self, value: PositioningOptions) -> Self {
        self.positioning = value;
        self
    }

    /// Sets [`Self::autocomplete`].
    #[must_use]
    pub fn autocomplete(mut self, value: impl Into<String>) -> Self {
        self.autocomplete = Some(value.into());
        self
    }

    /// Sets [`Self::disallow_empty_selection`].
    #[must_use]
    pub const fn disallow_empty_selection(mut self, value: bool) -> Self {
        self.disallow_empty_selection = value;
        self
    }

    /// Sets [`Self::multi_line_trigger`].
    #[must_use]
    pub const fn multi_line_trigger(mut self, value: bool) -> Self {
        self.multi_line_trigger = value;
        self
    }

    /// Sets [`Self::on_open_change`].
    #[must_use]
    pub fn on_open_change(mut self, value: Callback<OpenChangeCallback>) -> Self {
        self.on_open_change = Some(value);
        self
    }

    /// Sets [`Self::virtualized`].
    #[must_use]
    pub const fn virtualized(mut self, value: bool) -> Self {
        self.virtualized = value;
        self
    }
}

/// Localized messages for Select.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Placeholder text shown when nothing is selected.
    pub placeholder: MessageFn<LocaleMessage>,

    /// Empty-state text.
    pub empty: MessageFn<LocaleMessage>,

    /// Multi-select selected-count announcement.
    pub selected_count: MessageFn<SelectedCountMessage>,

    /// Clear-trigger label.
    pub clear_label: MessageFn<LocaleMessage>,

    /// Trigger fallback label.
    pub trigger_label: MessageFn<LocaleMessage>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            placeholder: MessageFn::static_str("Select an option"),
            empty: MessageFn::static_str("No options available"),
            selected_count: MessageFn::new(|count: usize, _locale: &Locale| match count {
                1 => "1 option selected".to_string(),
                count => format!("{count} options selected"),
            }),
            clear_label: MessageFn::static_str("Clear selection"),
            trigger_label: MessageFn::static_str("Open dropdown"),
        }
    }
}

impl ComponentMessages for Messages {}

/// Select anatomy parts.
#[derive(ComponentPart)]
#[scope = "select"]
pub enum Part {
    /// Root container.
    Root,

    /// Visible label.
    Label,

    /// Control wrapper.
    Control,

    /// Dropdown trigger.
    Trigger,

    /// Selected value text.
    ValueText,

    /// Visual dropdown indicator.
    Indicator,

    /// Clear-selection trigger.
    ClearTrigger,

    /// Positioned dropdown wrapper.
    Positioner,

    /// Listbox content.
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

    /// Option indicator.
    ItemIndicator {
        /// Item key.
        key: Key,
    },

    /// Hidden form input.
    HiddenInput,

    /// Help or description text.
    Description,

    /// Validation error message.
    ErrorMessage,

    /// Empty option-list status message.
    EmptyState,
}

/// Machine for Select.
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
        let selected = props
            .value
            .clone()
            .unwrap_or_else(|| props.default_value.clone());

        let (effective_multiple, effective_mode) = effective_selection_config(props);
        let selected = normalize_selection_for_mode(selected, effective_mode);

        let mut selection_state = selection::State::new(effective_mode, props.selection_behavior);

        selection_state.disabled_behavior = props.disabled_behavior;
        selection_state.disabled_keys = props.disabled_keys.clone();
        selection_state.selected_keys = selected.clone();

        (
            State::Closed,
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
                open: false,
                disabled: props.disabled,
                readonly: props.readonly,
                required: props.required,
                invalid: props.invalid,
                multiple: effective_multiple,
                focused: false,
                focus_visible: false,
                name: props.name.clone(),
                loop_focus: props.loop_focus,
                is_composing: false,
                has_description: false,
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
                Event::Open
                    | Event::Toggle
                    | Event::SelectItem(_)
                    | Event::DeselectItem(_)
                    | Event::Clear
                    | Event::HighlightFirst
                    | Event::HighlightLast
                    | Event::HighlightNext
                    | Event::HighlightPrev
                    | Event::HighlightItem(_)
                    | Event::TypeaheadSearch(_, _)
            )
        {
            return None;
        }

        if ctx.readonly
            && matches!(
                event,
                Event::SelectItem(_)
                    | Event::DeselectItem(_)
                    | Event::Clear
                    | Event::TypeaheadSearch(_, _)
            )
        {
            return None;
        }

        match (state, event) {
            (State::Closed, Event::Open) => {
                let highlighted = ctx
                    .selection
                    .get()
                    .first()
                    .cloned()
                    .or_else(|| first_key(ctx));

                Some(open_plan(highlighted, props))
            }

            (State::Open, Event::Close | Event::ClickOutside) => Some(close_plan(props)),

            (_, Event::Toggle) => {
                if ctx.open {
                    Self::transition(state, &Event::Close, ctx, props)
                } else {
                    Self::transition(state, &Event::Open, ctx, props)
                }
            }

            (State::Open, Event::SelectItem(key)) => select_item_plan(ctx, props, key.clone()),

            (_, Event::DeselectItem(key)) => {
                if !ctx.multiple {
                    return None;
                }
                if !is_selectable_key(ctx, key) {
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

            (_, Event::Clear) => {
                if props.disallow_empty_selection && !ctx.selection.get().is_empty() {
                    return None;
                }

                Some(apply_selection_plan(ctx.selection_state.clear()))
            }

            (State::Open, Event::HighlightItem(key)) => {
                let key = key
                    .clone()
                    .filter(|key| ctx.items.contains_key(key) && is_focusable_key(ctx, key));

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightFirst) => {
                let key = first_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightLast) => {
                let key = last_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightNext) => {
                let key = next_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightPrev) => {
                let key = prev_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (_, Event::TypeaheadSearch(ch, now_ms)) => {
                if ctx.is_composing {
                    return None;
                }

                let (typeahead, found) = process_typeahead(ctx, *ch, *now_ms);

                let highlighted_key = found.or_else(|| ctx.highlighted_key.clone());

                if ctx.open {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.typeahead = typeahead;
                        ctx.highlighted_key = highlighted_key;
                    }))
                } else {
                    Some(open_plan_with_typeahead(highlighted_key, typeahead, props))
                }
            }

            (_, Event::Focus { is_keyboard }) => {
                let focus_visible = *is_keyboard;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = focus_visible;
                }))
            }

            (_, Event::Blur) => {
                if ctx.open {
                    Some(close_plan(props).apply(|ctx: &mut Context| {
                        ctx.focused = false;
                        ctx.focus_visible = false;
                    }))
                } else {
                    Some(TransitionPlan::context_only(|ctx: &mut Context| {
                        ctx.focused = false;
                        ctx.focus_visible = false;
                    }))
                }
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

/// API for deriving Select attributes and dispatching Select events.
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

    /// Dispatches a trigger click.
    pub fn on_trigger_click(&self) {
        (self.send)(Event::Toggle);
    }

    /// Dispatches a trigger focus.
    pub fn on_trigger_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches a trigger blur.
    pub fn on_trigger_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches an item click.
    pub fn on_item_click(&self, key: Key) {
        (self.send)(Event::SelectItem(key));
    }

    /// Dispatches an item hover.
    pub fn on_item_hover(&self, key: Key) {
        (self.send)(Event::HighlightItem(Some(key)));
    }

    /// Dispatches an item leave.
    pub fn on_item_leave(&self) {
        (self.send)(Event::HighlightItem(None));
    }

    /// Dispatches a clear click.
    pub fn on_clear_click(&self) {
        (self.send)(Event::Clear);
    }

    /// Returns display text for the first selected item.
    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        self.ctx
            .selection
            .get()
            .first()
            .and_then(|key| self.ctx.items.text_value_of(key))
    }

    /// Iterates all collection nodes for rendering.
    pub fn items(&self) -> impl Iterator<Item = &Node<Item>> {
        self.ctx.items.nodes()
    }

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs.set(
            HtmlAttr::Data("ars-state"),
            if self.ctx.open { "open" } else { "closed" },
        );

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
        attrs.set(HtmlAttr::For, self.ctx.ids.part("trigger"));

        attrs
    }

    /// Attributes for the control wrapper.
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        part_attrs(&Part::Control)
    }

    /// Attributes for the trigger element.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Trigger);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("trigger"))
            .set(HtmlAttr::Role, "combobox")
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.ctx.open { "true" } else { "false" },
            )
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "listbox")
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            )
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.part("label"),
            );

        if let Some(key) = valid_highlight(self.ctx) {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::ActiveDescendant),
                self.ctx.ids.item("item", key),
            );
        }

        if self.ctx.selection_state.mode == selection::Mode::Multiple {
            let count = self.ctx.selection.get().len();

            if count > 0 {
                attrs.set(
                    HtmlAttr::Aria(AriaAttr::Description),
                    (self.ctx.messages.selected_count)(count, &self.ctx.locale),
                );
            }
        }

        if self.ctx.required {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), "true");
        }

        if self.ctx.invalid {
            attrs.set(HtmlAttr::Aria(AriaAttr::Invalid), "true");
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

        if self.ctx.disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
        }

        if self.ctx.readonly {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), "true");
        }

        attrs.set(
            HtmlAttr::TabIndex,
            if self.ctx.disabled { "-1" } else { "0" },
        );

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Dispatches trigger keydown events.
    pub fn on_trigger_keydown(&self, data: &KeyboardEventData, ctrl: bool, meta: bool) {
        self.on_trigger_keydown_impl(data, ctrl, meta, None);
    }

    /// Dispatches trigger keydown events with an adapter-provided monotonic timestamp.
    pub fn on_trigger_keydown_at(
        &self,
        data: &KeyboardEventData,
        ctrl: bool,
        meta: bool,
        now_ms: u64,
    ) {
        self.on_trigger_keydown_impl(data, ctrl, meta, Some(now_ms));
    }

    fn on_trigger_keydown_impl(
        &self,
        data: &KeyboardEventData,
        ctrl: bool,
        meta: bool,
        now_ms: Option<u64>,
    ) {
        match data.key {
            KeyboardKey::ArrowDown if self.ctx.open => (self.send)(Event::HighlightNext),

            KeyboardKey::ArrowUp if self.ctx.open => (self.send)(Event::HighlightPrev),

            KeyboardKey::ArrowDown | KeyboardKey::ArrowUp => (self.send)(Event::Open),

            KeyboardKey::Enter | KeyboardKey::Space if self.ctx.open => {
                if let Some(key) = &self.ctx.highlighted_key {
                    (self.send)(Event::SelectItem(key.clone()));
                }
            }

            KeyboardKey::Enter | KeyboardKey::Space => (self.send)(Event::Toggle),

            KeyboardKey::Escape if self.ctx.open => (self.send)(Event::Close),

            _ if data.character.is_some() && !ctrl && !meta && !data.is_composing => {
                (self.send)(Event::TypeaheadSearch(
                    data.character.expect("checked"),
                    typeahead_time(now_ms, &self.ctx.typeahead),
                ));
            }

            _ => {}
        }
    }

    /// Attributes for selected value text.
    #[must_use]
    pub fn value_text_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::ValueText);

        if self.ctx.selection.get().is_empty() {
            attrs.set_bool(HtmlAttr::Data("ars-placeholder"), true);
        }

        attrs
    }

    /// Placeholder text.
    #[must_use]
    pub fn placeholder_text(&self) -> String {
        self.props
            .placeholder
            .clone()
            .unwrap_or_else(|| (self.ctx.messages.placeholder)(&self.ctx.locale))
    }

    /// Fallback trigger label.
    #[must_use]
    pub fn trigger_label(&self) -> String {
        (self.ctx.messages.trigger_label)(&self.ctx.locale)
    }

    /// Attributes for the indicator element.
    #[must_use]
    pub fn indicator_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Indicator);

        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true").set(
            HtmlAttr::Data("ars-state"),
            if self.ctx.open { "open" } else { "closed" },
        );

        attrs
    }

    /// Attributes for the clear trigger.
    #[must_use]
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::ClearTrigger);

        attrs
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.clear_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::TabIndex, "-1");

        attrs
    }

    /// Attributes for the positioner.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Positioner);

        attrs.set(
            HtmlAttr::Data("ars-placement"),
            self.props.positioning.placement.to_string(),
        );

        attrs
    }

    /// Attributes for the listbox content.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Content);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, "listbox");

        if self.ctx.multiple {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }

        attrs.set(
            HtmlAttr::Aria(AriaAttr::LabelledBy),
            self.ctx.ids.part("label"),
        );

        if let Some(key) = valid_highlight(self.ctx) {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::ActiveDescendant),
                self.ctx.ids.item("item", key),
            );
        }

        attrs
    }

    /// Empty-state text.
    #[must_use]
    pub fn empty_text(&self) -> String {
        (self.ctx.messages.empty)(&self.ctx.locale)
    }

    /// Attributes for the empty-state status element.
    #[must_use]
    pub fn empty_state_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::EmptyState);

        attrs
            .set(HtmlAttr::Role, "status")
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "true");

        attrs
    }

    /// Dispatches content keydown events.
    pub fn on_content_keydown(&self, data: &KeyboardEventData, ctrl: bool, meta: bool) {
        self.on_content_keydown_impl(data, ctrl, meta, None);
    }

    /// Dispatches content keydown events with an adapter-provided monotonic timestamp.
    pub fn on_content_keydown_at(
        &self,
        data: &KeyboardEventData,
        ctrl: bool,
        meta: bool,
        now_ms: u64,
    ) {
        self.on_content_keydown_impl(data, ctrl, meta, Some(now_ms));
    }

    fn on_content_keydown_impl(
        &self,
        data: &KeyboardEventData,
        ctrl: bool,
        meta: bool,
        now_ms: Option<u64>,
    ) {
        match data.key {
            KeyboardKey::ArrowDown => (self.send)(Event::HighlightNext),

            KeyboardKey::ArrowUp => (self.send)(Event::HighlightPrev),

            KeyboardKey::Home => (self.send)(Event::HighlightFirst),

            KeyboardKey::End => (self.send)(Event::HighlightLast),

            KeyboardKey::Enter | KeyboardKey::Space => {
                if let Some(key) = &self.ctx.highlighted_key {
                    (self.send)(Event::SelectItem(key.clone()));
                }
            }

            KeyboardKey::Escape => (self.send)(Event::Close),

            _ if data.character.is_some() && !ctrl && !meta && !data.is_composing => {
                (self.send)(Event::TypeaheadSearch(
                    data.character.expect("checked"),
                    typeahead_time(now_ms, &self.ctx.typeahead),
                ));
            }

            _ => {}
        }
    }

    /// Attributes for the hidden input.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::HiddenInput);

        attrs
            .set(HtmlAttr::Type, "hidden")
            .set(
                HtmlAttr::Value,
                serialize_selection(self.ctx.selection.get()),
            )
            .set(HtmlAttr::TabIndex, "-1")
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name);
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if let Some(value) = &self.props.autocomplete {
            attrs.set(HtmlAttr::AutoComplete, value.as_str());
        }

        if let Some(value) = &self.props.form {
            attrs.set(HtmlAttr::Form, value.as_str());
        }

        attrs
    }

    /// Attributes for the description.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Description);

        attrs.set(HtmlAttr::Id, self.ctx.ids.part("description"));

        attrs
    }

    /// Attributes for the error message.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::ErrorMessage);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

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

        if highlighted {
            attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true);
        }

        if disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if let Some(node) = self.ctx.items.get(key) {
            attrs.set(HtmlAttr::Data("ars-value"), node.text_value.as_str());
        }

        attrs
    }

    /// Attributes for item text.
    #[must_use]
    pub fn item_text_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemText {
            key: Key::default(),
        });

        attrs.set(HtmlAttr::Id, self.ctx.ids.item_part("item", key, "text"));

        attrs
    }

    /// Attributes for item indicator.
    #[must_use]
    pub fn item_indicator_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemIndicator {
            key: Key::default(),
        });

        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true").set(
            HtmlAttr::Data("ars-state"),
            if self.ctx.selection.get().contains(key) {
                "selected"
            } else {
                "unselected"
            },
        );

        attrs
    }

    /// Attributes for item group.
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

    /// Attributes for item group label.
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
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::ValueText => self.value_text_attrs(),
            Part::Indicator => self.indicator_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::ItemGroup { ref key } => self.item_group_attrs(key),
            Part::ItemGroupLabel { ref key } => self.item_group_label_attrs(key),
            Part::Item { ref key } => self.item_attrs(key),
            Part::ItemText { ref key } => self.item_text_attrs(key),
            Part::ItemIndicator { ref key } => self.item_indicator_attrs(key),
            Part::HiddenInput => self.hidden_input_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::EmptyState => self.empty_state_attrs(),
        }
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

fn open_plan(highlighted_key: Option<Key>, props: &Props) -> TransitionPlan<Machine> {
    let on_open_change = props.on_open_change.clone();

    TransitionPlan::to(State::Open).apply(move |ctx: &mut Context| {
        ctx.open = true;
        ctx.highlighted_key = highlighted_key;

        if let Some(callback) = &on_open_change {
            callback(true);
        }
    })
}

fn open_plan_with_typeahead(
    highlighted_key: Option<Key>,
    typeahead: typeahead::State,
    props: &Props,
) -> TransitionPlan<Machine> {
    let on_open_change = props.on_open_change.clone();

    TransitionPlan::to(State::Open).apply(move |ctx: &mut Context| {
        ctx.open = true;
        ctx.typeahead = typeahead;
        ctx.highlighted_key = highlighted_key;

        if let Some(callback) = &on_open_change {
            callback(true);
        }
    })
}

fn close_plan(props: &Props) -> TransitionPlan<Machine> {
    let on_open_change = props.on_open_change.clone();

    TransitionPlan::to(State::Closed).apply(move |ctx: &mut Context| {
        ctx.open = false;
        ctx.highlighted_key = None;
        ctx.typeahead = typeahead::State::default();

        if let Some(callback) = &on_open_change {
            callback(false);
        }
    })
}

fn select_item_plan(ctx: &Context, props: &Props, key: Key) -> Option<TransitionPlan<Machine>> {
    if !is_selectable_key(ctx, &key) {
        return None;
    }

    let next = if ctx.multiple {
        ctx.selection_state.toggle(key, &ctx.items)
    } else {
        ctx.selection_state.select(key)
    };
    let next = normalize_selection_state(next);

    if props.disallow_empty_selection && selection_is_empty(&next.selected_keys) {
        return None;
    }

    let selected = next.selected_keys.clone();
    let close = props.close_on_select.unwrap_or(!ctx.multiple);
    let on_open_change = props.on_open_change.clone();

    if close {
        Some(
            TransitionPlan::to(State::Closed).apply(move |ctx: &mut Context| {
                ctx.selection.set(selected.clone());
                ctx.selection_state = next;
                ctx.selection_state.selected_keys = ctx.selection.get().clone();
                ctx.open = false;
                ctx.highlighted_key = None;
                ctx.typeahead = typeahead::State::default();

                if let Some(callback) = &on_open_change {
                    callback(false);
                }
            }),
        )
    } else {
        Some(apply_selection_plan(next))
    }
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
        let (effective_multiple, effective_mode) = effective_selection_config(&props);

        ctx.disabled = props.disabled;
        ctx.readonly = props.readonly;
        ctx.required = props.required;
        ctx.invalid = props.invalid;
        ctx.multiple = effective_multiple;
        ctx.name = props.name.clone();
        ctx.loop_focus = props.loop_focus;
        ctx.ids = ComponentIds::from_id(&props.id);

        ctx.selection_state.mode = effective_mode;
        ctx.selection_state.behavior = props.selection_behavior;
        ctx.selection_state.disabled_behavior = props.disabled_behavior;
        ctx.selection_state.disabled_keys = props.disabled_keys.clone();

        if props.value.is_some() || ctx.selection.is_controlled() {
            ctx.selection.sync_controlled(
                props
                    .value
                    .clone()
                    .map(|value| normalize_selection_for_mode(value, effective_mode)),
            );
        }

        set_current_selection(
            ctx,
            normalize_selection_for_mode(ctx.selection.get().clone(), effective_mode),
        );
        ctx.selection_state = normalize_selection_state(ctx.selection_state.clone());
        invalidate_collection_references(ctx);
    })
}

fn effective_selection_config(props: &Props) -> (bool, selection::Mode) {
    let multiple = props.multiple || props.selection_mode == selection::Mode::Multiple;
    let mode = if multiple {
        selection::Mode::Multiple
    } else {
        props.selection_mode
    };

    (multiple, mode)
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

fn is_selectable_key(ctx: &Context, key: &Key) -> bool {
    ctx.items.get(key).is_some_and(Node::is_focusable) && !ctx.selection_state.is_disabled(key)
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

fn serialize_selection(selection: &selection::Set) -> String {
    match selection {
        selection::Set::Single(key) => key.to_string(),

        selection::Set::Multiple(keys) => keys
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),

        selection::Set::All => "all".to_string(),

        _ => String::new(),
    }
}

fn typeahead_time(now_ms: Option<u64>, state: &typeahead::State) -> u64 {
    now_ms.unwrap_or_else(|| state.last_key_time_ms.saturating_add(1))
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, sync::Arc};
    use core::cell::RefCell;
    use std::sync::Mutex;

    use ars_collections::{CollectionBuilder, DisabledBehavior, Key, selection};
    use ars_core::{AriaAttr, Callback, ConnectApi, Env, HtmlAttr, KeyboardKey, Service};
    use ars_interactions::KeyboardEventData;

    use super::{Event, Machine, Messages, Props, State};
    use crate::overlay::positioning::{Placement, PositioningOptions};

    fn key(value: &'static str) -> Key {
        Key::str(value)
    }

    fn keyboard(key: KeyboardKey, character: Option<char>) -> KeyboardEventData {
        KeyboardEventData {
            key,
            character,
            code: String::new(),
            shift_key: false,
            ctrl_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
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

    fn grouped_collection() -> ars_collections::StaticCollection<super::Item> {
        CollectionBuilder::new()
            .section(key("group"), "Group")
            .item(
                key("alpha"),
                "Alpha",
                super::Item {
                    label: "Alpha".into(),
                },
            )
            .build()
    }

    fn make_service(props: Props) -> Service<Machine> {
        let mut service = Service::<Machine>::new(props, &Env::default(), &Messages::default());

        drop(service.send(Event::UpdateItems(collection())));

        service
    }

    fn snapshot_attrs(attrs: &ars_core::AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn builder_methods_preserve_public_props_contract() {
        let opened = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&opened);

        let positioning = PositioningOptions {
            placement: Placement::TopStart,
            ..PositioningOptions::default()
        };

        let props = Props::new()
            .id("sel")
            .value(selection::Set::Single(key("alpha")))
            .default_value(selection::Set::Single(key("bravo")))
            .placeholder("Choose")
            .positioning(positioning.clone())
            .on_open_change(Callback::new(move |open| {
                captured.lock().expect("open capture poisoned").push(open);
            }));

        assert_eq!(props.value, Some(selection::Set::Single(key("alpha"))));
        assert_eq!(props.default_value, selection::Set::Single(key("bravo")));
        assert_eq!(props.placeholder.as_deref(), Some("Choose"));
        assert_eq!(props.positioning.placement, Placement::TopStart);
        assert!(props.on_open_change.is_some());

        let mut service = make_service(props);

        drop(service.send(Event::Open));
        drop(service.send(Event::Close));

        assert_eq!(
            opened.lock().expect("open capture poisoned").as_slice(),
            &[true, false]
        );
    }

    #[test]
    fn trigger_content_and_selection_attrs_match_select_contract() {
        let mut service = make_service(Props::new().id("sel").name("country"));

        drop(service.send(Event::Open));
        drop(service.send(Event::HighlightItem(Some(key("bravo")))));
        assert_eq!(
            service
                .connect(&|_| {})
                .content_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant)),
            Some("sel-item-bravo")
        );
        drop(service.send(Event::SelectItem(key("bravo"))));

        let api = service.connect(&|_| {});

        let trigger = api.trigger_attrs();
        let content = api.content_attrs();
        let hidden = api.hidden_input_attrs();

        assert_eq!(trigger.get(&HtmlAttr::Role), Some("combobox"));
        assert_eq!(
            trigger.get(&HtmlAttr::Aria(AriaAttr::HasPopup)),
            Some("listbox")
        );
        assert_eq!(content.get(&HtmlAttr::Role), Some("listbox"));
        assert_eq!(api.selected_text(), Some("Bravo"));
        assert_eq!(hidden.get(&HtmlAttr::Name), Some("country"));
        assert_eq!(hidden.get(&HtmlAttr::Value), Some("bravo"));
        assert_eq!(api.items().count(), 3);
        assert_eq!(api.placeholder_text(), "Select an option");
        assert_eq!(api.trigger_label(), "Open dropdown");
        assert_eq!(api.empty_text(), "No options available");
        assert_eq!(
            api.part_attrs(super::Part::Trigger).get(&HtmlAttr::Role),
            Some("combobox")
        );

        let all = make_service(
            Props::new()
                .id("all")
                .selection_mode(selection::Mode::Multiple)
                .multiple(true)
                .default_value(selection::Set::All),
        );

        assert_eq!(
            all.connect(&|_| {})
                .hidden_input_attrs()
                .get(&HtmlAttr::Value),
            Some("all")
        );

        let unnamed = make_service(Props::new().id("unnamed"));

        assert!(
            !unnamed
                .connect(&|_| {})
                .hidden_input_attrs()
                .contains(&HtmlAttr::Name)
        );

        let disabled = make_service(Props::new().id("disabled").disabled(true));

        assert_eq!(
            disabled
                .connect(&|_| {})
                .item_attrs(&key("alpha"))
                .get(&HtmlAttr::Aria(AriaAttr::Disabled)),
            Some("true")
        );
    }

    #[test]
    fn open_close_toggle_and_keyboard_handlers_keep_open_state_in_sync() {
        let mut service = make_service(Props::new().id("sel"));

        drop(service.send(Event::Open));

        assert_eq!(*service.state(), State::Open);
        assert!(service.context().open);

        drop(service.send(Event::Close));

        assert_eq!(*service.state(), State::Closed);
        assert!(!service.context().open);

        drop(service.send(Event::Toggle));

        assert_eq!(*service.state(), State::Open);
        assert!(service.context().open);

        drop(service.send(Event::Toggle));

        assert_eq!(*service.state(), State::Closed);
        assert!(!service.context().open);

        let captured = RefCell::new(Vec::new());

        {
            let send = |event| captured.borrow_mut().push(event);

            let api = service.connect(&send);

            api.on_trigger_click();
            api.on_trigger_focus(true);
            api.on_trigger_blur();
            api.on_item_click(key("alpha"));
            api.on_item_hover(key("bravo"));
            api.on_item_leave();
            api.on_clear_click();
            api.on_trigger_keydown(&keyboard(KeyboardKey::Enter, None), false, false);
            api.on_trigger_keydown(&keyboard(KeyboardKey::ArrowDown, None), false, false);
            api.on_trigger_keydown(&keyboard(KeyboardKey::Escape, None), false, false);
            api.on_trigger_keydown(
                &keyboard(KeyboardKey::Unidentified, Some('a')),
                false,
                false,
            );
            api.on_trigger_keydown(&keyboard(KeyboardKey::Unidentified, Some('b')), true, false);

            let mut composing = keyboard(KeyboardKey::Unidentified, Some('c'));

            composing.is_composing = true;

            api.on_trigger_keydown(&composing, false, false);
            api.on_content_keydown(&keyboard(KeyboardKey::ArrowDown, None), false, false);
            api.on_content_keydown(&keyboard(KeyboardKey::ArrowUp, None), false, false);
            api.on_content_keydown(&keyboard(KeyboardKey::Home, None), false, false);
            api.on_content_keydown(&keyboard(KeyboardKey::End, None), false, false);
            api.on_content_keydown(&keyboard(KeyboardKey::Enter, None), false, false);
            api.on_content_keydown(
                &keyboard(KeyboardKey::Unidentified, Some('d')),
                false,
                false,
            );
            api.on_content_keydown(&keyboard(KeyboardKey::Unidentified, Some('e')), true, false);
            api.on_content_keydown(&keyboard(KeyboardKey::Unidentified, Some('f')), false, true);
            api.on_content_keydown(&composing, false, false);
            api.on_content_keydown(&keyboard(KeyboardKey::Escape, None), false, false);
        }

        assert_eq!(
            captured.into_inner(),
            vec![
                Event::Toggle,
                Event::Focus { is_keyboard: true },
                Event::Blur,
                Event::SelectItem(key("alpha")),
                Event::HighlightItem(Some(key("bravo"))),
                Event::HighlightItem(None),
                Event::Clear,
                Event::Toggle,
                Event::Open,
                Event::TypeaheadSearch('a', 1),
                Event::HighlightNext,
                Event::HighlightPrev,
                Event::HighlightFirst,
                Event::HighlightLast,
                Event::TypeaheadSearch('d', 1),
                Event::Close,
            ]
        );

        let open_capture = RefCell::new(Vec::new());

        let mut open_service = make_service(Props::new().id("open"));

        open_service.context_mut().open = true;
        open_service.context_mut().highlighted_key = Some(key("alpha"));

        {
            let send = |event| open_capture.borrow_mut().push(event);

            let api = open_service.connect(&send);

            api.on_trigger_keydown(&keyboard(KeyboardKey::ArrowDown, None), false, false);
            api.on_trigger_keydown(&keyboard(KeyboardKey::ArrowUp, None), false, false);
            api.on_trigger_keydown(&keyboard(KeyboardKey::Enter, None), false, false);
            api.on_trigger_keydown(&keyboard(KeyboardKey::Escape, None), false, false);
            api.on_content_keydown(&keyboard(KeyboardKey::Enter, None), false, false);
        }

        assert_eq!(
            open_capture.into_inner(),
            vec![
                Event::HighlightNext,
                Event::HighlightPrev,
                Event::SelectItem(key("alpha")),
                Event::Close,
                Event::SelectItem(key("alpha")),
            ]
        );
    }

    #[test]
    fn typeahead_opens_from_closed_and_disabled_items_are_skipped() {
        let disabled = BTreeSet::from([key("bravo")]);

        let mut service = make_service(
            Props::new()
                .id("sel")
                .disabled_keys(disabled)
                .disabled_behavior(DisabledBehavior::Skip),
        );

        drop(service.send(Event::TypeaheadSearch('b', 100)));

        assert_eq!(*service.state(), State::Open);
        assert_eq!(service.context().highlighted_key, None);

        drop(service.send(Event::TypeaheadSearch('c', 700)));

        assert_eq!(service.context().highlighted_key, Some(key("charlie")));

        drop(service.send(Event::ClearTypeahead));

        assert_eq!(service.context().typeahead, Default::default());

        drop(service.send(Event::HighlightItem(Some(key("bravo")))));

        assert_eq!(service.context().highlighted_key, None);
    }

    #[test]
    fn multi_select_stays_open_and_disabled_or_readonly_block_mutation() {
        let mut multi = make_service(
            Props::new()
                .id("multi")
                .multiple(true)
                .selection_mode(selection::Mode::Multiple),
        );

        drop(multi.send(Event::Open));
        drop(multi.send(Event::SelectItem(key("alpha"))));
        drop(multi.send(Event::SelectItem(key("bravo"))));

        assert_eq!(*multi.state(), State::Open);
        assert!(multi.context().selection.get().contains(&key("alpha")));
        assert!(multi.context().selection.get().contains(&key("bravo")));

        drop(multi.send(Event::DeselectItem(key("alpha"))));

        assert!(!multi.context().selection.get().contains(&key("alpha")));

        drop(multi.send(Event::Clear));

        assert!(multi.context().selection.get().is_empty());

        let mut allow_empty = make_service(
            Props::new()
                .id("allow-empty")
                .multiple(true)
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Single(key("alpha"))),
        );

        drop(allow_empty.send(Event::DeselectItem(key("alpha"))));

        assert!(allow_empty.context().selection.get().is_empty());

        let mut disallow = make_service(
            Props::new()
                .id("disallow")
                .multiple(true)
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Single(key("alpha")))
                .disallow_empty_selection(true),
        );

        drop(disallow.send(Event::DeselectItem(key("alpha"))));

        assert!(disallow.context().selection.get().contains(&key("alpha")));

        drop(disallow.send(Event::Clear));

        assert!(disallow.context().selection.get().contains(&key("alpha")));

        let mut readonly = make_service(Props::new().id("ro").readonly(true));

        drop(readonly.send(Event::Open));
        drop(readonly.send(Event::SelectItem(key("alpha"))));

        assert_eq!(*readonly.state(), State::Open);
        assert!(readonly.context().selection.get().is_empty());
    }

    #[test]
    fn readonly_select_ignores_typeahead_open_and_highlight() {
        let mut select = make_service(Props::new().id("sel").readonly(true));

        drop(select.send(Event::TypeaheadSearch('b', 100)));

        assert_eq!(select.state(), &State::Closed);
        assert!(!select.context().open);
        assert_eq!(select.context().highlighted_key, None);
        assert_eq!(select.context().typeahead.search, "");
    }

    #[test]
    fn disallow_empty_selection_allows_deselect_from_all_when_items_remain() {
        let mut select = make_service(
            Props::new()
                .id("sel")
                .multiple(true)
                .default_value(selection::Set::All)
                .disallow_empty_selection(true),
        );

        drop(select.send(Event::DeselectItem(key("alpha"))));

        assert_eq!(
            *select.context().selection.get(),
            selection::Set::Multiple(BTreeSet::from([key("bravo"), key("charlie")]))
        );
    }

    #[test]
    fn stale_item_selection_events_do_not_mutate_selection_or_hidden_value() {
        let mut select = make_service(
            Props::new()
                .id("sel")
                .name("choice")
                .multiple(true)
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::All),
        );

        drop(select.send(Event::DeselectItem(key("missing"))));

        assert_eq!(*select.context().selection.get(), selection::Set::All);
        assert_eq!(
            select
                .connect(&|_| {})
                .hidden_input_attrs()
                .get(&HtmlAttr::Value),
            Some("all")
        );

        let mut empty = make_service(Props::new().id("empty").name("choice"));

        drop(empty.send(Event::Open));
        drop(empty.send(Event::SelectItem(key("missing"))));

        assert_eq!(*empty.context().selection.get(), selection::Set::Empty);
        assert_eq!(
            empty
                .connect(&|_| {})
                .hidden_input_attrs()
                .get(&HtmlAttr::Value),
            Some("")
        );

        drop(empty.send(Event::UpdateItems(grouped_collection())));
        drop(empty.send(Event::Open));
        drop(empty.send(Event::SelectItem(key("group"))));

        assert_eq!(*empty.context().selection.get(), selection::Set::Empty);

        drop(select.send(Event::UpdateItems(grouped_collection())));
        drop(select.send(Event::DeselectItem(key("group"))));

        assert_eq!(*select.context().selection.get(), selection::Set::All);
    }

    #[test]
    fn disallow_empty_selection_blocks_empty_toggle_paths() {
        let mut only_selected = make_service(
            Props::new()
                .id("sel")
                .multiple(true)
                .default_value(selection::Set::Single(key("alpha")))
                .disallow_empty_selection(true),
        );

        drop(only_selected.send(Event::Open));
        drop(only_selected.send(Event::SelectItem(key("alpha"))));
        assert_eq!(
            *only_selected.context().selection.get(),
            selection::Set::Single(key("alpha"))
        );

        let mut all_singleton = make_service(
            Props::new()
                .id("sel")
                .multiple(true)
                .default_value(selection::Set::All)
                .disallow_empty_selection(true),
        );

        drop(all_singleton.send(Event::UpdateItems(single_item_collection())));
        drop(all_singleton.send(Event::DeselectItem(key("alpha"))));
        assert_eq!(
            *all_singleton.context().selection.get(),
            selection::Set::All
        );

        drop(all_singleton.send(Event::Open));
        drop(all_singleton.send(Event::SelectItem(key("alpha"))));
        assert_eq!(
            *all_singleton.context().selection.get(),
            selection::Set::All
        );
    }

    #[test]
    fn focus_only_disabled_items_keep_active_descendant() {
        let mut select = make_service(
            Props::new()
                .id("sel")
                .disabled_keys(BTreeSet::from([key("bravo")]))
                .disabled_behavior(DisabledBehavior::FocusOnly),
        );

        drop(select.send(Event::Open));
        drop(select.send(Event::HighlightItem(Some(key("bravo")))));

        assert_eq!(
            select
                .connect(&|_| {})
                .trigger_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant)),
            Some("sel-item-bravo")
        );
    }

    #[test]
    fn multiple_prop_normalizes_selection_mode_and_close_behavior() {
        let mut select = make_service(Props::new().id("sel").multiple(true));

        drop(select.send(Event::Open));
        drop(select.send(Event::SelectItem(key("alpha"))));
        drop(select.send(Event::SelectItem(key("bravo"))));

        assert_eq!(
            *select.context().selection.get(),
            selection::Set::Multiple(BTreeSet::from([key("alpha"), key("bravo")]))
        );
        assert_eq!(select.state(), &State::Open);
        assert!(select.context().multiple);

        let mut mode_select = make_service(
            Props::new()
                .id("sel")
                .selection_mode(selection::Mode::Multiple),
        );

        drop(mode_select.send(Event::Open));
        drop(mode_select.send(Event::SelectItem(key("alpha"))));

        assert_eq!(mode_select.state(), &State::Open);
        assert!(mode_select.context().multiple);
    }

    #[test]
    fn trigger_links_description_and_error_without_autocomplete() {
        let mut select = make_service(Props::new().id("sel").invalid(true));

        select.context_mut().has_description = true;

        let attrs = select.connect(&|_| {}).trigger_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("sel-description sel-error-message")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::AutoComplete)), None);
    }

    #[test]
    fn keydown_helpers_emit_typeahead_with_or_without_adapter_timestamps() {
        let mut select = make_service(Props::new().id("sel"));

        let sent = RefCell::new(Vec::new());
        {
            let send = |event| {
                sent.borrow_mut().push(event);
            };
            let api = select.connect(&send);
            api.on_trigger_keydown(
                &keyboard(KeyboardKey::Unidentified, Some('b')),
                false,
                false,
            );
            api.on_trigger_keydown_at(
                &keyboard(KeyboardKey::Unidentified, Some('b')),
                false,
                false,
                100,
            );
        }
        assert_eq!(
            sent.borrow().as_slice(),
            &[
                Event::TypeaheadSearch('b', 1),
                Event::TypeaheadSearch('b', 100)
            ]
        );
        for event in sent.take() {
            drop(select.send(event));
        }
        let first_time = select.context().typeahead.last_key_time_ms;
        let sent = RefCell::new(Vec::new());
        {
            let send = |event| {
                sent.borrow_mut().push(event);
            };
            let api = select.connect(&send);
            api.on_content_keydown_at(
                &keyboard(KeyboardKey::Unidentified, Some('r')),
                false,
                false,
                1_000,
            );
        }
        for event in sent.take() {
            drop(select.send(event));
        }

        assert_eq!(first_time, 100);
        assert_eq!(select.context().typeahead.last_key_time_ms, 1_000);
    }

    #[test]
    fn set_props_syncs_context_backed_select_fields() {
        let mut select = make_service(Props::new().id("sel"));

        drop(
            select.set_props(
                Props::new()
                    .id("sel-next")
                    .disabled(true)
                    .readonly(true)
                    .required(true)
                    .invalid(true)
                    .multiple(true)
                    .name("country")
                    .loop_focus(false)
                    .disabled_behavior(DisabledBehavior::FocusOnly)
                    .disabled_keys(BTreeSet::from([key("bravo")]))
                    .value(selection::Set::Single(key("alpha"))),
            ),
        );

        assert!(select.context().disabled);
        assert!(select.context().readonly);
        assert!(select.context().required);
        assert!(select.context().invalid);
        assert!(select.context().multiple);
        assert_eq!(select.context().name.as_deref(), Some("country"));
        assert!(!select.context().loop_focus);
        assert_eq!(
            select.context().selection_state.mode,
            selection::Mode::Multiple
        );
        assert_eq!(
            select.context().selection_state.disabled_behavior,
            DisabledBehavior::FocusOnly
        );
        assert_eq!(
            *select.context().selection.get(),
            selection::Set::Single(key("alpha"))
        );

        assert_eq!(select.context().ids.id(), "sel-next");

        let attrs = select.connect(&|_| {}).trigger_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("sel-next-trigger"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ReadOnly)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Required)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(
            select
                .connect(&|_| {})
                .hidden_input_attrs()
                .get(&HtmlAttr::Required),
            None
        );
    }

    #[test]
    fn initial_selection_is_normalized_for_selection_mode() {
        let single = make_service(
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

        let none = make_service(
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
        let mut select = make_service(
            Props::new()
                .id("sel")
                .multiple(true)
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Multiple(BTreeSet::from([
                    key("alpha"),
                    key("bravo"),
                ]))),
        );

        drop(
            select.set_props(
                Props::new()
                    .id("sel")
                    .selection_mode(selection::Mode::Single),
            ),
        );

        assert_eq!(
            *select.context().selection.get(),
            selection::Set::Single(key("alpha"))
        );
        assert_eq!(
            select.context().selection_state.selected_keys,
            selection::Set::Single(key("alpha"))
        );

        drop(select.set_props(Props::new().id("sel").selection_mode(selection::Mode::None)));

        assert_eq!(*select.context().selection.get(), selection::Set::Empty);
        assert_eq!(
            select.context().selection_state.selected_keys,
            selection::Set::Empty
        );
    }

    #[test]
    fn controlled_selection_pruning_keeps_binding_and_state_in_sync() {
        let mut select = make_service(
            Props::new()
                .id("sel")
                .multiple(true)
                .selection_mode(selection::Mode::Multiple)
                .value(selection::Set::Multiple(BTreeSet::from([
                    key("alpha"),
                    key("bravo"),
                ]))),
        );

        drop(select.send(Event::UpdateItems(single_item_collection())));

        assert_eq!(
            *select.context().selection.get(),
            selection::Set::Multiple(BTreeSet::from([key("alpha")]))
        );
        assert_eq!(
            select.context().selection_state.selected_keys,
            *select.context().selection.get()
        );
    }

    #[test]
    fn update_items_invalidates_stale_highlight_selection_and_anchor() {
        let mut service = make_service(
            Props::new()
                .id("sel")
                .multiple(true)
                .selection_mode(selection::Mode::Multiple),
        );

        drop(service.send(Event::Open));
        drop(service.send(Event::SelectItem(key("alpha"))));
        drop(service.send(Event::SelectItem(key("bravo"))));
        drop(service.send(Event::HighlightItem(Some(key("bravo")))));

        let new_items = CollectionBuilder::new()
            .item(
                key("charlie"),
                "Charlie",
                super::Item {
                    label: "Charlie".into(),
                },
            )
            .build();

        drop(service.send(Event::UpdateItems(new_items)));

        assert_eq!(service.context().highlighted_key, None);
        assert!(service.context().selection.get().is_empty());
        assert_eq!(service.context().selection_state.anchor_key, None);

        let mut retained = make_service(
            Props::new()
                .id("retained")
                .multiple(true)
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

        let mut single = make_service(
            Props::new()
                .id("single")
                .default_value(selection::Set::Single(key("alpha"))),
        );

        drop(single.send(Event::UpdateItems(CollectionBuilder::new().build())));

        assert!(single.context().selection.get().is_empty());

        let mut all = make_service(
            Props::new()
                .id("all")
                .multiple(true)
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::All),
        );

        drop(all.send(Event::UpdateItems(CollectionBuilder::new().build())));

        assert_eq!(*all.context().selection.get(), selection::Set::All);

        let mut valid_refs = make_service(Props::new().id("valid-refs"));

        valid_refs.context_mut().highlighted_key = Some(key("alpha"));
        valid_refs.context_mut().selection_state.anchor_key = Some(key("alpha"));

        drop(valid_refs.send(Event::UpdateItems(collection())));

        assert_eq!(valid_refs.context().highlighted_key, Some(key("alpha")));
        assert_eq!(
            valid_refs.context().selection_state.anchor_key,
            Some(key("alpha"))
        );

        let mut removed_refs = make_service(Props::new().id("removed-refs"));

        removed_refs.context_mut().highlighted_key = Some(key("ghost"));
        removed_refs.context_mut().selection_state.anchor_key = Some(key("ghost"));

        drop(removed_refs.send(Event::UpdateItems(collection())));

        assert_eq!(removed_refs.context().highlighted_key, None);
        assert_eq!(removed_refs.context().selection_state.anchor_key, None);

        let mut disabled_refs = make_service(
            Props::new()
                .id("disabled-refs")
                .disabled_keys(BTreeSet::from([key("bravo")])),
        );

        disabled_refs.context_mut().highlighted_key = Some(key("bravo"));
        disabled_refs.context_mut().selection_state.anchor_key = Some(key("bravo"));

        drop(disabled_refs.send(Event::UpdateItems(collection())));

        assert_eq!(disabled_refs.context().highlighted_key, None);
        assert_eq!(disabled_refs.context().selection_state.anchor_key, None);
    }

    #[test]
    fn focus_composition_and_navigation_edges_are_observable() {
        let mut service = make_service(Props::new().id("sel"));

        drop(service.send(Event::Focus { is_keyboard: true }));

        assert!(service.context().focused);
        assert!(service.context().focus_visible);

        drop(service.send(Event::CompositionStart));

        assert!(service.context().is_composing);

        drop(service.send(Event::TypeaheadSearch('c', 100)));

        assert_eq!(service.context().highlighted_key, None);
        assert_eq!(*service.state(), State::Closed);

        drop(service.send(Event::CompositionEnd));

        assert!(!service.context().is_composing);

        drop(service.send(Event::Open));
        drop(service.send(Event::HighlightLast));

        assert_eq!(service.context().highlighted_key, Some(key("charlie")));

        drop(service.send(Event::HighlightFirst));

        assert_eq!(service.context().highlighted_key, Some(key("alpha")));

        drop(service.send(Event::HighlightNext));

        assert_eq!(service.context().highlighted_key, Some(key("bravo")));

        drop(service.send(Event::HighlightPrev));

        assert_eq!(service.context().highlighted_key, Some(key("alpha")));

        drop(service.send(Event::Blur));

        assert_eq!(*service.state(), State::Closed);
        assert!(!service.context().open);
        assert!(!service.context().focused);

        let mut disabled_active = make_service(
            Props::new()
                .id("disabled-active")
                .disabled_keys(BTreeSet::from([key("bravo")])),
        );

        disabled_active.context_mut().highlighted_key = Some(key("bravo"));

        let api = disabled_active.connect(&|_| {});

        assert!(
            !api.trigger_attrs()
                .contains(&HtmlAttr::Aria(AriaAttr::ActiveDescendant))
        );
    }

    #[test]
    fn empty_state_attrs_emit_status_region() {
        let service = make_service(Props::new().id("sel"));

        let api = service.connect(&|_| {});

        let attrs = api.empty_state_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("status"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
    }

    #[test]
    fn select_attrs_snapshot_all_parts_and_state_branches() {
        let mut service = make_service(
            Props::new()
                .id("sel")
                .name("country")
                .form("checkout")
                .autocomplete("country")
                .required(true)
                .invalid(true)
                .multiple(true)
                .selection_mode(selection::Mode::Multiple),
        );

        service.context_mut().has_description = true;

        drop(service.send(Event::Open));
        drop(service.send(Event::SelectItem(key("bravo"))));
        drop(service.send(Event::HighlightItem(Some(key("bravo")))));

        let api = service.connect(&|_| {});

        insta::assert_snapshot!(
            "select_root_open_invalid",
            snapshot_attrs(&api.root_attrs())
        );
        insta::assert_snapshot!("select_label", snapshot_attrs(&api.label_attrs()));
        insta::assert_snapshot!("select_control", snapshot_attrs(&api.control_attrs()));
        insta::assert_snapshot!(
            "select_trigger_multi_active",
            snapshot_attrs(&api.trigger_attrs())
        );
        insta::assert_snapshot!(
            "select_value_text_selected",
            snapshot_attrs(&api.value_text_attrs())
        );
        insta::assert_snapshot!(
            "select_indicator_open",
            snapshot_attrs(&api.indicator_attrs())
        );
        insta::assert_snapshot!(
            "select_clear_trigger",
            snapshot_attrs(&api.clear_trigger_attrs())
        );
        insta::assert_snapshot!("select_positioner", snapshot_attrs(&api.positioner_attrs()));
        insta::assert_snapshot!("select_content_multi", snapshot_attrs(&api.content_attrs()));
        insta::assert_snapshot!(
            "select_empty_state",
            snapshot_attrs(&api.empty_state_attrs())
        );
        insta::assert_snapshot!(
            "select_hidden_input",
            snapshot_attrs(&api.hidden_input_attrs())
        );
        insta::assert_snapshot!(
            "select_item_selected_highlighted",
            snapshot_attrs(&api.item_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "select_item_text",
            snapshot_attrs(&api.item_text_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "select_item_indicator_selected",
            snapshot_attrs(&api.item_indicator_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "select_item_group",
            snapshot_attrs(&api.item_group_attrs(&key("group")))
        );
        insta::assert_snapshot!(
            "select_item_group_label",
            snapshot_attrs(&api.item_group_label_attrs(&key("group")))
        );
        insta::assert_snapshot!(
            "select_description",
            snapshot_attrs(&api.description_attrs())
        );
        insta::assert_snapshot!(
            "select_error_message",
            snapshot_attrs(&api.error_message_attrs())
        );
    }
}
