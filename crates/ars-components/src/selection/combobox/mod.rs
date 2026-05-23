//! Combobox selection component machine.

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_collections::{
    Collection, DisabledBehavior, Key, Node, StaticCollection, node::NodeType, selection,
};
use ars_core::{
    AriaAttr, AttrMap, Bindable, Callback, ComponentIds, ComponentMessages, ComponentPart,
    ConnectApi, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, NoEffect, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

use crate::overlay::positioning::PositioningOptions;

const ALL_SELECTION_SENTINEL: &str = "__ars_all";

/// Message function used by Combobox single-locale messages.
pub type LocaleMessage = dyn Fn(&Locale) -> String + Send + Sync;

/// Message function used by Combobox result-count announcements.
pub type ResultCountMessage = dyn Fn(usize, &Locale) -> String + Send + Sync;

/// Open-state change callback.
pub type OpenChangeCallback = dyn Fn(bool) + Send + Sync;

/// User-facing payload for Combobox items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The label of the item.
    pub label: String,
}

/// The states of the Combobox state machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The combobox is closed.
    #[default]
    Closed,

    /// The combobox is open.
    Open,
}

/// The events of the Combobox state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The input value changed.
    InputChange(String),

    /// The focus received on the input.
    Focus {
        /// Whether the focus is from a keyboard event.
        is_keyboard: bool,
    },

    /// The focus lost from the input.
    Blur,

    /// The combobox is opened.
    Open,

    /// The combobox is closed.
    Close,

    /// The item is selected.
    SelectItem(Key),

    /// Ctrl/Cmd-click toggle for replace-behavior multi-select comboboxes.
    SelectItemCtrl(Key),

    /// The item is deselected.
    DeselectItem(Key),

    /// The item is highlighted.
    HighlightItem(Option<Key>),

    /// The first item is highlighted.
    HighlightFirst,

    /// The last item is highlighted.
    HighlightLast,

    /// The next item is highlighted.
    HighlightNext,

    /// The previous item is highlighted.
    HighlightPrev,

    /// The combobox is dismissed.
    Dismiss,

    /// The combobox is clicked outside.
    ClickOutside,

    /// The combobox is cleared.
    Clear,

    /// Dynamically replace the item collection.
    UpdateItems(StaticCollection<Item>),

    /// IME composition started.
    CompositionStart,

    /// IME composition ended with the final committed input value.
    CompositionEnd(String),

    /// Commit the current input value when no option is highlighted.
    CommitInput,

    /// Clear the inline completion suffix while preserving the typed prefix.
    ClearInlineCompletion,

    /// Mark whether a description element is rendered.
    SetDescriptionPresent(bool),

    /// Synchronize context-backed fields from updated props.
    SyncProps,
}

/// Built-in filtering and autocomplete behavior for Combobox input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FilterMode {
    /// Filter items whose text contains the input text.
    #[default]
    Contains,

    /// Filter items whose text starts with the input text.
    StartsWith,

    /// Disable filtering and autocomplete.
    None,

    /// Do not filter items, but expose inline autocomplete semantics.
    Inline,

    /// Filter by prefix and expose combined list plus inline autocomplete semantics.
    InlineCompletion,

    /// Disable built-in filtering so adapters can apply custom predicates.
    Custom,
}

/// The context for the Combobox state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The full item collection.
    pub items: StaticCollection<Item>,

    /// Text value of the input field.
    pub input_value: Bindable<String>,

    /// Controlled or uncontrolled selected-key binding.
    pub selection: Bindable<selection::Set>,

    /// Full collection selection state.
    pub selection_state: selection::State,

    /// Currently highlighted item key.
    pub highlighted_key: Option<Key>,

    /// Keys that pass the current filter. `None` means no filter is active.
    pub visible_keys: Option<BTreeSet<Key>>,

    /// Whether the combobox is open.
    pub open: bool,

    /// Whether the input has focus.
    pub focused: bool,

    /// Whether focus-visible styling should be emitted.
    pub focus_visible: bool,

    /// Whether the combobox is disabled.
    pub disabled: bool,

    /// Whether the combobox is readonly.
    pub readonly: bool,

    /// Whether the combobox is required.
    pub required: bool,

    /// Whether the combobox is invalid.
    pub invalid: bool,

    /// Whether multiple items can be selected.
    pub multiple: bool,

    /// The filter mode.
    pub filter_mode: FilterMode,

    /// Whether focus opens the popup.
    pub open_on_focus: bool,

    /// Whether clicking the input opens the popup.
    pub open_on_click: bool,

    /// Form field name.
    pub name: Option<String>,

    /// Associated form id.
    pub form: Option<String>,

    /// Whether keyboard navigation wraps at collection boundaries.
    pub loop_focus: bool,

    /// Whether IME composition is active.
    pub is_composing: bool,

    /// Typed prefix that produced the current inline completion, if any.
    pub inline_completion_prefix: Option<String>,

    /// Whether a description element is rendered.
    pub has_description: bool,

    /// Whether iOS `VoiceOver` fallback semantics should be used.
    pub is_ios: bool,

    /// Stable component ID derivation helper.
    pub ids: ComponentIds,

    /// Resolved locale for message formatting.
    pub locale: Locale,

    /// Resolved localized messages.
    pub messages: Messages,
}

/// Props for the Combobox state machine.
#[derive(Clone, Debug, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Controlled input value.
    pub input_value: Option<String>,

    /// Initial uncontrolled input value.
    pub default_input_value: String,

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

    /// Whether the combobox is disabled.
    pub disabled: bool,

    /// Whether the combobox is readonly.
    pub readonly: bool,

    /// Whether the combobox is required.
    pub required: bool,

    /// Whether the combobox is invalid.
    pub invalid: bool,

    /// Placeholder text override.
    pub placeholder: Option<String>,

    /// Built-in filtering mode.
    pub filter_mode: FilterMode,

    /// Whether focus opens the popup.
    pub open_on_focus: bool,

    /// Whether input click opens the popup.
    pub open_on_click: bool,

    /// Form field name.
    pub name: Option<String>,

    /// Associated form id.
    pub form: Option<String>,

    /// Whether keyboard navigation wraps at boundaries.
    pub loop_focus: bool,

    /// Positioning options for adapter-resolved popup placement.
    pub positioning: PositioningOptions,

    /// Initial highlighted key when the popup first opens.
    pub default_highlighted_key: Option<Key>,

    /// Whether an empty list remains open for no-result rendering.
    pub allows_empty_collection: bool,

    /// Callback invoked when the popup open state changes.
    pub on_open_change: Option<Callback<OpenChangeCallback>>,

    /// Keys of items that should be disabled.
    pub disabled_keys: BTreeSet<Key>,

    /// Whether free-form values that do not match an item are accepted.
    pub allow_custom_value: bool,

    /// Whether adapters detected an iOS `VoiceOver` focus strategy.
    pub is_ios: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            input_value: None,
            default_input_value: String::new(),
            value: None,
            default_value: selection::Set::Empty,
            selection_mode: selection::Mode::Single,
            selection_behavior: selection::Behavior::Toggle,
            disabled_behavior: DisabledBehavior::default(),
            disabled: false,
            readonly: false,
            required: false,
            invalid: false,
            placeholder: None,
            filter_mode: FilterMode::Contains,
            open_on_focus: true,
            open_on_click: false,
            name: None,
            form: None,
            loop_focus: true,
            positioning: PositioningOptions::default(),
            default_highlighted_key: None,
            allows_empty_collection: false,
            on_open_change: None,
            disabled_keys: BTreeSet::new(),
            allow_custom_value: false,
            is_ios: false,
        }
    }
}

impl Props {
    /// Returns default Combobox props.
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

    /// Sets [`Self::input_value`].
    #[must_use]
    pub fn input_value(mut self, value: impl Into<String>) -> Self {
        self.input_value = Some(value.into());
        self
    }

    /// Sets [`Self::default_input_value`].
    #[must_use]
    pub fn default_input_value(mut self, value: impl Into<String>) -> Self {
        self.default_input_value = value.into();
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

    /// Sets [`Self::placeholder`].
    #[must_use]
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = Some(value.into());
        self
    }

    /// Sets [`Self::filter_mode`].
    #[must_use]
    pub const fn filter_mode(mut self, value: FilterMode) -> Self {
        self.filter_mode = value;
        self
    }

    /// Sets [`Self::open_on_focus`].
    #[must_use]
    pub const fn open_on_focus(mut self, value: bool) -> Self {
        self.open_on_focus = value;
        self
    }

    /// Sets [`Self::open_on_click`].
    #[must_use]
    pub const fn open_on_click(mut self, value: bool) -> Self {
        self.open_on_click = value;
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

    /// Sets [`Self::default_highlighted_key`].
    #[must_use]
    pub fn default_highlighted_key(mut self, value: Key) -> Self {
        self.default_highlighted_key = Some(value);
        self
    }

    /// Sets [`Self::allows_empty_collection`].
    #[must_use]
    pub const fn allows_empty_collection(mut self, value: bool) -> Self {
        self.allows_empty_collection = value;
        self
    }

    /// Sets [`Self::on_open_change`].
    #[must_use]
    pub fn on_open_change(mut self, value: Callback<OpenChangeCallback>) -> Self {
        self.on_open_change = Some(value);
        self
    }

    /// Sets [`Self::disabled_keys`].
    #[must_use]
    pub fn disabled_keys(mut self, value: BTreeSet<Key>) -> Self {
        self.disabled_keys = value;
        self
    }

    /// Sets [`Self::allow_custom_value`].
    #[must_use]
    pub const fn allow_custom_value(mut self, value: bool) -> Self {
        self.allow_custom_value = value;
        self
    }

    /// Sets [`Self::is_ios`].
    #[must_use]
    pub const fn is_ios(mut self, value: bool) -> Self {
        self.is_ios = value;
        self
    }
}

/// Messages for the Combobox component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the open trigger button.
    pub trigger_label: MessageFn<LocaleMessage>,

    /// Accessible label for the clear button.
    pub clear_label: MessageFn<LocaleMessage>,

    /// Text shown when no results match the filter.
    pub no_results: MessageFn<LocaleMessage>,

    /// Text shown while async options are loading.
    pub loading: MessageFn<LocaleMessage>,

