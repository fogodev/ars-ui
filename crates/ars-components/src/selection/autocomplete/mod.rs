//! Autocomplete selection controller machine.
//!
//! This module implements the framework-agnostic `Autocomplete` controller
//! described in `spec/components/selection/autocomplete.md`. The core owns the
//! input value, filtered suggestion set, highlighted option, loading state, and
//! ARIA/data attributes. Framework adapters remain responsible for live DOM
//! focus, caret management, popup positioning, scrolling, and async fetching.

use alloc::{
    collections::BTreeSet,
    string::{String, ToString as _},
    vec::Vec,
};
use core::{
    fmt::{self, Debug},
    time::Duration,
};

use ars_collections::{Collection, Key, Node, NodeType, StaticCollection};
use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env,
    HtmlAttr, KeyboardKey, Locale, MessageFn, PendingEffect, TransitionPlan,
};
use ars_interactions::KeyboardEventData;

pub use super::combobox::FilterMode;

/// Message function used by Autocomplete single-locale messages.
pub type LocaleMessage = dyn Fn(&Locale) -> String + Send + Sync;

/// Message function used by Autocomplete result-count announcements.
pub type ResultCountMessage = dyn Fn(usize, &Locale) -> String + Send + Sync;

/// User-facing payload for Autocomplete suggestion items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The label shown for the suggestion.
    pub label: String,
}

/// The states of the Autocomplete state machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The input is not focused and no interaction is active.
    #[default]
    Idle,

    /// The input is focused without an active filter change.
    Focused,

    /// The user is typing or navigating filtered suggestions.
    Interacting,

    /// Suggestions are being fetched or refreshed.
    Loading,
}

/// Events accepted by the Autocomplete state machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The input received focus.
    Focus {
        /// Whether focus was initiated by keyboard interaction.
        is_keyboard: bool,
    },

    /// Focus left the autocomplete region.
    Blur,

    /// The input value changed.
    InputChange(String),

    /// A debounce timer expired.
    DebounceExpired,

    /// Cancel the active debounce timer.
    CancelDebounce,

    /// Restart the debounce timer after a debounce prop change.
    RestartDebounce,

    /// Set whether suggestions are loading.
    SetLoading(bool),

    /// Replace the suggestion collection.
    UpdateItems(StaticCollection<Item>),

    /// Highlight a specific item.
    HighlightItem(Option<Key>),

    /// Highlight the first visible item.
    HighlightFirst,

    /// Highlight the last visible item.
    HighlightLast,

    /// Highlight the next visible item.
    HighlightNext,

    /// Highlight the previous visible item.
    HighlightPrev,

    /// Select a suggestion item.
    SelectItem(Key),

    /// Select the currently highlighted suggestion item.
    SelectHighlighted,

    /// Clear the input value and selection.
    Clear,

    /// Synchronize the externally controlled input prop.
    SetInputValue(Option<String>),

    /// Synchronize output-affecting props.
    SyncProps,
}

/// Context held by the Autocomplete state machine.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Full suggestion item collection.
    pub items: StaticCollection<Item>,

    /// Controlled or uncontrolled input value.
    pub input_value: Bindable<String>,

    /// Keys passing the current built-in filter. `None` means no filter is active.
    pub visible_keys: Option<BTreeSet<Key>>,

    /// Currently highlighted visible suggestion key.
    pub highlighted_key: Option<Key>,

    /// Last selected suggestion key.
    pub selected_key: Option<Key>,

    /// Whether the input has focus.
    pub focused: bool,

    /// Whether focus-visible styling should be emitted.
    pub focus_visible: bool,

    /// Whether suggestions are currently loading.
    pub loading: bool,

    /// Whether an adapter debounce timer is currently pending.
    pub debounce_pending: bool,

    /// Whether interaction is disabled.
    pub disabled: bool,

    /// Filter mode for built-in suggestion filtering.
    pub filter_mode: FilterMode,

    /// ID of the owned listbox content element.
    pub collection_id: String,

    /// Stable component ID derivation helper.
    pub ids: ComponentIds,

    /// Resolved locale for i18n.
    pub locale: Locale,

    /// Resolved localized messages.
    pub messages: Messages,
}

/// Props for the Autocomplete state machine.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Suggestion items.
    pub items: StaticCollection<Item>,

    /// Controlled input value.
    pub input_value: Option<String>,

    /// Initial uncontrolled input value.
    pub default_input_value: String,

    /// Built-in filtering mode.
    pub filter_mode: FilterMode,

    /// Optional debounce interval for search-as-you-type.
    pub debounce: Option<Duration>,

    /// Whether suggestions are loading.
    pub loading: bool,

    /// Whether interaction is disabled.
    pub disabled: bool,

    /// ID for the owned listbox content element.
    pub collection_id: String,
}

impl Props {
    /// Returns default Autocomplete props.
    ///
    /// # Examples
    ///
    /// ```
    /// use ars_collections::CollectionBuilder;
    /// use ars_components::selection::autocomplete::{Item, Props};
    ///
    /// let items = CollectionBuilder::new()
    ///     .item("rust", "Rust", Item { label: "Rust".into() })
    ///     .build();
    ///
    /// let props = Props::new().id("language").items(items);
    ///
    /// assert_eq!(props.id, "language");
    /// ```
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

    /// Sets [`Self::items`].
    #[must_use]
    pub fn items(mut self, items: StaticCollection<Item>) -> Self {
        self.items = items;
        self
    }

    /// Sets [`Self::input_value`], switching to controlled mode.
    #[must_use]
    pub fn input_value(mut self, value: impl Into<String>) -> Self {
        self.input_value = Some(value.into());
        self
    }

    /// Clears [`Self::input_value`], switching to uncontrolled mode.
    #[must_use]
    pub fn uncontrolled(mut self) -> Self {
        self.input_value = None;
        self
    }

    /// Sets [`Self::default_input_value`].
    #[must_use]
    pub fn default_input_value(mut self, value: impl Into<String>) -> Self {
        self.default_input_value = value.into();
        self
    }

    /// Sets [`Self::filter_mode`].
    #[must_use]
    pub const fn filter_mode(mut self, value: FilterMode) -> Self {
        self.filter_mode = value;
        self
    }

    /// Sets [`Self::debounce`].
    #[must_use]
    pub const fn debounce(mut self, value: Duration) -> Self {
        self.debounce = Some(value);
        self
    }

    /// Clears [`Self::debounce`].
    #[must_use]
    pub const fn no_debounce(mut self) -> Self {
        self.debounce = None;
        self
    }

    /// Sets [`Self::loading`].
    #[must_use]
    pub const fn loading(mut self, value: bool) -> Self {
        self.loading = value;
        self
    }

