//! Leptos Tabs adapter.
//!
//! Renders the framework-agnostic [`ars_components::navigation::tabs`]
//! machine as a single compound `<Tabs>` Leptos component. The adapter owns
//! the full anatomy (Root, List, Tab×N, Indicator, Panel×N, optional
//! CloseTrigger, optional reorder live region), drives DOM focus on the
//! [`Effect::FocusFocusedTab`] intent emitted by the core, and surfaces tab
//! data through a per-row [`Tab`] value.
//!
//! See `spec/leptos-components/navigation/tabs.md` for the full adapter
//! contract.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Debug},
    sync::Arc,
};

use ars_collections::{Key, TabKey};
use ars_components::navigation::tabs::{
    self, TabMeta, drag_reorder_plan, registrations_from_meta, typed_key_for_key,
};
// Re-export the agnostic core surface so consumers reach it through the
// adapter namespace. `tabs::Effect` is intentionally NOT re-exported because
// it would shadow `leptos::prelude::Effect`; consumers who need the named
// effect type can reach it through `ars_components::navigation::tabs::Effect`.
pub use ars_components::navigation::tabs::{
    ActivationMode, CloseTabLabelFn, Context, Event, Messages, Part, Props, ReorderAnnounceLabelFn,
    ReorderEvent, State,
};
use ars_core::{
    AriaAttr, AttrMap, AttrValue, Direction, HtmlAttr, KeyModifiers, ModalityContext, Orientation,
    PlatformEffects, PointerType, SafeUrl,
};
use ars_i18n::Translate;
use ars_interactions::{KeyboardEventData, KeyboardKey};
#[cfg(all(not(feature = "ssr"), target_arch = "wasm32"))]
use leptos::wasm_bindgen::{self, JsCast};
use leptos::{
    children::{Children, ViewFn},
    either::Either,
    html,
    prelude::*,
    reactive::owner::LocalStorage,
    web_sys,
};
use reactive_stores::Subfield;
pub use reactive_stores::{Field, Store};

use crate::{
    attrs::attr_map_to_leptos_inline_attrs,
    event_mapping::leptos_key_to_keyboard_key,
    id::use_id,
    provider::{current_ars_context, t, use_locale, use_modality_context, use_platform_effects},
    use_machine::use_machine_with_reactive_props,
};

// ────────────────────────────────────────────────────────────────────
// Tab
// ────────────────────────────────────────────────────────────────────

/// Semantic text source for a Leptos [`Tab`] trigger.
///
/// The adapter uses this label for accessible close-button names and
/// reorder announcements. The default trigger rendering also uses this
/// label unless [`Tab::trigger`] supplies richer visual content.
#[derive(Clone)]
pub enum TabLabel {
    /// Non-reactive label text, optimized for static string literals.
    Static(Oco<'static, str>),

    /// Dynamic label text resolved from provider state or other runtime inputs.
    Dynamic(TextProp),
}

impl TabLabel {
    /// Builds a non-reactive static label.
    #[must_use]
    pub fn static_text(text: impl Into<Oco<'static, str>>) -> Self {
        Self::Static(text.into())
    }

    /// Builds a provider-backed translated label.
    #[must_use]
    pub fn translated<T>(message: T) -> Self
    where
        T: Translate + Send + Sync + 'static,
    {
        Self::Dynamic(TextProp::from(t(message)))
    }

    /// Builds a dynamic label from a Leptos string signal.
    #[must_use]
    pub fn dynamic(text: impl Into<TextProp>) -> Self {
        Self::Dynamic(text.into())
    }

    /// Resolves the label text for the current provider locale.
    #[must_use]
    pub fn resolve(&self) -> String {
        match self {
            Self::Static(text) => text.to_string(),
            Self::Dynamic(text) => text.get().into_owned(),
        }
    }

    /// Resolves the label text for diagnostics without evaluating dynamic text.
    #[must_use]
    pub fn debug_label(&self) -> String {
        match self {
            Self::Static(text) => text.to_string(),
            Self::Dynamic(_) => String::from("<dynamic>"),
        }
    }
}

impl Debug for TabLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TabLabel")
            .field(&self.debug_label())
            .finish()
    }
}

/// Per-tab render data consumed by the [`Tabs`] component.
///
/// The agnostic core does not own per-tab labels (see
/// `spec/components/navigation/tabs.md` §5.1) — it tracks only registration
/// keys and the closable flag. Adapters introduce a `Tab` value to
/// carry the rendered button/panel content, plus optional consumer
/// affordances (per-row `disabled` flag, optional `link` for tabs that
/// render as `<a>` instead of `<button>`).
#[derive(Clone)]
pub struct Tab<K: TabKey> {
    /// Stable identifier for the tab. Used for ARIA wiring, registration,
    /// and DOM-id derivation through [`ars_core::ComponentIds`].
    pub key: K,

    /// Visible label content rendered inside the tab trigger.
    pub label: ViewFn,

    /// Semantic label text source. Used for the close-button accessible
    /// name (`Messages::close_tab_label`) and the reorder announcement
    /// (`Messages::reorder_announce_label`).
    pub label_text: TabLabel,

    /// Panel body rendered inside the matching tabpanel.
    pub panel: ViewFn,

    /// When `true` this row is disabled. Merged with the `Tabs` component's
    /// `disabled_keys` prop before being threaded to the core machine.
    pub disabled: bool,

    /// When `true` the adapter renders a close button inside the tab
    /// and the core forwards `Delete` / `Backspace` keystrokes as
    /// [`tabs::Event::CloseTab`].
    pub closable: bool,

    /// When `Some`, the tab renders as `<a href=…>` instead of the
    /// default `<button>`.
    pub link: Option<SafeUrl>,
}

impl<K: TabKey> Tab<K> {
    /// Builds a non-closable, non-disabled tab row with a static text label.
    #[must_use]
    pub fn new_static(
        key: K,
        label_text: impl Into<Oco<'static, str>>,
        panel: impl Into<ViewFn>,
    ) -> Self {
        let label_text = TabLabel::static_text(label_text);

        Self {
            key,
            label: ViewFn::from({
                let label_text = label_text.clone();
                move || label_text.resolve()
            }),
            label_text,
            panel: panel.into(),
            disabled: false,
            closable: false,
            link: None,
        }
    }

    /// Builds a non-closable, non-disabled tab row with explicit semantic
    /// label text and custom trigger content.
    #[must_use]
    pub fn new_with_label(
        key: K,
        label_text: impl Into<Oco<'static, str>>,
        label: impl Into<ViewFn>,
        panel: impl Into<ViewFn>,
    ) -> Self {
        Self {
            key,
            label: label.into(),
            label_text: TabLabel::static_text(label_text),
            panel: panel.into(),
            disabled: false,
            closable: false,
            link: None,
        }
    }

    /// Replaces the visible trigger content while preserving this tab's
    /// semantic label for accessibility and announcements.
    #[must_use]
    pub fn trigger(mut self, label: impl Into<ViewFn>) -> Self {
        self.label = label.into();

        self
    }

    /// Marks this tab as disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;

        self
    }

    /// Marks this tab as closable.
    #[must_use]
    pub const fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;

        self
    }

    /// Renders the tab as a link with the given href.
    #[must_use]
    pub fn link(mut self, href: SafeUrl) -> Self {
        self.link = Some(href);

        self
    }
}

impl<K> Tab<K>
where
    K: TabKey + Translate + Send + Sync + 'static,
{
    /// Builds a non-closable, non-disabled tab row whose semantic label
    /// and default visible trigger are translated from the key itself.
    #[must_use]
    pub fn new(key: K, panel: impl Into<ViewFn>) -> Self {
        let label_text = TabLabel::translated(key);
        let label_for_trigger = label_text.clone();

        Self {
            key,
            label: ViewFn::from(move || label_for_trigger.resolve()),
            label_text,
            panel: panel.into(),
            disabled: false,
            closable: false,
            link: None,
        }
    }
}

impl<K: TabKey> Debug for Tab<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tab")
            .field("key", &self.key.into_key())
            .field("label_text", &self.label_text.debug_label())
            .field("disabled", &self.disabled)
            .field("closable", &self.closable)
            .field("link", &self.link)
            .finish_non_exhaustive()
    }
}

// ────────────────────────────────────────────────────────────────────
// Tabs component
// ────────────────────────────────────────────────────────────────────

/// Configuration captured at component init for use inside event handlers.
///
/// Stored in `StoredValue` so the per-handler closures can read it without
/// cloning a fresh copy each render.
#[derive(Clone, Copy)]
struct TabsConfig<K: TabKey> {
    orientation: Orientation,
    dir: Direction,
    activation_mode: ActivationMode,
    reorderable: bool,
    /// Live per-row metadata. Read with `.get_untracked()` from inside
    /// event handlers — the closure has its own DOM-event scope and
    /// doesn't need a reactive subscription on the meta itself; the
    /// memo reflects the current store state regardless.
    tabs_meta: Memo<Vec<TabMeta<K>>>,
    messages_revision: RwSignal<u64>,
}

struct TabsOptions<K: TabKey> {
    orientation: Signal<Orientation>,
    activation_mode: Signal<ActivationMode>,
    dir: Signal<Direction>,
    loop_focus: Signal<bool>,
    disallow_empty_selection: Signal<bool>,
    lazy_mount: Signal<bool>,
    unmount_on_exit: Signal<bool>,
    disabled_keys: Signal<BTreeSet<K>>,
    reorderable: Signal<bool>,
}

struct TabsReactiveSetup<K: TabKey> {
    tabs_field: Field<Vec<Tab<K>>>,
    owned_tabs_field: Option<Field<Vec<Tab<K>>>>,
    registrations: Memo<Vec<tabs::TabRegistration>>,
    config: StoredValue<TabsConfig<K>>,
    props_signal: Signal<Props>,
}

type TabNodeRegistry = StoredValue<BTreeMap<Key, web_sys::HtmlElement>, LocalStorage>;

/// Source of tab rows for the Leptos [`Tabs`] component.
///
/// Consumers can pass inline rows (`Vec<Tab<K>>` or `[Tab<K>; N]`) for
/// adapter-owned tabs, or pass a `reactive_stores` field when they own a
/// dynamic tab list. Owned tabs still use an internal store, so close and
/// reorder interactions work without any consumer store setup.
pub enum TabsSource<K: TabKey> {
    /// Adapter-owned tabs initialized from inline rows.
    Owned(Vec<Tab<K>>),