    /// Live region announcement for filtered result count.
    pub results_count: MessageFn<ResultCountMessage>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            trigger_label: MessageFn::static_str("Show suggestions"),
            clear_label: MessageFn::static_str("Clear value"),
            no_results: MessageFn::static_str("No results found"),
            loading: MessageFn::static_str("Loading options..."),
            results_count: MessageFn::new(|count: usize, _locale: &Locale| {
                format!("{count} results available")
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// The anatomy parts exposed by the Combobox connect API.
#[derive(ComponentPart)]
#[scope = "combobox"]
pub enum Part {
    /// Root wrapper.
    Root,

    /// Form label.
    Label,

    /// Control wrapper around input and trigger controls.
    Control,

    /// Text input.
    Input,

    /// Popup trigger button.
    Trigger,

    /// Clear-value trigger button.
    ClearTrigger,

    /// Popup positioner.
    Positioner,

    /// Listbox popup content.
    Content,

    /// Item group.
    ItemGroup {
        /// Section key.
        key: Key,
    },

    /// Item group label.
    ItemGroupLabel {
        /// Section key.
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

    /// Empty result display.
    Empty,

    /// Help or description text.
    Description,

    /// Validation error message.
    ErrorMessage,

    /// Result-count announcement region.
    LiveRegion,
}

/// Machine for Combobox.
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
            State::Closed,
            Context {
                items: StaticCollection::default(),
                input_value: if let Some(value) = &props.input_value {
                    Bindable::controlled(value.clone())
                } else {
                    Bindable::uncontrolled(props.default_input_value.clone())
                },
                selection: if props.value.is_some() {
                    Bindable::controlled(selected.clone())
                } else {
                    Bindable::uncontrolled(selected.clone())
                },
                selection_state,
                highlighted_key: props.default_highlighted_key.clone(),
                visible_keys: None,
                open: false,
                focused: false,
                focus_visible: false,
                disabled: props.disabled,
                readonly: props.readonly,
                required: props.required,
                invalid: props.invalid,
                multiple: props.selection_mode == selection::Mode::Multiple,
                filter_mode: props.filter_mode,
                open_on_focus: props.open_on_focus,
                open_on_click: props.open_on_click,
                name: props.name.clone(),
                form: props.form.clone(),
                loop_focus: props.loop_focus,
                is_composing: false,
                inline_completion_prefix: None,
                has_description: false,
                is_ios: props.is_ios,
                ids: ComponentIds::from_id(&props.id),
                locale: env.locale.clone(),
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
                Event::InputChange(_)
                    | Event::Open
                    | Event::SelectItem(_)
                    | Event::SelectItemCtrl(_)
                    | Event::DeselectItem(_)
                    | Event::HighlightItem(_)
                    | Event::HighlightFirst
                    | Event::HighlightLast
                    | Event::HighlightNext
                    | Event::HighlightPrev
                    | Event::Focus { .. }
                    | Event::Clear
                    | Event::CommitInput
                    | Event::ClearInlineCompletion
            )
        {
            return None;
        }

        if ctx.readonly
            && matches!(
                event,
                Event::InputChange(_)
                    | Event::SelectItem(_)
                    | Event::SelectItemCtrl(_)
                    | Event::DeselectItem(_)
                    | Event::Clear
                    | Event::CommitInput
                    | Event::ClearInlineCompletion
            )
        {
            return None;
        }

        match (state, event) {
            (_, Event::InputChange(_)) if ctx.is_composing => None,

            (_, Event::InputChange(value)) => {
                let value = value.clone();
                let (visible_keys, highlighted_key, input_value, inline_completion_prefix) =
                    input_change_values(ctx, &value);
                let should_open = should_open_for_input(ctx, props, &value, visible_keys.as_ref());

                if should_open {
                    Some(open_plan(props).apply(move |ctx: &mut Context| {
                        ctx.input_value.set(input_value);
                        ctx.visible_keys = visible_keys;
                        ctx.highlighted_key = highlighted_key;
                        ctx.inline_completion_prefix = inline_completion_prefix;
                    }))
                } else if ctx.open && !props.allows_empty_collection {
                    Some(close_plan(props).apply(move |ctx: &mut Context| {
                        ctx.input_value.set(input_value);
                        ctx.visible_keys = visible_keys;
                        ctx.highlighted_key = highlighted_key;
                        ctx.inline_completion_prefix = inline_completion_prefix;
                    }))
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.input_value.set(input_value);
                        ctx.visible_keys = visible_keys;
                        ctx.highlighted_key = highlighted_key;
                        ctx.inline_completion_prefix = inline_completion_prefix;
                    }))
                }
            }

            (State::Closed, Event::Open) => {
                let highlighted_key = default_open_highlight(ctx, props);
                Some(open_plan(props).apply(move |ctx: &mut Context| {
                    ctx.highlighted_key = highlighted_key;
                }))
            }

            (State::Open, Event::Close | Event::Dismiss | Event::ClickOutside) => {
                Some(close_plan(props))
            }

            (_, Event::Focus { is_keyboard }) => {
                let is_keyboard = *is_keyboard;
                let should_open = ctx.open_on_focus || (!is_keyboard && ctx.open_on_click);

                if should_open && !ctx.open {
                    let highlighted_key = default_open_highlight(ctx, props);
                    Some(open_plan(props).apply(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = is_keyboard;
                        ctx.highlighted_key = highlighted_key;
                    }))
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.focused = true;
                        ctx.focus_visible = is_keyboard;
                    }))
                }
            }

            (_, Event::Blur) => Some(close_plan(props).apply({
                let allow_custom_value = props.allow_custom_value;

                move |ctx: &mut Context| {
                    if !ctx.disabled && !ctx.readonly {
                        if allow_custom_value {
                            let input = ctx.input_value.get().clone();

                            if !input_matches_selected_item_label(ctx, &input) {
                                apply_custom_input_commit(ctx, input);
                            }
                        } else {
                            revert_input_to_selection(ctx);
                        }
                    }

                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.highlighted_key = None;
                }
            })),

            (State::Open, Event::HighlightNext) => {
                let key = next_visible_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightPrev) => {
                let key = prev_visible_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightFirst) => {
                let key = first_visible_key(ctx, ctx.visible_keys.as_ref());
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightLast) => {
                let key = last_visible_key(ctx);
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (State::Open, Event::HighlightItem(key)) => {
                let key = key.clone().filter(|key| is_visible_focusable_key(ctx, key));
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            (_, Event::SelectItem(key)) => select_item_plan(ctx, props, key, false),

            (_, Event::SelectItemCtrl(key)) => select_item_plan(ctx, props, key, true),

            (_, Event::DeselectItem(key)) => {
                if !ctx.multiple
                    || !ctx.selection.get().contains(key)
                    || !is_selectable_key(ctx, key)
                {
                    return None;
                }

                let mut next = ctx.selection_state.deselect_from_all(key, &ctx.items);

                remove_disabled_selection_keys(&mut next);

                Some(apply_selection_plan(next))
            }

            (_, Event::Clear) => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                let next = ctx.selection_state.clear();
                ctx.selection_state = next;
                let selected = ctx.selection_state.selected_keys.clone();
                ctx.selection_state.selected_keys = set_selection_value(ctx, selected);
                ctx.input_value.set(String::new());
                ctx.visible_keys = None;
                ctx.highlighted_key = None;
                ctx.inline_completion_prefix = None;
            })),

            (_, Event::UpdateItems(items)) => {
                let items = items.clone();
                let allow_custom_value = props.allow_custom_value;
                let mut next_ctx = ctx.clone();
                let input = next_ctx.input_value.get().clone();

                next_ctx.items = items.clone();
                next_ctx.visible_keys = visible_keys_for(&next_ctx, &input);
                invalidate_collection_references(&mut next_ctx, allow_custom_value);

                let should_open = should_open_for_items_update(
                    &next_ctx,
                    props,
                    &input,
                    next_ctx.visible_keys.as_ref(),
                );

                if should_open {
                    let input = input.clone();
                    Some(open_plan(props).apply(move |ctx: &mut Context| {
                        ctx.items = items;
                        refresh_filter_and_highlight(ctx, &input);
                        invalidate_collection_references(ctx, allow_custom_value);
                    }))
                } else if ctx.open && !props.allows_empty_collection {
                    let input = input.clone();
                    Some(close_plan(props).apply(move |ctx: &mut Context| {
                        ctx.items = items;
                        refresh_filter_and_highlight(ctx, &input);
                        invalidate_collection_references(ctx, allow_custom_value);
                    }))
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.items = items;
                        refresh_filter_and_highlight(ctx, &input);
                        invalidate_collection_references(ctx, allow_custom_value);
                    }))
                }
            }

            (_, Event::CompositionStart) => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.is_composing = true;
                }))
            }

            (_, Event::CompositionEnd(_)) if ctx.disabled || ctx.readonly => {
                Some(TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.is_composing = false;
                }))
            }

            (_, Event::CompositionEnd(value)) => {
                let value = value.clone();
                let (visible_keys, highlighted_key, input_value, inline_completion_prefix) =
                    input_change_values(ctx, &value);
                let should_open = should_open_for_input(ctx, props, &value, visible_keys.as_ref());

                if should_open {
                    Some(open_plan(props).apply(move |ctx: &mut Context| {
                        ctx.is_composing = false;
                        ctx.input_value.set(input_value);
                        ctx.visible_keys = visible_keys;
                        ctx.highlighted_key = highlighted_key;
                        ctx.inline_completion_prefix = inline_completion_prefix;
                    }))
                } else if ctx.open && !props.allows_empty_collection {
                    Some(close_plan(props).apply(move |ctx: &mut Context| {
                        ctx.is_composing = false;
                        ctx.input_value.set(input_value);
                        ctx.visible_keys = visible_keys;
                        ctx.highlighted_key = highlighted_key;
                        ctx.inline_completion_prefix = inline_completion_prefix;
                    }))
                } else {
                    Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.is_composing = false;
                        ctx.input_value.set(input_value);
                        ctx.visible_keys = visible_keys;
                        ctx.highlighted_key = highlighted_key;
                        ctx.inline_completion_prefix = inline_completion_prefix;
                    }))
                }
            }

            (_, Event::CommitInput) => Some(commit_input_plan(ctx, props)),

            (_, Event::ClearInlineCompletion) => {
                ctx.inline_completion_prefix.clone().map(|prefix| {
                    TransitionPlan::context_only(move |ctx: &mut Context| {
                        ctx.input_value.set(prefix);
                        ctx.inline_completion_prefix = None;
                    })
                })
            }

            (_, Event::SetDescriptionPresent(present)) => {
                let present = *present;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = present;
                }))
            }

            (_, Event::SyncProps) => Some(sync_props_plan(ctx, props)),

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

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        if old == new {
            Vec::new()
        } else {
            vec![Event::SyncProps]
        }
    }
}

/// The API for the Combobox state machine.
pub struct Api<'a> {
    /// The current state of the Combobox.
    state: &'a State,

    /// The context of the Combobox.
    ctx: &'a Context,

    /// The props of the Combobox.
    props: &'a Props,

    /// The send function to send events to the Combobox state machine.
    send: &'a dyn Fn(Event),
}