    /// Sets [`Self::disabled`].
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`Self::collection_id`].
    #[must_use]
    pub fn collection_id(mut self, value: impl Into<String>) -> Self {
        self.collection_id = value.into();
        self
    }
}

/// Locale-specific labels for the Autocomplete component.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Accessible label for the input.
    pub input_label: MessageFn<LocaleMessage>,

    /// Accessible label for the clear trigger.
    pub clear_label: MessageFn<LocaleMessage>,

    /// Accessible label for the suggestion listbox.
    pub listbox_label: MessageFn<LocaleMessage>,

    /// Loading indicator text.
    pub loading_label: MessageFn<LocaleMessage>,

    /// Empty-state text.
    pub empty_label: MessageFn<LocaleMessage>,

    /// Live-region announcement for visible result count.
    pub results_count: MessageFn<ResultCountMessage>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            input_label: MessageFn::static_str("Search"),
            clear_label: MessageFn::static_str("Clear search"),
            listbox_label: MessageFn::static_str("Suggestions"),
            loading_label: MessageFn::static_str("Loading suggestions"),
            empty_label: MessageFn::static_str("No results found"),
            results_count: MessageFn::new(|count: usize, _locale: &Locale| match count {
                0 => "No results found".to_string(),
                1 => "1 result available".to_string(),
                n => format!("{n} results available"),
            }),
        }
    }
}

impl ComponentMessages for Messages {}

/// Typed identifier for named effect intents emitted by Autocomplete.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter starts or restarts the debounce timer for input changes.
    AutocompleteDebounce,
}

/// Structural parts exposed by the Autocomplete connect API.
#[derive(ComponentPart)]
#[scope = "autocomplete"]
pub enum Part {
    /// Root container.
    Root,

    /// Search input.
    Input,

    /// Clear-input button.
    ClearTrigger,

    /// Owned listbox content.
    Content,

    /// Suggestion item.
    Item {
        /// Item key.
        key: Key,
    },

    /// Suggestion item text.
    ItemText {
        /// Item key.
        key: Key,
    },

    /// Empty-result state.
    EmptyState,

    /// Loading indicator.
    LoadingIndicator,

    /// Live region for suggestion announcements.
    LiveRegion,
}

/// Machine for the Autocomplete component.
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

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (State, Context) {
        let ids = ComponentIds::from_id(&props.id);

        let collection_id = if props.collection_id.is_empty() {
            ids.part("content")
        } else {
            props.collection_id.clone()
        };

        let input_value = props
            .input_value
            .clone()
            .unwrap_or_else(|| props.default_input_value.clone());

        let mut ctx = Context {
            items: props.items.clone(),
            input_value: if let Some(value) = &props.input_value {
                Bindable::controlled(value.clone())
            } else {
                Bindable::uncontrolled(props.default_input_value.clone())
            },
            visible_keys: None,
            highlighted_key: None,
            selected_key: None,
            focused: false,
            focus_visible: false,
            loading: props.loading,
            debounce_pending: false,
            disabled: props.disabled,
            filter_mode: props.filter_mode,
            collection_id,
            ids,
            locale: env.locale.clone(),
            messages: messages.clone(),
        };

        refresh_filter_and_highlight(&mut ctx, &input_value);

        let state = if props.loading {
            State::Loading
        } else {
            State::Idle
        };

        (state, ctx)
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
                    | Event::InputChange(_)
                    | Event::HighlightItem(_)
                    | Event::HighlightFirst
                    | Event::HighlightLast
                    | Event::HighlightNext
                    | Event::HighlightPrev
                    | Event::SelectItem(_)
                    | Event::SelectHighlighted
                    | Event::Clear
            )
        {
            return None;
        }

        match event {
            Event::Focus { is_keyboard } => {
                let is_keyboard = *is_keyboard;

                let target = if ctx.loading {
                    State::Loading
                } else {
                    State::Focused
                };

                Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    ctx.focused = true;
                    ctx.focus_visible = is_keyboard;
                }))
            }

            Event::Blur => {
                let target = if ctx.loading {
                    State::Loading
                } else {
                    State::Idle
                };

                Some(TransitionPlan::to(target).apply(|ctx: &mut Context| {
                    ctx.focused = false;
                    ctx.focus_visible = false;
                    ctx.highlighted_key = None;
                }))
            }

            Event::InputChange(value) => {
                let value = value.clone();
                let schedule_debounce = props.debounce.is_some();

                let plan = TransitionPlan::to(if ctx.loading {
                    State::Loading
                } else {
                    State::Interacting
                })
                .apply(move |ctx: &mut Context| {
                    if !ctx.input_value.is_controlled() {
                        ctx.input_value.set(value.clone());
                    }

                    ctx.selected_key = None;
                    ctx.debounce_pending = schedule_debounce;

                    refresh_filter_and_highlight(ctx, &value);
                })
                .cancel_effect(Effect::AutocompleteDebounce);

                Some(if schedule_debounce {
                    plan.with_effect(PendingEffect::named(Effect::AutocompleteDebounce))
                } else {
                    plan
                })
            }

            Event::DebounceExpired => Some(TransitionPlan::context_only(|ctx: &mut Context| {
                ctx.debounce_pending = false;
            })),

            Event::CancelDebounce => Some(
                TransitionPlan::context_only(|ctx: &mut Context| {
                    ctx.debounce_pending = false;
                })
                .cancel_effect(Effect::AutocompleteDebounce),
            ),

            Event::RestartDebounce => {
                let was_pending = ctx.debounce_pending;
                let has_debounce_prop = props.debounce.is_some();

                let plan = TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.debounce_pending = was_pending && has_debounce_prop;
                })
                .cancel_effect(Effect::AutocompleteDebounce);

                if was_pending && has_debounce_prop {
                    Some(plan.with_effect(PendingEffect::named(Effect::AutocompleteDebounce)))
                } else {
                    Some(plan)
                }
            }

            Event::SetLoading(loading) => {
                let loading = *loading;
                let target = next_state_for_loading(*state, ctx.focused, loading);

                Some(TransitionPlan::to(target).apply(move |ctx: &mut Context| {
                    ctx.loading = loading;
                }))
            }

            Event::UpdateItems(items) => {
                let items = items.clone();
                let input = ctx.input_value.get().clone();

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.items = items.clone();

                    refresh_filter_and_highlight(ctx, &input);

                    if ctx
                        .selected_key
                        .as_ref()
                        .is_some_and(|key| !ctx.items.contains_key(key))
                    {
                        ctx.selected_key = None;
                    }
                }))
            }

            Event::HighlightItem(key) => {
                let key = key.clone().filter(|key| is_visible_focusable_key(ctx, key));

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            Event::HighlightFirst => {
                let key = first_visible_key(ctx);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            Event::HighlightLast => {
                let key = last_visible_key(ctx);

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.highlighted_key = key;
                }))
            }

            Event::HighlightNext => {
                let key = next_visible_key(ctx);

                Some(
                    TransitionPlan::to(if ctx.loading {
                        State::Loading
                    } else {
                        State::Interacting
                    })
                    .apply(move |ctx: &mut Context| {
                        ctx.highlighted_key = key;
                    }),
                )
            }

            Event::HighlightPrev => {
                let key = prev_visible_key(ctx);

                Some(
                    TransitionPlan::to(if ctx.loading {
                        State::Loading
                    } else {
                        State::Interacting
                    })
                    .apply(move |ctx: &mut Context| {
                        ctx.highlighted_key = key;
                    }),
                )
            }

            Event::SelectItem(key) => select_item_plan(ctx, key.clone()),

            Event::SelectHighlighted => ctx
                .highlighted_key
                .clone()
                .and_then(|key| select_item_plan(ctx, key)),

            Event::Clear => Some(
                TransitionPlan::to(if ctx.loading {
                    State::Loading
                } else if ctx.focused {
                    State::Focused
                } else {
                    State::Idle
                })
                .apply(|ctx: &mut Context| {
                    let empty = String::new();

                    ctx.input_value.set(empty.clone());

                    if ctx.input_value.is_controlled() {
                        ctx.input_value.sync_controlled(Some(empty));
                    }

                    ctx.visible_keys = None;
                    ctx.highlighted_key = None;
                    ctx.selected_key = None;
                    ctx.debounce_pending = false;
                })
                .cancel_effect(Effect::AutocompleteDebounce),
            ),

            Event::SetInputValue(value) => {
                let value = value.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    if let Some(value) = &value {
                        let preserve_selection = ctx
                            .selected_key
                            .as_ref()
                            .and_then(|key| ctx.items.get(key))
                            .is_some_and(|node| node.text_value == *value);

                        ctx.input_value.set(value.clone());
                        ctx.input_value.sync_controlled(Some(value.clone()));

                        if !preserve_selection {
                            ctx.selected_key = None;
                        }

                        refresh_filter_and_highlight(ctx, value);
                    } else {
                        ctx.input_value.sync_controlled(None);

                        let input = ctx.input_value.get().clone();

                        refresh_filter_and_highlight(ctx, &input);
                    }
                }))
            }

            Event::SyncProps => {
                let props = props.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = props.disabled;
                    ctx.loading = props.loading;
                    ctx.filter_mode = props.filter_mode;

                    ctx.collection_id = if props.collection_id.is_empty() {
                        ctx.ids.part("content")
                    } else {
                        props.collection_id.clone()
                    };

                    let input = ctx.input_value.get().clone();

                    refresh_filter_and_highlight(ctx, &input);
                }))
            }
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "autocomplete::Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.input_value != new.input_value {
            events.push(Event::SetInputValue(new.input_value.clone()));
        }

        if old.loading != new.loading {
            events.push(Event::SetLoading(new.loading));
        }

        if old.debounce != new.debounce {
            events.push(Event::RestartDebounce);
        }

        if old.items != new.items {
            events.push(Event::UpdateItems(new.items.clone()));
        }

        if props_output_changed(old, new) {
            events.push(Event::SyncProps);
        }

        events
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