    /// Consumer-owned reactive tab field.
    Field(Field<Vec<Tab<K>>>),
}

impl<K: TabKey> TabsSource<K> {
    fn into_field(self) -> TabsFieldPair<K> {
        match self {
            Self::Owned(tabs) => {
                let field = Field::from(Store::new(tabs));

                (field, Some(field))
            }

            Self::Field(field) => (field, None),
        }
    }
}

type TabsFieldPair<K> = (Field<Vec<Tab<K>>>, Option<Field<Vec<Tab<K>>>>);

impl<K: TabKey> From<Vec<Tab<K>>> for TabsSource<K> {
    fn from(value: Vec<Tab<K>>) -> Self {
        Self::Owned(value)
    }
}

impl<K: TabKey, const N: usize> From<[Tab<K>; N]> for TabsSource<K> {
    fn from(value: [Tab<K>; N]) -> Self {
        Self::Owned(Vec::from(value))
    }
}

impl<K: TabKey> From<Field<Vec<Tab<K>>>> for TabsSource<K> {
    fn from(value: Field<Vec<Tab<K>>>) -> Self {
        Self::Field(value)
    }
}

impl<K: TabKey, State> From<Subfield<Store<State>, State, Vec<Tab<K>>>> for TabsSource<K>
where
    Field<Vec<Tab<K>>>: From<Subfield<Store<State>, State, Vec<Tab<K>>>>,
{
    fn from(value: Subfield<Store<State>, State, Vec<Tab<K>>>) -> Self {
        Self::Field(Field::from(value))
    }
}

impl<K: TabKey> Debug for TabsSource<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owned(tabs) => f.debug_tuple("TabsSource::Owned").field(tabs).finish(),
            Self::Field(_) => f.debug_tuple("TabsSource::Field").finish_non_exhaustive(),
        }
    }
}

/// Renders the agnostic Tabs machine as a Leptos compound component.
///
/// The single component owns the full anatomy (Root, List, Tab×N,
/// Indicator, Panel×N, optional CloseTrigger, optional reorder live
/// region). Per-tab content comes through the [`Tab`] rows in `tabs`;
/// `children` is rendered inside the root after the panels for any
/// adapter-side decoration consumers want next to the tablist.
///
/// `default_value` is the initial selection; `value` switches the
/// component into controlled mode (the consumer Signal becomes the
/// authoritative source of truth and changes flow through the core via
/// [`ars_core::Bindable::sync_controlled`]).
#[component]
#[expect(
    clippy::too_many_arguments,
    reason = "spec-defined component signature with one argument per documented prop"
)]
pub fn Tabs<K: TabKey>(
    /// Controlled selected tab. When `Some`, the consumer's
    /// `Signal<Option<K>>` is the authoritative source for selection.
    /// The inner `Option` lets controlled consumers express
    /// "no tab selected" without flipping out of controlled mode.
    #[prop(optional, into)]
    value: Option<Signal<Option<K>>>,

    /// Initial selected tab key in uncontrolled mode.
    #[prop(into)]
    default_value: K,

    /// Per-tab render rows in DOM order. Pass inline rows (`Vec<Tab<K>>`
    /// or `[Tab<K>; N]`) for adapter-owned tabs, or a reactive
    /// [`Field<Vec<Tab<K>>>`] from the consumer's
    /// `ars_leptos::reactive_stores::Store` for a consumer-owned dynamic
    /// tab list. Both sources flow through the same reactive store
    /// machinery, so pushes, removes, reorders, and per-row `closable`
    /// flag swaps re-dispatch [`Event::SetTabs`] without remounting the
    /// `Tabs` component.
    ///
    /// Accepts any `Subfield<…, Vec<Tab>>` via `Into` so consumers
    /// can pass `store.tabs()` directly without an explicit `.into()`.
    #[prop(into)]
    tabs: TabsSource<K>,

    /// Layout orientation. Defaults to `Horizontal`.
    #[prop(optional, into)]
    orientation: Option<Signal<Orientation>>,

    /// How keyboard focus interacts with selection.
    #[prop(optional, into)]
    activation_mode: Option<Signal<ActivationMode>>,

    /// Text direction.
    #[prop(optional, into)]
    dir: Option<Signal<Direction>>,

    /// When `true`, arrow-key focus wraps from last to first.
    #[prop(optional, into)]
    loop_focus: Option<Signal<bool>>,

    /// When `true` the only remaining tab cannot be closed.
    #[prop(optional, into)]
    disallow_empty_selection: Option<Signal<bool>>,

    /// When `true`, panels are not rendered until first selected.
    #[prop(optional, into)]
    lazy_mount: Option<Signal<bool>>,

    /// When `true`, panels are removed from the DOM when their tab is
    /// deselected.
    #[prop(optional, into)]
    unmount_on_exit: Option<Signal<bool>>,

    /// Set of disabled tab keys merged with each row's `Tab::disabled`.
    #[prop(optional, into)]
    disabled_keys: Option<Signal<BTreeSet<K>>>,

    /// When `true`, Ctrl+Arrow reorders tabs.
    #[prop(optional, into)]
    reorderable: Option<Signal<bool>>,

    /// Called after a user action commits a new selected key.
    #[prop(optional)]
    on_value_change: Option<Callback<Option<K>>>,

    /// Called when a close trigger or close key requests closing a tab.
    #[prop(optional)]
    on_close_tab: Option<Callback<K>>,

    /// Called before a reorder request is emitted to the core. Return
    /// `false` to veto the reorder and suppress its live announcement.
    #[prop(optional)]
    on_reorder: Option<Callback<ReorderEvent<K>, bool>>,

    /// Consumer class tokens appended to the Tabs root `<div>`. Tokens
    /// merge with whatever class the component itself emits so both reach
    /// the rendered root as a single `class` attribute. Inner parts (tab
    /// list, triggers, panels) carry their own `data-ars-part` attrs for
    /// finer-grained styling.
    #[prop(optional, into)]
    class: Option<TextProp>,

    /// Optional adapter-user content rendered inside Root after panels.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let id = use_id("tabs");

    let options = normalize_tabs_options(
        orientation,
        activation_mode,
        dir,
        loop_focus,
        disallow_empty_selection,
        lazy_mount,
        unmount_on_exit,
        disabled_keys,
        reorderable,
    );

    let modality = use_modality_context();
    let platform = use_platform_effects();

    let TabsReactiveSetup {
        tabs_field,
        owned_tabs_field,
        registrations,
        config,
        props_signal,
    } = setup_tabs_reactivity(&id, default_value, value, tabs, &options);

    let machine = use_machine_with_reactive_props::<tabs::Machine>(props_signal);

    setup_messages_sync(machine, config);
    register_tabs(machine, registrations);

    let tab_nodes = StoredValue::new_local(BTreeMap::<Key, web_sys::HtmlElement>::new());
    let send_with_focus_pulse = setup_focus_dispatch(machine, &platform, tab_nodes);

    let reorder_status = RwSignal::new(String::new());

    let drag_source = RwSignal::new(None::<Key>);

    let modality_revision = RwSignal::new(0_u64);

    let ever_selected = setup_lazy_mount_tracking(machine);

    let root_attrs = tabs_root_attrs(machine, class);
    let list_attrs = tabs_list_attrs(machine, tabs_field);

    #[cfg(not(feature = "ssr"))]
    setup_config_sync(&options, config);
    #[cfg(feature = "ssr")]
    let _ = &config;
    setup_auto_direction_effect(options.dir, machine, config, Arc::clone(&platform));

    let indicator_revision = RwSignal::new(0_u64);

    let tabs_meta = config.with_value(|cfg| cfg.tabs_meta);

    let (tab_indicator_attrs, indicator_style) = setup_tab_indicator(
        machine,
        Arc::clone(&platform),
        indicator_revision,
        tabs_meta,
    );

    let disallow_empty_selection = options.disallow_empty_selection;
    let lazy_mount = options.lazy_mount;
    let unmount_on_exit = options.unmount_on_exit;
    let reorderable = options.reorderable;

    // Keyed `<For>` iteration so per-tab DOM nodes (and their event
    // handlers / refs) survive list mutations: closing or reordering a
    // tab no longer tears down sibling rows. The element-kind bit is part
    // of the key because switching between button-like `<div>` and link
    // `<a>` rows must recreate the DOM node.
    let each_tabs = move || tabs_field.read().iter().cloned().collect::<Vec<_>>();
    let each_panels = move || tabs_field.read().iter().cloned().collect::<Vec<_>>();

    let render_button = {
        let modality = Arc::clone(&modality);
        move |tab: Tab<K>| {
            render_tab_button(
                tab,
                machine,
                send_with_focus_pulse,
                config,
                reorder_status,
                drag_source,
                modality_revision,
                tabs_field,
                tab_nodes,
                &modality,
                on_value_change,
                on_close_tab,
                on_reorder,
                owned_tabs_field,
                disallow_empty_selection,
                reorderable,
                indicator_revision,
            )
        }
    };

    let render_panel = move |tab: Tab<K>| {
        render_tab_panel(
            tab,
            machine,
            lazy_mount,
            unmount_on_exit,
            ever_selected,
            tabs_field,
        )
    };

    view! {
        <div {..root_attrs}>
            <div {..list_attrs}>
                <For
                    each=each_tabs
                    key=|tab| (tab.key.into_key(), tab.link.is_some())
                    children=render_button
                />
                <span {..tab_indicator_attrs} style=indicator_style></span>
            </div>
            <For each=each_panels key=|tab| tab.key.into_key() children=render_panel />
            {children.map(|c| c())}
            <Show when=move || reorderable.get()>
                <div
                    aria-live="polite"
                    aria-atomic="true"
                    style="position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;"
                >
                    {reorder_status}
                </div>
            </Show>
        </div>
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "normalizes the public Tabs component props into one internal options struct"
)]
fn normalize_tabs_options<K: TabKey>(
    orientation: Option<Signal<Orientation>>,
    activation_mode: Option<Signal<ActivationMode>>,
    dir: Option<Signal<Direction>>,
    loop_focus: Option<Signal<bool>>,
    disallow_empty_selection: Option<Signal<bool>>,
    lazy_mount: Option<Signal<bool>>,
    unmount_on_exit: Option<Signal<bool>>,
    disabled_keys: Option<Signal<BTreeSet<K>>>,
    reorderable: Option<Signal<bool>>,
) -> TabsOptions<K> {
    TabsOptions {
        orientation: orientation.unwrap_or_else(|| Signal::from(Orientation::Horizontal)),
        activation_mode: activation_mode.unwrap_or_else(|| Signal::from(ActivationMode::default())),
        dir: dir.unwrap_or_else(|| Signal::from(Direction::Ltr)),
        loop_focus: loop_focus.unwrap_or_else(|| Signal::from(true)),
        disallow_empty_selection: disallow_empty_selection.unwrap_or_else(|| Signal::from(false)),
        lazy_mount: lazy_mount.unwrap_or_else(|| Signal::from(false)),
        unmount_on_exit: unmount_on_exit.unwrap_or_else(|| Signal::from(false)),
        disabled_keys: disabled_keys.unwrap_or_else(|| Signal::from(BTreeSet::new())),
        reorderable: reorderable.unwrap_or_else(|| Signal::from(false)),
    }
}