impl Api<'_> {
    /// Returns `true` when the connected state is open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        matches!(self.state, State::Open)
    }

    /// Attributes for the root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs.set(HtmlAttr::Id, self.ctx.ids.id()).set(
            HtmlAttr::Data("ars-state"),
            if self.ctx.open { "open" } else { "closed" },
        );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.invalid {
            attrs.set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        attrs
    }

    /// Attributes for the label element.
    #[must_use]
    pub fn label_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Label);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("label"))
            .set(HtmlAttr::For, self.ctx.ids.part("input"));

        attrs
    }

    /// Attributes for the control wrapper.
    #[must_use]
    pub fn control_attrs(&self) -> AttrMap {
        part_attrs(&Part::Control)
    }

    /// Attributes for the input element.
    #[must_use]
    pub fn input_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Input);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("input"))
            .set(HtmlAttr::Role, "combobox")
            .set(HtmlAttr::Type, "text")
            .set(HtmlAttr::Value, self.ctx.input_value.get().as_str())
            .set(
                HtmlAttr::Aria(AriaAttr::Expanded),
                if self.ctx.open { "true" } else { "false" },
            )
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "listbox")
            .set(
                HtmlAttr::Aria(AriaAttr::AutoComplete),
                self.aria_autocomplete(),
            )
            .set(HtmlAttr::EnterKeyHint, self.enter_key_hint());

        if self.ctx.open {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            );
        }

        if self.ctx.open
            && !self.ctx.is_ios
            && let Some(key) = valid_highlight(self.ctx)
        {
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
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.readonly {
            attrs.set_bool(HtmlAttr::ReadOnly, true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        let mut described_by = Vec::new();

        if self.ctx.invalid {
            described_by.push(self.ctx.ids.part("error-message"));
        }

        if self.ctx.has_description {
            described_by.push(self.ctx.ids.part("description"));
        }

        if !described_by.is_empty() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                described_by.join(" "),
            );
        }

        if let Some(placeholder) = &self.props.placeholder {
            attrs.set(HtmlAttr::Placeholder, placeholder.as_str());
        }

        attrs
    }

    /// Dispatches an input change.
    pub fn on_input_change(&self, value: impl Into<String>) {
        (self.send)(Event::InputChange(value.into()));
    }

    /// Dispatches input focus.
    pub fn on_input_focus(&self, is_keyboard: bool) {
        (self.send)(Event::Focus { is_keyboard });
    }

    /// Dispatches an input pointer click.
    pub fn on_input_click(&self) {
        (self.send)(Event::Focus { is_keyboard: false });
    }

    /// Dispatches input blur.
    pub fn on_input_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Dispatches input keydown events.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        self.on_input_keydown_impl(data, false);
    }

    /// Re-checks Enter handling after an adapter-scheduled composition microtask.
    pub fn on_input_keydown_after_composition_check(&self, data: &KeyboardEventData) {
        self.on_input_keydown_impl(data, true);
    }

    fn on_input_keydown_impl(&self, data: &KeyboardEventData, after_composition_check: bool) {
        let composing = self.is_keyboard_composing(data);

        match data.key {
            KeyboardKey::Process => {
                (self.send)(Event::CompositionStart);
            }

            KeyboardKey::ArrowDown if data.alt_key => (self.send)(Event::Open),

            KeyboardKey::ArrowDown => {
                if !self.ctx.open {
                    (self.send)(Event::Open);
                    return;
                }

                (self.send)(Event::HighlightNext);
            }

            KeyboardKey::ArrowUp if data.alt_key => (self.send)(Event::Close),

            KeyboardKey::ArrowUp if self.ctx.open => (self.send)(Event::HighlightPrev),

            KeyboardKey::Home if data.alt_key => (self.send)(Event::HighlightFirst),

            KeyboardKey::End if data.alt_key => (self.send)(Event::HighlightLast),

            KeyboardKey::Enter if !composing || after_composition_check => {
                if self.ctx.open
                    && let Some(key) = &self.ctx.highlighted_key
                {
                    (self.send)(Event::SelectItem(key.clone()));
                } else {
                    (self.send)(Event::CommitInput);
                }
            }

            KeyboardKey::Tab if !composing => (self.send)(Event::Close),

            KeyboardKey::Escape => {
                if self.ctx.inline_completion_prefix.is_some()
                    && matches!(
                        self.ctx.filter_mode,
                        FilterMode::Inline | FilterMode::InlineCompletion
                    )
                {
                    (self.send)(Event::ClearInlineCompletion);
                } else if self.ctx.open {
                    (self.send)(Event::Close);
                } else if !self.ctx.input_value.get().is_empty() {
                    (self.send)(Event::Clear);
                }
            }

            _ => {}
        }
    }

    /// Dispatches IME composition start.
    pub fn on_input_composition_start(&self) {
        (self.send)(Event::CompositionStart);
    }

    /// Dispatches IME composition end with the committed input value.
    pub fn on_input_composition_end(&self, final_value: impl Into<String>) {
        (self.send)(Event::CompositionEnd(final_value.into()));
    }

    /// Attributes for the trigger element.
    #[must_use]
    pub fn trigger_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Trigger);

        attrs
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.trigger_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::TabIndex, "-1")
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.part("content"),
            );

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Dispatches trigger click.
    pub fn on_trigger_click(&self) {
        if self.ctx.open {
            (self.send)(Event::Close);
        } else {
            (self.send)(Event::Open);
        }
    }

    /// Attributes for the clear trigger.
    #[must_use]
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::ClearTrigger);

        attrs
            .set(HtmlAttr::Type, "button")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.clear_label)(&self.ctx.locale),
            )
            .set(HtmlAttr::TabIndex, "-1");

        if self.ctx.disabled || self.ctx.readonly {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Dispatches clear click.
    pub fn on_clear_click(&self) {
        (self.send)(Event::Clear);
    }

    /// Attributes for the positioner element.
    #[must_use]
    pub fn positioner_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Positioner);

        attrs.set(
            HtmlAttr::Data("ars-placement"),
            self.props.positioning.placement.to_string(),
        );

        attrs
    }

    /// Attributes for the content element.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Content);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("content"))
            .set(HtmlAttr::Role, "listbox")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.trigger_label)(&self.ctx.locale),
            );

        if self.ctx.multiple {
            attrs.set(HtmlAttr::Aria(AriaAttr::MultiSelectable), "true");
        }

        attrs
    }

    /// Attributes for an item element.
    #[must_use]
    pub fn item_attrs(&self, key: &Key) -> AttrMap {
        let is_selected = self.ctx.selection.get().contains(key);
        let is_highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let is_disabled = self.ctx.disabled || self.ctx.selection_state.is_disabled(key);

        let aria_selected = if self.ctx.is_ios {
            is_selected || is_highlighted
        } else {
            is_selected
        };

        let mut attrs = part_attrs(&Part::Item { key: key.clone() });

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("item", key))
            .set(HtmlAttr::Role, "option")
            .set(
                HtmlAttr::Aria(AriaAttr::Selected),
                if aria_selected { "true" } else { "false" },
            )
            .set(
                HtmlAttr::Data("ars-state"),
                if is_selected {
                    "selected"
                } else {
                    "unselected"
                },
            );

        if is_highlighted {
            attrs.set_bool(HtmlAttr::Data("ars-highlighted"), true);
        }

        if is_disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if let Some(node) = self.ctx.items.get(key) {
            attrs.set(HtmlAttr::Data("ars-value"), node.text_value.as_str());
        }

        attrs
    }

    /// Attributes for an item text element.
    #[must_use]
    pub fn item_text_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemText { key: key.clone() });

        attrs.set(
            HtmlAttr::Id,
            self.ctx.ids.item_part("item", &key.to_string(), "text"),
        );

        attrs
    }

    /// Attributes for an item indicator element.
    #[must_use]
    pub fn item_indicator_attrs(&self, key: &Key) -> AttrMap {
        let is_selected = self.ctx.selection.get().contains(key);
        let mut attrs = part_attrs(&Part::ItemIndicator { key: key.clone() });

        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true").set(
            HtmlAttr::Data("ars-state"),
            if is_selected {
                "selected"
            } else {
                "unselected"
            },
        );

        attrs
    }

    /// Attributes for an item group element.
    #[must_use]
    pub fn item_group_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemGroup { key: key.clone() });

        attrs.set(HtmlAttr::Role, "group").set(
            HtmlAttr::Aria(AriaAttr::LabelledBy),
            self.ctx.ids.item_part("group", &key.to_string(), "label"),
        );

        attrs
    }

    /// Attributes for an item group label element.
    #[must_use]
    pub fn item_group_label_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemGroupLabel { key: key.clone() });

        attrs.set(
            HtmlAttr::Id,
            self.ctx.ids.item_part("group", &key.to_string(), "label"),
        );

        attrs
    }

    /// Dispatches item click.
    pub fn on_item_click(&self, key: Key) {
        (self.send)(Event::SelectItem(key));
    }

    /// Dispatches ctrl/cmd item click.
    pub fn on_item_ctrl_click(&self, key: Key) {
        (self.send)(Event::SelectItemCtrl(key));
    }

    /// Dispatches item hover.
    pub fn on_item_hover(&self, key: Key) {
        (self.send)(Event::HighlightItem(Some(key)));
    }

    /// Dispatches item leave.
    pub fn on_item_leave(&self) {
        (self.send)(Event::HighlightItem(None));
    }

    /// Attributes for the empty state element.
    #[must_use]
    pub fn empty_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Empty);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("empty"))
            .set(HtmlAttr::Role, "none");

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

    /// Attributes for the live region element.
    #[must_use]
    pub fn live_region_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::LiveRegion);

        attrs
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "true");

        attrs
    }

    /// Attributes for the hidden form input.
    #[must_use]
    pub fn hidden_input_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();

        attrs
            .set(HtmlAttr::Type, "hidden")
            .set(
                HtmlAttr::Value,
                serialize_selection(self.ctx.selection.get()),
            )
            .set(HtmlAttr::TabIndex, "-1")
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        if let Some(name) = &self.ctx.name {
            attrs.set(HtmlAttr::Name, name.as_str());
        }

        if let Some(form) = &self.ctx.form {
            attrs.set(HtmlAttr::Form, form.as_str());
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        attrs
    }

    /// Iterates visible item nodes.
    pub fn visible_items(&self) -> impl Iterator<Item = &Node<Item>> {
        self.ctx.items.nodes().filter(|node| {
            node.node_type == NodeType::Item
                && self
                    .ctx
                    .visible_keys
                    .as_ref()
                    .is_none_or(|keys| keys.contains(&node.key))
        })
    }

    /// Iterates all collection nodes, including structural nodes.
    pub fn items(&self) -> impl Iterator<Item = &Node<Item>> {
        self.ctx.items.nodes()
    }

    /// Returns the text of the first selected item.
    #[must_use]
    pub fn selected_text(&self) -> Option<&str> {
        self.ctx
            .selection
            .get()
            .first()
            .and_then(|key| self.ctx.items.get(key))
            .map(|node| node.text_value.as_str())
    }

    /// Returns the current visible item count.
    #[must_use]
    pub fn visible_count(&self) -> usize {
        self.visible_items().count()
    }

    /// Text to display when no results match.
    #[must_use]
    pub fn no_results_text(&self) -> String {
        (self.ctx.messages.no_results)(&self.ctx.locale)
    }

    /// Text to display while options are loading.
    #[must_use]
    pub fn loading_text(&self) -> String {
        (self.ctx.messages.loading)(&self.ctx.locale)
    }

    /// Accessible trigger label.
    #[must_use]
    pub fn trigger_label(&self) -> String {
        (self.ctx.messages.trigger_label)(&self.ctx.locale)
    }

    /// Accessible clear trigger label.
    #[must_use]
    pub fn clear_label(&self) -> String {
        (self.ctx.messages.clear_label)(&self.ctx.locale)
    }

    /// Live region result-count text.
    #[must_use]
    pub fn results_count_text(&self) -> String {
        if self.visible_count() == 0 {
            self.no_results_text()
        } else {
            let count_text =
                (self.ctx.messages.results_count)(self.visible_count(), &self.ctx.locale);

            if self.ctx.filter_mode == FilterMode::InlineCompletion
                && let Some(key) = valid_highlight(self.ctx)
                && let Some(node) = self.ctx.items.get(key)
            {
                format!("{count_text}. {} highlighted.", node.text_value)
            } else {
                count_text
            }
        }
    }

    const fn aria_autocomplete(&self) -> &'static str {
        match self.ctx.filter_mode {
            FilterMode::None => "none",
            FilterMode::Inline => "inline",
            FilterMode::InlineCompletion => "both",
            FilterMode::Contains | FilterMode::StartsWith | FilterMode::Custom => "list",
        }
    }

    const fn enter_key_hint(&self) -> &'static str {
        match self.ctx.filter_mode {
            FilterMode::None => "done",
            _ => "search",
        }
    }

    fn is_keyboard_composing(&self, data: &KeyboardEventData) -> bool {
        self.ctx.is_composing || data.is_composing || data.key == KeyboardKey::Process
    }
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

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Label => self.label_attrs(),
            Part::Control => self.control_attrs(),
            Part::Input => self.input_attrs(),
            Part::Trigger => self.trigger_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::Positioner => self.positioner_attrs(),
            Part::Content => self.content_attrs(),
            Part::ItemGroup { ref key } => self.item_group_attrs(key),
            Part::ItemGroupLabel { ref key } => self.item_group_label_attrs(key),
            Part::Item { ref key } => self.item_attrs(key),
            Part::ItemText { ref key } => self.item_text_attrs(key),
            Part::ItemIndicator { ref key } => self.item_indicator_attrs(key),
            Part::Empty => self.empty_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::LiveRegion => self.live_region_attrs(),
        }
    }
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_val), (part_attr, part_val)] = part.data_attrs();

    attrs.set(scope_attr, scope_val).set(part_attr, part_val);

    attrs
}

fn open_plan(props: &Props) -> TransitionPlan<Machine> {
    let on_open_change = props.on_open_change.clone();

    TransitionPlan::to(State::Open).apply(move |ctx: &mut Context| {
        let was_open = ctx.open;
        ctx.open = true;

        if !was_open && let Some(callback) = &on_open_change {
            callback(true);
        }
    })
}

fn close_plan(props: &Props) -> TransitionPlan<Machine> {
    let on_open_change = props.on_open_change.clone();

    TransitionPlan::to(State::Closed).apply(move |ctx: &mut Context| {
        let was_open = ctx.open;
        ctx.open = false;
        ctx.highlighted_key = None;

        if was_open && let Some(callback) = &on_open_change {
            callback(false);
        }
    })
}

fn apply_selection_plan(next: selection::State) -> TransitionPlan<Machine> {
    let selected = next.selected_keys.clone();

    TransitionPlan::context_only(move |ctx: &mut Context| {
        ctx.selection_state = next.clone();
        ctx.selection_state.selected_keys = set_selection_value(ctx, selected.clone());
    })
}

fn set_selection_value(ctx: &mut Context, selected: selection::Set) -> selection::Set {
    if ctx.selection.is_controlled() {
        ctx.selection.sync_controlled(Some(selected.clone()));
    }

    ctx.selection.set(selected);
    ctx.selection.get().clone()
}

fn select_item_plan(
    ctx: &Context,
    props: &Props,
    key: &Key,
    ctrl_toggle: bool,
) -> Option<TransitionPlan<Machine>> {
    if !is_selectable_key(ctx, key) {
        return None;
    }

    let next = if ctx.multiple
        && ctrl_toggle
        && ctx.selection_state.behavior == selection::Behavior::Replace
    {
        if ctx.selection_state.is_selected(key) {
            if matches!(ctx.selection.get(), selection::Set::All) {
                ctx.selection_state.deselect_from_all(key, &ctx.items)
            } else {
                ctx.selection_state.deselect(key)
            }
        } else {
            let mut selected = match ctx.selection.get() {
                selection::Set::Multiple(keys) => keys.clone(),
                selection::Set::Single(existing) => BTreeSet::from([existing.clone()]),
                selection::Set::All => ctx.items.item_keys().cloned().collect(),
                selection::Set::Empty | _ => BTreeSet::new(),
            };

            selected.insert(key.clone());

            let mut next = ctx.selection_state.clone();

            next.selected_keys = selection::Set::Multiple(selected);
            next.anchor_key = Some(key.clone());
            next
        }
    } else if ctx.multiple
        && (ctx.selection_state.behavior == selection::Behavior::Toggle || ctrl_toggle)
    {
        ctx.selection_state.toggle(key.clone(), &ctx.items)
    } else {
        ctx.selection_state.select(key.clone())
    };

    let mut next = normalize_selection_state(next);

    remove_disabled_selection_keys(&mut next);

    let selected = next.selected_keys.clone();

    if ctx.multiple {
        Some(TransitionPlan::context_only(move |ctx: &mut Context| {
            ctx.selection_state = next.clone();
            ctx.selection_state.selected_keys = set_selection_value(ctx, selected.clone());
            ctx.input_value.set(String::new());
            ctx.visible_keys = None;
            ctx.inline_completion_prefix = None;
        }))
    } else {
        let label = ctx
            .items
            .get(key)
            .map(|node| node.text_value.clone())
            .unwrap_or_default();

        let on_open_change = props.on_open_change.clone();

        Some(
            TransitionPlan::to(State::Closed).apply(move |ctx: &mut Context| {
                let was_open = ctx.open;

                ctx.selection_state = next.clone();
                ctx.selection_state.selected_keys = set_selection_value(ctx, selected.clone());
                ctx.input_value.set(label.clone());
                ctx.open = false;
                ctx.highlighted_key = None;
                ctx.visible_keys = None;
                ctx.inline_completion_prefix = None;

                if was_open && let Some(callback) = &on_open_change {
                    callback(false);
                }
            }),
        )
    }
}

fn sync_props_plan(ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    let props = props.clone();
    let mut next_ctx = ctx.clone();

    apply_props_to_context(&mut next_ctx, &props);

    let input = next_ctx.input_value.get().clone();
    let should_open =
        should_open_for_items_update(&next_ctx, &props, &input, next_ctx.visible_keys.as_ref());

    let plan = if ctx.open && props.disabled {
        close_plan(&props)
    } else if should_open {
        open_plan(&props)
    } else if ctx.open && !props.allows_empty_collection {
        close_plan(&props)
    } else {
        TransitionPlan::context_only(|_: &mut Context| {})
    };

    plan.apply(move |ctx: &mut Context| {
        apply_props_to_context(ctx, &props);
    })
}

fn apply_props_to_context(ctx: &mut Context, props: &Props) {
    let selected = props
        .value
        .clone()
        .unwrap_or_else(|| ctx.selection.get().clone());

    let selected = normalize_selection_for_mode(selected, props.selection_mode);

    ctx.input_value.sync_controlled(props.input_value.clone());

    if let Some(value) = &props.input_value {
        ctx.input_value.set(value.clone());
    }

    ctx.selection.sync_controlled(if props.value.is_some() {
        Some(selected.clone())
    } else {
        None
    });
    ctx.selection.set(selected.clone());
    ctx.selection_state.mode = props.selection_mode;
    ctx.selection_state.behavior = props.selection_behavior;
    ctx.selection_state.disabled_behavior = props.disabled_behavior;
    ctx.selection_state.disabled_keys = props.disabled_keys.clone();
    ctx.selection_state.selected_keys = selected;

    remove_disabled_selection_keys(&mut ctx.selection_state);

    let selected = ctx.selection_state.selected_keys.clone();
    ctx.selection.sync_controlled(if props.value.is_some() {
        Some(selected.clone())
    } else {
        None
    });
    ctx.selection.set(selected);

    ctx.disabled = props.disabled;
    ctx.readonly = props.readonly;
    ctx.required = props.required;
    ctx.invalid = props.invalid;
    ctx.multiple = props.selection_mode == selection::Mode::Multiple;
    ctx.filter_mode = props.filter_mode;
    ctx.open_on_focus = props.open_on_focus;
    ctx.open_on_click = props.open_on_click;
    ctx.name = props.name.clone();
    ctx.form = props.form.clone();
    ctx.loop_focus = props.loop_focus;
    ctx.is_ios = props.is_ios;
    ctx.ids = ComponentIds::from_id(&props.id);

    invalidate_collection_references(ctx, props.allow_custom_value);

    let input = ctx.input_value.get().clone();

    if !input.is_empty() || ctx.visible_keys.is_some() {
        refresh_filter_and_highlight(ctx, &input);
    }
}