/// API for deriving Autocomplete attributes and dispatching events.
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
    /// Attributes for the root container.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Root);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(HtmlAttr::Data("ars-state"), state_name(*self.state));

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.ctx.loading {
            attrs
                .set_bool(HtmlAttr::Data("ars-loading"), true)
                .set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        }

        attrs
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
                if self.popup_visible() {
                    "true"
                } else {
                    "false"
                },
            )
            .set(HtmlAttr::Aria(AriaAttr::HasPopup), "listbox")
            .set(HtmlAttr::Aria(AriaAttr::AutoComplete), "list")
            .set(HtmlAttr::Aria(AriaAttr::Controls), self.collection_id_ref())
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.input_label)(&self.ctx.locale),
            );

        if self.popup_visible()
            && !self.ctx.disabled
            && let Some(key) = valid_highlight(self.ctx)
        {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::ActiveDescendant),
                self.ctx.ids.item("item", key),
            );
        }

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        attrs
    }

    /// Attributes for the clear trigger button.
    #[must_use]
    pub fn clear_trigger_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::ClearTrigger);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("clear-trigger"))
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::TabIndex, "-1")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.clear_label)(&self.ctx.locale),
            );

        if self.ctx.input_value.get().is_empty() || self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Attributes for the owned listbox content.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::Content);

        attrs
            .set(HtmlAttr::Id, self.collection_id_ref())
            .set(HtmlAttr::Role, "listbox")
            .set(
                HtmlAttr::Aria(AriaAttr::Label),
                (self.ctx.messages.listbox_label)(&self.ctx.locale),
            );

        if !self.popup_visible() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        if self.ctx.loading {
            attrs.set(HtmlAttr::Aria(AriaAttr::Busy), "true");
        }

        attrs
    }

    /// Attributes for a suggestion item.
    #[must_use]
    pub fn item_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::Item { key: key.clone() });

        let is_highlighted = self.ctx.highlighted_key.as_ref() == Some(key);
        let is_selected = self.ctx.selected_key.as_ref() == Some(key);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("item", key))
            .set(HtmlAttr::Role, "option")
            .set(
                HtmlAttr::Aria(AriaAttr::Selected),
                if is_selected { "true" } else { "false" },
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

        if let Some(node) = self.ctx.items.get(key) {
            attrs.set(HtmlAttr::Data("ars-value"), node.text_value.as_str());
        }

        attrs
    }

    /// Attributes for suggestion item text.
    #[must_use]
    pub fn item_text_attrs(&self, key: &Key) -> AttrMap {
        let mut attrs = part_attrs(&Part::ItemText { key: key.clone() });

        attrs.set(
            HtmlAttr::Id,
            self.ctx.ids.item_part("item", &key.to_string(), "text"),
        );

        attrs
    }

    /// Attributes for the empty state element.
    #[must_use]
    pub fn empty_state_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::EmptyState);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("empty-state"))
            .set(HtmlAttr::Role, "status")
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

        if !self.empty_visible() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Attributes for the loading indicator element.
    #[must_use]
    pub fn loading_indicator_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::LoadingIndicator);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("loading-indicator"))
            .set(HtmlAttr::Role, "status")
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite");

        if self.ctx.loading {
            attrs.set_bool(HtmlAttr::Data("ars-loading"), true);
        } else {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Attributes for the live region element.
    #[must_use]
    pub fn live_region_attrs(&self) -> AttrMap {
        let mut attrs = part_attrs(&Part::LiveRegion);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("live-region"))
            .set(HtmlAttr::Aria(AriaAttr::Live), "polite")
            .set(HtmlAttr::Aria(AriaAttr::Atomic), "true");

        attrs
    }

    /// Visible suggestion items.
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

    /// Number of currently visible suggestions.
    #[must_use]
    pub fn visible_count(&self) -> usize {
        self.visible_items().count()
    }

    /// Whether the empty state should be shown.
    #[must_use]
    pub fn empty_visible(&self) -> bool {
        !self.ctx.loading && !self.ctx.input_value.get().is_empty() && self.visible_count() == 0
    }

    /// Current input value.
    #[must_use]
    pub fn input_value(&self) -> &str {
        self.ctx.input_value.get()
    }

    /// Selected suggestion key, if any.
    #[must_use]
    pub const fn selected_key(&self) -> Option<&Key> {
        self.ctx.selected_key.as_ref()
    }

    /// Highlighted suggestion key, if any.
    #[must_use]
    pub const fn highlighted_key(&self) -> Option<&Key> {
        self.ctx.highlighted_key.as_ref()
    }

    /// ID of the owned collection content.
    #[must_use]
    pub const fn collection_id(&self) -> &str {
        self.collection_id_ref()
    }

    /// Loading indicator text.
    #[must_use]
    pub fn loading_text(&self) -> String {
        (self.ctx.messages.loading_label)(&self.ctx.locale)
    }

    /// Empty-state text.
    #[must_use]
    pub fn empty_text(&self) -> String {
        (self.ctx.messages.empty_label)(&self.ctx.locale)
    }

    /// Live-region result announcement.
    #[must_use]
    pub fn results_announcement(&self) -> String {
        (self.ctx.messages.results_count)(self.visible_count(), &self.ctx.locale)
    }

    /// Dispatches input changes.
    pub fn on_input_change(&self, value: String) {
        (self.send)(Event::InputChange(value));
    }

    /// Dispatches clear trigger activation.
    pub fn on_clear(&self) {
        (self.send)(Event::Clear);
    }

    /// Dispatches item activation.
    pub fn on_item_select(&self, key: Key) {
        (self.send)(Event::SelectItem(key));
    }

    /// Dispatches item hover.
    pub fn on_item_hover(&self, key: Key) {
        (self.send)(Event::HighlightItem(Some(key)));
    }

    /// Dispatches item leave.
    pub fn on_item_leave(&self) {
        (self.send)(Event::HighlightItem(None));
    }

    /// Handles keydown events on the input.
    pub fn on_input_keydown(&self, data: &KeyboardEventData) {
        if data.is_composing {
            return;
        }

        match data.key {
            KeyboardKey::ArrowDown => (self.send)(Event::HighlightNext),
            KeyboardKey::ArrowUp => (self.send)(Event::HighlightPrev),
            KeyboardKey::Home if self.popup_visible() => (self.send)(Event::HighlightFirst),
            KeyboardKey::End if self.popup_visible() => (self.send)(Event::HighlightLast),
            KeyboardKey::Enter => (self.send)(Event::SelectHighlighted),
            KeyboardKey::Escape if !self.ctx.input_value.get().is_empty() => {
                (self.send)(Event::Clear);
            }
            _ => {}
        }
    }

    const fn popup_visible(&self) -> bool {
        self.ctx.focused || self.ctx.loading || matches!(self.state, State::Interacting)
    }

    const fn collection_id_ref(&self) -> &str {
        if self.ctx.collection_id.is_empty() {
            self.props.collection_id.as_str()
        } else {
            self.ctx.collection_id.as_str()
        }
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Input => self.input_attrs(),
            Part::ClearTrigger => self.clear_trigger_attrs(),
            Part::Content => self.content_attrs(),
            Part::Item { key } => self.item_attrs(&key),
            Part::ItemText { key } => self.item_text_attrs(&key),
            Part::EmptyState => self.empty_state_attrs(),
            Part::LoadingIndicator => self.loading_indicator_attrs(),
            Part::LiveRegion => self.live_region_attrs(),
        }
    }
}