fn setup_tabs_reactivity<K: TabKey>(
    id: &str,
    default_value: K,
    value: Option<Signal<Option<K>>>,
    tabs: TabsSource<K>,
    options: &TabsOptions<K>,
) -> TabsReactiveSetup<K> {
    let (tabs_field, owned_tabs_field) = tabs.into_field();

    let tabs_meta = tabs_meta_memo(tabs_field, options.disabled_keys);

    let registrations = tabs_registrations(tabs_meta);

    let config = StoredValue::new(TabsConfig {
        orientation: options.orientation.get_untracked(),
        dir: options.dir.get_untracked(),
        activation_mode: options.activation_mode.get_untracked(),
        reorderable: options.reorderable.get_untracked(),
        tabs_meta,
        messages_revision: RwSignal::new(0),
    });

    let props_signal = tabs_props_signal(id, default_value, value, tabs_field, options);

    TabsReactiveSetup {
        tabs_field,
        owned_tabs_field,
        registrations,
        config,
        props_signal,
    }
}

fn tabs_meta_memo<K: TabKey>(
    tabs: Field<Vec<Tab<K>>>,
    extra_disabled: Signal<BTreeSet<K>>,
) -> Memo<Vec<TabMeta<K>>> {
    Memo::new(move |_| {
        let extra_disabled = extra_disabled.get();

        tabs.read()
            .iter()
            .map(|t| {
                TabMeta::new(
                    t.key,
                    t.label_text.resolve(),
                    t.closable,
                    t.disabled || extra_disabled.contains(&t.key),
                )
            })
            .collect::<Vec<_>>()
    })
}

fn tabs_registrations<K: TabKey>(
    tabs_meta: Memo<Vec<TabMeta<K>>>,
) -> Memo<Vec<tabs::TabRegistration>> {
    Memo::new(move |_| tabs_meta.with(|meta| registrations_from_meta(meta)))
}

fn tabs_props_signal<K: TabKey>(
    id: &str,
    default_value: K,
    value: Option<Signal<Option<K>>>,
    tabs: Field<Vec<Tab<K>>>,
    options: &TabsOptions<K>,
) -> Signal<Props> {
    let disabled_keys = options.disabled_keys;
    let initial_disabled = disabled_keys
        .with_untracked(|extra| aggregate_disabled_keys_from_tabs_untracked(tabs, extra));

    let base_props = Props::new()
        .id(id)
        .default_value(Some(default_value.into_key()))
        .orientation(options.orientation.get_untracked())
        .activation_mode(options.activation_mode.get_untracked())
        .dir(options.dir.get_untracked())
        .loop_focus(options.loop_focus.get_untracked())
        .disallow_empty_selection(options.disallow_empty_selection.get_untracked())
        .lazy_mount(options.lazy_mount.get_untracked())
        .unmount_on_exit(options.unmount_on_exit.get_untracked())
        .disabled_keys(initial_disabled)
        .reorderable(options.reorderable.get_untracked());

    let props_for_signal = base_props.clone();

    let orientation = options.orientation;
    let activation_mode = options.activation_mode;
    let dir = options.dir;
    let loop_focus = options.loop_focus;
    let disallow_empty_selection = options.disallow_empty_selection;
    let lazy_mount = options.lazy_mount;
    let unmount_on_exit = options.unmount_on_exit;
    let reorderable = options.reorderable;

    Signal::derive(move || {
        let mut props = props_for_signal.clone();

        props.orientation = orientation.get();
        props.activation_mode = activation_mode.get();
        props.dir = dir.get();
        props.loop_focus = loop_focus.get();
        props.disallow_empty_selection = disallow_empty_selection.get();
        props.lazy_mount = lazy_mount.get();
        props.unmount_on_exit = unmount_on_exit.get();
        props.reorderable = reorderable.get();
        disabled_keys.with(|extra| {
            props.disabled_keys = aggregate_disabled_keys_from_tabs(tabs, extra);
        });

        if let Some(value_signal) = value {
            props.value = Some(value_signal.get().map(TabKey::into_key));
        }

        props
    })
}

fn register_tabs(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    registrations: Memo<Vec<tabs::TabRegistration>>,
) {
    machine
        .send
        .run(Event::SetTabs(registrations.get_untracked()));

    #[cfg(not(feature = "ssr"))]
    Effect::watch(
        move || registrations.get(),
        move |new, _old, _| {
            machine.send.run(Event::SetTabs(new.clone()));
        },
        false,
    );

    #[cfg(feature = "ssr")]
    let _ = registrations;
}

fn setup_messages_sync<K: TabKey>(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    config: StoredValue<TabsConfig<K>>,
) {
    let locale = use_locale();

    let registries = current_ars_context().map_or_else(
        || Arc::new(ars_core::I18nRegistries::new()),
        |ctx| Arc::clone(&ctx.i18n_registries),
    );

    let previous = StoredValue::new(None::<(ars_core::Locale, Messages)>);

    let sync = move |locale: ars_core::Locale| {
        let messages = ars_core::resolve_messages::<Messages>(None, registries.as_ref(), &locale);

        let next = (locale, messages);

        let changed = previous.with_value(|previous| previous.as_ref() != Some(&next));

        if changed {
            previous.set_value(Some(next.clone()));
            machine.send.run(Event::SyncMessages {
                locale: next.0,
                messages: next.1,
            });
            config.with_value(|cfg| cfg.messages_revision.update(|revision| *revision += 1));
        }
    };

    sync(locale.get_untracked());

    #[cfg(not(feature = "ssr"))]
    Effect::watch(
        move || locale.get(),
        move |locale, _old, _| sync(locale.clone()),
        false,
    );
}

#[cfg(not(feature = "ssr"))]
fn setup_config_sync<K: TabKey>(options: &TabsOptions<K>, config: StoredValue<TabsConfig<K>>) {
    let orientation = options.orientation;
    let activation_mode = options.activation_mode;
    let dir = options.dir;
    let reorderable = options.reorderable;

    Effect::new(move |_| {
        config.update_value(|cfg| {
            cfg.orientation = orientation.get();
            cfg.activation_mode = activation_mode.get();
            cfg.dir = dir.get();
            cfg.reorderable = reorderable.get();
        });
    });
}

fn setup_focus_dispatch(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    platform: &Arc<dyn PlatformEffects>,
    tab_nodes: TabNodeRegistry,
) -> Callback<Event> {
    let focus_pulse = RwSignal::new(0_u64);

    let focused_tab_key = machine.derive(|api| api.focused_tab().cloned());

    #[cfg(not(feature = "ssr"))]
    {
        let platform = Arc::clone(platform);

        Effect::new(move |_| {
            focus_pulse.track();

            let Some(key) = focused_tab_key.get_untracked() else {
                return;
            };

            if focus_tab_node(tab_nodes, &key) {
                return;
            }

            let Some(element_id) = machine.with_api_snapshot(|api| tab_id_from_api(api, &key))
            else {
                return;
            };

            platform.focus_element_by_id(&element_id);
            focus_tab_element_by_id(&element_id);
        });
    }

    #[cfg(feature = "ssr")]
    {
        let _ = (platform, focused_tab_key, tab_nodes);
    }

    Callback::new(move |event: Event| {
        let needs_focus = matches!(
            event,
            Event::FocusNext | Event::FocusPrev | Event::FocusFirst | Event::FocusLast
        );

        machine.send.run(event);

        if needs_focus {
            focus_pulse.update(|n| *n += 1);
        }
    })
}

fn setup_lazy_mount_tracking(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
) -> RwSignal<BTreeSet<Key>> {
    let initial_selection = machine
        .with_api_snapshot(|api| api.selected_tab().cloned())
        .map(|key| BTreeSet::from([key]))
        .unwrap_or_default();

    let ever_selected = RwSignal::new(initial_selection);

    #[cfg(not(feature = "ssr"))]
    {
        let selected_memo = machine.derive(|api| api.selected_tab().cloned());

        Effect::new(move |_| {
            if let Some(key) = selected_memo.get() {
                ever_selected.update(|set| {
                    set.insert(key);
                });
            }
        });
    }

    ever_selected
}

#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "method-pointer form fails HRTB inference for Api::*_attrs"
)]
fn tabs_root_attrs(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    consumer_class: Option<TextProp>,
) -> Vec<crate::LeptosAttribute> {
    let mut attrs = machine.with_api_snapshot(|api| api.root_attrs());

    let dir = machine.derive(|api| {
        api.root_attrs()
            .get(&HtmlAttr::Dir)
            .map(str::to_owned)
            .unwrap_or_default()
    });

    let orientation = machine.derive(|api| {
        api.root_attrs()
            .get(&HtmlAttr::Data("ars-orientation"))
            .map(str::to_owned)
            .unwrap_or_default()
    });

    attrs
        .set(HtmlAttr::Dir, AttrValue::reactive(move || dir.get()))
        .set(
            HtmlAttr::Data("ars-orientation"),
            AttrValue::reactive(move || orientation.get()),
        );

    crate::merge_consumer_class_prop_into(&mut attrs, consumer_class);

    attr_map_to_leptos_inline_attrs(attrs)
}