fn should_open_for_input(
    ctx: &Context,
    props: &Props,
    value: &str,
    visible_keys: Option<&BTreeSet<Key>>,
) -> bool {
    !props.disabled
        && (props.allows_empty_collection
            || ((ctx.open || !value.is_empty()) && first_visible_key(ctx, visible_keys).is_some()))
}

fn should_open_for_items_update(
    ctx: &Context,
    props: &Props,
    value: &str,
    visible_keys: Option<&BTreeSet<Key>>,
) -> bool {
    !props.disabled
        && ((ctx.open
            && (props.allows_empty_collection || first_visible_key(ctx, visible_keys).is_some()))
            || (!ctx.open && !value.is_empty() && first_visible_key(ctx, visible_keys).is_some()))
}

fn refresh_filter_and_highlight(ctx: &mut Context, input: &str) {
    ctx.visible_keys = visible_keys_for(ctx, input);

    if !ctx
        .highlighted_key
        .as_ref()
        .is_some_and(|key| is_visible_focusable_key(ctx, key))
    {
        ctx.highlighted_key = first_visible_key(ctx, ctx.visible_keys.as_ref());
    }
}

fn commit_input_plan(ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    if ctx.input_value.get().is_empty() {
        return if props.allow_custom_value {
            close_plan(props)
        } else {
            close_plan(props).apply(|ctx: &mut Context| {
                revert_input_to_selection(ctx);
            })
        };
    }

    let input = ctx.input_value.get().clone();

    if props.allow_custom_value {
        let on_open_change = props.on_open_change.clone();

        if ctx.multiple {
            TransitionPlan::context_only(move |ctx: &mut Context| {
                apply_custom_input_commit(ctx, input);
            })
        } else {
            TransitionPlan::to(State::Closed).apply(move |ctx: &mut Context| {
                let was_open = ctx.open;

                apply_custom_input_commit(ctx, input.clone());
                ctx.open = false;

                if was_open && let Some(callback) = &on_open_change {
                    callback(false);
                }
            })
        }
    } else {
        close_plan(props).apply(|ctx: &mut Context| {
            revert_input_to_selection(ctx);
        })
    }
}

fn apply_custom_input_commit(ctx: &mut Context, input: String) {
    if input.is_empty() || input == ALL_SELECTION_SENTINEL {
        return;
    }

    let key = Key::str(input.clone());
    let next = ctx.selection_state.select(key);
    let selected = next.selected_keys.clone();

    ctx.selection_state = next;
    ctx.selection_state.selected_keys = set_selection_value(ctx, selected);

    if ctx.multiple {
        ctx.input_value.set(String::new());
    } else {
        ctx.input_value.set(input);
    }

    ctx.highlighted_key = None;
    ctx.visible_keys = None;
    ctx.inline_completion_prefix = None;
}

fn input_matches_selected_item_label(ctx: &Context, input: &str) -> bool {
    match ctx.selection.get() {
        selection::Set::Single(key) => ctx
            .items
            .get(key)
            .is_some_and(|node| node.text_value == input),
        _ => false,
    }
}

fn revert_input_to_selection(ctx: &mut Context) {
    if ctx.multiple {
        ctx.input_value.set(String::new());
        ctx.visible_keys = None;
        ctx.inline_completion_prefix = None;
        return;
    }

    let label = ctx
        .selection
        .get()
        .first()
        .and_then(|key| ctx.items.get(key))
        .map(|node| node.text_value.clone())
        .unwrap_or_default();

    ctx.input_value.set(label);
    ctx.visible_keys = None;
    ctx.inline_completion_prefix = None;
}

fn default_open_highlight(ctx: &Context, props: &Props) -> Option<Key> {
    props
        .default_highlighted_key
        .clone()
        .filter(|key| is_visible_focusable_key(ctx, key))
        .or_else(|| first_visible_key(ctx, ctx.visible_keys.as_ref()))
}

fn visible_keys_for(ctx: &Context, input: &str) -> Option<BTreeSet<Key>> {
    if input.is_empty()
        || matches!(
            ctx.filter_mode,
            FilterMode::None | FilterMode::Inline | FilterMode::Custom
        )
    {
        return None;
    }

    let input = input.to_lowercase();

    Some(
        ctx.items
            .nodes()
            .filter(|node| node.node_type == NodeType::Item)
            .filter(|node| match ctx.filter_mode {
                FilterMode::Contains => node.text_value.to_lowercase().contains(&input),
                FilterMode::StartsWith | FilterMode::InlineCompletion => {
                    node.text_value.to_lowercase().starts_with(&input)
                }
                FilterMode::None | FilterMode::Inline | FilterMode::Custom => true,
            })
            .map(|node| node.key.clone())
            .collect(),
    )
}

fn input_change_values(
    ctx: &Context,
    raw_value: &str,
) -> (Option<BTreeSet<Key>>, Option<Key>, String, Option<String>) {
    let visible_keys = visible_keys_for(ctx, raw_value);
    let mut highlighted_key = first_visible_key(ctx, visible_keys.as_ref());
    let mut input_value = raw_value.to_string();
    let mut inline_completion_prefix = None;

    if matches!(
        ctx.filter_mode,
        FilterMode::Inline | FilterMode::InlineCompletion
    ) && !raw_value.is_empty()
    {
        if let Some((key, label)) = inline_completion(ctx, raw_value) {
            highlighted_key = Some(key);
            inline_completion_prefix = (label != raw_value).then(|| raw_value.to_string());
            input_value = label;
        } else if matches!(ctx.filter_mode, FilterMode::Inline) {
            highlighted_key = None;
        }
    }

    (
        visible_keys,
        highlighted_key,
        input_value,
        inline_completion_prefix,
    )
}

fn inline_completion(ctx: &Context, input: &str) -> Option<(Key, String)> {
    let input = input.to_lowercase();

    ctx.items
        .nodes()
        .find(|node| {
            node.node_type == NodeType::Item
                && node.text_value.to_lowercase().starts_with(&input)
                && is_focusable_key(ctx, &node.key)
        })
        .map(|node| (node.key.clone(), node.text_value.clone()))
}

fn first_visible_key(ctx: &Context, visible_keys: Option<&BTreeSet<Key>>) -> Option<Key> {
    ctx.items
        .nodes()
        .find(|node| {
            node.node_type == NodeType::Item
                && visible_keys.is_none_or(|keys| keys.contains(&node.key))
                && is_focusable_key(ctx, &node.key)
        })
        .map(|node| node.key.clone())
}

fn last_visible_key(ctx: &Context) -> Option<Key> {
    ctx.items
        .nodes()
        .filter(|node| {
            node.node_type == NodeType::Item
                && ctx
                    .visible_keys
                    .as_ref()
                    .is_none_or(|keys| keys.contains(&node.key))
                && is_focusable_key(ctx, &node.key)
        })
        .last()
        .map(|node| node.key.clone())
}

fn next_visible_key(ctx: &Context) -> Option<Key> {
    step_visible_key(ctx, true)
}

fn prev_visible_key(ctx: &Context) -> Option<Key> {
    step_visible_key(ctx, false)
}

fn step_visible_key(ctx: &Context, forward: bool) -> Option<Key> {
    let keys = visible_focusable_keys(ctx);

    if keys.is_empty() {
        return None;
    }

    let Some(current) = &ctx.highlighted_key else {
        return if forward {
            keys.first().cloned()
        } else {
            keys.last().cloned()
        };
    };

    let Some(index) = keys.iter().position(|key| key == current) else {
        return if forward {
            keys.first().cloned()
        } else {
            keys.last().cloned()
        };
    };

    if forward {
        keys.get(index + 1)
            .cloned()
            .or_else(|| ctx.loop_focus.then(|| keys[0].clone()))
            .or_else(|| keys.get(index).cloned())
    } else if index > 0 {
        keys.get(index - 1).cloned()
    } else {
        ctx.loop_focus
            .then(|| keys[keys.len() - 1].clone())
            .or_else(|| keys.first().cloned())
    }
}

fn visible_focusable_keys(ctx: &Context) -> Vec<Key> {
    ctx.items
        .nodes()
        .filter(|node| {
            node.node_type == NodeType::Item
                && ctx
                    .visible_keys
                    .as_ref()
                    .is_none_or(|keys| keys.contains(&node.key))
                && is_focusable_key(ctx, &node.key)
        })
        .map(|node| node.key.clone())
        .collect()
}

fn invalidate_collection_references(ctx: &mut Context, allow_custom_value: bool) {
    let selected_single_was_pruned = matches!(ctx.selection.get(), selection::Set::Single(key) if !ctx.items.contains_key(key))
        && !allow_custom_value;

    if ctx
        .highlighted_key
        .as_ref()
        .is_some_and(|key| !is_visible_focusable_key(ctx, key))
    {
        ctx.highlighted_key = first_visible_key(ctx, ctx.visible_keys.as_ref());
    }

    if let Some(keys) = &mut ctx.visible_keys {
        keys.retain(|key| ctx.items.contains_key(key));

        if keys.is_empty() {
            ctx.highlighted_key = None;
        }
    }

    ctx.selection_state.selected_keys =
        retain_present_selection(ctx.selection.get(), &ctx.items, allow_custom_value);

    remove_disabled_selection_keys(&mut ctx.selection_state);

    let selected = ctx.selection_state.selected_keys.clone();

    let selected = set_selection_value(ctx, selected);

    if selected_single_was_pruned && selected.is_empty() && !ctx.multiple {
        ctx.input_value.set(String::new());
        ctx.visible_keys = None;
    }

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
    allow_custom_value: bool,
) -> selection::Set {
    if allow_custom_value {
        return selection.clone();
    }

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
        .filter(|key| is_visible_focusable_key(ctx, key))
}

fn is_visible_focusable_key(ctx: &Context, key: &Key) -> bool {
    ctx.visible_keys
        .as_ref()
        .is_none_or(|keys| keys.contains(key))
        && is_focusable_key(ctx, key)
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

fn remove_disabled_selection_keys(state: &mut selection::State) {
    match &mut state.selected_keys {
        selection::Set::Single(key) if state.disabled_keys.contains(key) => {
            state.selected_keys = selection::Set::Empty;
        }

        selection::Set::Multiple(keys) => {
            keys.retain(|key| !state.disabled_keys.contains(key));

            if keys.is_empty() {
                state.selected_keys = selection::Set::Empty;
            }
        }

        _ => {}
    }
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
        selection::Mode::Multiple => match selected {
            selection::Set::Single(key) => {
                let mut keys = BTreeSet::new();

                keys.insert(key);

                selection::Set::Multiple(keys)
            }

            selection::Set::Multiple(keys) => selection::Set::Multiple(keys),

            selection::Set::All => selection::Set::All,

            _ => selection::Set::Empty,
        },
    }
}

fn serialize_selection(selection: &selection::Set) -> String {
    match selection {
        selection::Set::Single(key) => serialize_key(key),

        selection::Set::Multiple(keys) => {
            keys.iter().map(serialize_key).collect::<Vec<_>>().join(",")
        }

        selection::Set::All => ALL_SELECTION_SENTINEL.to_string(),

        _ => String::new(),
    }
}