fn props_output_changed(old: &Props, new: &Props) -> bool {
    old.disabled != new.disabled
        || old.loading != new.loading
        || old.filter_mode != new.filter_mode
        || old.collection_id != new.collection_id
}

const fn next_state_for_loading(current: State, focused: bool, loading: bool) -> State {
    if loading {
        State::Loading
    } else if focused {
        State::Focused
    } else if matches!(current, State::Interacting) {
        State::Interacting
    } else {
        State::Idle
    }
}

fn select_item_plan(ctx: &Context, key: Key) -> Option<TransitionPlan<Machine>> {
    if !is_visible_focusable_key(ctx, &key) {
        return None;
    }

    let label = ctx.items.get(&key).map(|node| node.text_value.clone())?;

    Some(
        TransitionPlan::to(if ctx.loading {
            State::Loading
        } else {
            State::Focused
        })
        .apply(move |ctx: &mut Context| {
            ctx.selected_key = Some(key.clone());
            ctx.highlighted_key = Some(key.clone());

            if !ctx.input_value.is_controlled() {
                ctx.input_value.set(label.clone());
                refresh_filter_and_highlight(ctx, &label);
            }

            ctx.debounce_pending = false;
        })
        .cancel_effect(Effect::AutocompleteDebounce),
    )
}

fn refresh_filter_and_highlight(ctx: &mut Context, input: &str) {
    ctx.visible_keys = visible_keys_for(ctx, input);
    ctx.highlighted_key = ctx
        .highlighted_key
        .clone()
        .filter(|key| is_visible_focusable_key(ctx, key));
}

fn visible_keys_for(ctx: &Context, input: &str) -> Option<BTreeSet<Key>> {
    match ctx.filter_mode {
        FilterMode::Custom | FilterMode::None | FilterMode::Inline => None,

        FilterMode::Contains | FilterMode::StartsWith | FilterMode::InlineCompletion => {
            if input.is_empty() {
                return None;
            }

            let needle = input.to_lowercase();

            Some(
                ctx.items
                    .nodes()
                    .filter(|node| node.node_type == NodeType::Item)
                    .filter(|node| {
                        let haystack = node.text_value.to_lowercase();
                        match ctx.filter_mode {
                            FilterMode::StartsWith | FilterMode::InlineCompletion => {
                                haystack.starts_with(&needle)
                            }

                            _ => haystack.contains(&needle),
                        }
                    })
                    .map(|node| node.key.clone())
                    .collect(),
            )
        }
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
        })
        .map(|node| node.key.clone())
        .collect()
}