#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "method-pointer form fails HRTB inference for Api::list_attrs"
)]
fn tabs_list_attrs<K: TabKey>(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    tabs_field: Field<Vec<Tab<K>>>,
) -> Vec<crate::LeptosAttribute> {
    let mut attrs = machine.with_api_snapshot(|api| api.list_attrs());

    let owns = machine.derive(move |api| {
        tabs_field
            .read()
            .iter()
            .filter_map(|tab| tab_id_from_api(api, &tab.key.into_key()))
            .collect::<Vec<_>>()
            .join(" ")
    });
    let orientation = machine.derive(|api| {
        api.list_attrs()
            .get(&HtmlAttr::Aria(AriaAttr::Orientation))
            .map(str::to_owned)
            .unwrap_or_default()
    });

    attrs
        .set(
            HtmlAttr::Aria(AriaAttr::Owns),
            AttrValue::reactive(move || owns.get()),
        )
        .set(
            HtmlAttr::Aria(AriaAttr::Orientation),
            AttrValue::reactive(move || orientation.get()),
        );

    attr_map_to_leptos_inline_attrs(attrs)
}

fn tab_id_from_api(api: &tabs::Api<'_>, key: &Key) -> Option<String> {
    api.tab_attrs(key, false)
        .get(&HtmlAttr::Id)
        .map(String::from)
}

fn setup_auto_direction_effect<K: TabKey>(
    dir: Signal<Direction>,
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    config: StoredValue<TabsConfig<K>>,
    platform: Arc<dyn PlatformEffects>,
) {
    #[cfg(feature = "ssr")]
    drop((dir, machine, config, platform));

    #[cfg(not(feature = "ssr"))]
    {
        let list_id =
            machine.with_api_snapshot(|api| api.list_attrs().get(&HtmlAttr::Id).map(String::from));

        if let Some(list_id) = list_id {
            let send = machine.send;

            Effect::new(move |_| {
                if dir.get() != Direction::Auto {
                    return;
                }

                let resolved = Direction::from(platform.resolved_direction(&list_id));

                send.run(Event::SetDirection(resolved));

                config.update_value(|cfg| {
                    cfg.dir = resolved;
                });
            });
        }
    }
}

#[expect(
    clippy::redundant_closure_for_method_calls,
    reason = "method-pointer form fails HRTB inference for Api::*_attrs"
)]
fn setup_tab_indicator<K: TabKey>(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    platform: Arc<dyn PlatformEffects>,
    indicator_revision: RwSignal<u64>,
    tabs_meta: Memo<Vec<TabMeta<K>>>,
) -> (Vec<crate::LeptosAttribute>, RwSignal<String>) {
    let attrs =
        attr_map_to_leptos_inline_attrs(machine.with_api_snapshot(|api| api.tab_indicator_attrs()));

    let style = RwSignal::new(String::new());

    #[cfg(feature = "ssr")]
    drop((platform, indicator_revision, tabs_meta));

    #[cfg(not(feature = "ssr"))]
    {
        let selected_for_indicator = machine.derive(|api| api.selected_tab().cloned());

        let layout_for_indicator = machine.derive(indicator_layout_signature);

        let auto_update_cleanup = StoredValue::new_local(None::<Box<dyn FnOnce()>>);

        Effect::new(move |_| {
            selected_for_indicator.track();
            layout_for_indicator.track();
            tabs_meta.track();
            indicator_revision.track();
            style.set(indicator_measurement_style(machine, platform.as_ref()));
            setup_indicator_auto_update(machine, Arc::clone(&platform), style, auto_update_cleanup);
        });

        on_cleanup(move || clear_indicator_auto_update(auto_update_cleanup));
    }

    (attrs, style)
}

// ────────────────────────────────────────────────────────────────────
// Tab button rendering
// ────────────────────────────────────────────────────────────────────

#[expect(
    clippy::too_many_arguments,
    reason = "tab rendering needs machine, event callbacks, and reactive store handles"
)]
#[expect(
    clippy::too_many_lines,
    reason = "Leptos tab rendering keeps related DOM event closures in one helper"
)]
#[expect(
    clippy::needless_pass_by_value,
    reason = "Leptos keyed For hands owned rows to child renderers, and closures keep cloned fallbacks"
)]
fn render_tab_button<K: TabKey>(
    tab: Tab<K>,
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: Callback<Event>,
    config: StoredValue<TabsConfig<K>>,
    reorder_status: RwSignal<String>,
    drag_source: RwSignal<Option<Key>>,
    modality_revision: RwSignal<u64>,
    tabs_field: Field<Vec<Tab<K>>>,
    tab_nodes: TabNodeRegistry,
    modality: &Arc<dyn ModalityContext>,
    on_value_change: Option<Callback<Option<K>>>,
    on_close_tab: Option<Callback<K>>,
    on_reorder: Option<Callback<ReorderEvent<K>, bool>>,
    owned_tabs_field: Option<Field<Vec<Tab<K>>>>,
    disallow_empty_selection: Signal<bool>,
    reorderable: Signal<bool>,
    indicator_revision: RwSignal<u64>,
) -> impl IntoView + use<K> {
    let typed_key = tab.key;
    let key = typed_key.into_key();

    let tab_attrs = attr_map_to_leptos_inline_attrs(reactive_tab_attrs(
        machine,
        &key,
        Arc::clone(modality),
        modality_revision,
        config,
    ));

    let link = tab.link.clone();

    let prevent_default_on_click = link.is_some();

    let on_click = {
        let key = key.clone();
        move |event: web_sys::MouseEvent| {
            if prevent_default_on_click {
                event.prevent_default();
            }

            select_and_emit_value_change(machine, send, &key, on_value_change, config);
        }
    };

    let on_pointerdown = {
        let modality = Arc::clone(modality);
        move |event: web_sys::PointerEvent| {
            modality.on_pointer_down(pointer_type_from_leptos(&event.pointer_type()));
            bump_revision(modality_revision);
        }
    };

    let on_focus = {
        let key = key.clone();
        move |_event: web_sys::FocusEvent| {
            focus_event_and_emit_value_change(machine, send, &key, on_value_change, config);
        }
    };

    let on_blur = move |_event: web_sys::FocusEvent| {
        send.run(Event::Blur);
    };

    let on_keydown = {
        let key = key.clone();
        let fallback = tab.clone();
        let modality = Arc::clone(modality);
        move |event: web_sys::KeyboardEvent| {
            let label_text = current_tab_by_key(tabs_field, typed_key, &fallback)
                .label_text
                .resolve();
            bump_revision(modality_revision);

            handle_tab_keydown(
                &event,
                &key,
                &label_text,
                machine,
                send,
                config,
                reorder_status,
                on_value_change,
                on_close_tab,
                on_reorder,
                &modality,
                tab_nodes,
                owned_tabs_field,
                disallow_empty_selection,
                indicator_revision,
            );
        }
    };

    let is_draggable = move || reorderable.get().to_string();

    let on_dragstart = {
        let key = key.clone();
        move |_event: web_sys::DragEvent| {
            if config.with_value(|cfg| cfg.reorderable) {
                drag_source.set(Some(key.clone()));
            }
        }
    };

    let on_dragend = move |_event: web_sys::DragEvent| {
        drag_source.set(None);
    };

    let on_dragover = {
        let key = key.clone();
        move |event: web_sys::DragEvent| {
            if can_accept_drag(config, drag_source, &key) {
                event.prevent_default();
            }
        }
    };

    let on_drop = {
        let key = key.clone();
        move |event: web_sys::DragEvent| {
            handle_tab_drop(
                &event,
                &key,
                machine,
                send,
                config,
                drag_source,
                reorder_status,
                tab_nodes,
                on_reorder,
                owned_tabs_field,
                indicator_revision,
            );
        }
    };

    // Reactive close-button rendering: subscribes to the row's
    // `closable` flag via the consumer's reactive store. When the
    // store mutates (e.g. `tabs[i].closable = true`), Leptos re-runs
    // this closure and renders/unrenders the close button without
    // remounting the parent row's DOM node.
    let close_button = {
        let key = key.clone();
        let fallback = tab.clone();

        move || {
            let is_closable = config.with_value(|cfg| {
                cfg.messages_revision.get();

                cfg.tabs_meta
                    .get()
                    .iter()
                    .any(|meta| meta.key == key && meta.closable && !meta.disabled)
            });

            if !is_closable {
                return None;
            }

            let fallback = fallback.clone();
            let close_attrs =
                attr_map_to_leptos_inline_attrs(machine.with_api_snapshot(move |api| {
                    let label_text = current_tab_by_key(tabs_field, typed_key, &fallback)
                        .label_text
                        .resolve();

                    let mut attrs = api.close_trigger_attrs(&label_text);

                    attrs.set(HtmlAttr::TabIndex, "-1");

                    attrs
                }));

            Some(view! {
                <button
                    {..close_attrs}
                    on:click={
                        let key = key.clone();
                        move |event: web_sys::MouseEvent| {
                            event.prevent_default();
                            event.stop_propagation();
                            let successor = selected_close_successor(machine, &key);
                            emit_close_request(
                                machine,
                                send,
                                on_close_tab,
                                on_value_change,
                                typed_key,
                                &key,
                                successor.as_ref().map(|(successor_key, _)| successor_key.clone()),
                                config,
                                owned_tabs_field,
                                disallow_empty_selection,
                            );
                            if let Some((successor_key, element_id)) = successor {
                                defer_focus_tab_node_or_id(tab_nodes, successor_key, element_id);
                            }
                        }
                    }
                ></button>
            })
        }
    };

    if let Some(href) = link {
        let fallback_label = tab.clone();
        Either::Left(view! {
            <>
                <a
                    {..tab_attrs}
                    node_ref=tab_anchor_node_ref(tab_nodes, key.clone())
                    href=href.as_str().to_string()
                    aria-roledescription=move || reorderable.get().then_some("draggable tab")
                    on:click=on_click
                    on:focus=on_focus
                    on:blur=on_blur
                    on:keydown=on_keydown
                    on:pointerdown=on_pointerdown
                    on:dragstart=on_dragstart
                    on:dragover=on_dragover
                    on:drop=on_drop
                    on:dragend=on_dragend
                    draggable=is_draggable
                >
                    {move || current_tab_by_key(tabs_field, typed_key, &fallback_label).label.run()}
                </a>
                {close_button}
            </>
        })
    } else {
        let fallback_label = tab.clone();
        Either::Right(view! {
            <div
                {..tab_attrs}
                node_ref=tab_div_node_ref(tab_nodes, key.clone())
                aria-roledescription=move || reorderable.get().then_some("draggable tab")
                on:click=on_click
                on:focus=on_focus
                on:blur=on_blur
                on:keydown=on_keydown
                on:pointerdown=on_pointerdown
                on:dragstart=on_dragstart
                on:dragover=on_dragover
                on:drop=on_drop
                on:dragend=on_dragend
                draggable=is_draggable
            >
                {move || current_tab_by_key(tabs_field, typed_key, &fallback_label).label.run()}
                {close_button}
            </div>
        })
    }
}