fn serialize_key(key: &Key) -> String {
    let raw = key.to_string();

    if raw == ALL_SELECTION_SENTINEL {
        return "%5F%5Fars_all".to_string();
    }

    let mut encoded = String::new();

    for ch in raw.chars() {
        match ch {
            '%' => encoded.push_str("%25"),
            ',' => encoded.push_str("%2C"),
            _ => encoded.push(ch),
        }
    }

    encoded
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use core::cell::RefCell;
    use std::sync::{Arc, Mutex};

    use ars_collections::{CollectionBuilder, selection};
    use ars_core::{AttrValue, Env, SendResult, Service};

    use super::*;

    fn key(value: u64) -> Key {
        Key::int(value)
    }

    fn collection() -> StaticCollection<Item> {
        CollectionBuilder::new()
            .item(
                key(1),
                "Apple",
                Item {
                    label: "Apple".into(),
                },
            )
            .item(
                key(2),
                "Banana",
                Item {
                    label: "Banana".into(),
                },
            )
            .item(
                key(3),
                "Apricot",
                Item {
                    label: "Apricot".into(),
                },
            )
            .build()
    }

    fn make_service(props: Props) -> Service<Machine> {
        let mut service = Service::<Machine>::new(props, &Env::default(), &Messages::default());

        send_event(&mut service, Event::UpdateItems(collection()));

        service
    }

    fn send_event(service: &mut Service<Machine>, event: Event) {
        drop(service.send(event));
    }

    fn with_api<R>(service: &Service<Machine>, f: impl FnOnce(&Api<'_>) -> R) -> R {
        let send = |_| {};

        let api = service.connect(&send);

        f(&api)
    }

    fn dispatch_api(service: &mut Service<Machine>, f: impl FnOnce(&Api<'_>)) {
        let events = RefCell::new(Vec::new());
        {
            let send = |event| {
                events.borrow_mut().push(event);
            };

            let api = service.connect(&send);

            f(&api);
        }

        for event in events.into_inner() {
            send_event(service, event);
        }
    }

    fn api_events(service: &Service<Machine>, f: impl FnOnce(&Api<'_>)) -> Vec<Event> {
        let events = RefCell::new(Vec::new());
        {
            let send = |event| {
                events.borrow_mut().push(event);
            };

            let api = service.connect(&send);

            f(&api);
        }

        events.into_inner()
    }

    fn keyboard(key: KeyboardKey, character: Option<char>) -> KeyboardEventData {
        KeyboardEventData {
            key,
            code: String::new(),
            ctrl_key: false,
            shift_key: false,
            alt_key: false,
            meta_key: false,
            repeat: false,
            is_composing: false,
            character,
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{:#?}", attrs.iter_attrs().collect::<Vec<_>>())
    }

    #[test]
    fn input_and_listbox_attrs_expose_combobox_contract() {
        let service = make_service(Props::new().id("combo"));

        with_api(&service, |api| {
            assert_eq!(api.input_attrs().get(&HtmlAttr::Role), Some("combobox"));
            assert_eq!(
                api.input_attrs()
                    .get(&HtmlAttr::Aria(AriaAttr::AutoComplete)),
                Some("list")
            );
            assert_eq!(api.content_attrs().get(&HtmlAttr::Role), Some("listbox"));
            assert_eq!(
                api.content_attrs().get(&HtmlAttr::Aria(AriaAttr::Label)),
                Some("Show suggestions")
            );
            assert!(
                !api.content_attrs()
                    .contains(&HtmlAttr::Aria(AriaAttr::LabelledBy))
            );
        });
    }

    #[test]
    fn typing_filters_options_and_opens_popup() {
        let mut service = make_service(Props::new().id("combo").open_on_focus(false));

        let result = service.send(Event::InputChange("ap".into()));

        assert!(result.state_changed);
        assert!(service.context().open);
        assert_eq!(service.context().input_value.get(), "ap");
        assert_eq!(service.context().highlighted_key, Some(key(1)));

        let visible = with_api(&service, |api| {
            api.visible_items()
                .map(|node| node.key.clone())
                .collect::<Vec<_>>()
        });

        assert_eq!(visible, vec![key(1), key(3)]);
    }

    #[test]
    fn empty_input_only_opens_when_empty_collections_are_allowed() {
        let mut closed = make_service(
            Props::new()
                .id("combo")
                .open_on_focus(false)
                .allows_empty_collection(false),
        );

        send_event(&mut closed, Event::InputChange(String::new()));

        assert_eq!(closed.state(), &State::Closed);

        let mut allowed = make_service(
            Props::new()
                .id("combo")
                .open_on_focus(false)
                .allows_empty_collection(true),
        );

        send_event(&mut allowed, Event::InputChange(String::new()));

        assert_eq!(allowed.state(), &State::Open);
    }

    #[test]
    fn zero_result_input_respects_empty_collection_policy() {
        let mut disallowed = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut disallowed, Event::InputChange("zz".into()));

        assert_eq!(disallowed.state(), &State::Closed);
        assert_eq!(disallowed.context().highlighted_key, None);

        let mut allowed = make_service(
            Props::new()
                .id("combo")
                .open_on_focus(false)
                .allows_empty_collection(true),
        );

        send_event(&mut allowed, Event::InputChange("zz".into()));

        assert_eq!(allowed.state(), &State::Open);
        assert_eq!(allowed.context().highlighted_key, None);
    }

    #[test]
    fn passive_update_items_does_not_open_empty_allowed_combobox() {
        let mut service = Service::<Machine>::new(
            Props::new()
                .id("combo")
                .open_on_focus(false)
                .allows_empty_collection(true),
            &Env::default(),
            &Messages::default(),
        );

        send_event(&mut service, Event::UpdateItems(collection()));

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
    }

    #[test]
    fn arrow_keys_navigate_filtered_list() {
        let mut service = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut service, Event::InputChange("ap".into()));

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::ArrowDown, None));
        });

        assert_eq!(service.context().highlighted_key, Some(key(3)));

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::ArrowUp, None));
        });

        assert_eq!(service.context().highlighted_key, Some(key(1)));
    }

    #[test]
    fn enter_selects_highlighted_item() {
        let mut service = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut service, Event::InputChange("ban".into()));

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        assert_eq!(service.state(), &State::Closed);
        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Single(key(2))
        );
        assert_eq!(service.context().input_value.get(), "Banana");
    }

    #[test]
    fn enter_commits_or_reverts_unmatched_custom_input() {
        let mut custom = make_service(
            Props::new()
                .id("combo")
                .open_on_focus(false)
                .allow_custom_value(true),
        );

        send_event(&mut custom, Event::InputChange("Dragonfruit".into()));

        dispatch_api(&mut custom, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        assert_eq!(custom.state(), &State::Closed);
        assert_eq!(
            custom.context().selection.get(),
            &selection::Set::Single(Key::str("Dragonfruit"))
        );
        assert_eq!(custom.context().input_value.get(), "Dragonfruit");

        let mut strict = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut strict, Event::Open);
        send_event(&mut strict, Event::SelectItem(key(1)));
        send_event(&mut strict, Event::InputChange("Dragonfruit".into()));

        dispatch_api(&mut strict, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        assert_eq!(strict.state(), &State::Closed);
        assert_eq!(
            strict.context().selection.get(),
            &selection::Set::Single(key(1))
        );
        assert_eq!(strict.context().input_value.get(), "Apple");

        send_event(&mut strict, Event::InputChange(String::new()));
        dispatch_api(&mut strict, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        assert_eq!(
            strict.context().selection.get(),
            &selection::Set::Single(key(1))
        );
        assert_eq!(strict.context().input_value.get(), "Apple");
    }

    #[test]
    fn controlled_custom_input_commit_preserves_pending_selection() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .name("fruit")
                .value(selection::Set::Empty)
                .open_on_focus(false)
                .allow_custom_value(true),
        );

        send_event(&mut service, Event::InputChange("Dragonfruit".into()));
        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Single(Key::str("Dragonfruit"))
        );
        assert_eq!(
            service.context().selection_state.selected_keys,
            selection::Set::Single(Key::str("Dragonfruit"))
        );
        with_api(&service, |api| {
            assert_eq!(
                api.hidden_input_attrs().get(&HtmlAttr::Value),
                Some("Dragonfruit")
            );
        });
    }

    #[test]
    fn reserved_all_sentinel_custom_input_is_not_selected() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .name("fruit")
                .open_on_focus(false)
                .allow_custom_value(true),
        );

        send_event(&mut service, Event::InputChange("__ars_all".into()));

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        assert!(service.context().selection.get().is_empty());
        with_api(&service, |api| {
            assert_eq!(api.hidden_input_attrs().get(&HtmlAttr::Value), Some(""));
        });
    }

    #[test]
    fn custom_input_commit_preserves_multiple_selection_mode() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .allow_custom_value(true)
                .default_value(selection::Set::Multiple(BTreeSet::from([key(1)])))
                .open_on_focus(false),
        );

        send_event(&mut service, Event::InputChange("Dragonfruit".into()));
        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Multiple(BTreeSet::from([key(1), Key::str("Dragonfruit")]))
        );
        assert_eq!(
            service.context().selection_state.mode,
            selection::Mode::Multiple
        );
    }

    #[test]
    fn custom_input_blur_commits_and_item_updates_preserve_custom_values() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .name("fruit")
                .allow_custom_value(true)
                .open_on_focus(false),
        );

        send_event(&mut service, Event::InputChange("Dragonfruit".into()));
        send_event(&mut service, Event::Blur);

        let custom_key = Key::str("Dragonfruit");

        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Single(custom_key.clone())
        );
        with_api(&service, |api| {
            assert_eq!(
                api.hidden_input_attrs().get(&HtmlAttr::Value),
                Some("Dragonfruit")
            );
        });

        send_event(&mut service, Event::UpdateItems(collection()));

        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Single(custom_key)
        );

        service.set_props(
            Props::new()
                .id("combo")
                .name("fruit")
                .allow_custom_value(true),
        );
        send_event(&mut service, Event::SyncProps);

        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Single(Key::str("Dragonfruit"))
        );
    }

    #[test]
    fn custom_value_blur_preserves_selected_item_key() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .name("fruit")
                .allow_custom_value(true),
        );

        send_event(&mut service, Event::Open);
        send_event(&mut service, Event::SelectItem(key(1)));

        assert_eq!(service.context().input_value.get(), "Apple");

        send_event(&mut service, Event::Blur);

        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Single(key(1))
        );
        with_api(&service, |api| {
            assert_eq!(api.hidden_input_attrs().get(&HtmlAttr::Value), Some("1"));
        });
    }

    #[test]
    fn multi_select_revert_clears_transient_input() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Multiple(BTreeSet::from([key(1)])))
                .open_on_focus(false),
        );

        send_event(&mut service, Event::InputChange("Dragonfruit".into()));
        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Multiple(BTreeSet::from([key(1)]))
        );
        assert_eq!(service.context().input_value.get(), "");
        assert_eq!(service.context().visible_keys, None);
    }

    #[test]
    fn ime_composition_suppresses_filtering_until_end() {
        let mut service = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut service, Event::CompositionStart);
        send_event(&mut service, Event::InputChange("ba".into()));

        assert!(service.context().is_composing);
        assert_eq!(service.context().input_value.get(), "");
        assert_eq!(service.context().visible_keys, None);

        send_event(&mut service, Event::CompositionEnd("ba".into()));

        assert!(!service.context().is_composing);
        assert_eq!(service.context().input_value.get(), "ba");
        assert_eq!(service.state(), &State::Open);
        assert_eq!(service.context().highlighted_key, Some(key(2)));
    }

    #[test]
    fn composition_end_is_ignored_when_disabled_or_readonly() {
        let mut disabled = make_service(
            Props::new()
                .id("disabled")
                .disabled(true)
                .default_input_value("before"),
        );
        let disabled_visible = disabled.context().visible_keys.clone();
        let disabled_highlighted = disabled.context().highlighted_key.clone();

        send_event(&mut disabled, Event::CompositionStart);
        send_event(&mut disabled, Event::CompositionEnd("ba".into()));

        assert!(!disabled.context().is_composing);
        assert_eq!(disabled.context().input_value.get(), "before");
        assert_eq!(disabled.context().visible_keys, disabled_visible);
        assert_eq!(disabled.context().highlighted_key, disabled_highlighted);

        let mut readonly = make_service(
            Props::new()
                .id("readonly")
                .readonly(true)
                .default_input_value("before"),
        );
        let readonly_visible = readonly.context().visible_keys.clone();
        let readonly_highlighted = readonly.context().highlighted_key.clone();

        send_event(&mut readonly, Event::CompositionStart);
        send_event(&mut readonly, Event::CompositionEnd("ba".into()));

        assert!(!readonly.context().is_composing);
        assert_eq!(readonly.context().input_value.get(), "before");
        assert_eq!(readonly.context().visible_keys, readonly_visible);
        assert_eq!(readonly.context().highlighted_key, readonly_highlighted);
    }

    #[test]
    fn keyboard_process_marks_composition_and_suppresses_enter() {
        let mut service = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut service, Event::InputChange("ap".into()));

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Process, None));
        });

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        assert!(service.context().is_composing);
        assert!(service.context().selection.get().is_empty());
    }

    #[test]
    fn autocomplete_modes_map_to_aria_values() {
        let cases = [
            (FilterMode::Contains, "list"),
            (FilterMode::StartsWith, "list"),
            (FilterMode::Custom, "list"),
            (FilterMode::Inline, "inline"),
            (FilterMode::InlineCompletion, "both"),
            (FilterMode::None, "none"),
        ];

        for (mode, expected) in cases {
            let service = make_service(Props::new().id("combo").filter_mode(mode));

            with_api(&service, |api| {
                assert_eq!(
                    api.input_attrs()
                        .get(&HtmlAttr::Aria(AriaAttr::AutoComplete)),
                    Some(expected)
                );
            });
        }
    }

    #[test]
    fn hidden_input_reflects_form_integration() {
        let mut service = make_service(Props::new().id("combo").name("fruit").form("checkout"));

        send_event(&mut service, Event::Open);
        send_event(&mut service, Event::SelectItem(key(1)));

        let send = |_| {};

        let api = service.connect(&send);

        let attrs = api.hidden_input_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Type), Some("hidden"));
        assert_eq!(attrs.get(&HtmlAttr::Name), Some("fruit"));
        assert_eq!(attrs.get(&HtmlAttr::Form), Some("checkout"));
        assert_eq!(attrs.get(&HtmlAttr::Value), Some("1"));
    }

    #[test]
    fn hidden_input_serializes_all_without_key_collision() {
        let service = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::All),
        );

        with_api(&service, |api| {
            assert_eq!(
                api.hidden_input_attrs().get(&HtmlAttr::Value),
                Some("__ars_all")
            );
        });

        let mut sentinel_key = Service::<Machine>::new(
            Props::new().id("combo").name("fruit"),
            &Env::default(),
            &Messages::default(),
        );
        let sentinel = Key::str("__ars_all");

        send_event(
            &mut sentinel_key,
            Event::UpdateItems(StaticCollection::new([(
                sentinel.clone(),
                "__ars_all".to_string(),
                Item {
                    label: "__ars_all".into(),
                },
            )])),
        );
        send_event(&mut sentinel_key, Event::SelectItem(sentinel));

        with_api(&sentinel_key, |api| {
            assert_eq!(
                api.hidden_input_attrs().get(&HtmlAttr::Value),
                Some("%5F%5Fars_all")
            );
        });
    }

    #[test]
    fn hidden_input_escapes_multiple_key_delimiters() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .name("fruit")
                .selection_mode(selection::Mode::Multiple)
                .allow_custom_value(true)
                .open_on_focus(false),
        );

        send_event(&mut service, Event::InputChange("a,b".into()));
        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        with_api(&service, |api| {
            assert_eq!(
                api.hidden_input_attrs().get(&HtmlAttr::Value),
                Some("a%2Cb")
            );
        });
    }

    #[test]
    fn disabled_and_readonly_guards_block_mutation() {
        let mut disabled = make_service(Props::new().id("disabled").disabled(true));
        let mut readonly = make_service(Props::new().id("readonly").readonly(true));

        assert!(disabled.send(Event::InputChange("ap".into())).is_noop());
        assert!(readonly.send(Event::InputChange("ap".into())).is_noop());

        send_event(&mut disabled, Event::Open);
        send_event(&mut readonly, Event::Open);

        assert_eq!(disabled.state(), &State::Closed);
        assert_eq!(readonly.state(), &State::Open);
        assert!(readonly.send(Event::SelectItem(key(1))).is_noop());

        let mut disabled_custom = make_service(
            Props::new()
                .id("disabled-custom")
                .disabled(true)
                .default_input_value("Dragonfruit")
                .allow_custom_value(true),
        );

        send_event(&mut disabled_custom, Event::Blur);

        assert!(disabled_custom.context().selection.get().is_empty());
        assert_eq!(disabled_custom.context().input_value.get(), "Dragonfruit");

        let mut readonly_custom = make_service(
            Props::new()
                .id("readonly-custom")
                .readonly(true)
                .default_input_value("Dragonfruit")
                .allow_custom_value(true),
        );

        send_event(&mut readonly_custom, Event::Blur);

        assert!(readonly_custom.context().selection.get().is_empty());
        assert_eq!(readonly_custom.context().input_value.get(), "Dragonfruit");
    }

    #[test]
    fn readonly_clear_trigger_is_disabled() {
        let service = make_service(Props::new().id("readonly").readonly(true));

        with_api(&service, |api| {
            assert_eq!(
                api.clear_trigger_attrs().get(&HtmlAttr::Disabled),
                Some("true")
            );
        });
    }

    #[test]
    fn disabled_update_items_does_not_open_popup() {
        let mut service = Service::<Machine>::new(
            Props::new()
                .id("combo")
                .disabled(true)
                .default_input_value("ap"),
            &Env::default(),
            &Messages::default(),
        );

        send_event(&mut service, Event::UpdateItems(collection()));

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(service.context().highlighted_key, Some(key(1)));
    }

    #[test]
    fn multiple_selection_stays_open_and_ctrl_toggles_replace_mode() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .selection_behavior(selection::Behavior::Replace),
        );

        send_event(&mut service, Event::Open);
        send_event(&mut service, Event::SelectItem(key(1)));
        send_event(&mut service, Event::SelectItemCtrl(key(2)));
        send_event(&mut service, Event::SelectItemCtrl(key(1)));

        assert_eq!(service.state(), &State::Open);
        assert!(!service.context().selection.get().contains(&key(1)));
        assert!(service.context().selection.get().contains(&key(2)));
    }

    #[test]
    fn disabled_items_are_skipped_during_navigation() {
        let disabled_keys = BTreeSet::from([key(2)]);

        let mut service = make_service(
            Props::new()
                .id("combo")
                .disabled_keys(disabled_keys)
                .open_on_focus(false),
        );

        send_event(&mut service, Event::InputChange("".into()));
        send_event(&mut service, Event::Open);
        send_event(&mut service, Event::HighlightNext);

        assert_eq!(service.context().highlighted_key, Some(key(3)));
    }

    #[test]
    fn custom_filter_mode_disables_builtin_filtering() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .filter_mode(FilterMode::Custom)
                .open_on_focus(false),
        );

        send_event(&mut service, Event::InputChange("does-not-match".into()));

        let send = |_| {};

        let api = service.connect(&send);

        assert_eq!(api.visible_count(), 3);
        assert_eq!(service.context().visible_keys, None);
    }

    #[test]
    fn inline_filter_modes_apply_completion_to_input() {
        let mut inline = make_service(
            Props::new()
                .id("combo")
                .filter_mode(FilterMode::Inline)
                .open_on_focus(false),
        );

        send_event(&mut inline, Event::InputChange("ap".into()));

        assert_eq!(inline.context().input_value.get(), "Apple");
        assert_eq!(inline.context().highlighted_key, Some(key(1)));
        assert!(inline.context().visible_keys.is_none());

        let mut both = make_service(
            Props::new()
                .id("combo")
                .filter_mode(FilterMode::InlineCompletion)
                .open_on_focus(false),
        );

        send_event(&mut both, Event::InputChange("ba".into()));

        assert_eq!(both.context().input_value.get(), "Banana");
        assert_eq!(both.context().highlighted_key, Some(key(2)));
        assert_eq!(both.context().visible_keys, Some(BTreeSet::from([key(2)])));
    }

    #[test]
    fn escape_clears_inline_completion_before_closing_or_clearing_input() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .filter_mode(FilterMode::InlineCompletion)
                .open_on_focus(false),
        );

        send_event(&mut service, Event::InputChange("ap".into()));

        assert_eq!(service.context().input_value.get(), "Apple");
        assert_eq!(service.state(), &State::Open);

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Escape, None));
        });

        assert_eq!(service.context().input_value.get(), "ap");
        assert_eq!(service.state(), &State::Open);

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Escape, None));
        });

        assert_eq!(service.context().input_value.get(), "ap");
        assert_eq!(service.state(), &State::Closed);

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Escape, None));
        });

        assert_eq!(service.context().input_value.get(), "");
    }

    #[test]
    fn update_items_invalidates_stale_references() {
        let mut service = make_service(Props::new().id("combo"));

        send_event(&mut service, Event::Open);
        send_event(&mut service, Event::SelectItem(key(1)));
        send_event(&mut service, Event::Open);
        send_event(&mut service, Event::InputChange(String::new()));
        send_event(&mut service, Event::HighlightItem(Some(key(2))));
        send_event(
            &mut service,
            Event::UpdateItems(StaticCollection::new([(
                key(9),
                "Pear".to_string(),
                Item {
                    label: "Pear".into(),
                },
            )])),
        );

        assert_eq!(service.context().highlighted_key, Some(key(9)));
        assert!(service.context().selection.get().is_empty());
    }

    #[test]
    fn update_items_prunes_controlled_selection_value() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .name("fruit")
                .value(selection::Set::Single(key(1))),
        );

        send_event(
            &mut service,
            Event::UpdateItems(StaticCollection::new([(
                key(9),
                "Pear".to_string(),
                Item {
                    label: "Pear".into(),
                },
            )])),
        );

        assert!(service.context().selection.get().is_empty());
        assert!(service.context().selection_state.selected_keys.is_empty());
        with_api(&service, |api| {
            assert_eq!(api.hidden_input_attrs().get(&HtmlAttr::Value), Some(""));
        });
    }

    #[test]
    fn update_items_pruning_selected_single_value_clears_input() {
        let mut service = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut service, Event::Open);
        send_event(&mut service, Event::SelectItem(key(1)));

        assert_eq!(service.context().input_value.get(), "Apple");

        send_event(
            &mut service,
            Event::UpdateItems(StaticCollection::new([(
                key(9),
                "Pear".to_string(),
                Item {
                    label: "Pear".into(),
                },
            )])),
        );

        assert!(service.context().selection.get().is_empty());
        assert_eq!(service.context().input_value.get(), "");
    }

    #[test]
    fn update_items_recomputes_filter_against_replacement_collection() {
        let mut service = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut service, Event::InputChange("cher".into()));

        assert_eq!(service.state(), &State::Closed);

        let replacement = CollectionBuilder::new()
            .item(
                key(10),
                "Cherry",
                Item {
                    label: "Cherry".into(),
                },
            )
            .item(
                key(11),
                "Cherimoya",
                Item {
                    label: "Cherimoya".into(),
                },
            )
            .build();

        send_event(&mut service, Event::UpdateItems(replacement));

        assert_eq!(service.state(), &State::Open);
        assert_eq!(service.context().highlighted_key, Some(key(10)));

        let visible = with_api(&service, |api| {
            api.visible_items()
                .map(|node| node.key.clone())
                .collect::<Vec<_>>()
        });

        assert_eq!(visible, vec![key(10), key(11)]);
    }

    #[test]
    fn describedby_orders_error_before_description() {
        let mut service = make_service(Props::new().id("combo").invalid(true));

        send_event(&mut service, Event::SetDescriptionPresent(true));

        assert_eq!(
            with_api(&service, |api| {
                api.input_attrs()
                    .get(&HtmlAttr::Aria(AriaAttr::DescribedBy))
                    .map(str::to_string)
            }),
            Some(String::from("combo-error-message combo-description"))
        );
    }

    #[test]
    fn empty_state_and_live_region_attrs_are_exposed() {
        let service = make_service(Props::new().id("combo"));

        assert_eq!(
            with_api(&service, |api| {
                api.empty_attrs().get(&HtmlAttr::Role).map(str::to_string)
            }),
            Some(String::from("none"))
        );
        assert_eq!(
            with_api(&service, |api| {
                api.live_region_attrs()
                    .get(&HtmlAttr::Aria(AriaAttr::Live))
                    .map(str::to_string)
            }),
            Some(String::from("polite"))
        );
    }

    #[test]
    fn combobox_attrs_snapshot_all_parts_and_state_branches() {
        let mut combo = make_service(
            Props::new()
                .id("combo")
                .placeholder("Search fruit")
                .required(true)
                .invalid(true)
                .name("fruit")
                .form("checkout")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Multiple(BTreeSet::from([key(1)]))),
        );

        send_event(&mut combo, Event::SetDescriptionPresent(true));
        send_event(&mut combo, Event::InputChange("ap".into()));
        send_event(&mut combo, Event::HighlightItem(Some(key(1))));

        with_api(&combo, |api| {
            insta::assert_snapshot!(
                "combobox_root_open_invalid",
                snapshot_attrs(&api.root_attrs())
            );
            insta::assert_snapshot!("combobox_label", snapshot_attrs(&api.label_attrs()));
            insta::assert_snapshot!("combobox_control", snapshot_attrs(&api.control_attrs()));
            insta::assert_snapshot!("combobox_input_open", snapshot_attrs(&api.input_attrs()));
            insta::assert_snapshot!("combobox_trigger", snapshot_attrs(&api.trigger_attrs()));
            insta::assert_snapshot!(
                "combobox_clear_trigger",
                snapshot_attrs(&api.clear_trigger_attrs())
            );
            insta::assert_snapshot!(
                "combobox_positioner",
                snapshot_attrs(&api.positioner_attrs())
            );
            insta::assert_snapshot!(
                "combobox_content_multi",
                snapshot_attrs(&api.content_attrs())
            );
            insta::assert_snapshot!(
                "combobox_item_selected_highlighted",
                snapshot_attrs(&api.item_attrs(&key(1)))
            );
            insta::assert_snapshot!(
                "combobox_item_text",
                snapshot_attrs(&api.item_text_attrs(&key(1)))
            );
            insta::assert_snapshot!(
                "combobox_item_indicator_selected",
                snapshot_attrs(&api.item_indicator_attrs(&key(1)))
            );
            insta::assert_snapshot!(
                "combobox_item_group",
                snapshot_attrs(&api.item_group_attrs(&Key::str("group")))
            );
            insta::assert_snapshot!(
                "combobox_item_group_label",
                snapshot_attrs(&api.item_group_label_attrs(&Key::str("group")))
            );
            insta::assert_snapshot!("combobox_empty", snapshot_attrs(&api.empty_attrs()));
            insta::assert_snapshot!(
                "combobox_description",
                snapshot_attrs(&api.description_attrs())
            );
            insta::assert_snapshot!(
                "combobox_error_message",
                snapshot_attrs(&api.error_message_attrs())
            );
            insta::assert_snapshot!(
                "combobox_live_region",
                snapshot_attrs(&api.live_region_attrs())
            );
            insta::assert_snapshot!(
                "combobox_hidden_input",
                snapshot_attrs(&api.hidden_input_attrs())
            );
        });

        let closed_disabled =
            make_service(Props::new().id("disabled").disabled(true).readonly(true));

        with_api(&closed_disabled, |api| {
            insta::assert_snapshot!("combobox_root_disabled", snapshot_attrs(&api.root_attrs()));
            insta::assert_snapshot!(
                "combobox_input_closed_disabled",
                snapshot_attrs(&api.input_attrs())
            );
            insta::assert_snapshot!(
                "combobox_trigger_disabled",
                snapshot_attrs(&api.trigger_attrs())
            );
            insta::assert_snapshot!(
                "combobox_clear_trigger_disabled",
                snapshot_attrs(&api.clear_trigger_attrs())
            );
        });

        for mode in [
            FilterMode::Contains,
            FilterMode::StartsWith,
            FilterMode::Custom,
            FilterMode::None,
            FilterMode::Inline,
            FilterMode::InlineCompletion,
        ] {
            let mode_service = make_service(Props::new().id("mode").filter_mode(mode));

            with_api(&mode_service, |api| {
                insta::assert_snapshot!(
                    format!("combobox_input_mode_{mode:?}").to_lowercase(),
                    snapshot_attrs(&api.input_attrs())
                );
            });
        }

        let send = |_| {};

        let api = closed_disabled.connect(&send);

        assert!(matches!(
            api.hidden_input_attrs().get_value(&HtmlAttr::Value),
            Some(AttrValue::String(_))
        ));
    }

    #[test]
    fn props_builder_round_trips_controlled_and_misc_fields() {
        let positioning = PositioningOptions {
            shift_padding: 12.0,
            ..PositioningOptions::default()
        };

        let opened = Arc::new(Mutex::new(Vec::new()));
        let callback = Callback::new({
            let opened = Arc::clone(&opened);
            move |open| {
                opened.lock().unwrap().push(open);
            }
        });

        let props = Props::new()
            .id("combo")
            .input_value("ap")
            .default_input_value("default input")
            .value(selection::Set::Single(key(1)))
            .default_value(selection::Set::Single(key(2)))
            .disabled_behavior(DisabledBehavior::FocusOnly)
            .open_on_click(true)
            .loop_focus(false)
            .positioning(positioning.clone())
            .default_highlighted_key(key(3))
            .allows_empty_collection(true)
            .on_open_change(callback)
            .is_ios(true)
            .allow_custom_value(true);

        assert_eq!(props.input_value.as_deref(), Some("ap"));
        assert_eq!(props.default_input_value, "default input");
        assert_eq!(props.value, Some(selection::Set::Single(key(1))));
        assert_eq!(props.default_value, selection::Set::Single(key(2)));
        assert_eq!(props.disabled_behavior, DisabledBehavior::FocusOnly);
        assert!(props.open_on_click);
        assert!(!props.loop_focus);
        assert_eq!(props.positioning, positioning);
        assert_eq!(props.default_highlighted_key, Some(key(3)));
        assert!(props.allows_empty_collection);
        assert!(props.on_open_change.is_some());
        assert!(props.is_ios);
        assert!(props.allow_custom_value);

        let mut service = make_service(props);

        send_event(&mut service, Event::Open);

        assert_eq!(*opened.lock().unwrap(), vec![true]);
        assert_eq!(service.context().input_value.get(), "ap");
        assert_eq!(service.context().highlighted_key, Some(key(3)));
        assert!(service.context().is_ios);
    }

    #[test]
    fn lifecycle_events_focus_blur_close_and_callbacks() {
        let opened = Arc::new(Mutex::new(Vec::new()));

        let callback = Callback::new({
            let opened = Arc::clone(&opened);
            move |open| {
                opened.lock().unwrap().push(open);
            }
        });

        let mut combo = make_service(Props::new().id("combo").on_open_change(callback));

        send_event(&mut combo, Event::Focus { is_keyboard: true });

        assert_eq!(combo.state(), &State::Open);
        assert!(combo.context().focused);
        assert!(combo.context().focus_visible);
        assert_eq!(*opened.lock().unwrap(), vec![true]);

        send_event(&mut combo, Event::Close);

        assert_eq!(combo.state(), &State::Closed);
        assert_eq!(*opened.lock().unwrap(), vec![true, false]);

        send_event(&mut combo, Event::Open);
        send_event(&mut combo, Event::Dismiss);

        assert_eq!(combo.state(), &State::Closed);

        send_event(&mut combo, Event::Open);
        send_event(&mut combo, Event::ClickOutside);

        assert_eq!(combo.state(), &State::Closed);

        send_event(&mut combo, Event::Focus { is_keyboard: false });
        send_event(&mut combo, Event::Blur);

        assert_eq!(combo.state(), &State::Closed);
        assert!(!combo.context().focused);
        assert!(!combo.context().focus_visible);
        assert_eq!(
            *opened.lock().unwrap(),
            vec![true, false, true, false, true, false, true, false]
        );

        let mut no_focus_open = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut no_focus_open, Event::Focus { is_keyboard: true });

        assert_eq!(no_focus_open.state(), &State::Closed);
        assert!(no_focus_open.context().focused);

        let mut click_open = make_service(
            Props::new()
                .id("combo")
                .open_on_focus(false)
                .open_on_click(true),
        );

        send_event(&mut click_open, Event::Focus { is_keyboard: false });

        assert_eq!(click_open.state(), &State::Open);
        assert!(click_open.context().focused);

        send_event(&mut click_open, Event::Close);
        dispatch_api(&mut click_open, |api| {
            api.on_input_click();
        });

        assert_eq!(click_open.state(), &State::Open);

        let mut disabled_focus = make_service(Props::new().id("combo").disabled(true));
        let result = disabled_focus.send(Event::Focus { is_keyboard: true });

        assert!(result.is_noop());
        assert_eq!(disabled_focus.state(), &State::Closed);
        assert!(!disabled_focus.context().open);
    }

    #[test]
    fn prop_sync_disabling_open_combobox_closes_and_notifies() {
        let opened = Arc::new(Mutex::new(Vec::new()));

        let callback = Callback::new({
            let opened = Arc::clone(&opened);
            move |open| {
                opened.lock().unwrap().push(open);
            }
        });

        let mut combo = make_service(
            Props::new()
                .id("combo")
                .on_open_change(callback.clone())
                .open_on_focus(false),
        );

        send_event(&mut combo, Event::Open);

        assert_eq!(combo.state(), &State::Open);
        assert_eq!(*opened.lock().unwrap(), vec![true]);

        combo.set_props(
            Props::new()
                .id("combo")
                .disabled(true)
                .on_open_change(callback),
        );
        send_event(&mut combo, Event::SyncProps);

        assert_eq!(combo.state(), &State::Closed);
        assert!(!combo.context().open);
        assert!(combo.context().disabled);
        assert_eq!(combo.context().highlighted_key, None);
        assert_eq!(*opened.lock().unwrap(), vec![true, false]);
    }

    #[test]
    fn prop_sync_controlled_input_recomputes_open_state() {
        let opened = Arc::new(Mutex::new(Vec::new()));

        let callback = Callback::new({
            let opened = Arc::clone(&opened);
            move |open| {
                opened.lock().unwrap().push(open);
            }
        });

        let mut service = make_service(
            Props::new()
                .id("combo")
                .on_open_change(callback.clone())
                .open_on_focus(false),
        );

        send_event(&mut service, Event::Open);

        assert_eq!(service.state(), &State::Open);

        service.set_props(
            Props::new()
                .id("combo")
                .input_value("zz")
                .on_open_change(callback)
                .open_on_focus(false),
        );
        send_event(&mut service, Event::SyncProps);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert_eq!(service.context().visible_keys, Some(BTreeSet::new()));
        assert_eq!(service.context().highlighted_key, None);
        assert_eq!(*opened.lock().unwrap(), vec![true, false]);
    }

    #[test]
    fn prop_sync_does_not_passively_open_empty_allowed_combobox() {
        let mut service = Service::<Machine>::new(
            Props::new()
                .id("combo")
                .allows_empty_collection(true)
                .open_on_focus(false),
            &Env::default(),
            &Messages::default(),
        );

        send_event(&mut service, Event::UpdateItems(collection()));

        assert_eq!(service.state(), &State::Closed);

        service.set_props(
            Props::new()
                .id("combo")
                .allows_empty_collection(true)
                .open_on_focus(false)
                .invalid(true),
        );
        send_event(&mut service, Event::SyncProps);

        assert_eq!(service.state(), &State::Closed);
        assert!(!service.context().open);
        assert!(service.context().invalid);
    }

    #[test]
    fn highlight_first_last_explicit_and_no_loop_edges() {
        let mut combo = make_service(Props::new().id("combo").loop_focus(false));

        send_event(&mut combo, Event::Open);
        send_event(&mut combo, Event::HighlightLast);

        assert_eq!(combo.context().highlighted_key, Some(key(3)));

        send_event(&mut combo, Event::HighlightNext);

        assert_eq!(combo.context().highlighted_key, Some(key(3)));

        send_event(&mut combo, Event::HighlightFirst);

        assert_eq!(combo.context().highlighted_key, Some(key(1)));

        send_event(&mut combo, Event::HighlightPrev);

        assert_eq!(combo.context().highlighted_key, Some(key(1)));

        send_event(&mut combo, Event::HighlightItem(Some(key(3))));

        assert_eq!(combo.context().highlighted_key, Some(key(3)));

        send_event(&mut combo, Event::HighlightItem(Some(key(99))));

        assert_eq!(combo.context().highlighted_key, None);

        let mut disabled_tail = make_service(
            Props::new()
                .id("combo")
                .disabled_keys(BTreeSet::from([key(3)])),
        );

        send_event(&mut disabled_tail, Event::Open);
        send_event(&mut disabled_tail, Event::HighlightLast);

        assert_eq!(disabled_tail.context().highlighted_key, Some(key(2)));

        let mut looped = make_service(Props::new().id("combo"));

        send_event(&mut looped, Event::Open);
        send_event(&mut looped, Event::HighlightFirst);
        send_event(&mut looped, Event::HighlightPrev);

        assert_eq!(looped.context().highlighted_key, Some(key(3)));
    }

    #[test]
    fn clear_and_deselect_item_update_selection_state() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Multiple(BTreeSet::from([key(1), key(2)]))),
        );

        send_event(&mut service, Event::DeselectItem(key(1)));

        assert!(!service.context().selection.get().contains(&key(1)));
        assert!(service.context().selection.get().contains(&key(2)));

        send_event(&mut service, Event::InputChange("ap".into()));
        send_event(&mut service, Event::Clear);

        assert!(service.context().selection.get().is_empty());
        assert_eq!(service.context().input_value.get(), "");
        assert_eq!(service.context().visible_keys, None);
        assert_eq!(service.context().highlighted_key, None);

        let mut controlled = make_service(
            Props::new()
                .id("combo")
                .name("fruit")
                .value(selection::Set::Single(key(1))),
        );

        send_event(&mut controlled, Event::InputChange("ap".into()));
        send_event(&mut controlled, Event::Clear);

        assert!(controlled.context().selection.get().is_empty());
        assert!(
            controlled
                .context()
                .selection_state
                .selected_keys
                .is_empty()
        );
        with_api(&controlled, |api| {
            assert_eq!(api.hidden_input_attrs().get(&HtmlAttr::Value), Some(""));
        });

        let mut single = make_service(
            Props::new()
                .id("combo")
                .default_value(selection::Set::Single(key(1))),
        );

        assert!(single.send(Event::DeselectItem(key(1))).is_noop());

        let mut absent = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Multiple(BTreeSet::from([key(1)]))),
        );

        assert!(absent.send(Event::DeselectItem(key(2))).is_noop());

        let mut disabled = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .default_value(selection::Set::Multiple(BTreeSet::from([key(2)])))
                .disabled_keys(BTreeSet::from([key(2)])),
        );

        assert!(disabled.send(Event::DeselectItem(key(2))).is_noop());
    }

    #[test]
    fn prop_sync_updates_controlled_values_mode_flags_and_ids() {
        let mut service = make_service(Props::new().id("combo").default_value(selection::Set::All));

        let result = service.set_props(
            Props::new()
                .id("combo-next")
                .input_value("Ap")
                .value(selection::Set::Multiple(BTreeSet::from([key(1), key(2)])))
                .selection_mode(selection::Mode::Single)
                .selection_behavior(selection::Behavior::Replace)
                .disabled_keys(BTreeSet::from([key(2)]))
                .disabled_behavior(DisabledBehavior::FocusOnly)
                .disabled(true)
                .readonly(true)
                .required(true)
                .invalid(true)
                .filter_mode(FilterMode::StartsWith)
                .open_on_focus(false)
                .open_on_click(true)
                .name("fruit")
                .form("checkout")
                .loop_focus(false),
        );

        assert!(result.context_changed);
        assert_eq!(service.context().input_value.get(), "Ap");
        assert_eq!(
            service.context().visible_keys,
            Some(BTreeSet::from([key(1), key(3)]))
        );
        assert_eq!(service.context().highlighted_key, Some(key(1)));
        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Single(key(1))
        );
        with_api(&service, |api| {
            assert_eq!(api.hidden_input_attrs().get(&HtmlAttr::Value), Some("1"));
        });
        assert_eq!(
            service.context().selection_state.mode,
            selection::Mode::Single
        );
        assert_eq!(
            service.context().selection_state.behavior,
            selection::Behavior::Replace
        );
        assert_eq!(
            service.context().selection_state.disabled_behavior,
            DisabledBehavior::FocusOnly
        );
        assert!(service.context().disabled);
        assert!(service.context().readonly);
        assert!(service.context().required);
        assert!(service.context().invalid);
        assert!(!service.context().multiple);
        assert_eq!(service.context().filter_mode, FilterMode::StartsWith);
        assert!(!service.context().open_on_focus);
        assert!(service.context().open_on_click);
        assert_eq!(service.context().name.as_deref(), Some("fruit"));
        assert_eq!(service.context().form.as_deref(), Some("checkout"));
        assert!(!service.context().loop_focus);
        assert_eq!(service.context().ids.id(), "combo-next");

        assert!(
            <Machine as ars_core::Machine>::on_props_changed(service.props(), service.props())
                .is_empty()
        );

        let result = service.set_props(
            Props::new()
                .id("combo-next")
                .selection_mode(selection::Mode::Multiple),
        );

        assert!(result.context_changed);
        assert!(service.context().multiple);
    }

    #[test]
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "method items are not general enough over the API lifetime here"
    )]
    fn api_event_helpers_dispatch_expected_events() {
        let service = make_service(Props::new().id("combo"));

        assert_eq!(
            api_events(&service, |api| api.on_input_change("ap")),
            vec![Event::InputChange("ap".into())]
        );
        assert_eq!(
            api_events(&service, |api| api.on_input_focus(true)),
            vec![Event::Focus { is_keyboard: true }]
        );
        assert_eq!(
            api_events(&service, |api| api.on_input_click()),
            vec![Event::Focus { is_keyboard: false }]
        );
        assert_eq!(
            api_events(&service, |api| api.on_input_blur()),
            vec![Event::Blur]
        );
        assert_eq!(
            api_events(&service, |api| api.on_input_composition_start()),
            vec![Event::CompositionStart]
        );
        assert_eq!(
            api_events(&service, |api| api.on_input_composition_end("ba")),
            vec![Event::CompositionEnd("ba".into())]
        );
        assert_eq!(
            api_events(&service, |api| api.on_clear_click()),
            vec![Event::Clear]
        );
        assert_eq!(
            api_events(&service, |api| api.on_item_click(key(1))),
            vec![Event::SelectItem(key(1))]
        );
        assert_eq!(
            api_events(&service, |api| api.on_item_ctrl_click(key(1))),
            vec![Event::SelectItemCtrl(key(1))]
        );
        assert_eq!(
            api_events(&service, |api| api.on_item_hover(key(1))),
            vec![Event::HighlightItem(Some(key(1)))]
        );
        assert_eq!(
            api_events(&service, |api| api.on_item_leave()),
            vec![Event::HighlightItem(None)]
        );
    }

    #[test]
    fn keyboard_helpers_cover_alt_escape_tab_and_late_composition_paths() {
        let mut closed = make_service(Props::new().id("combo").open_on_focus(false));

        dispatch_api(&mut closed, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::ArrowDown, None));
        });

        assert_eq!(closed.state(), &State::Open);
        assert_eq!(closed.context().highlighted_key, Some(key(1)));

        let alt_down_events = {
            let mut data = keyboard(KeyboardKey::ArrowDown, None);
            data.alt_key = true;
            api_events(&closed, |api| api.on_input_keydown(&data))
        };

        assert_eq!(alt_down_events, vec![Event::Open]);

        let closed_up = make_service(Props::new().id("combo").open_on_focus(false));

        assert!(
            api_events(&closed_up, |api| {
                api.on_input_keydown(&keyboard(KeyboardKey::ArrowUp, None));
            })
            .is_empty()
        );

        let mut closed_highlight = make_service(
            Props::new()
                .id("combo")
                .default_highlighted_key(key(1))
                .open_on_focus(false),
        );

        dispatch_api(&mut closed_highlight, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Enter, None));
        });

        assert_eq!(closed_highlight.state(), &State::Closed);
        assert!(closed_highlight.context().selection.get().is_empty());

        let mut service = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut service, Event::Open);
        send_event(&mut service, Event::InputChange("ap".into()));

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::ArrowDown, None));
        });

        assert_eq!(service.context().highlighted_key, Some(key(3)));

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Home, None));
        });

        assert_eq!(service.context().highlighted_key, Some(key(3)));

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Home, Some('x')));
        });

        assert_eq!(service.context().highlighted_key, Some(key(3)));

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::End, None));
        });

        assert_eq!(service.context().highlighted_key, Some(key(3)));

        send_event(&mut service, Event::HighlightFirst);

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::End, None));
        });

        assert_eq!(service.context().highlighted_key, Some(key(1)));

        send_event(&mut service, Event::HighlightLast);

        let mut alt_home = keyboard(KeyboardKey::Home, None);

        alt_home.alt_key = true;

        dispatch_api(&mut service, |api| api.on_input_keydown(&alt_home));

        assert_eq!(service.context().highlighted_key, Some(key(1)));

        let mut alt_end = keyboard(KeyboardKey::End, None);

        alt_end.alt_key = true;

        dispatch_api(&mut service, |api| api.on_input_keydown(&alt_end));

        assert_eq!(service.context().highlighted_key, Some(key(3)));

        let mut alt_up = keyboard(KeyboardKey::ArrowUp, None);

        alt_up.alt_key = true;

        dispatch_api(&mut service, |api| api.on_input_keydown(&alt_up));

        assert_eq!(service.state(), &State::Closed);

        send_event(&mut service, Event::Open);

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Tab, None));
        });

        assert_eq!(service.state(), &State::Closed);

        send_event(&mut service, Event::InputChange("ap".into()));

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Escape, None));
        });

        assert_eq!(service.state(), &State::Closed);

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Escape, None));
        });

        assert_eq!(service.context().input_value.get(), "");

        send_event(&mut service, Event::InputChange("ap".into()));
        send_event(&mut service, Event::CompositionStart);

        let mut composing_enter = keyboard(KeyboardKey::Enter, None);

        composing_enter.is_composing = true;

        assert!(api_events(&service, |api| api.on_input_keydown(&composing_enter)).is_empty());

        dispatch_api(&mut service, |api| {
            api.on_input_keydown(&keyboard(KeyboardKey::Tab, None));
        });

        assert_eq!(service.state(), &State::Open);

        dispatch_api(&mut service, |api| {
            api.on_input_keydown_after_composition_check(&keyboard(KeyboardKey::Enter, None));
        });

        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Single(key(1))
        );

        let mut data_composing = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut data_composing, Event::InputChange("ap".into()));

        let mut composing_enter = keyboard(KeyboardKey::Enter, None);

        composing_enter.is_composing = true;

        dispatch_api(&mut data_composing, |api| {
            api.on_input_keydown(&composing_enter);
        });

        assert!(data_composing.context().selection.get().is_empty());
    }

    #[test]
    #[expect(
        clippy::redundant_closure_for_method_calls,
        reason = "method items are not general enough over the API lifetime here"
    )]
    fn trigger_helper_toggles_open_state() {
        let mut service = make_service(Props::new().id("combo"));

        dispatch_api(&mut service, |api| api.on_trigger_click());

        assert_eq!(service.state(), &State::Open);

        dispatch_api(&mut service, |api| api.on_trigger_click());

        assert_eq!(service.state(), &State::Closed);
    }

    #[test]
    fn api_accessors_messages_and_connect_dispatch_are_observable() {
        let mut combobox = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .allows_empty_collection(true)
                .default_value(selection::Set::Multiple(BTreeSet::from([key(1)]))),
        );

        send_event(&mut combobox, Event::InputChange("zz".into()));

        with_api(&combobox, |api| {
            assert!(api.is_open());
            assert_eq!(api.items().count(), 3);
            assert_eq!(api.selected_text(), Some("Apple"));
            assert_eq!(api.no_results_text(), "No results found");
            assert_eq!(api.loading_text(), "Loading options...");
            assert_eq!(api.trigger_label(), "Show suggestions");
            assert_eq!(api.clear_label(), "Clear value");
            assert_eq!(api.results_count_text(), "No results found");
            assert!(format!("{api:?}").contains("Api"));

            let parts = [
                Part::Root,
                Part::Label,
                Part::Control,
                Part::Input,
                Part::Trigger,
                Part::ClearTrigger,
                Part::Positioner,
                Part::Content,
                Part::ItemGroup { key: key(1) },
                Part::ItemGroupLabel { key: key(1) },
                Part::Item { key: key(1) },
                Part::ItemText { key: key(1) },
                Part::ItemIndicator { key: key(1) },
                Part::Empty,
                Part::Description,
                Part::ErrorMessage,
                Part::LiveRegion,
            ];

            for part in parts {
                assert_eq!(
                    api.part_attrs(part.clone())
                        .get(&HtmlAttr::Data("ars-part")),
                    part.data_attrs().iter().find_map(|(name, value)| {
                        (*name == HtmlAttr::Data("ars-part")).then_some(*value)
                    })
                );
            }
        });

        let closed_service = make_service(Props::new().id("closed"));

        with_api(&closed_service, |api| {
            assert!(!api.is_open());
        });

        send_event(&mut combobox, Event::InputChange("ap".into()));

        with_api(&combobox, |api| {
            assert_eq!(api.results_count_text(), "2 results available");
            assert!(api.is_open());
        });

        let mut inline_completion = make_service(
            Props::new()
                .id("inline-completion")
                .filter_mode(FilterMode::InlineCompletion)
                .open_on_focus(false),
        );

        send_event(&mut inline_completion, Event::InputChange("ap".into()));

        with_api(&inline_completion, |api| {
            assert_eq!(
                api.results_count_text(),
                "2 results available. Apple highlighted."
            );
        });
    }

    #[test]
    fn item_attrs_ios_highlight_and_disabled_branches() {
        let mut service = make_service(
            Props::new()
                .id("combo")
                .disabled_keys(BTreeSet::from([key(2)]))
                .disabled_behavior(DisabledBehavior::FocusOnly),
        );

        send_event(&mut service, Event::Open);
        send_event(&mut service, Event::HighlightItem(Some(key(2))));

        service.context_mut().is_ios = true;

        with_api(&service, |api| {
            let attrs = api.item_attrs(&key(2));

            assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Selected)), Some("true"));
            assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
            assert_eq!(
                attrs.get_value(&HtmlAttr::Data("ars-disabled")),
                Some(&AttrValue::Bool(true))
            );
        });
    }

    #[test]
    fn selection_helper_edges_are_pinned() {
        let items = collection();

        assert_eq!(
            retain_present_selection(&selection::Set::Single(key(1)), &items, false),
            selection::Set::Single(key(1))
        );
        assert_eq!(
            retain_present_selection(&selection::Set::Single(key(99)), &items, false),
            selection::Set::Empty
        );
        assert_eq!(
            retain_present_selection(&selection::Set::Single(key(99)), &items, true),
            selection::Set::Single(key(99))
        );
        assert_eq!(
            retain_present_selection(&selection::Set::All, &items, false),
            selection::Set::All
        );

        assert_eq!(
            normalize_selection_for_mode(
                selection::Set::Multiple(BTreeSet::from([key(1), key(2)])),
                selection::Mode::Single,
            )
            .len(),
            1
        );
        assert_eq!(
            normalize_selection_for_mode(selection::Set::Single(key(1)), selection::Mode::Multiple),
            selection::Set::Multiple(BTreeSet::from([key(1)]))
        );
        assert_eq!(
            normalize_selection_for_mode(selection::Set::All, selection::Mode::Multiple),
            selection::Set::All
        );
        assert_eq!(
            normalize_selection_for_mode(selection::Set::Single(key(1)), selection::Mode::None),
            selection::Set::Empty
        );
        assert_eq!(serialize_selection(&selection::Set::All), "__ars_all");

        let mut state =
            selection::State::new(selection::Mode::Multiple, selection::Behavior::Toggle);

        state.disabled_keys = BTreeSet::from([key(1)]);
        state.selected_keys = selection::Set::Multiple(BTreeSet::from([key(1), key(2)]));

        remove_disabled_selection_keys(&mut state);

        assert_eq!(
            state.selected_keys,
            selection::Set::Multiple(BTreeSet::from([key(2)]))
        );
    }

    #[test]
    fn disabled_selection_and_ctrl_toggle_edges_are_pinned() {
        let mut combo = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .selection_behavior(selection::Behavior::Toggle)
                .disabled_keys(BTreeSet::from([key(2)])),
        );

        send_event(&mut combo, Event::Open);

        assert!(combo.send(Event::SelectItem(key(2))).is_noop());

        send_event(&mut combo, Event::SelectItem(key(1)));
        send_event(&mut combo, Event::SelectItemCtrl(key(1)));

        assert!(combo.context().selection.get().is_empty());

        combo.context_mut().selection_state.selected_keys = selection::Set::Single(key(2));
        combo
            .context_mut()
            .selection
            .set(selection::Set::Single(key(2)));

        send_event(&mut combo, Event::UpdateItems(collection()));

        assert!(combo.context().selection.get().is_empty());

        let mut single_ctrl = make_service(Props::new().id("combo"));

        send_event(&mut single_ctrl, Event::Open);
        send_event(&mut single_ctrl, Event::SelectItemCtrl(key(1)));

        assert_eq!(
            single_ctrl.context().selection.get(),
            &selection::Set::Single(key(1))
        );
        assert_eq!(single_ctrl.state(), &State::Closed);

        let mut single_selected = make_service(
            Props::new()
                .id("combo")
                .default_value(selection::Set::Single(key(1))),
        );

        send_event(&mut single_selected, Event::Open);
        send_event(&mut single_selected, Event::SelectItem(key(1)));

        assert_eq!(
            single_selected.context().selection.get(),
            &selection::Set::Single(key(1))
        );

        let mut replace_select = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .selection_behavior(selection::Behavior::Replace)
                .default_value(selection::Set::Multiple(BTreeSet::from([key(1)]))),
        );

        send_event(&mut replace_select, Event::Open);
        send_event(&mut replace_select, Event::SelectItem(key(2)));

        assert_eq!(
            replace_select.context().selection.get(),
            &selection::Set::Multiple(BTreeSet::from([key(2)]))
        );

        let mut replace_ctrl = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .selection_behavior(selection::Behavior::Replace)
                .default_value(selection::Set::Multiple(BTreeSet::from([key(1)]))),
        );

        send_event(&mut replace_ctrl, Event::Open);
        send_event(&mut replace_ctrl, Event::SelectItemCtrl(key(2)));

        assert_eq!(
            replace_ctrl.context().selection.get(),
            &selection::Set::Multiple(BTreeSet::from([key(1), key(2)]))
        );

        let mut replace_all_ctrl = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .selection_behavior(selection::Behavior::Replace)
                .default_value(selection::Set::All),
        );

        send_event(&mut replace_all_ctrl, Event::Open);
        send_event(&mut replace_all_ctrl, Event::SelectItemCtrl(key(1)));

        assert_eq!(
            replace_all_ctrl.context().selection.get(),
            &selection::Set::Multiple(BTreeSet::from([key(2), key(3)]))
        );

        let mut toggle_select = make_service(
            Props::new()
                .id("combo")
                .selection_mode(selection::Mode::Multiple)
                .selection_behavior(selection::Behavior::Toggle)
                .default_value(selection::Set::Multiple(BTreeSet::from([key(1)]))),
        );

        send_event(&mut toggle_select, Event::Open);
        send_event(&mut toggle_select, Event::SelectItem(key(1)));

        assert!(toggle_select.context().selection.get().is_empty());
    }

    #[test]
    fn item_click_after_blur_still_selects_item() {
        let mut service = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut service, Event::Open);
        send_event(&mut service, Event::Blur);
        send_event(&mut service, Event::SelectItem(key(1)));

        assert_eq!(
            service.context().selection.get(),
            &selection::Set::Single(key(1))
        );
        assert_eq!(service.context().input_value.get(), "Apple");
        assert_eq!(service.state(), &State::Closed);
    }

    #[test]
    fn update_items_preserves_or_clears_collection_references_precisely() {
        let mut service = make_service(Props::new().id("combo").open_on_focus(false));

        send_event(&mut service, Event::InputChange("ap".into()));
        send_event(&mut service, Event::HighlightItem(Some(key(3))));

        service.context_mut().selection_state.anchor_key = Some(key(3));

        let next_items = StaticCollection::new([
            (
                key(1),
                "Apple".to_string(),
                Item {
                    label: "Apple".into(),
                },
            ),
            (
                key(3),
                "Apricot".to_string(),
                Item {
                    label: "Apricot".into(),
                },
            ),
        ]);

        send_event(&mut service, Event::UpdateItems(next_items));

        assert_eq!(
            service.context().visible_keys,
            Some(BTreeSet::from([key(1), key(3)]))
        );
        assert_eq!(service.context().highlighted_key, Some(key(3)));
        assert_eq!(service.context().selection_state.anchor_key, Some(key(3)));

        service.context_mut().selection_state.anchor_key = Some(key(99));

        send_event(&mut service, Event::UpdateItems(collection()));

        assert_eq!(service.context().selection_state.anchor_key, None);

        service.context_mut().selection_state.anchor_key = Some(key(2));
        service.context_mut().selection_state.disabled_keys = BTreeSet::from([key(2)]);

        send_event(&mut service, Event::UpdateItems(collection()));

        assert_eq!(service.context().selection_state.anchor_key, None);
    }

    trait NoopResult {
        fn is_noop(&self) -> bool;
    }

    impl NoopResult for SendResult<Machine> {
        fn is_noop(&self) -> bool {
            !self.state_changed && !self.context_changed
        }
    }
}