fn is_visible_focusable_key(ctx: &Context, key: &Key) -> bool {
    ctx.items
        .get(key)
        .is_some_and(|node| node.node_type == NodeType::Item)
        && ctx
            .visible_keys
            .as_ref()
            .is_none_or(|keys| keys.contains(key))
}

fn valid_highlight(ctx: &Context) -> Option<&Key> {
    ctx.highlighted_key
        .as_ref()
        .filter(|key| is_visible_focusable_key(ctx, key))
}

fn first_visible_key(ctx: &Context) -> Option<Key> {
    visible_focusable_keys(ctx).into_iter().next()
}

fn last_visible_key(ctx: &Context) -> Option<Key> {
    visible_focusable_keys(ctx).into_iter().next_back()
}

fn next_visible_key(ctx: &Context) -> Option<Key> {
    let keys = visible_focusable_keys(ctx);

    if keys.is_empty() {
        return None;
    }

    let current = ctx.highlighted_key.as_ref();

    let index = current
        .and_then(|key| keys.iter().position(|candidate| candidate == key))
        .map_or(0, |index| (index + 1) % keys.len());

    keys.get(index).cloned()
}

fn prev_visible_key(ctx: &Context) -> Option<Key> {
    let keys = visible_focusable_keys(ctx);

    if keys.is_empty() {
        return None;
    }

    let current = ctx.highlighted_key.as_ref();

    let index = current
        .and_then(|key| keys.iter().position(|candidate| candidate == key))
        .map_or(keys.len() - 1, |index| {
            if index == 0 {
                keys.len() - 1
            } else {
                index - 1
            }
        });

    keys.get(index).cloned()
}

fn part_attrs(part: &Part) -> AttrMap {
    let mut attrs = AttrMap::new();
    let [(scope_attr, scope_value), (part_attr, part_value)] = part.data_attrs();

    attrs
        .set(scope_attr, scope_value)
        .set(part_attr, part_value);

    attrs
}