fn current_tab_by_key<K: TabKey>(
    tabs_field: Field<Vec<Tab<K>>>,
    typed_key: K,
    fallback: &Tab<K>,
) -> Tab<K> {
    tabs_field
        .read()
        .iter()
        .find(|tab| tab.key == typed_key)
        .cloned()
        .unwrap_or_else(|| fallback.clone())
}

fn tab_anchor_node_ref(tab_nodes: TabNodeRegistry, key: Key) -> NodeRef<html::A> {
    let node_ref = NodeRef::<html::A>::new();

    register_anchor_tab_node(node_ref, tab_nodes, key);

    node_ref
}

fn tab_div_node_ref(tab_nodes: TabNodeRegistry, key: Key) -> NodeRef<html::Div> {
    let node_ref = NodeRef::<html::Div>::new();

    register_div_tab_node(node_ref, tab_nodes, key);

    node_ref
}

#[cfg(all(not(feature = "ssr"), target_arch = "wasm32"))]
fn register_anchor_tab_node(node_ref: NodeRef<html::A>, tab_nodes: TabNodeRegistry, key: Key) {
    node_ref.on_load(move |node| {
        tab_nodes.update_value(|nodes| {
            nodes.insert(key, node.unchecked_into::<web_sys::HtmlElement>());
        });
    });
}

#[cfg(any(
    feature = "ssr",
    all(not(feature = "ssr"), not(target_arch = "wasm32"))
))]
fn register_anchor_tab_node(_node_ref: NodeRef<html::A>, _tab_nodes: TabNodeRegistry, _key: Key) {}

#[cfg(all(not(feature = "ssr"), target_arch = "wasm32"))]
fn register_div_tab_node(node_ref: NodeRef<html::Div>, tab_nodes: TabNodeRegistry, key: Key) {
    node_ref.on_load(move |node| {
        tab_nodes.update_value(|nodes| {
            nodes.insert(key, node.unchecked_into::<web_sys::HtmlElement>());
        });
    });
}

#[cfg(any(
    feature = "ssr",
    all(not(feature = "ssr"), not(target_arch = "wasm32"))
))]
fn register_div_tab_node(_node_ref: NodeRef<html::Div>, _tab_nodes: TabNodeRegistry, _key: Key) {}

fn can_accept_drag<K: TabKey>(
    config: StoredValue<TabsConfig<K>>,
    drag_source: RwSignal<Option<Key>>,
    target_key: &Key,
) -> bool {
    if !config.with_value(|cfg| cfg.reorderable) {
        return false;
    }

    let Some(source_key) = drag_source.get_untracked() else {
        return false;
    };

    source_key != *target_key
        && config.with_value(|cfg| {
            drag_reorder_plan(&cfg.tabs_meta.get_untracked(), &source_key, target_key).is_some()
        })
}

#[expect(
    clippy::too_many_arguments,
    reason = "drop handling mirrors keyboard reorder callbacks and live-region state"
)]
fn handle_tab_drop<K: TabKey>(
    event: &web_sys::DragEvent,
    target_key: &Key,
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: Callback<Event>,
    config: StoredValue<TabsConfig<K>>,
    drag_source: RwSignal<Option<Key>>,
    reorder_status: RwSignal<String>,
    tab_nodes: TabNodeRegistry,
    on_reorder: Option<Callback<ReorderEvent<K>, bool>>,
    owned_tabs_field: Option<Field<Vec<Tab<K>>>>,
    indicator_revision: RwSignal<u64>,
) {
    if !config.with_value(|cfg| cfg.reorderable) {
        drag_source.set(None);

        return;
    }

    let Some(source_key) = drag_source.get_untracked() else {
        return;
    };

    drag_source.set(None);

    if source_key == *target_key {
        return;
    }

    let Some(plan) = config.with_value(|cfg| {
        drag_reorder_plan(&cfg.tabs_meta.get_untracked(), &source_key, target_key)
    }) else {
        return;
    };

    let reorder_event = plan.event;
    let new_index = reorder_event.new_index;

    event.prevent_default();

    let Some(external_reorder_committed) = external_reorder_committed(on_reorder, &reorder_event)
    else {
        return;
    };

    let focus_key = source_key.clone();

    if !reorder_owned_tab(owned_tabs_field, &reorder_event, external_reorder_committed) {
        return;
    }

    send.run(Event::ReorderTab {
        tab: source_key,
        new_index,
    });

    if let Some(element_id) = tab_element_id(machine, &focus_key) {
        defer_focus_tab_node_or_id(tab_nodes, focus_key, element_id);
    }

    let announcement = machine.with_api_snapshot(|api| {
        api.reorder_announcement(&plan.label_text, new_index + 1, plan.total)
    });

    reorder_status.set(announcement);
    bump_revision(indicator_revision);
}

#[expect(
    clippy::too_many_arguments,
    reason = "keyboard handling needs machine, callbacks, config, and live region state"
)]
fn handle_tab_keydown<K: TabKey>(
    event: &web_sys::KeyboardEvent,
    key: &Key,
    label_text: &str,
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: Callback<Event>,
    config: StoredValue<TabsConfig<K>>,
    reorder_status: RwSignal<String>,
    on_value_change: Option<Callback<Option<K>>>,
    on_close_tab: Option<Callback<K>>,
    on_reorder: Option<Callback<ReorderEvent<K>, bool>>,
    modality: &Arc<dyn ModalityContext>,
    tab_nodes: TabNodeRegistry,
    owned_tabs_field: Option<Field<Vec<Tab<K>>>>,
    disallow_empty_selection: Signal<bool>,
    indicator_revision: RwSignal<u64>,
) {
    let data = keyboard_event_data(event);
    modality.on_key_down(
        data.key,
        KeyModifiers {
            shift: data.shift_key,
            ctrl: data.ctrl_key,
            alt: data.alt_key,
            meta: data.meta_key,
        },
    );

    let cfg = config.with_value(|c| *c);

    let tabs_meta = cfg.tabs_meta.get_untracked();

    let effective_dir = effective_direction(machine, cfg.dir);

    let (prev_key, next_key) = arrow_pair_for(cfg.orientation, effective_dir);

    // §6.4 — Ctrl+Arrow reorder takes priority and short-circuits.
    if cfg.reorderable && data.ctrl_key {
        let forward = match cfg.orientation {
            Orientation::Horizontal => match data.key {
                KeyboardKey::ArrowRight => Some(true),
                KeyboardKey::ArrowLeft => Some(false),
                _ => None,
            },

            Orientation::Vertical => match data.key {
                KeyboardKey::ArrowDown => Some(true),
                KeyboardKey::ArrowUp => Some(false),
                _ => None,
            },
        };

        if let Some(forward) = forward {
            // Route through the agnostic core so clamp / disable guard
            // logic stays single-sourced. The announcement template is
            // also fetched from the core (Messages-driven, localizable).
            let total = tabs_meta.len();

            let old_index = tabs_meta.iter().position(|meta| meta.key == *key);

            let plan = machine.with_api_snapshot(|api| {
                api.next_reorder_index(key, forward).map(|new_index| {
                    let announcement = api.reorder_announcement(label_text, new_index + 1, total);

                    (new_index, announcement)
                })
            });

            if let (Some(old_index), Some((new_index, announcement))) = (old_index, plan) {
                let Some(typed_key) = tabs_meta.get(old_index).map(|meta| meta.typed_key) else {
                    return;
                };

                let reorder_event = ReorderEvent {
                    key: typed_key,
                    old_index,
                    new_index,
                };

                event.prevent_default();

                let Some(external_reorder_committed) =
                    external_reorder_committed(on_reorder, &reorder_event)
                else {
                    return;
                };

                if reorder_owned_tab(owned_tabs_field, &reorder_event, external_reorder_committed) {
                    send.run(Event::ReorderTab {
                        tab: key.clone(),
                        new_index,
                    });

                    if let Some(element_id) = tab_element_id(machine, key) {
                        defer_focus_tab_node_or_id(tab_nodes, key.clone(), element_id);
                    }

                    reorder_status.set(announcement);
                    bump_revision(indicator_revision);
                }
            }

            return;
        }
    }

    // §1.6 — focus / activation / close keystrokes.
    let manual = matches!(cfg.activation_mode, ActivationMode::Manual);

    let is_closable = tabs_meta
        .iter()
        .find(|meta| meta.key == *key)
        .is_some_and(|meta| meta.closable && !meta.disabled);

    if data.key == next_key {
        event.prevent_default();

        if manual {
            send.run(Event::FocusNext);
        } else {
            focus_and_emit_value_change(machine, send, Event::FocusNext, on_value_change, config);
        }
    } else if data.key == prev_key {
        event.prevent_default();

        if manual {
            send.run(Event::FocusPrev);
        } else {
            focus_and_emit_value_change(machine, send, Event::FocusPrev, on_value_change, config);
        }
    } else if data.key == KeyboardKey::Home {
        event.prevent_default();

        if manual {
            send.run(Event::FocusFirst);
        } else {
            focus_and_emit_value_change(machine, send, Event::FocusFirst, on_value_change, config);
        }
    } else if data.key == KeyboardKey::End {
        event.prevent_default();

        if manual {
            send.run(Event::FocusLast);
        } else {
            focus_and_emit_value_change(machine, send, Event::FocusLast, on_value_change, config);
        }
    } else if (data.key == KeyboardKey::Enter || data.key == KeyboardKey::Space)
        && manual
        && !data.repeat
        && !data.is_composing
    {
        event.prevent_default();

        select_and_emit_value_change(machine, send, key, on_value_change, config);
    } else if matches!(data.key, KeyboardKey::Delete | KeyboardKey::Backspace)
        && is_closable
        && !data.repeat
        && !data.is_composing
    {
        event.prevent_default();

        let successor = selected_close_successor(machine, key);

        let Some(typed_key) = typed_key_for_key(&tabs_meta, key) else {
            return;
        };

        emit_close_request(
            machine,
            send,
            on_close_tab,
            on_value_change,
            typed_key,
            key,
            successor
                .as_ref()
                .map(|(successor_key, _)| successor_key.clone()),
            config,
            owned_tabs_field,
            disallow_empty_selection,
        );

        if let Some((successor_key, element_id)) = successor {
            defer_focus_tab_node_or_id(tab_nodes, successor_key, element_id);
        }
    }
}

fn pointer_type_from_leptos(pointer_type: &str) -> PointerType {
    match pointer_type {
        "mouse" => PointerType::Mouse,
        "touch" => PointerType::Touch,
        "pen" => PointerType::Pen,
        _ => PointerType::Virtual,
    }
}

fn selected_close_successor(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    key: &Key,
) -> Option<(Key, String)> {
    machine.with_api_snapshot(|api| {
        if api.selected_tab() != Some(key) {
            return None;
        }

        api.successor_for_close(key).and_then(|successor_key| {
            tab_id_from_api(api, &successor_key).map(|id| (successor_key, id))
        })
    })
}

// ────────────────────────────────────────────────────────────────────
// Tab panel rendering
// ────────────────────────────────────────────────────────────────────

fn render_tab_panel<K: TabKey>(
    tab: Tab<K>,
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    lazy_mount: Signal<bool>,
    unmount_on_exit: Signal<bool>,
    ever_selected: RwSignal<BTreeSet<Key>>,
    tabs_field: Field<Vec<Tab<K>>>,
) -> impl IntoView + use<K> {
    let typed_key = tab.key;
    let key_for_attrs = tab.key.into_key();
    let key_for_panel = tab.key.into_key();

    let panel_attrs =
        attr_map_to_leptos_inline_attrs(reactive_panel_attrs(machine, &key_for_attrs));

    let is_selected_memo = {
        let key = key_for_panel.clone();

        machine.derive(move |api| api.is_tab_selected(&key))
    };

    let fallback_panel = tab;

    let panel_body = move || {
        let is_selected = is_selected_memo.get();

        let already_selected = ever_selected.with(|set| set.contains(&key_for_panel));

        let should_render = should_render_panel_body(
            is_selected,
            already_selected,
            lazy_mount.get(),
            unmount_on_exit.get(),
        );

        if should_render {
            Some(
                current_tab_by_key(tabs_field, typed_key, &fallback_panel)
                    .panel
                    .run(),
            )
        } else {
            None
        }
    };

    view! { <div {..panel_attrs}>{panel_body}</div> }
}

const fn should_render_panel_body(
    is_selected: bool,
    already_selected: bool,
    lazy_mount: bool,
    unmount_on_exit: bool,
) -> bool {
    if unmount_on_exit {
        is_selected
    } else if lazy_mount {
        is_selected || already_selected
    } else {
        true
    }
}

// ────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────

fn focus_and_emit_value_change<K: TabKey>(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: Callback<Event>,
    event: Event,
    on_value_change: Option<Callback<Option<K>>>,
    config: StoredValue<TabsConfig<K>>,
) {
    let before = machine.with_api_snapshot(|api| api.selected_tab().cloned());

    send.run(event);

    let after = machine.with_api_snapshot(|api| api.selected_tab().cloned());

    let emitted = if before == after {
        let focused = machine.with_api_snapshot(|api| api.focused_tab().cloned());

        if before == focused {
            return;
        }

        focused
    } else {
        after
    };

    if let Some(callback) = on_value_change {
        let tabs_meta = config.with_value(|cfg| cfg.tabs_meta.get_untracked());

        callback.run(emitted.and_then(|key| typed_key_for_key(&tabs_meta, &key)));
    }
}

fn focus_event_and_emit_value_change<K: TabKey>(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: Callback<Event>,
    key: &Key,
    on_value_change: Option<Callback<Option<K>>>,
    config: StoredValue<TabsConfig<K>>,
) {
    let before_selected = machine.with_api_snapshot(|api| api.selected_tab().cloned());
    let before_focused = machine.with_api_snapshot(|api| api.focused_tab().cloned());

    send.run(Event::Focus(key.clone()));

    if before_focused.as_ref() == Some(key) {
        return;
    }

    let after_selected = machine.with_api_snapshot(|api| api.selected_tab().cloned());
    let after_focused = machine.with_api_snapshot(|api| api.focused_tab().cloned());

    let emitted = if before_selected != after_selected {
        after_selected
    } else if before_selected.is_none() && after_focused.as_ref() == Some(key) {
        after_focused
    } else {
        return;
    };

    if let Some(callback) = on_value_change {
        let tabs_meta = config.with_value(|cfg| cfg.tabs_meta.get_untracked());

        callback.run(emitted.and_then(|key| typed_key_for_key(&tabs_meta, &key)));
    }
}

#[cfg(all(not(feature = "ssr"), target_arch = "wasm32"))]
fn focus_tab_node(tab_nodes: TabNodeRegistry, key: &Key) -> bool {
    let Some(element) = tab_nodes
        .try_with_value(|nodes| nodes.get(key).cloned())
        .flatten()
    else {
        return false;
    };

    element.is_connected() && element.focus().is_ok()
}

#[cfg(all(not(feature = "ssr"), not(target_arch = "wasm32")))]
const fn focus_tab_node(_tab_nodes: TabNodeRegistry, _key: &Key) -> bool {
    false
}

#[cfg(all(not(feature = "ssr"), target_arch = "wasm32"))]
fn focus_tab_element_by_id(id: &str) {
    // Tabs must provide roving DOM focus even when the consumer has not
    // installed an ArsProvider platform. Keep the provider call above as
    // the primary path, then fall back to the browser implementation for
    // standalone adapter usage and examples.
    ars_dom::focus_element_by_id(id);
}

#[cfg(all(not(feature = "ssr"), not(target_arch = "wasm32")))]
const fn focus_tab_element_by_id(_id: &str) {}

#[cfg(all(not(feature = "ssr"), target_arch = "wasm32"))]
fn defer_focus_tab_node_or_id(tab_nodes: TabNodeRegistry, key: Key, id: String) {
    let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
        let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
            if !focus_tab_node(tab_nodes, &key) {
                focus_tab_element_by_id(&id);
            }
        });

        if let Some(window) = web_sys::window() {
            drop(window.request_animation_frame(callback.as_ref().unchecked_ref()));
        }
    });

    if let Some(window) = web_sys::window() {
        drop(window.request_animation_frame(callback.as_ref().unchecked_ref()));
    }
}

#[cfg(any(
    feature = "ssr",
    all(not(feature = "ssr"), not(target_arch = "wasm32"))
))]
fn defer_focus_tab_node_or_id(_tab_nodes: TabNodeRegistry, _key: Key, _id: String) {}

fn tab_element_id(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    key: &Key,
) -> Option<String> {
    machine.with_api_snapshot(|api| {
        api.tab_attrs(key, false)
            .get(&HtmlAttr::Id)
            .map(String::from)
    })
}

fn select_and_emit_value_change<K: TabKey>(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: Callback<Event>,
    key: &Key,
    on_value_change: Option<Callback<Option<K>>>,
    config: StoredValue<TabsConfig<K>>,
) {
    let before = machine.with_api_snapshot(|api| api.selected_tab().cloned());

    send.run(Event::SelectTab(key.clone()));

    let after = machine.with_api_snapshot(|api| api.selected_tab().cloned());

    let Some(callback) = on_value_change else {
        return;
    };

    let tabs_meta = config.with_value(|cfg| cfg.tabs_meta.get_untracked());

    let emitted = if before != after {
        Some(after.and_then(|key| typed_key_for_key(&tabs_meta, &key)))
    } else if before.as_ref() != Some(key) {
        tabs_meta
            .iter()
            .find(|tab| tab.key == *key && !tab.disabled)
            .map(|tab| Some(tab.typed_key))
    } else {
        return;
    };

    if let Some(emitted) = emitted {
        callback.run(emitted);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "close requests need machine dispatch, callbacks, key metadata, and owned-store context"
)]
fn emit_close_request<K: TabKey>(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: Callback<Event>,
    on_close_tab: Option<Callback<K>>,
    on_value_change: Option<Callback<Option<K>>>,
    typed_key: K,
    key: &Key,
    successor: Option<Key>,
    config: StoredValue<TabsConfig<K>>,
    owned_tabs_field: Option<Field<Vec<Tab<K>>>>,
    disallow_empty_selection: Signal<bool>,
) {
    if !machine.with_api_snapshot(|api| api.can_close_tab(key)) {
        return;
    }

    let was_selected = machine.with_api_snapshot(|api| api.selected_tab() == Some(key));

    send.run(Event::CloseTab(key.clone()));

    if let Some(callback) = on_close_tab {
        callback.run(typed_key);
    }

    close_owned_tab(
        owned_tabs_field,
        key,
        disallow_empty_selection.get_untracked(),
    );

    if let Some(successor) = successor {
        machine.send.run(Event::SelectTab(successor.clone()));

        if was_selected && let Some(callback) = on_value_change {
            let tabs_meta = config.with_value(|cfg| cfg.tabs_meta.get_untracked());

            callback.run(typed_key_for_key(&tabs_meta, &successor));
        }
    } else if was_selected && let Some(callback) = on_value_change {
        callback.run(None);
    }
}

fn close_owned_tab<K: TabKey>(
    owned_tabs_field: Option<Field<Vec<Tab<K>>>>,
    key: &Key,
    disallow_empty_selection: bool,
) {
    let Some(tabs_field) = owned_tabs_field else {
        return;
    };

    let mut tabs = tabs_field.write();

    if disallow_empty_selection && tabs.len() <= 1 {
        return;
    }

    tabs.retain(|tab| tab.key.into_key() != *key);
}

fn reorder_owned_tab<K: TabKey>(
    owned_tabs_field: Option<Field<Vec<Tab<K>>>>,
    event: &ReorderEvent<K>,
    external_reorder_committed: bool,
) -> bool {
    let Some(tabs_field) = owned_tabs_field else {
        return external_reorder_committed;
    };

    let mut tabs = tabs_field.write();

    if event.old_index >= tabs.len() || event.new_index >= tabs.len() {
        return false;
    }

    let tab = tabs.remove(event.old_index);

    tabs.insert(event.new_index, tab);

    true
}