const fn state_name(state: State) -> &'static str {
    match state {
        State::Idle => "idle",
        State::Focused => "focused",
        State::Interacting => "interacting",
        State::Loading => "loading",
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, vec::Vec};
    use core::{cell::RefCell, time::Duration};

    use ars_collections::{CollectionBuilder, Key};
    use ars_core::{AriaAttr, Env, HtmlAttr, Service};

    use super::*;

    fn key(value: &str) -> Key {
        Key::str(value)
    }

    fn items() -> StaticCollection<Item> {
        CollectionBuilder::new()
            .item(
                key("alpha"),
                "Alpha",
                Item {
                    label: "Alpha".into(),
                },
            )
            .item(
                key("bravo"),
                "Bravo",
                Item {
                    label: "Bravo".into(),
                },
            )
            .item(
                key("charlie"),
                "Charlie",
                Item {
                    label: "Charlie".into(),
                },
            )
            .build()
    }

    fn props() -> Props {
        Props::new().id("ac").items(items())
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages::default())
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

    fn composing_keyboard(key: KeyboardKey) -> KeyboardEventData {
        KeyboardEventData {
            is_composing: true,
            ..keyboard(key)
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn input_change_filters_suggestions_and_schedules_debounce() {
        let mut autocomplete = service(props().debounce(Duration::from_millis(200)));

        let result = autocomplete.send(Event::InputChange("br".into()));

        assert_eq!(autocomplete.state(), &State::Interacting);
        assert_eq!(autocomplete.context().input_value.get(), "br");
        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("bravo")]))
        );
        assert_eq!(autocomplete.context().highlighted_key, None);
        assert!(autocomplete.context().debounce_pending);
        assert_eq!(result.pending_effects.len(), 1);
        assert_eq!(result.pending_effects[0].name, Effect::AutocompleteDebounce);
    }

    #[test]
    fn input_change_does_not_highlight_until_user_navigation() {
        let mut autocomplete = service(props());

        drop(autocomplete.send(Event::InputChange("br".into())));

        assert_eq!(autocomplete.context().highlighted_key, None);

        drop(autocomplete.send(Event::HighlightNext));

        assert_eq!(autocomplete.context().highlighted_key, Some(key("bravo")));
    }

    #[test]
    fn debounce_expired_clears_pending_without_changing_selection() {
        let mut autocomplete = service(props().debounce(Duration::from_millis(200)));

        drop(autocomplete.send(Event::InputChange("a".into())));

        drop(autocomplete.send(Event::DebounceExpired));

        assert!(!autocomplete.context().debounce_pending);
        assert_eq!(autocomplete.context().selected_key, None);
        assert_eq!(autocomplete.context().input_value.get(), "a");
    }

    #[test]
    fn loading_state_exposes_busy_and_loading_indicator() {
        let mut autocomplete = service(props());

        drop(autocomplete.send(Event::SetLoading(true)));

        let api = autocomplete.connect(&|_| {});

        assert_eq!(autocomplete.state(), &State::Loading);
        assert_eq!(
            api.root_attrs().get(&HtmlAttr::Aria(AriaAttr::Busy)),
            Some("true")
        );
        assert_eq!(
            api.loading_indicator_attrs()
                .get(&HtmlAttr::Data("ars-loading")),
            Some("true")
        );
        assert_eq!(api.loading_text(), "Loading suggestions");
    }

    #[test]
    fn empty_state_is_visible_for_empty_filtered_results() {
        let mut autocomplete = service(props());

        drop(autocomplete.send(Event::InputChange("zzz".into())));

        let api = autocomplete.connect(&|_| {});

        assert!(api.empty_visible());
        assert_eq!(api.empty_state_attrs().get(&HtmlAttr::Hidden), None);
        assert_eq!(api.empty_text(), "No results found");
        assert_eq!(api.results_announcement(), "No results found");
    }

    #[test]
    fn keyboard_navigation_updates_active_descendant() {
        let mut autocomplete = service(props());

        drop(autocomplete.send(Event::Focus { is_keyboard: true }));
        drop(autocomplete.send(Event::HighlightNext));

        let api = autocomplete.connect(&|_| {});

        assert_eq!(autocomplete.context().highlighted_key, Some(key("alpha")));
        assert_eq!(
            api.input_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant)),
            Some("ac-item-alpha")
        );

        drop(autocomplete.send(Event::HighlightNext));

        assert_eq!(autocomplete.context().highlighted_key, Some(key("bravo")));

        drop(autocomplete.send(Event::HighlightLast));

        assert_eq!(autocomplete.context().highlighted_key, Some(key("charlie")));

        drop(autocomplete.send(Event::HighlightFirst));

        assert_eq!(autocomplete.context().highlighted_key, Some(key("alpha")));
    }

    #[test]
    fn selecting_item_populates_uncontrolled_input() {
        let mut autocomplete = service(props());

        drop(autocomplete.send(Event::InputChange("br".into())));
        drop(autocomplete.send(Event::HighlightNext));
        drop(autocomplete.send(Event::SelectHighlighted));

        assert_eq!(autocomplete.context().selected_key, Some(key("bravo")));
        assert_eq!(autocomplete.context().input_value.get(), "Bravo");
    }

    #[test]
    fn uncontrolled_selection_refreshes_visible_keys_for_selected_label() {
        let mut autocomplete = service(props());

        drop(autocomplete.send(Event::InputChange("a".into())));

        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("alpha"), key("bravo"), key("charlie")]))
        );

        drop(autocomplete.send(Event::SelectItem(key("bravo"))));

        assert_eq!(autocomplete.context().input_value.get(), "Bravo");
        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("bravo")]))
        );
        assert_eq!(autocomplete.connect(&|_| {}).visible_count(), 1);
    }

    #[test]
    fn controlled_selection_records_key_without_overwriting_input() {
        let mut autocomplete = service(props().input_value("br"));

        drop(autocomplete.send(Event::InputChange("bra".into())));
        drop(autocomplete.send(Event::SelectItem(key("bravo"))));

        assert_eq!(autocomplete.context().selected_key, Some(key("bravo")));
        assert_eq!(autocomplete.context().input_value.get(), "br");
    }

    #[test]
    fn disabled_autocomplete_ignores_input_navigation_and_selection() {
        let mut autocomplete = service(props().disabled(true));

        drop(autocomplete.send(Event::InputChange("br".into())));
        drop(autocomplete.send(Event::HighlightNext));
        drop(autocomplete.send(Event::SelectItem(key("bravo"))));

        assert_eq!(autocomplete.context().input_value.get(), "");
        assert_eq!(autocomplete.context().highlighted_key, None);
        assert_eq!(autocomplete.context().selected_key, None);
    }

    #[test]
    fn uncontrolled_constructor_preserves_non_input_configuration() {
        let configured = Props::new()
            .id("configured")
            .items(items())
            .input_value("Alpha")
            .default_input_value("fallback")
            .filter_mode(FilterMode::StartsWith)
            .debounce(Duration::from_millis(50))
            .loading(true)
            .disabled(true)
            .collection_id("configured-list")
            .uncontrolled();

        assert_eq!(configured.id, "configured");
        assert_eq!(configured.items, items());
        assert_eq!(configured.input_value, None);
        assert_eq!(configured.default_input_value, "fallback");
        assert_eq!(configured.filter_mode, FilterMode::StartsWith);
        assert_eq!(configured.debounce, Some(Duration::from_millis(50)));
        assert!(configured.loading);
        assert!(configured.disabled);
        assert_eq!(configured.collection_id, "configured-list");
    }

    #[test]
    fn props_sync_updates_each_output_branch_independently() {
        let mut autocomplete = service(props().debounce(Duration::from_millis(100)));

        drop(autocomplete.send(Event::InputChange("a".into())));

        assert!(autocomplete.context().debounce_pending);

        let result = autocomplete.set_props(
            props()
                .input_value("br")
                .filter_mode(FilterMode::StartsWith)
                .debounce(Duration::from_millis(200))
                .loading(true)
                .disabled(true)
                .collection_id("synced-list"),
        );

        assert_eq!(autocomplete.context().input_value.get(), "br");
        assert_eq!(autocomplete.context().filter_mode, FilterMode::StartsWith);
        assert!(autocomplete.context().debounce_pending);
        assert!(autocomplete.context().loading);
        assert!(autocomplete.context().disabled);
        assert_eq!(autocomplete.context().collection_id, "synced-list");
        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("bravo")]))
        );
        assert_eq!(result.pending_effects[0].name, Effect::AutocompleteDebounce);

        drop(
            autocomplete.set_props(
                props()
                    .input_value("a")
                    .filter_mode(FilterMode::StartsWith)
                    .debounce(Duration::from_millis(200))
                    .collection_id("synced-list"),
            ),
        );

        assert_eq!(autocomplete.context().input_value.get(), "a");
        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("alpha")]))
        );
        assert!(!autocomplete.context().loading);
        assert!(!autocomplete.context().disabled);

        drop(autocomplete.set_props(props().input_value("a").collection_id("alternate-list")));

        assert_eq!(autocomplete.context().collection_id, "alternate-list");
        assert_eq!(autocomplete.context().filter_mode, FilterMode::Contains);
        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("alpha"), key("bravo"), key("charlie")]))
        );

        let unchanged =
            autocomplete.set_props(props().input_value("a").collection_id("alternate-list"));

        assert!(!unchanged.context_changed);
        assert!(!props_output_changed(&props(), &props()));
        assert!(props_output_changed(&props(), &props().disabled(true)));
        assert!(props_output_changed(&props(), &props().loading(true)));
        assert!(props_output_changed(
            &props(),
            &props().filter_mode(FilterMode::StartsWith)
        ));
        assert!(props_output_changed(
            &props(),
            &props().collection_id("changed")
        ));
    }

    #[test]
    fn loading_prop_sync_transitions_state_with_context() {
        let mut autocomplete = service(props());

        drop(autocomplete.set_props(props().loading(true)));

        assert_eq!(autocomplete.state(), &State::Loading);
        assert!(autocomplete.context().loading);

        drop(autocomplete.set_props(props()));

        assert_eq!(autocomplete.state(), &State::Idle);
        assert!(!autocomplete.context().loading);
    }

    #[test]
    fn controlled_input_sync_clears_stale_selected_key() {
        let mut autocomplete = service(props().input_value("br"));

        drop(autocomplete.send(Event::SelectItem(key("bravo"))));

        assert_eq!(autocomplete.context().selected_key, Some(key("bravo")));

        drop(autocomplete.set_props(props().input_value("ch")));

        assert_eq!(autocomplete.context().input_value.get(), "ch");
        assert_eq!(autocomplete.context().selected_key, None);
        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("charlie")]))
        );
    }

    #[test]
    fn controlled_input_sync_preserves_matching_selected_key() {
        let mut autocomplete = service(props().input_value("br"));

        drop(autocomplete.send(Event::SelectItem(key("bravo"))));

        assert_eq!(autocomplete.context().selected_key, Some(key("bravo")));

        drop(autocomplete.set_props(props().input_value("Bravo")));

        assert_eq!(autocomplete.context().input_value.get(), "Bravo");
        assert_eq!(autocomplete.context().selected_key, Some(key("bravo")));
        assert_eq!(
            autocomplete
                .connect(&|_| {})
                .item_attrs(&key("bravo"))
                .get(&HtmlAttr::Aria(AriaAttr::Selected)),
            Some("true")
        );
    }

    #[test]
    fn debounce_prop_sync_cancels_pending_when_debounce_is_removed() {
        let mut autocomplete = service(props().debounce(Duration::from_millis(100)));

        drop(autocomplete.send(Event::InputChange("a".into())));

        assert!(autocomplete.context().debounce_pending);

        let result = autocomplete.set_props(props());

        assert!(!autocomplete.context().debounce_pending);
        assert!(result.pending_effects.is_empty());
        assert_eq!(result.cancel_effects, vec![Effect::AutocompleteDebounce]);
    }

    #[test]
    fn clear_updates_controlled_input_value() {
        let mut autocomplete = service(props().input_value("Alpha"));

        drop(autocomplete.send(Event::SelectItem(key("alpha"))));

        drop(autocomplete.send(Event::Clear));

        assert_eq!(autocomplete.context().input_value.get(), "");
        assert_eq!(autocomplete.context().selected_key, None);
        assert_eq!(autocomplete.context().highlighted_key, None);
        assert_eq!(autocomplete.context().visible_keys, None);

        drop(autocomplete.set_props(props().uncontrolled()));

        assert_eq!(autocomplete.context().input_value.get(), "");
    }

    #[test]
    fn clear_keeps_idle_state_when_input_is_unfocused() {
        let mut autocomplete = service(props().default_input_value("Alpha"));

        drop(autocomplete.send(Event::Clear));

        assert_eq!(autocomplete.state(), &State::Idle);
        assert!(!autocomplete.context().focused);
        assert_eq!(autocomplete.context().input_value.get(), "");
    }

    #[test]
    fn controlled_clear_clears_visible_keys_with_input_value() {
        let mut autocomplete = service(props().input_value("br"));

        drop(autocomplete.send(Event::SelectItem(key("bravo"))));

        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("bravo")]))
        );

        drop(autocomplete.send(Event::Clear));

        assert_eq!(autocomplete.context().input_value.get(), "");
        assert_eq!(autocomplete.context().selected_key, None);
        assert_eq!(autocomplete.context().visible_keys, None);
    }

    #[test]
    fn item_prop_sync_replaces_suggestion_collection() {
        let mut autocomplete = service(props());

        drop(autocomplete.send(Event::InputChange("br".into())));

        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("bravo")]))
        );

        let replacement = CollectionBuilder::new()
            .item(
                key("brook"),
                "Brook",
                Item {
                    label: "Brook".into(),
                },
            )
            .item(
                key("delta"),
                "Delta",
                Item {
                    label: "Delta".into(),
                },
            )
            .build();

        drop(autocomplete.set_props(props().items(replacement)));

        assert!(autocomplete.context().items.contains_key(&key("brook")));
        assert!(!autocomplete.context().items.contains_key(&key("bravo")));
        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("brook")]))
        );
    }

    #[test]
    fn visible_items_and_public_getters_reflect_current_context() {
        let mut autocomplete = service(props().collection_id("visible-list"));

        drop(autocomplete.send(Event::InputChange("a".into())));
        drop(autocomplete.send(Event::SelectItem(key("alpha"))));

        let api = autocomplete.connect(&|_| {});

        let visible: Vec<_> = api
            .visible_items()
            .map(|node| (node.key.clone(), node.text_value.clone()))
            .collect();

        assert_eq!(visible, vec![(key("alpha"), "Alpha".to_string())]);
        assert_eq!(api.visible_count(), 1);
        assert_eq!(api.results_announcement(), "1 result available");
        assert!(!api.empty_visible());
        assert_eq!(api.input_value(), "Alpha");
        assert_eq!(api.selected_key(), Some(&key("alpha")));
        assert_eq!(api.highlighted_key(), Some(&key("alpha")));
        assert_eq!(api.collection_id(), "visible-list");
    }

    #[test]
    fn public_event_handlers_dispatch_expected_events() {
        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let mut autocomplete = service(props().default_input_value("br"));

        drop(autocomplete.send(Event::InputChange("br".into())));

        let api = autocomplete.connect(&send);

        api.on_input_change("char".to_string());
        api.on_clear();
        api.on_item_select(key("alpha"));
        api.on_item_hover(key("bravo"));
        api.on_item_leave();
        api.on_input_keydown(&keyboard(KeyboardKey::ArrowDown));
        api.on_input_keydown(&keyboard(KeyboardKey::ArrowUp));
        api.on_input_keydown(&keyboard(KeyboardKey::Enter));
        api.on_input_keydown(&keyboard(KeyboardKey::Escape));

        assert_eq!(
            events.into_inner(),
            vec![
                Event::InputChange("char".to_string()),
                Event::Clear,
                Event::SelectItem(key("alpha")),
                Event::HighlightItem(Some(key("bravo"))),
                Event::HighlightItem(None),
                Event::HighlightNext,
                Event::HighlightPrev,
                Event::SelectHighlighted,
                Event::Clear,
            ]
        );
    }

    #[test]
    fn input_keydown_preserves_native_home_end_and_composing_enter() {
        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);
        let autocomplete = service(props().default_input_value("br"));

        let api = autocomplete.connect(&send);

        api.on_input_keydown(&keyboard(KeyboardKey::Home));
        api.on_input_keydown(&keyboard(KeyboardKey::End));
        api.on_input_keydown(&composing_keyboard(KeyboardKey::Enter));

        assert!(events.into_inner().is_empty());
    }

    #[test]
    fn input_keydown_ignores_highlight_navigation_during_composition() {
        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);
        let mut autocomplete = service(props().default_input_value("br"));

        drop(autocomplete.send(Event::Focus { is_keyboard: true }));

        let api = autocomplete.connect(&send);

        api.on_input_keydown(&composing_keyboard(KeyboardKey::ArrowDown));
        api.on_input_keydown(&composing_keyboard(KeyboardKey::ArrowUp));
        api.on_input_keydown(&composing_keyboard(KeyboardKey::Home));
        api.on_input_keydown(&composing_keyboard(KeyboardKey::End));

        assert!(events.into_inner().is_empty());
    }

    #[test]
    fn input_keydown_handles_home_end_when_popup_is_active() {
        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);
        let mut autocomplete = service(props().default_input_value("br"));

        drop(autocomplete.send(Event::Focus { is_keyboard: true }));

        let api = autocomplete.connect(&send);

        api.on_input_keydown(&keyboard(KeyboardKey::Home));
        api.on_input_keydown(&keyboard(KeyboardKey::End));

        assert_eq!(
            events.into_inner(),
            vec![Event::HighlightFirst, Event::HighlightLast]
        );
    }

    #[test]
    fn escape_key_ignores_empty_input_and_clears_non_empty_input() {
        let events = RefCell::new(Vec::new());
        let send = |event| events.borrow_mut().push(event);

        let autocomplete = service(props());

        autocomplete
            .connect(&send)
            .on_input_keydown(&keyboard(KeyboardKey::Escape));

        assert!(events.borrow().is_empty());

        let autocomplete = service(props().default_input_value("a"));

        autocomplete
            .connect(&send)
            .on_input_keydown(&keyboard(KeyboardKey::Escape));

        assert_eq!(events.into_inner(), vec![Event::Clear]);
    }

    #[test]
    fn starts_with_filter_and_previous_navigation_wrap_visible_keys() {
        let mut autocomplete = service(props().filter_mode(FilterMode::StartsWith));

        drop(autocomplete.send(Event::InputChange("br".into())));

        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("bravo")]))
        );
        assert_eq!(autocomplete.context().highlighted_key, None);

        drop(autocomplete.send(Event::InputChange("a".into())));

        assert_eq!(
            autocomplete.context().visible_keys,
            Some(BTreeSet::from([key("alpha")]))
        );
        assert_eq!(autocomplete.context().highlighted_key, None);

        drop(autocomplete.send(Event::InputChange(String::new())));
        drop(autocomplete.send(Event::HighlightFirst));

        assert_eq!(autocomplete.context().highlighted_key, Some(key("alpha")));

        drop(autocomplete.send(Event::HighlightPrev));

        assert_eq!(autocomplete.context().highlighted_key, Some(key("charlie")));

        drop(autocomplete.send(Event::HighlightPrev));

        assert_eq!(autocomplete.context().highlighted_key, Some(key("bravo")));

        drop(autocomplete.send(Event::HighlightItem(None)));
        drop(autocomplete.send(Event::HighlightPrev));

        assert_eq!(autocomplete.context().highlighted_key, Some(key("charlie")));
    }

    #[test]
    fn active_descendant_is_gated_to_active_popup_navigation() {
        let mut autocomplete = service(props());

        drop(autocomplete.send(Event::HighlightFirst));

        let idle_api = autocomplete.connect(&|_| {});

        assert_eq!(
            idle_api
                .input_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant)),
            None
        );

        drop(autocomplete.send(Event::Focus { is_keyboard: true }));

        let focused_api = autocomplete.connect(&|_| {});

        assert_eq!(
            focused_api
                .input_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::ActiveDescendant)),
            Some("ac-item-alpha")
        );
    }

    #[test]
    fn connect_api_emits_combobox_and_listbox_contract() {
        let mut autocomplete = service(props().collection_id("suggestions"));

        drop(autocomplete.send(Event::Focus { is_keyboard: true }));

        let api = autocomplete.connect(&|_| {});

        assert_eq!(api.input_attrs().get(&HtmlAttr::Role), Some("combobox"));
        assert_eq!(
            api.input_attrs()
                .get(&HtmlAttr::Aria(AriaAttr::AutoComplete)),
            Some("list")
        );
        assert_eq!(
            api.input_attrs().get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );
        assert_eq!(
            api.input_attrs().get(&HtmlAttr::Aria(AriaAttr::HasPopup)),
            Some("listbox")
        );
        assert_eq!(
            api.input_attrs().get(&HtmlAttr::Aria(AriaAttr::Controls)),
            Some("suggestions")
        );
        assert_eq!(api.content_attrs().get(&HtmlAttr::Role), Some("listbox"));
        assert_eq!(
            api.item_attrs(&key("alpha")).get(&HtmlAttr::Role),
            Some("option")
        );
        assert_eq!(
            ConnectApi::part_attrs(&api, Part::Input).get(&HtmlAttr::Role),
            Some("combobox")
        );
        assert_eq!(api.results_announcement(), "3 results available");
    }

    #[test]
    fn popup_visibility_covers_interacting_and_loading_states() {
        let mut autocomplete = service(props());

        drop(autocomplete.send(Event::InputChange("br".into())));

        let api = autocomplete.connect(&|_| {});

        assert_eq!(
            api.input_attrs().get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );

        let mut autocomplete = service(props().loading(true));

        let api = autocomplete.connect(&|_| {});

        assert_eq!(
            api.input_attrs().get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );

        drop(autocomplete.send(Event::SetLoading(false)));

        let api = autocomplete.connect(&|_| {});

        assert_eq!(
            api.input_attrs().get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("false")
        );
    }

    #[test]
    fn autocomplete_attrs_snapshot_parts_and_state_branches() {
        let mut autocomplete = service(props().debounce(Duration::from_millis(200)));

        drop(autocomplete.send(Event::Focus { is_keyboard: true }));
        drop(autocomplete.send(Event::InputChange("br".into())));
        drop(autocomplete.send(Event::HighlightNext));

        let api = autocomplete.connect(&|_| {});

        insta::assert_snapshot!(
            "autocomplete_root_interacting",
            snapshot_attrs(&api.root_attrs())
        );
        insta::assert_snapshot!(
            "autocomplete_input_highlighted",
            snapshot_attrs(&api.input_attrs())
        );
        insta::assert_snapshot!(
            "autocomplete_clear_trigger",
            snapshot_attrs(&api.clear_trigger_attrs())
        );
        insta::assert_snapshot!("autocomplete_content", snapshot_attrs(&api.content_attrs()));
        insta::assert_snapshot!(
            "autocomplete_item_highlighted",
            snapshot_attrs(&api.item_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "autocomplete_item_text",
            snapshot_attrs(&api.item_text_attrs(&key("bravo")))
        );
        insta::assert_snapshot!(
            "autocomplete_live_region",
            snapshot_attrs(&api.live_region_attrs())
        );

        drop(autocomplete.send(Event::InputChange("zzz".into())));

        let empty_api = autocomplete.connect(&|_| {});

        insta::assert_snapshot!(
            "autocomplete_empty_state",
            snapshot_attrs(&empty_api.empty_state_attrs())
        );

        drop(autocomplete.send(Event::SetLoading(true)));

        let loading_api = autocomplete.connect(&|_| {});

        insta::assert_snapshot!(
            "autocomplete_loading_indicator",
            snapshot_attrs(&loading_api.loading_indicator_attrs())
        );
        insta::assert_snapshot!(
            "autocomplete_root_loading",
            snapshot_attrs(&loading_api.root_attrs())
        );

        let disabled = service(props().disabled(true).default_input_value("Alpha"));

        let disabled_api = disabled.connect(&|_| {});

        insta::assert_snapshot!(
            "autocomplete_input_disabled",
            snapshot_attrs(&disabled_api.input_attrs())
        );
        insta::assert_snapshot!(
            "autocomplete_clear_trigger_disabled",
            snapshot_attrs(&disabled_api.clear_trigger_attrs())
        );
    }
}