fn external_reorder_committed<K: TabKey>(
    on_reorder: Option<Callback<ReorderEvent<K>, bool>>,
    event: &ReorderEvent<K>,
) -> Option<bool> {
    on_reorder.map_or(Some(false), |callback| {
        callback.run(event.clone()).then_some(true)
    })
}

fn bump_revision(revision_signal: RwSignal<u64>) {
    revision_signal.update(|revision| *revision = revision.wrapping_add(1));
}

#[cfg(not(feature = "ssr"))]
fn setup_indicator_auto_update(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    platform: Arc<dyn PlatformEffects>,
    style: RwSignal<String>,
    cleanup_store: StoredValue<Option<Box<dyn FnOnce()>>, LocalStorage>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        clear_indicator_auto_update(cleanup_store);

        let Some((list_id, tab_id)) = indicator_element_ids(machine) else {
            return;
        };

        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };

        let (Some(list), Some(tab)) = (
            document.get_element_by_id(&list_id),
            document.get_element_by_id(&tab_id),
        ) else {
            return;
        };

        let cleanup = observe_indicator_geometry(&tab, &list, move || {
            style.set(indicator_measurement_style(machine, platform.as_ref()));
        });

        cleanup_store.set_value(Some(cleanup));
    }

    #[cfg(not(target_arch = "wasm32"))]
    drop((machine, platform, style, cleanup_store));
}

#[cfg(not(feature = "ssr"))]
#[cfg(target_arch = "wasm32")]
fn observe_indicator_geometry(
    anchor: &web_sys::Element,
    list: &web_sys::Element,
    update: impl Fn() + 'static,
) -> Box<dyn FnOnce()> {
    let update = std::rc::Rc::new(update);

    let resize_cb = wasm_bindgen::closure::Closure::wrap(Box::new({
        let update = std::rc::Rc::clone(&update);
        move |_entries: wasm_bindgen::JsValue, _observer: web_sys::ResizeObserver| {
            update();
        }
    })
        as Box<dyn FnMut(wasm_bindgen::JsValue, web_sys::ResizeObserver)>);

    let resize_observer = web_sys::ResizeObserver::new(resize_cb.as_ref().unchecked_ref()).ok();

    if let Some(resize_observer) = resize_observer.as_ref() {
        resize_observer.observe(anchor);
        resize_observer.observe(list);
    }

    let mutation_cb = wasm_bindgen::closure::Closure::wrap(Box::new({
        let update = std::rc::Rc::clone(&update);
        move |_entries: wasm_bindgen::JsValue, _observer: web_sys::MutationObserver| {
            update();
        }
    })
        as Box<dyn FnMut(wasm_bindgen::JsValue, web_sys::MutationObserver)>);

    let mutation_observer =
        web_sys::MutationObserver::new(mutation_cb.as_ref().unchecked_ref()).ok();

    if let Some(mutation_observer) = mutation_observer.as_ref() {
        let anchor_opts = web_sys::MutationObserverInit::new();
        anchor_opts.set_attributes(true);
        anchor_opts.set_child_list(true);
        anchor_opts.set_character_data(true);
        anchor_opts.set_subtree(true);

        drop(mutation_observer.observe_with_options(anchor, &anchor_opts));

        let list_opts = web_sys::MutationObserverInit::new();
        list_opts.set_attributes(true);

        drop(mutation_observer.observe_with_options(list, &list_opts));
    }

    update();

    Box::new(move || {
        if let Some(resize_observer) = resize_observer {
            resize_observer.disconnect();
        }

        if let Some(mutation_observer) = mutation_observer {
            mutation_observer.disconnect();
        }

        drop(resize_cb);
        drop(mutation_cb);
    })
}

#[cfg(not(feature = "ssr"))]
fn clear_indicator_auto_update(
    cleanup_store: StoredValue<Option<Box<dyn FnOnce()>>, LocalStorage>,
) {
    cleanup_store.update_value(|cleanup| {
        if let Some(cleanup) = cleanup.take() {
            cleanup();
        }
    });
}

#[cfg(not(feature = "ssr"))]
fn indicator_element_ids(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
) -> Option<(String, String)> {
    machine.with_api_snapshot(|api| {
        let selected = api.selected_tab()?;

        let list_id = api.list_attrs().get(&HtmlAttr::Id).map(String::from)?;

        let tab_id = api
            .tab_attrs(selected, false)
            .get(&HtmlAttr::Id)
            .map(String::from)?;

        Some((list_id, tab_id))
    })
}

#[cfg(not(feature = "ssr"))]
fn indicator_layout_signature(api: &tabs::Api<'_>) -> String {
    let root_attrs = api.root_attrs();
    let list_attrs = api.list_attrs();
    let root_dir = root_attrs.get(&HtmlAttr::Dir).unwrap_or_default();
    let orientation = list_attrs
        .get(&HtmlAttr::Aria(AriaAttr::Orientation))
        .unwrap_or_default();

    format!("{root_dir}:{orientation}")
}

#[cfg(not(feature = "ssr"))]
fn indicator_measurement_style(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    platform: &dyn PlatformEffects,
) -> String {
    let Some((list_id, tab_id)) = indicator_element_ids(machine) else {
        return String::new();
    };

    let (Some(list), Some(tab)) = (
        platform.get_bounding_rect(&list_id),
        platform.get_bounding_rect(&tab_id),
    ) else {
        return String::new();
    };

    format!(
        "--ars-indicator-left: {}px; --ars-indicator-top: {}px; --ars-indicator-width: {}px; --ars-indicator-height: {}px;",
        tab.x - list.x,
        tab.y - list.y,
        tab.width,
        tab.height
    )
}

fn effective_direction(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    fallback: Direction,
) -> Direction {
    machine.with_api_snapshot(|api| {
        api.root_attrs()
            .get(&HtmlAttr::Dir)
            .map_or(fallback, parse_direction_token)
    })
}

fn parse_direction_token(token: &str) -> Direction {
    match token {
        "rtl" => Direction::Rtl,
        "ltr" => Direction::Ltr,
        _ => Direction::Auto,
    }
}

/// Builds a reactive `AttrMap` for a tab trigger.
///
/// The base attributes come from `Api::tab_attrs` at first render; the
/// dynamic attributes (`aria-selected`, `tabindex`, `data-ars-selected`,
/// `data-ars-focus-visible`) are replaced with `AttrValue::reactive` /
/// `AttrValue::reactive_bool` closures that re-read the live `Api` memo.
/// This keeps the `<button>` DOM node stable while letting the rendered
/// attributes update on machine state changes.
///
/// The `modality` handle is read on every memo re-evaluation so
/// `data-ars-focus-visible` reflects whether the most recent focus
/// arrived via keyboard (only). The agnostic core's `tab_attrs`
/// internally guards on `tab_key == ctx.focused_tab`, so non-focused
/// tabs never render `data-ars-focus-visible` even when modality is
/// "keyboard."
fn reactive_tab_attrs<K: TabKey>(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    key: &Key,
    modality: Arc<dyn ModalityContext>,
    modality_revision: RwSignal<u64>,
    config: StoredValue<TabsConfig<K>>,
) -> AttrMap {
    let memo = machine.derive({
        let key = key.clone();
        move |api| {
            modality_revision.track();
            let is_keyboard = !modality.had_pointer_interaction();
            api.tab_attrs(&key, is_keyboard)
        }
    });

    let mut attrs = memo.get_untracked();

    // `reorderable` is an adapter-live signal, so the rendered
    // roledescription is attached directly in `render_tab_button`.
    // Avoid also spreading the core snapshot value here, which would
    // produce duplicate static/dynamic attributes.
    attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), AttrValue::None);

    // Reactive string attributes (always present, value changes).
    for &dynamic_key in &[HtmlAttr::Aria(AriaAttr::Selected), HtmlAttr::TabIndex] {
        attrs.set(
            dynamic_key,
            AttrValue::reactive(move || {
                memo.with(|live| {
                    live.get_value(&dynamic_key)
                        .and_then(AttrValue::materialize_string)
                        .unwrap_or_default()
                })
            }),
        );
    }

    // Reactive boolean attributes (presence-based — pre-add even if absent
    // from the snapshot so the resulting Leptos attribute updates when
    // selection / focus change cause them to appear later).
    for &dynamic_key in &[
        HtmlAttr::Data("ars-selected"),
        HtmlAttr::Data("ars-focus-visible"),
    ] {
        attrs.set(
            dynamic_key,
            AttrValue::reactive_bool(move || {
                memo.with(|live| {
                    if let Some(AttrValue::Bool(b)) = live.get_value(&dynamic_key) {
                        *b
                    } else {
                        false
                    }
                })
            }),
        );
    }

    for &dynamic_key in &[
        HtmlAttr::Data("ars-disabled"),
        HtmlAttr::Aria(AriaAttr::Disabled),
    ] {
        attrs.set(
            dynamic_key,
            AttrValue::reactive_bool({
                let key = key.clone();
                move || {
                    config.with_value(|cfg| {
                        cfg.tabs_meta
                            .get()
                            .iter()
                            .any(|meta| meta.key == key && meta.disabled)
                    })
                }
            }),
        );
    }

    attrs
}

/// Builds a reactive `AttrMap` for a tab panel.
///
/// The dynamic attributes are `hidden` (panels are hidden when their tab
/// is not selected) and `data-ars-selected` (presence-only).
fn reactive_panel_attrs(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    key: &Key,
) -> AttrMap {
    let memo = machine.derive({
        let key = key.clone();
        move |api| api.panel_attrs(&key, None)
    });

    let mut attrs = memo.get_untracked();

    for &dynamic_key in &[HtmlAttr::Hidden, HtmlAttr::Data("ars-selected")] {
        attrs.set(
            dynamic_key,
            AttrValue::reactive_bool(move || {
                memo.with(|live| {
                    if let Some(AttrValue::Bool(b)) = live.get_value(&dynamic_key) {
                        *b
                    } else {
                        false
                    }
                })
            }),
        );
    }

    attrs
}

fn aggregate_disabled_keys_from_tabs<K: TabKey>(
    tabs: Field<Vec<Tab<K>>>,
    extra: &BTreeSet<K>,
) -> BTreeSet<Key> {
    tabs.read()
        .iter()
        .filter(|tab| tab.disabled || extra.contains(&tab.key))
        .map(|tab| tab.key.into_key())
        .collect()
}

/// Builds the disabled-keys set from the current tab store without subscribing.
fn aggregate_disabled_keys_from_tabs_untracked<K: TabKey>(
    tabs: Field<Vec<Tab<K>>>,
    extra: &BTreeSet<K>,
) -> BTreeSet<Key> {
    tabs.read_untracked()
        .iter()
        .filter(|tab| tab.disabled || extra.contains(&tab.key))
        .map(|tab| tab.key.into_key())
        .collect()
}

fn keyboard_event_data(event: &web_sys::KeyboardEvent) -> KeyboardEventData {
    let (key, character) = leptos_key_to_keyboard_key(event);

    KeyboardEventData {
        key,
        character,
        code: event.code(),
        shift_key: event.shift_key(),
        ctrl_key: event.ctrl_key(),
        alt_key: event.alt_key(),
        meta_key: event.meta_key(),
        repeat: event.repeat(),
        is_composing: event.is_composing(),
    }
}

const fn arrow_pair_for(orientation: Orientation, dir: Direction) -> (KeyboardKey, KeyboardKey) {
    match (orientation, dir) {
        (Orientation::Horizontal, Direction::Rtl) => {
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        }

        (Orientation::Horizontal, Direction::Ltr | Direction::Auto) => {
            (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
        }

        (Orientation::Vertical, _) => (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown),
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    type TestTab = Tab<&'static str>;

    fn key(value: &str) -> Key {
        Key::str(value)
    }

    fn tab(key: &'static str, label: &'static str) -> TestTab {
        Tab::new_static(key, label, || view! { "Panel" })
    }

    #[test]
    fn tab_builders_debug_and_source_conversions_cover_public_api() {
        let custom = Tab::new_with_label(
            "custom",
            "Custom label",
            || view! { <strong>"Custom"</strong> },
            || view! { "Custom panel" },
        )
        .trigger(|| view! { <em>"Overridden"</em> })
        .disabled(true)
        .closable(true)
        .link(SafeUrl::from_static("/custom"));

        assert_eq!(custom.key, "custom");
        assert_eq!(custom.label_text.resolve(), "Custom label");
        assert!(custom.disabled);
        assert!(custom.closable);
        assert_eq!(
            custom.link.as_ref().map(ToString::to_string),
            Some("/custom".to_owned())
        );

        let label_debug = format!("{:?}", custom.label_text);

        assert!(label_debug.contains("Custom label"));

        let tab_debug = format!("{custom:?}");

        assert!(tab_debug.contains("Tab"));
        assert!(tab_debug.contains("custom"));
        assert!(tab_debug.contains("Custom label"));

        let from_vec = TabsSource::from(vec![tab("first", "First")]);

        assert!(matches!(from_vec, TabsSource::Owned(_)));
        assert!(format!("{from_vec:?}").contains("TabsSource::Owned"));

        let from_array = TabsSource::from([tab("second", "Second")]);

        assert!(matches!(from_array, TabsSource::Owned(_)));

        let owner = Owner::new();
        owner.with(|| {
            let field = Field::from(Store::new(vec![tab("third", "Third")]));
            let source = TabsSource::from(field);

            assert!(matches!(source, TabsSource::Field(_)));
            assert!(format!("{source:?}").contains("TabsSource::Field"));
        });
    }

    #[test]
    fn normalize_tabs_options_applies_defaults_and_overrides() {
        let defaults = normalize_tabs_options::<&'static str>(
            None, None, None, None, None, None, None, None, None,
        );

        assert_eq!(
            defaults.orientation.get_untracked(),
            Orientation::Horizontal
        );
        assert_eq!(
            defaults.activation_mode.get_untracked(),
            ActivationMode::Automatic
        );
        assert_eq!(defaults.dir.get_untracked(), Direction::Ltr);
        assert!(defaults.loop_focus.get_untracked());
        assert!(!defaults.disallow_empty_selection.get_untracked());
        assert!(!defaults.lazy_mount.get_untracked());
        assert!(!defaults.unmount_on_exit.get_untracked());
        assert!(defaults.disabled_keys.get_untracked().is_empty());
        assert!(!defaults.reorderable.get_untracked());

        let disabled_keys = BTreeSet::from(["second"]);

        let overrides = normalize_tabs_options(
            Some(Signal::from(Orientation::Vertical)),
            Some(Signal::from(ActivationMode::Manual)),
            Some(Signal::from(Direction::Rtl)),
            Some(Signal::from(false)),
            Some(Signal::from(true)),
            Some(Signal::from(true)),
            Some(Signal::from(true)),
            Some(Signal::from(disabled_keys.clone())),
            Some(Signal::from(true)),
        );

        assert_eq!(overrides.orientation.get_untracked(), Orientation::Vertical);
        assert_eq!(
            overrides.activation_mode.get_untracked(),
            ActivationMode::Manual
        );
        assert_eq!(overrides.dir.get_untracked(), Direction::Rtl);
        assert!(!overrides.loop_focus.get_untracked());
        assert!(overrides.disallow_empty_selection.get_untracked());
        assert!(overrides.lazy_mount.get_untracked());
        assert!(overrides.unmount_on_exit.get_untracked());
        assert_eq!(overrides.disabled_keys.get_untracked(), disabled_keys);
        assert!(overrides.reorderable.get_untracked());
    }

    #[test]
    fn tabs_meta_memo_merges_row_and_prop_disabled_state() {
        let owner = Owner::new();

        owner.with(|| {
            let field = Field::from(Store::new(vec![
                tab("first", "First").closable(true),
                tab("second", "Second").disabled(true),
                tab("third", "Third"),
            ]));

            let meta =
                tabs_meta_memo(field, Signal::from(BTreeSet::from(["third"]))).get_untracked();

            assert_eq!(
                meta.iter()
                    .map(|tab| {
                        (
                            tab.typed_key,
                            tab.label_text.as_str(),
                            tab.closable,
                            tab.disabled,
                        )
                    })
                    .collect::<Vec<_>>(),
                vec![
                    ("first", "First", true, false),
                    ("second", "Second", false, true),
                    ("third", "Third", false, true),
                ]
            );
        });
    }

    #[test]
    fn tabs_props_signal_tracks_controlled_value_and_disabled_keys() {
        let owner = Owner::new();

        owner.with(|| {
            let selected = RwSignal::new(Some("first"));

            let field = Field::from(Store::new(vec![
                tab("first", "First"),
                tab("second", "Second").disabled(true),
            ]));

            let options = normalize_tabs_options(
                Some(Signal::from(Orientation::Vertical)),
                Some(Signal::from(ActivationMode::Manual)),
                Some(Signal::from(Direction::Rtl)),
                Some(Signal::from(false)),
                Some(Signal::from(true)),
                Some(Signal::from(true)),
                Some(Signal::from(true)),
                Some(Signal::from(BTreeSet::from(["first"]))),
                Some(Signal::from(true)),
            );

            let props_signal = tabs_props_signal(
                "tabs-test",
                "first",
                Some(Signal::derive(move || selected.get())),
                field,
                &options,
            );

            let props = props_signal.get_untracked();

            assert_eq!(props.id, "tabs-test");
            assert_eq!(props.value, Some(Some(key("first"))));
            assert_eq!(props.default_value, Some(key("first")));
            assert_eq!(props.orientation, Orientation::Vertical);
            assert_eq!(props.activation_mode, ActivationMode::Manual);
            assert_eq!(props.dir, Direction::Rtl);
            assert!(!props.loop_focus);
            assert!(props.disallow_empty_selection);
            assert!(props.lazy_mount);
            assert!(props.unmount_on_exit);
            assert!(props.reorderable);
            assert_eq!(
                props.disabled_keys,
                BTreeSet::from([key("first"), key("second")])
            );

            selected.set(Some("second"));

            assert_eq!(
                props_signal.get_untracked().value,
                Some(Some(key("second")))
            );
        });
    }

    #[test]
    fn aggregate_disabled_keys_uses_row_and_extra_state() {
        let owner = Owner::new();

        owner.with(|| {
            let field = Field::from(Store::new(vec![
                tab("first", "First"),
                tab("second", "Second").disabled(true),
                tab("third", "Third"),
            ]));

            let extra = BTreeSet::from(["third"]);

            assert_eq!(
                aggregate_disabled_keys_from_tabs(field, &extra),
                BTreeSet::from([key("second"), key("third")])
            );
            assert_eq!(
                aggregate_disabled_keys_from_tabs_untracked(field, &extra),
                BTreeSet::from([key("second"), key("third")])
            );
        });
    }

    #[test]
    fn should_render_panel_body_mounts_newly_selected_lazy_panel_immediately() {
        assert!(should_render_panel_body(true, false, true, false));
        assert!(should_render_panel_body(false, true, true, false));
        assert!(!should_render_panel_body(false, false, true, false));
        assert!(should_render_panel_body(true, false, true, true));
        assert!(!should_render_panel_body(false, true, true, true));
        assert!(should_render_panel_body(false, false, false, false));
    }

    #[test]
    fn pointer_type_mapping_matches_browser_tokens() {
        assert_eq!(pointer_type_from_leptos("mouse"), PointerType::Mouse);
        assert_eq!(pointer_type_from_leptos("touch"), PointerType::Touch);
        assert_eq!(pointer_type_from_leptos("pen"), PointerType::Pen);
        assert_eq!(pointer_type_from_leptos(""), PointerType::Virtual);
        assert_eq!(pointer_type_from_leptos("unknown"), PointerType::Virtual);
    }

    #[test]
    fn direction_and_arrow_helpers_follow_orientation_and_dir() {
        assert_eq!(parse_direction_token("ltr"), Direction::Ltr);
        assert_eq!(parse_direction_token("rtl"), Direction::Rtl);
        assert_eq!(parse_direction_token("sideways"), Direction::Auto);
        assert_eq!(
            arrow_pair_for(Orientation::Horizontal, Direction::Ltr),
            (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
        );
        assert_eq!(
            arrow_pair_for(Orientation::Horizontal, Direction::Rtl),
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        );
        assert_eq!(
            arrow_pair_for(Orientation::Vertical, Direction::Rtl),
            (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown)
        );
    }
}
