//! Dioxus Tabs adapter.
//!
//! Renders the framework-agnostic [`ars_components::navigation::tabs`]
//! machine as a single compound `<Tabs>` Dioxus component. The adapter
//! owns the full anatomy (Root, List, Tab×N, Indicator, Panel×N, optional
//! CloseTrigger, optional reorder live region), drives DOM focus on the
//! [`Effect::FocusFocusedTab`] intent emitted by the core, and surfaces
//! tab data through a per-row [`Tab`] value.
//!
//! See `spec/dioxus-components/navigation/tabs.md` for the full adapter
//! contract.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Debug},
    rc::Rc,
    sync::Arc,
};

use ars_collections::{Key, TabKey};
use ars_components::navigation::tabs::{
    self, TabMeta, disabled_keys_from_meta, drag_reorder_plan, registrations_from_meta,
    typed_key_for_key,
};
// Re-export the agnostic core surface so consumers reach it through the
// adapter namespace. `Effect` is intentionally NOT re-exported because it
// would shadow Dioxus reactive `Effect` types in consumer code.
//
// `Event` is re-exported here so `tabs::Event::SelectTab` is reachable
// through the adapter namespace, and within this file `Event` resolves
// to the agnostic core enum. Type signatures that need Dioxus's DOM
// event wrapper use the fully-qualified `dioxus::prelude::Event<T>`
// form to avoid the local-name collision.
pub use ars_components::navigation::tabs::{
    ActivationMode, CloseTabLabelFn, Context, Event, Messages, Part, Props, ReorderAnnounceLabelFn,
    ReorderEvent, State,
};
use ars_core::{
    AriaAttr, Direction, HtmlAttr, KeyModifiers, ModalityContext, Orientation, PlatformEffects,
    PointerType, SafeUrl,
};
use ars_i18n::Translate;
use ars_interactions::{KeyboardEventData, KeyboardKey};
use dioxus::{events::MountedData, prelude::*};
pub use dioxus_stores::ReadStore;
use dioxus_stores::{Store, use_store};
#[cfg(all(feature = "web", target_arch = "wasm32"))]
use wasm_bindgen::JsCast;

use crate::{
    attrs::attr_map_to_dioxus_inline_attrs,
    event_mapping::dioxus_key_to_keyboard_key,
    id::use_stable_id,
    platform::{DioxusPlatform, use_platform},
    provider::{use_modality_context, use_platform_effects},
    use_machine::use_machine_with_reactive_props,
};

// ────────────────────────────────────────────────────────────────────
// Tab
// ────────────────────────────────────────────────────────────────────

/// Semantic text source for a Dioxus [`Tab`] trigger.
///
/// The adapter uses this label for accessible close-button names and
/// reorder announcements. The default trigger rendering also uses this
/// label unless [`Tab::trigger`] supplies richer visual content.
#[derive(Clone)]
pub enum TabLabel {
    /// A fixed semantic label.
    Static(String),

    /// A provider-backed label resolved during render.
    Translated(Arc<dyn Fn() -> String>),
}

impl TabLabel {
    /// Builds a static label.
    #[must_use]
    pub fn static_text(text: impl Into<String>) -> Self {
        Self::Static(text.into())
    }

    /// Builds a provider-backed translated label for the current render.
    #[must_use]
    pub fn translated<T: Clone + Translate + 'static>(message: T) -> Self {
        Self::Translated(Arc::new(move || crate::provider::t(message.clone())))
    }

    /// Resolves the label text for this render.
    #[must_use]
    pub fn resolve(&self) -> String {
        match self {
            Self::Static(text) => text.clone(),
            Self::Translated(resolve) => resolve(),
        }
    }
}

impl Debug for TabLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TabLabel").field(&self.resolve()).finish()
    }
}

impl PartialEq for TabLabel {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Static(left), Self::Static(right)) => left == right,
            (Self::Translated(left), Self::Translated(right)) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }
}

impl Eq for TabLabel {}

#[component]
fn TabLabelText(label_text: TabLabel) -> Element {
    let text = label_text.resolve();

    rsx! { "{text}" }
}

/// Per-tab render data consumed by the [`Tabs`] component.
///
/// The agnostic core does not own per-tab labels (see
/// `spec/components/navigation/tabs.md` §5.1) — it tracks only registration
/// keys and the closable flag. Adapters introduce a `Tab` value to
/// carry the rendered button/panel content, plus optional consumer
/// affordances (per-row `disabled` flag, optional `link` for tabs that
/// render as `<a>` instead of `<button>`).
#[derive(Clone, PartialEq)]
pub struct Tab<K: TabKey> {
    /// Stable identifier for the tab. Used for ARIA wiring, registration,
    /// and DOM-id derivation through [`ars_core::ComponentIds`].
    pub key: K,

    /// Visible label content rendered inside the tab trigger.
    pub label: Element,

    /// Semantic label text source. Used for the close-button accessible
    /// name (`Messages::close_tab_label`) and the reorder announcement
    /// (`Messages::reorder_announce_label`).
    pub label_text: TabLabel,

    /// Panel body rendered inside the matching tabpanel.
    pub panel: Element,

    /// When `true` this row is disabled. Merged with the `Tabs` component's
    /// `disabled_keys` prop before being threaded to the core machine.
    pub disabled: bool,

    /// When `true` the adapter renders a close button inside the tab and
    /// the core forwards `Delete` / `Backspace` keystrokes as
    /// [`tabs::Event::CloseTab`].
    pub closable: bool,

    /// When `Some`, the tab renders as `<a href=…>` instead of the
    /// default `<button>`.
    pub link: Option<SafeUrl>,
}

impl<K: TabKey> Tab<K> {
    /// Builds a non-closable, non-disabled tab row with a static text label.
    #[must_use]
    pub fn new_static(key: K, label_text: impl Into<String>, panel: Element) -> Self {
        let label_text = TabLabel::static_text(label_text);

        Self {
            key,
            label: rsx! {
                TabLabelText { label_text: label_text.clone() }
            },
            label_text,
            panel,
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
        label_text: impl Into<String>,
        label: Element,
        panel: Element,
    ) -> Self {
        Self {
            key,
            label,
            label_text: TabLabel::static_text(label_text),
            panel,
            disabled: false,
            closable: false,
            link: None,
        }
    }

    /// Replaces the visible trigger content while preserving this tab's
    /// semantic label for accessibility and announcements.
    #[must_use]
    pub fn trigger(mut self, label: Element) -> Self {
        self.label = label;

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
    K: TabKey + Translate,
{
    /// Builds a non-closable, non-disabled tab row whose semantic label
    /// and default visible trigger are translated from the key itself.
    #[must_use]
    pub fn new(key: K, panel: Element) -> Self {
        let label_text = TabLabel::translated(key);

        Self {
            key,
            label: rsx! {
                TabLabelText { label_text: label_text.clone() }
            },
            label_text,
            panel,
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
            .field("label_text", &self.label_text.resolve())
            .field("disabled", &self.disabled)
            .field("closable", &self.closable)
            .field("link", &self.link)
            .finish_non_exhaustive()
    }
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Props for the Dioxus [`Tabs`] component.
#[derive(Props, Clone, PartialEq)]
pub struct TabsProps<K: TabKey> {
    /// Controlled selected tab key. The outer `Option` distinguishes
    /// controlled-vs-uncontrolled mode (fixed at mount); the inner
    /// `Option<K>` carries the actual selection so a controlled
    /// consumer can express "no tab selected" without flipping out
    /// of controlled mode.
    #[props(optional)]
    pub value: Option<Option<K>>,

    /// Initial selected tab key in uncontrolled mode.
    #[props(into)]
    pub default_value: K,

    /// Per-tab render rows in DOM order. Pass inline rows (`Vec<Tab<K>>`
    /// or `[Tab<K>; N]`) for adapter-owned tabs, or a reactive
    /// [`ReadStore<Vec<Tab<K>>>`] from the consumer's
    /// `ars_dioxus::dioxus_stores::Store` for a consumer-owned dynamic
    /// tab list. Both sources flow through the same reactive store
    /// machinery, so pushes, removes, reorders, and per-row `closable`
    /// flag swaps re-dispatch [`Event::SetTabs`] without remounting the
    /// `Tabs` component.
    #[props(into)]
    pub tabs: TabsSource<K>,

    /// Layout orientation. Defaults to `Horizontal`.
    #[props(default)]
    pub orientation: Orientation,

    /// How keyboard focus interacts with selection.
    #[props(default)]
    pub activation_mode: ActivationMode,

    /// Text direction.
    #[props(default = Direction::Ltr)]
    pub dir: Direction,

    /// When `true`, arrow-key focus wraps from last to first.
    #[props(default = true)]
    pub loop_focus: bool,

    /// When `true` the only remaining tab cannot be closed.
    #[props(default = false)]
    pub disallow_empty_selection: bool,

    /// When `true`, panels are not rendered until first selected.
    #[props(default = false)]
    pub lazy_mount: bool,

    /// When `true`, panels are removed from the DOM when their tab is
    /// deselected.
    #[props(default = false)]
    pub unmount_on_exit: bool,

    /// Set of disabled tab keys merged with each row's `Tab::disabled`.
    #[props(default)]
    pub disabled_keys: BTreeSet<K>,

    /// When `true`, Ctrl+Arrow reorders tabs.
    #[props(default = false)]
    pub reorderable: bool,

    /// Called after a user action commits a new selected key.
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<K>>>,

    /// Called when a close trigger or close key requests closing a tab.
    #[props(optional)]
    pub on_close_tab: Option<EventHandler<K>>,

    /// Called before a reorder request is emitted to the core. Return
    /// `false` to veto the reorder and suppress its live announcement.
    #[props(optional)]
    pub on_reorder: Option<Callback<ReorderEvent<K>, bool>>,

    /// Optional adapter-user content rendered inside Root after panels.
    #[props(default)]
    pub children: Element,
}

impl<K: TabKey> Debug for TabsProps<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = self.value.map(|selected| selected.map(TabKey::into_key));

        let default_value = self.default_value.into_key();

        let disabled_keys = self
            .disabled_keys
            .iter()
            .copied()
            .map(TabKey::into_key)
            .collect::<Vec<_>>();

        f.debug_struct("TabsProps")
            .field("value", &value)
            .field("default_value", &default_value)
            .field("tabs", &self.tabs)
            .field("orientation", &self.orientation)
            .field("activation_mode", &self.activation_mode)
            .field("dir", &self.dir)
            .field("loop_focus", &self.loop_focus)
            .field("disallow_empty_selection", &self.disallow_empty_selection)
            .field("lazy_mount", &self.lazy_mount)
            .field("unmount_on_exit", &self.unmount_on_exit)
            .field("disabled_keys", &disabled_keys)
            .field("reorderable", &self.reorderable)
            .field("on_value_change", &self.on_value_change.is_some())
            .field("on_close_tab", &self.on_close_tab.is_some())
            .field("on_reorder", &self.on_reorder.is_some())
            .finish_non_exhaustive()
    }
}

/// Source of tab rows for the Dioxus [`Tabs`] component.
///
/// Consumers can pass inline rows (`Vec<Tab<K>>` or `[Tab<K>; N]`) for
/// adapter-owned tabs, or pass a `dioxus_stores` read store when they own
/// a dynamic tab list. Owned tabs still use an internal store, so close
/// and reorder interactions work without any consumer store setup.
#[derive(Clone, PartialEq)]
pub enum TabsSource<K: TabKey> {
    /// Adapter-owned tabs initialized from inline rows.
    Owned(Vec<Tab<K>>),

    /// Consumer-owned reactive tab store.
    Store(ReadStore<Vec<Tab<K>>>),
}

impl<K: TabKey> Debug for TabsSource<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Owned(tabs) => f.debug_tuple("TabsSource::Owned").field(tabs).finish(),
            Self::Store(_) => f.debug_tuple("TabsSource::Store").finish_non_exhaustive(),
        }
    }
}

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

impl<K: TabKey> From<ReadStore<Vec<Tab<K>>>> for TabsSource<K> {
    fn from(value: ReadStore<Vec<Tab<K>>>) -> Self {
        Self::Store(value)
    }
}

impl<K: TabKey> From<Store<Vec<Tab<K>>>> for TabsSource<K> {
    fn from(value: Store<Vec<Tab<K>>>) -> Self {
        Self::Store(ReadStore::from(value))
    }
}

struct TabsStoreSetup<K: TabKey> {
    tabs: Vec<Tab<K>>,
    owned_tabs_store: Option<Store<Vec<Tab<K>>>>,
}

struct TabsRenderSnapshot<K: TabKey> {
    config: TabsConfig<K>,
    registrations: Vec<tabs::TabRegistration>,
    core_props: Props,
}

// ────────────────────────────────────────────────────────────────────
// Tabs component
// ────────────────────────────────────────────────────────────────────

/// Renders the agnostic Tabs machine as a Dioxus compound component.
///
/// The single component owns the full anatomy (Root, List, Tab×N,
/// Indicator, Panel×N, optional `CloseTrigger`, optional reorder live
/// region). Per-tab content comes through the [`Tab`] rows in `tabs`;
/// `children` is rendered inside the root after the panels for any
/// adapter-side decoration consumers want next to the tablist.
#[component]
pub fn Tabs<K: TabKey>(props: TabsProps<K>) -> Element {
    let TabsProps {
        value,
        default_value,
        tabs,
        orientation,
        activation_mode,
        dir,
        loop_focus,
        disallow_empty_selection,
        lazy_mount,
        unmount_on_exit,
        disabled_keys,
        reorderable,
        on_value_change,
        on_close_tab,
        on_reorder,
        children,
    } = props;

    let id = use_stable_id("tabs");

    let TabsStoreSetup {
        tabs,
        owned_tabs_store,
    } = use_tabs_store(&tabs);

    let modality = use_modality_context();
    let platform = use_platform_effects();
    let dioxus_platform = use_platform();

    let TabsRenderSnapshot {
        config,
        registrations,
        core_props,
    } = build_tabs_render_snapshot(
        &id,
        value,
        default_value,
        &tabs,
        orientation,
        activation_mode,
        dir,
        loop_focus,
        disallow_empty_selection,
        lazy_mount,
        unmount_on_exit,
        &disabled_keys,
        reorderable,
    );

    let machine = use_tabs_machine(core_props);

    use_tabs_registration(machine, registrations);

    let tab_nodes = use_signal(BTreeMap::<Key, Rc<MountedData>>::new);

    let send_with_focus_pulse =
        use_focus_dispatch(machine, &id, Arc::clone(&dioxus_platform), tab_nodes);

    let reorder_status = use_signal(String::new);
    let drag_source = use_signal(|| None::<Key>);
    let modality_revision = use_signal(|| 0_u64);

    let ever_selected = use_lazy_mount_tracking(machine);

    let root_attrs = tabs_root_attrs(machine);
    let list_attrs = tabs_list_attrs(machine, &config.tabs_meta);

    use_auto_direction_sync(machine, dir, &platform);

    let tab_indicator_attrs = tabs_indicator_attrs(machine);

    let mut indicator_revision = use_signal(|| 0_u64);

    let mut indicator_signature = use_signal(String::new);

    sync_indicator_signature(
        &mut indicator_signature,
        &mut indicator_revision,
        &config.tabs_meta,
    );

    let indicator_style = use_indicator_style(machine, &platform, indicator_revision);

    rsx! {
        div {..root_attrs(),
            div {..list_attrs,
                {
                    tabs
                        .iter()
                        .map(|tab| {
                            render_tab_button(
                                tab.clone(),
                                machine,
                                send_with_focus_pulse,
                                &config,
                                reorder_status,
                                drag_source,
                                modality_revision,
                                tab_nodes,
                                &modality,
                                on_value_change,
                                on_close_tab,
                                on_reorder,
                                owned_tabs_store,
                                disallow_empty_selection,
                                indicator_revision,
                            )
                        })
                }
                span { style: "{indicator_style}", ..tab_indicator_attrs() }
            }
            {
                tabs
                    .iter()
                    .map(|tab| {
                        render_tab_panel(
                            tab.clone(),
                            machine,
                            lazy_mount,
                            unmount_on_exit,
                            ever_selected,
                        )
                    })
            }
            {children}
            if reorderable {
                div {
                    "aria-live": "polite",
                    "aria-atomic": "true",
                    style: "position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0;",
                    "{reorder_status}"
                }
            }
        }
    }
}

fn use_tabs_store<K: TabKey>(tabs: &TabsSource<K>) -> TabsStoreSetup<K> {
    let owned_initial_tabs = match tabs {
        TabsSource::Owned(tabs) => tabs.clone(),
        TabsSource::Store(_) => Vec::new(),
    };

    let mut owned_tabs_store = use_store(move || owned_initial_tabs);
    let mut owned_prop_keys = use_signal(|| {
        if let TabsSource::Owned(tabs) = tabs {
            tabs.iter().map(|tab| tab.key).collect()
        } else {
            Vec::new()
        }
    });

    let (tabs, owned_tabs_store) = match tabs {
        TabsSource::Owned(latest_tabs) => {
            let previous_prop_keys = owned_prop_keys.peek().clone();
            let reconciled =
                owned_tabs_for_render(&owned_tabs_store.read(), latest_tabs, &previous_prop_keys);
            let latest_prop_keys = latest_tabs.iter().map(|tab| tab.key).collect::<Vec<_>>();

            if owned_tabs_changed(&owned_tabs_store.read(), &reconciled) {
                owned_tabs_store.write().clone_from(&reconciled);
            }

            if *owned_prop_keys.peek() != latest_prop_keys {
                owned_prop_keys.set(latest_prop_keys);
            }

            (reconciled, Some(owned_tabs_store))
        }

        TabsSource::Store(tabs) => (tabs.read().clone(), None),
    };

    TabsStoreSetup {
        tabs,
        owned_tabs_store,
    }
}

fn owned_tabs_for_render<K: TabKey>(
    current_tabs: &[Tab<K>],
    latest_tabs: &[Tab<K>],
    previous_prop_keys: &[K],
) -> Vec<Tab<K>> {
    let latest_prop_keys = latest_tabs.iter().map(|tab| tab.key).collect::<Vec<_>>();

    let current_keys = current_tabs
        .iter()
        .map(|tab| tab.key)
        .collect::<BTreeSet<_>>();

    let previous_prop_keys_set = previous_prop_keys.iter().copied().collect::<BTreeSet<_>>();

    if latest_prop_keys != previous_prop_keys {
        return latest_tabs
            .iter()
            .filter(|latest| {
                current_keys.contains(&latest.key) || !previous_prop_keys_set.contains(&latest.key)
            })
            .cloned()
            .collect();
    }

    let latest_keys = latest_tabs
        .iter()
        .map(|tab| tab.key)
        .collect::<BTreeSet<_>>();

    let mut rendered = current_tabs
        .iter()
        .filter(|current| latest_keys.contains(&current.key))
        .filter_map(|current| {
            latest_tabs
                .iter()
                .find(|latest| latest.key == current.key)
                .cloned()
        })
        .collect::<Vec<_>>();

    rendered.extend(
        latest_tabs
            .iter()
            .filter(|latest| {
                !current_keys.contains(&latest.key) && !previous_prop_keys_set.contains(&latest.key)
            })
            .cloned(),
    );

    rendered
}

fn owned_tabs_changed<K: TabKey>(current_tabs: &[Tab<K>], next_tabs: &[Tab<K>]) -> bool {
    current_tabs != next_tabs
}

#[expect(
    clippy::too_many_arguments,
    reason = "builds the per-render Dioxus Tabs snapshot from the public prop surface"
)]
fn build_tabs_render_snapshot<K: TabKey>(
    id: &str,
    value: Option<Option<K>>,
    default_value: K,
    tabs: &[Tab<K>],
    orientation: Orientation,
    activation_mode: ActivationMode,
    dir: Direction,
    loop_focus: bool,
    disallow_empty_selection: bool,
    lazy_mount: bool,
    unmount_on_exit: bool,
    disabled_keys: &BTreeSet<K>,
    reorderable: bool,
) -> TabsRenderSnapshot<K> {
    let tabs_meta = tabs_meta_snapshot(tabs, disabled_keys);

    let registrations = registrations_from_meta(&tabs_meta);

    let config = TabsConfig {
        orientation,
        dir,
        activation_mode,
        reorderable,
        tabs_meta: tabs_meta.clone(),
    };

    let core_props = tabs_core_props(
        id,
        value,
        default_value,
        orientation,
        activation_mode,
        dir,
        loop_focus,
        disallow_empty_selection,
        lazy_mount,
        unmount_on_exit,
        disabled_keys_from_meta(&tabs_meta),
        reorderable,
    );

    TabsRenderSnapshot {
        config,
        registrations,
        core_props,
    }
}

fn tabs_meta_snapshot<K: TabKey>(tabs: &[Tab<K>], disabled_keys: &BTreeSet<K>) -> Vec<TabMeta<K>> {
    tabs.iter()
        .map(|tab| {
            TabMeta::new(
                tab.key,
                tab.label_text.resolve(),
                tab.closable,
                tab.disabled || disabled_keys.contains(&tab.key),
            )
        })
        .collect()
}

#[expect(
    clippy::too_many_arguments,
    reason = "threads normalized Tabs props into the agnostic core Props builder"
)]
fn tabs_core_props<K: TabKey>(
    id: &str,
    value: Option<Option<K>>,
    default_value: K,
    orientation: Orientation,
    activation_mode: ActivationMode,
    dir: Direction,
    loop_focus: bool,
    disallow_empty_selection: bool,
    lazy_mount: bool,
    unmount_on_exit: bool,
    disabled_keys: BTreeSet<Key>,
    reorderable: bool,
) -> Props {
    let mut props = Props::new()
        .id(id)
        .default_value(Some(default_value.into_key()))
        .orientation(orientation)
        .activation_mode(activation_mode)
        .dir(dir)
        .loop_focus(loop_focus)
        .disallow_empty_selection(disallow_empty_selection)
        .lazy_mount(lazy_mount)
        .unmount_on_exit(unmount_on_exit)
        .disabled_keys(disabled_keys)
        .reorderable(reorderable);

    if let Some(controlled_value) = value {
        props = props.value(Some(controlled_value.map(TabKey::into_key)));
    }

    props
}

fn use_tabs_machine(core_props: Props) -> crate::use_machine::UseMachineReturn<tabs::Machine> {
    let mut props_signal = use_signal(|| core_props.clone());

    // `use_signal` initializes only once for this component instance. On
    // later renders, `core_props` may reflect new controlled value,
    // disabled/reorder flags, or tab-derived disabled keys while the signal
    // still holds the previous render's props. Keep the signal current before
    // handing it to the reactive-props hook so the existing machine service
    // observes prop changes in the same render pass.
    if *props_signal.peek() != core_props {
        props_signal.set(core_props);
    }

    use_machine_with_reactive_props::<tabs::Machine>(props_signal)
}

fn use_tabs_registration(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    registrations: Vec<tabs::TabRegistration>,
) {
    let mut prev_registrations = use_signal(Vec::<tabs::TabRegistration>::new);

    if *prev_registrations.peek() != registrations {
        machine.send.call(Event::SetTabs(registrations.clone()));

        prev_registrations.set(registrations);
    }
}

fn use_focus_dispatch(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    id: &str,
    dioxus_platform: Arc<dyn DioxusPlatform>,
    tab_nodes: Signal<BTreeMap<Key, Rc<MountedData>>>,
) -> EventHandler<Event> {
    let mut focus_pulse = use_signal(|| 0_u64);

    let focused_tab_key = machine.derive(|api| api.focused_tab().cloned());

    let id_for_focus = id.to_owned();

    use_effect(move || {
        if focus_pulse() == 0 {
            return;
        }

        let Some(key) = focused_tab_key().clone() else {
            return;
        };

        let element_id = format!("{id_for_focus}-tab-{key}");

        focus_tab_element(tab_nodes, &key, &element_id, Arc::clone(&dioxus_platform));
    });

    EventHandler::new(move |event: Event| {
        let needs_focus = matches!(
            event,
            Event::FocusNext | Event::FocusPrev | Event::FocusFirst | Event::FocusLast
        );

        machine.send.call(event);

        if needs_focus {
            *focus_pulse.write() += 1;
        }
    })
}

fn use_lazy_mount_tracking(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
) -> Signal<BTreeSet<Key>> {
    let initial_selection = machine
        .with_api_snapshot(|api| api.selected_tab().cloned())
        .map(|key| BTreeSet::from([key]))
        .unwrap_or_default();

    let mut ever_selected = use_signal(|| initial_selection);

    let selected_memo = machine.derive(|api| api.selected_tab().cloned());

    use_effect(move || {
        if let Some(key) = selected_memo() {
            ever_selected.write().insert(key);
        }
    });

    ever_selected
}

fn tabs_root_attrs(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
) -> Memo<Vec<Attribute>> {
    machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.root_attrs()))
}

fn tabs_list_attrs<K: TabKey>(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    tabs_meta: &[TabMeta<K>],
) -> Vec<Attribute> {
    machine.with_api_snapshot(|api| {
        let mut attrs = api.list_attrs();

        let owns = tabs_meta
            .iter()
            .filter_map(|tab| {
                api.tab_attrs(&tab.key, false)
                    .get(&HtmlAttr::Id)
                    .map(String::from)
            })
            .collect::<Vec<_>>()
            .join(" ");

        if !owns.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::Owns), owns);
        }

        attr_map_to_dioxus_inline_attrs(attrs)
    })
}

fn use_auto_direction_sync(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    dir: Direction,
    platform: &Arc<dyn PlatformEffects>,
) {
    let list_id = (dir == Direction::Auto)
        .then(|| {
            machine.with_api_snapshot(|api| api.list_attrs().get(&HtmlAttr::Id).map(String::from))
        })
        .flatten();

    let platform = Arc::clone(platform);

    use_effect(move || {
        if let Some(list_id) = &list_id {
            let resolved = Direction::from(platform.resolved_direction(list_id));

            machine.send.call(Event::SetDirection(resolved));
        }
    });
}

fn tabs_indicator_attrs(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
) -> Memo<Vec<Attribute>> {
    machine.derive(|api| attr_map_to_dioxus_inline_attrs(api.tab_indicator_attrs()))
}

fn use_indicator_style(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    platform: &Arc<dyn PlatformEffects>,
    indicator_revision: Signal<u64>,
) -> Signal<String> {
    let mut indicator_style = use_signal(String::new);

    let selected_memo = machine.derive(|api| api.selected_tab().cloned());

    use_effect({
        let platform = Arc::clone(platform);
        move || {
            // Just to trigger the effect when selection changes; the actual measurement happens
            drop(selected_memo());
            let _ = indicator_revision();

            indicator_style.set(indicator_measurement_style(machine, platform.as_ref()));
        }
    });

    indicator_style
}

// ────────────────────────────────────────────────────────────────────
// Tab button rendering
// ────────────────────────────────────────────────────────────────────

#[expect(
    unused_qualifications,
    reason = "rsx! macro expansion currently reports event-handler attribute names as redundant qualifications"
)]
#[expect(
    clippy::too_many_arguments,
    reason = "tab rendering needs machine, event callbacks, and reactive store handles"
)]
#[expect(
    clippy::too_many_lines,
    reason = "Dioxus tab rendering keeps related DOM event closures in one helper"
)]
fn render_tab_button<K: TabKey>(
    tab: Tab<K>,
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: EventHandler<Event>,
    config: &TabsConfig<K>,
    reorder_status: Signal<String>,
    drag_source: Signal<Option<Key>>,
    mut modality_revision: Signal<u64>,
    mut tab_nodes: Signal<BTreeMap<Key, Rc<MountedData>>>,
    modality: &Arc<dyn ModalityContext>,
    on_value_change: Option<EventHandler<Option<K>>>,
    on_close_tab: Option<EventHandler<K>>,
    on_reorder: Option<Callback<ReorderEvent<K>, bool>>,
    owned_tabs_store: Option<Store<Vec<Tab<K>>>>,
    disallow_empty_selection: bool,
    indicator_revision: Signal<u64>,
) -> Element {
    let typed_key = tab.key;
    let key = typed_key.into_key();
    let vdom_key = dioxus_vdom_key(&key);

    let tab_attrs = machine.derive({
        let key = key.clone();
        let modality = Arc::clone(modality);

        move |api| {
            modality_revision();

            attr_map_to_dioxus_inline_attrs(
                api.tab_attrs(&key, !modality.had_pointer_interaction()),
            )
        }
    });

    let label = tab.label;

    let link = tab.link;

    let prevent_default_on_click = link.is_some();

    let on_click = {
        let key = key.clone();
        let config = (*config).clone();
        move |event: dioxus::prelude::Event<MouseData>| {
            if prevent_default_on_click {
                event.prevent_default();
            }

            select_and_emit_value_change(machine, send, &key, on_value_change, &config);
        }
    };

    let on_pointerdown = {
        let modality = Arc::clone(modality);
        move |event: dioxus::prelude::Event<PointerData>| {
            let data = event.data();

            modality.on_pointer_down(pointer_type_from_dioxus(&data.pointer_type()));
            modality_revision += 1;
        }
    };

    let on_focus = {
        let key = key.clone();
        move |_event: dioxus::prelude::Event<FocusData>| {
            send.call(Event::Focus(key.clone()));
        }
    };

    let on_blur = move |_event: dioxus::prelude::Event<FocusData>| {
        send.call(Event::Blur);
    };

    let on_keydown = {
        let key = key.clone();
        let config = (*config).clone();
        let modality = Arc::clone(modality);
        let label_text = tab.label_text.resolve();
        move |event: dioxus::prelude::Event<KeyboardData>| {
            modality_revision += 1;

            handle_tab_keydown(
                &event,
                &key,
                &label_text,
                machine,
                send,
                &config,
                reorder_status,
                on_value_change,
                on_close_tab,
                on_reorder,
                &modality,
                owned_tabs_store,
                disallow_empty_selection,
                indicator_revision,
            );
        }
    };

    let draggable = config.reorderable.to_string();

    let on_dragstart = {
        let key = key.clone();
        let config = (*config).clone();
        let mut drag_source = drag_source;

        move |_event: dioxus::prelude::Event<DragData>| {
            if config.reorderable {
                drag_source.set(Some(key.clone()));
            }
        }
    };

    let on_dragover = {
        let key = key.clone();
        let config = (*config).clone();
        move |event: dioxus::prelude::Event<DragData>| {
            if can_accept_drag(&config, drag_source, &key) {
                event.prevent_default();
            }
        }
    };

    let on_drop = {
        let key = key.clone();
        let config = (*config).clone();

        move |event: dioxus::prelude::Event<DragData>| {
            handle_tab_drop(
                &event,
                &key,
                machine,
                send,
                &config,
                drag_source,
                reorder_status,
                on_reorder,
                owned_tabs_store,
                indicator_revision,
            );
        }
    };

    let can_close = config
        .tabs_meta
        .iter()
        .any(|meta| meta.key == key && meta.closable && !meta.disabled);

    let close_button = if can_close {
        let close_attrs = attr_map_to_dioxus_inline_attrs(machine.with_api_snapshot({
            let label_text = tab.label_text.resolve();
            move |api| {
                let mut attrs = api.close_trigger_attrs(&label_text);

                attrs.set(HtmlAttr::TabIndex, "-1");

                attrs
            }
        }));

        rsx! {
            span {
                onclick: {
                    let key = key.clone();
                    let tabs_meta = config.tabs_meta.clone();
                    move |event: dioxus::prelude::Event<MouseData>| {
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
                            &tabs_meta,
                            owned_tabs_store,
                            disallow_empty_selection,
                        );

                        if let Some((_successor_key, element_id)) = successor {
                            defer_focus_tab_element_by_id(element_id);
                        }
                    }
                },
                ..close_attrs,
            }
        }
    } else {
        rsx! {}
    };

    if let Some(href) = link {
        rsx! {
            a {
                key: "{vdom_key}",
                href: "{href}",
                onclick: on_click,
                onfocus: on_focus,
                onblur: on_blur,
                onkeydown: on_keydown,
                onpointerdown: on_pointerdown,
                ondragstart: on_dragstart,
                ondragover: on_dragover,
                ondrop: on_drop,
                onmounted: {
                    let key = key.clone();
                    move |event: dioxus::prelude::Event<MountedData>| {
                        tab_nodes
                            .write()
                            .insert(key.clone(), event.data());
                    }
                },
                draggable: "{draggable}",
                ..tab_attrs(),
                {label}
                {close_button}
            }
        }
    } else {
        rsx! {
            div {
                key: "{vdom_key}",
                onclick: on_click,
                onfocus: on_focus,
                onblur: on_blur,
                onkeydown: on_keydown,
                onpointerdown: on_pointerdown,
                ondragstart: on_dragstart,
                ondragover: on_dragover,
                ondrop: on_drop,
                onmounted: {
                    let key = key.clone();
                    move |event: dioxus::prelude::Event<MountedData>| {
                        tab_nodes
                            .write()
                            .insert(key.clone(), event.data());
                    }
                },
                draggable: "{draggable}",
                ..tab_attrs(),
                {label}
                {close_button}
            }
        }
    }
}

fn can_accept_drag<K: TabKey>(
    config: &TabsConfig<K>,
    drag_source: Signal<Option<Key>>,
    target_key: &Key,
) -> bool {
    let Some(source_key) = drag_source.peek().clone() else {
        return false;
    };

    source_key != *target_key
        && drag_reorder_plan(&config.tabs_meta, &source_key, target_key).is_some()
}

#[expect(
    clippy::too_many_arguments,
    reason = "drop handling mirrors keyboard reorder callbacks and live-region state"
)]
fn handle_tab_drop<K: TabKey>(
    event: &dioxus::prelude::Event<DragData>,
    target_key: &Key,
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: EventHandler<Event>,
    config: &TabsConfig<K>,
    mut drag_source: Signal<Option<Key>>,
    mut reorder_status: Signal<String>,
    on_reorder: Option<Callback<ReorderEvent<K>, bool>>,
    owned_tabs_store: Option<Store<Vec<Tab<K>>>>,
    mut indicator_revision: Signal<u64>,
) {
    let Some(source_key) = drag_source.peek().clone() else {
        return;
    };

    drag_source.set(None);

    if source_key == *target_key {
        return;
    }

    let Some(plan) = drag_reorder_plan(&config.tabs_meta, &source_key, target_key) else {
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

    if !reorder_owned_tab(owned_tabs_store, &reorder_event, external_reorder_committed) {
        return;
    }

    send.call(Event::ReorderTab {
        tab: source_key,
        new_index,
    });

    if let Some(element_id) = tab_element_id(machine, &focus_key) {
        defer_focus_tab_element_by_id(element_id);
    }

    let announcement = machine.with_api_snapshot(|api| {
        api.reorder_announcement(&plan.label_text, new_index + 1, plan.total)
    });

    reorder_status.set(announcement);
    bump_indicator_revision(&mut indicator_revision);
}

#[expect(
    clippy::too_many_arguments,
    reason = "keyboard handling needs machine, callbacks, config, and live region state"
)]
fn handle_tab_keydown<K: TabKey>(
    event: &dioxus::prelude::Event<KeyboardData>,
    key: &Key,
    label_text: &str,
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: EventHandler<Event>,
    config: &TabsConfig<K>,
    mut reorder_status: Signal<String>,
    on_value_change: Option<EventHandler<Option<K>>>,
    on_close_tab: Option<EventHandler<K>>,
    on_reorder: Option<Callback<ReorderEvent<K>, bool>>,
    modality: &Arc<dyn ModalityContext>,
    owned_tabs_store: Option<Store<Vec<Tab<K>>>>,
    disallow_empty_selection: bool,
    mut indicator_revision: Signal<u64>,
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

    let effective_dir = effective_direction(machine, config.dir);

    let (prev_key, next_key) = arrow_pair_for(config.orientation, effective_dir);

    // §6.4 — Ctrl+Arrow reorder takes priority and short-circuits.
    if config.reorderable && data.ctrl_key {
        let forward = match config.orientation {
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
            let total = config.tabs_meta.len();

            let plan = machine.with_api_snapshot(|api| {
                api.next_reorder_index(key, forward).map(|new_index| {
                    let announcement = api.reorder_announcement(label_text, new_index + 1, total);
                    (new_index, announcement)
                })
            });

            let old_index = config.tabs_meta.iter().position(|meta| meta.key == *key);

            if let (Some(old_index), Some((new_index, announcement))) = (old_index, plan) {
                let Some(typed_key) = config.tabs_meta.get(old_index).map(|meta| meta.typed_key)
                else {
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

                if reorder_owned_tab(owned_tabs_store, &reorder_event, external_reorder_committed) {
                    send.call(Event::ReorderTab {
                        tab: key.clone(),
                        new_index,
                    });

                    if let Some(element_id) = tab_element_id(machine, key) {
                        defer_focus_tab_element_by_id(element_id);
                    }

                    reorder_status.set(announcement);
                    bump_indicator_revision(&mut indicator_revision);
                }
            }

            return;
        }
    }

    let manual = matches!(config.activation_mode, ActivationMode::Manual);

    let is_closable = config
        .tabs_meta
        .iter()
        .find(|meta| meta.key == *key)
        .is_some_and(|meta| meta.closable && !meta.disabled);

    if data.key == next_key {
        event.prevent_default();

        if manual {
            send.call(Event::FocusNext);
        } else {
            focus_and_emit_value_change(machine, send, Event::FocusNext, on_value_change, config);
        }
    } else if data.key == prev_key {
        event.prevent_default();

        if manual {
            send.call(Event::FocusPrev);
        } else {
            focus_and_emit_value_change(machine, send, Event::FocusPrev, on_value_change, config);
        }
    } else if data.key == KeyboardKey::Home {
        event.prevent_default();

        if manual {
            send.call(Event::FocusFirst);
        } else {
            focus_and_emit_value_change(machine, send, Event::FocusFirst, on_value_change, config);
        }
    } else if data.key == KeyboardKey::End {
        event.prevent_default();

        if manual {
            send.call(Event::FocusLast);
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

        let Some(typed_key) = typed_key_for_key(&config.tabs_meta, key) else {
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
            &config.tabs_meta,
            owned_tabs_store,
            disallow_empty_selection,
        );

        if let Some((_successor_key, element_id)) = successor {
            defer_focus_tab_element_by_id(element_id);
        }
    }
}

fn pointer_type_from_dioxus(pointer_type: &str) -> PointerType {
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
            api.tab_attrs(&successor_key, false)
                .get(&HtmlAttr::Id)
                .map(String::from)
                .map(|id| (successor_key, id))
        })
    })
}

// ────────────────────────────────────────────────────────────────────
// Tab panel rendering
// ────────────────────────────────────────────────────────────────────

fn render_tab_panel<K: TabKey>(
    tab: Tab<K>,
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    lazy_mount: bool,
    unmount_on_exit: bool,
    ever_selected: Signal<BTreeSet<Key>>,
) -> Element {
    let key = tab.key.into_key();
    let vdom_key = dioxus_vdom_key(&key);

    let panel_attrs = attr_map_to_dioxus_inline_attrs(machine.with_api_snapshot({
        let key = key.clone();
        move |api| api.panel_attrs(&key, None)
    }));

    let is_selected = machine.with_api_snapshot({
        let key = key.clone();
        move |api| api.is_tab_selected(&key)
    });

    let already_selected = ever_selected.read().contains(&key);

    let should_render_body =
        should_render_panel_body(is_selected, already_selected, lazy_mount, unmount_on_exit);

    let panel_body = if should_render_body {
        tab.panel
    } else {
        rsx! {}
    };

    rsx! {
        div { key: "{vdom_key}", ..panel_attrs, {panel_body} }
    }
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
    send: EventHandler<Event>,
    event: Event,
    on_value_change: Option<EventHandler<Option<K>>>,
    config: &TabsConfig<K>,
) {
    let before = machine.with_api_snapshot(|api| api.selected_tab().cloned());

    send.call(event);

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
        callback.call(emitted.and_then(|key| typed_key_for_key(&config.tabs_meta, &key)));
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn focus_tab_element_by_id(id: &str) {
    // Tabs must provide roving DOM focus even when the consumer has not
    // installed an ArsProvider platform. Keep the provider call above as
    // the primary path, then fall back to the browser implementation for
    // standalone adapter usage and examples.
    ars_dom::focus_element_by_id(id);
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
const fn focus_tab_element_by_id(_id: &str) {}

fn focus_tab_element(
    tab_nodes: Signal<BTreeMap<Key, Rc<MountedData>>>,
    key: &Key,
    fallback_id: &str,
    dioxus_platform: Arc<dyn DioxusPlatform>,
) {
    let element = tab_nodes.peek().get(key).cloned();

    if let Some(element) = element {
        spawn(async move {
            drop(dioxus_platform.focus_mounted_element(element).await);
        });
    } else {
        focus_tab_element_by_id(fallback_id);
    }
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn defer_focus_tab_element_by_id(id: String) {
    let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
        let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
            focus_tab_element_by_id(&id);
        });

        if let Some(window) = web_sys::window() {
            drop(window.request_animation_frame(callback.as_ref().unchecked_ref()));
        }
    });

    if let Some(window) = web_sys::window() {
        drop(window.request_animation_frame(callback.as_ref().unchecked_ref()));
    }
}

#[cfg(any(not(feature = "web"), not(target_arch = "wasm32")))]
fn defer_focus_tab_element_by_id(_id: String) {}

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
    send: EventHandler<Event>,
    key: &Key,
    on_value_change: Option<EventHandler<Option<K>>>,
    config: &TabsConfig<K>,
) {
    let before = machine.with_api_snapshot(|api| api.selected_tab().cloned());

    send.call(Event::SelectTab(key.clone()));

    let after = machine.with_api_snapshot(|api| api.selected_tab().cloned());

    let Some(callback) = on_value_change else {
        return;
    };

    let emitted = if before != after {
        Some(after.and_then(|key| typed_key_for_key(&config.tabs_meta, &key)))
    } else if before.as_ref() != Some(key) {
        config
            .tabs_meta
            .iter()
            .find(|tab| tab.key == *key && !tab.disabled)
            .map(|tab| Some(tab.typed_key))
    } else {
        return;
    };

    if let Some(emitted) = emitted {
        callback.call(emitted);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "close requests need machine dispatch, callbacks, key metadata, and owned-store context"
)]
fn emit_close_request<K: TabKey>(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    send: EventHandler<Event>,
    on_close_tab: Option<EventHandler<K>>,
    on_value_change: Option<EventHandler<Option<K>>>,
    typed_key: K,
    key: &Key,
    successor: Option<Key>,
    tabs_meta: &[TabMeta<K>],
    owned_tabs_store: Option<Store<Vec<Tab<K>>>>,
    disallow_empty_selection: bool,
) {
    if !machine.with_api_snapshot(|api| api.can_close_tab(key)) {
        return;
    }

    let was_selected = machine.with_api_snapshot(|api| api.selected_tab() == Some(key));

    send.call(Event::CloseTab(key.clone()));

    if let Some(callback) = on_close_tab {
        callback.call(typed_key);
    }

    close_owned_tab(owned_tabs_store, key, disallow_empty_selection);

    if let Some(successor) = successor {
        machine.send.call(Event::SelectTab(successor.clone()));

        if was_selected && let Some(callback) = on_value_change {
            callback.call(typed_key_for_key(tabs_meta, &successor));
        }
    } else if was_selected && let Some(callback) = on_value_change {
        callback.call(None);
    }
}

fn close_owned_tab<K: TabKey>(
    owned_tabs_store: Option<Store<Vec<Tab<K>>>>,
    key: &Key,
    disallow_empty_selection: bool,
) {
    let Some(mut tabs_store) = owned_tabs_store else {
        return;
    };

    let mut tabs = tabs_store.write();

    if disallow_empty_selection && tabs.len() <= 1 {
        return;
    }

    tabs.retain(|tab| tab.key.into_key() != *key);
}

fn reorder_owned_tab<K: TabKey>(
    owned_tabs_store: Option<Store<Vec<Tab<K>>>>,
    event: &ReorderEvent<K>,
    external_reorder_committed: bool,
) -> bool {
    let Some(mut tabs_store) = owned_tabs_store else {
        return external_reorder_committed;
    };

    let mut tabs = tabs_store.write();

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
        callback.call(event.clone()).then_some(true)
    })
}

fn bump_indicator_revision(indicator_revision: &mut Signal<u64>) {
    let next = {
        let revision = indicator_revision.peek();
        revision.wrapping_add(1)
    };

    indicator_revision.set(next);
}

fn sync_indicator_signature<K: TabKey>(
    signature: &mut Signal<String>,
    indicator_revision: &mut Signal<u64>,
    tabs_meta: &[TabMeta<K>],
) {
    let next = tabs_meta_indicator_signature(tabs_meta);

    if signature.peek().as_str() != next {
        signature.set(next);
        bump_indicator_revision(indicator_revision);
    }
}

fn tabs_meta_indicator_signature<K: TabKey>(tabs_meta: &[TabMeta<K>]) -> String {
    tabs_meta
        .iter()
        .map(|meta| {
            format!(
                "{}:{}:{}:{}",
                dioxus_vdom_key(&meta.key),
                meta.label_text,
                meta.closable,
                meta.disabled
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn dioxus_vdom_key(key: &Key) -> String {
    match key {
        Key::Int(value) => format!("int:{value}"),
        Key::String(value) => format!("str:{value}"),
        #[cfg(feature = "uuid")]
        Key::Uuid(value) => format!("uuid:{value}"),
    }
}

#[derive(Clone)]
struct TabsConfig<K: TabKey> {
    orientation: Orientation,
    dir: Direction,
    activation_mode: ActivationMode,
    reorderable: bool,
    tabs_meta: Vec<TabMeta<K>>,
}

fn indicator_measurement_style(
    machine: crate::use_machine::UseMachineReturn<tabs::Machine>,
    platform: &dyn PlatformEffects,
) -> String {
    let Some((list_id, tab_id)) = machine.with_api_snapshot(|api| {
        let selected = api.selected_tab()?;

        let list_id = api.list_attrs().get(&HtmlAttr::Id).map(String::from)?;

        let tab_id = api
            .tab_attrs(selected, false)
            .get(&HtmlAttr::Id)
            .map(String::from)?;

        Some((list_id, tab_id))
    }) else {
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

fn keyboard_event_data(event: &dioxus::prelude::Event<KeyboardData>) -> KeyboardEventData {
    let data = event.data();

    let (key, character) = dioxus_key_to_keyboard_key(&data.key());

    let modifiers = data.modifiers();

    KeyboardEventData {
        key,
        character,
        code: format!("{:?}", data.code()),
        shift_key: modifiers.shift(),
        ctrl_key: modifiers.ctrl(),
        alt_key: modifiers.alt(),
        meta_key: modifiers.meta(),
        repeat: data.is_auto_repeating(),
        is_composing: data.is_composing(),
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
        Tab::new_static(key, label, rsx! { "Panel" })
    }

    #[test]
    fn tab_builders_debug_and_source_conversions_cover_public_api() {
        let custom = Tab::new_with_label(
            "custom",
            "Custom label",
            rsx! {
                strong { "Custom" }
            },
            rsx! { "Custom panel" },
        )
        .trigger(rsx! {
            em { "Overridden" }
        })
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

        let debug = format!("{custom:?}");

        assert!(debug.contains("Tab"));
        assert!(debug.contains("custom"));
        assert!(debug.contains("Custom label"));

        let from_vec = TabsSource::from(vec![tab("first", "First")]);

        assert!(matches!(from_vec, TabsSource::Owned(_)));
        assert!(format!("{from_vec:?}").contains("TabsSource::Owned"));

        let from_array = TabsSource::from([tab("second", "Second")]);

        assert!(matches!(from_array, TabsSource::Owned(_)));
    }

    #[test]
    fn owned_tabs_for_render_preserves_runtime_order_with_latest_rows() {
        let current = vec![tab("second", "Old second"), tab("first", "Old first")];
        let latest = vec![tab("first", "New first"), tab("second", "New second")];

        let previous_prop_keys = ["first", "second"];

        let rendered = owned_tabs_for_render(&current, &latest, &previous_prop_keys);

        assert_eq!(
            rendered
                .iter()
                .map(|tab| (tab.key, tab.label_text.resolve()))
                .collect::<Vec<_>>(),
            vec![
                ("second", "New second".to_owned()),
                ("first", "New first".to_owned())
            ]
        );
    }

    #[test]
    fn owned_tabs_for_render_adopts_parent_prop_reorders() {
        let current = vec![
            tab("second", "Old second"),
            tab("first", "Old first"),
            tab("third", "Old third"),
        ];
        let latest = vec![
            tab("third", "New third"),
            tab("second", "New second"),
            tab("first", "New first"),
        ];

        let previous_prop_keys = ["first", "second", "third"];

        let rendered = owned_tabs_for_render(&current, &latest, &previous_prop_keys);

        assert_eq!(
            rendered
                .iter()
                .map(|tab| (tab.key, tab.label_text.resolve()))
                .collect::<Vec<_>>(),
            vec![
                ("third", "New third".to_owned()),
                ("second", "New second".to_owned()),
                ("first", "New first".to_owned())
            ]
        );
    }

    #[test]
    fn owned_tabs_for_render_drops_removed_rows_before_index_based_mutation() {
        let current = vec![
            tab("first", "Old first"),
            tab("second", "Old second"),
            tab("third", "Old third"),
        ];
        let latest = vec![tab("first", "New first"), tab("third", "New third")];

        let previous_prop_keys = ["first", "second", "third"];

        let rendered = owned_tabs_for_render(&current, &latest, &previous_prop_keys);

        assert_eq!(
            rendered
                .iter()
                .map(|tab| (tab.key, tab.label_text.resolve()))
                .collect::<Vec<_>>(),
            vec![
                ("first", "New first".to_owned()),
                ("third", "New third".to_owned())
            ]
        );
        assert!(
            owned_tabs_changed(&current, &rendered),
            "internal owned store must be synchronized before close/reorder indices are applied"
        );
    }

    #[test]
    fn owned_tabs_for_render_does_not_readd_adapter_closed_rows() {
        let current = vec![tab("first", "Old first"), tab("third", "Old third")];
        let latest = vec![
            tab("first", "New first"),
            tab("second", "New second"),
            tab("third", "New third"),
        ];
        let previous_prop_keys = ["first", "second", "third"];

        let rendered = owned_tabs_for_render(&current, &latest, &previous_prop_keys);

        assert_eq!(
            rendered
                .iter()
                .map(|tab| (tab.key, tab.label_text.resolve()))
                .collect::<Vec<_>>(),
            vec![
                ("first", "New first".to_owned()),
                ("third", "New third".to_owned())
            ]
        );
    }

    #[test]
    fn owned_tabs_changed_tracks_non_key_row_updates() {
        let current = vec![tab("first", "First"), tab("second", "Second")];
        let rendered = vec![
            tab("first", "First"),
            tab("second", "Second").disabled(true).closable(true),
        ];

        assert!(
            owned_tabs_changed(&current, &rendered),
            "same-key row metadata changes must refresh the owned store"
        );
    }

    #[test]
    fn owned_tabs_for_render_keeps_adapter_closed_rows_closed_when_parent_adds_and_reorders() {
        let current = vec![tab("first", "Old first"), tab("third", "Old third")];
        let latest = vec![
            tab("third", "New third"),
            tab("second", "New second"),
            tab("first", "New first"),
            tab("fourth", "New fourth"),
        ];

        let previous_prop_keys = ["first", "second", "third"];

        let rendered = owned_tabs_for_render(&current, &latest, &previous_prop_keys);

        assert_eq!(
            rendered
                .iter()
                .map(|tab| (tab.key, tab.label_text.resolve()))
                .collect::<Vec<_>>(),
            vec![
                ("third", "New third".to_owned()),
                ("first", "New first".to_owned()),
                ("fourth", "New fourth".to_owned())
            ]
        );
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
    fn dioxus_vdom_key_includes_key_variant() {
        assert_eq!(dioxus_vdom_key(&Key::Int(1)), "int:1");
        assert_eq!(dioxus_vdom_key(&Key::str("1")), "str:1");
    }

    #[test]
    fn tabs_meta_snapshot_merges_row_and_prop_disabled_state() {
        let tabs = vec![
            tab("first", "First").closable(true),
            tab("second", "Second").disabled(true),
            tab("third", "Third"),
        ];
        let disabled_keys = BTreeSet::from(["third"]);

        let meta = tabs_meta_snapshot(&tabs, &disabled_keys);

        assert_eq!(
            meta.iter()
                .map(|tab| (
                    tab.typed_key,
                    tab.label_text.as_str(),
                    tab.closable,
                    tab.disabled
                ))
                .collect::<Vec<_>>(),
            vec![
                ("first", "First", true, false),
                ("second", "Second", false, true),
                ("third", "Third", false, true),
            ]
        );
    }

    #[test]
    fn build_tabs_render_snapshot_threads_adapter_state_to_core_props() {
        let tabs = vec![
            tab("first", "First"),
            tab("second", "Second").disabled(true),
            tab("third", "Third").closable(true),
        ];
        let disabled_keys = BTreeSet::from(["first"]);

        let snapshot = build_tabs_render_snapshot(
            "tabs-test",
            Some(Some("third")),
            "first",
            &tabs,
            Orientation::Vertical,
            ActivationMode::Manual,
            Direction::Rtl,
            false,
            true,
            true,
            true,
            &disabled_keys,
            true,
        );

        assert_eq!(snapshot.core_props.id, "tabs-test");
        assert_eq!(snapshot.core_props.value, Some(Some(key("third"))));
        assert_eq!(snapshot.core_props.default_value, Some(key("first")));
        assert_eq!(snapshot.core_props.orientation, Orientation::Vertical);
        assert_eq!(snapshot.core_props.activation_mode, ActivationMode::Manual);
        assert_eq!(snapshot.core_props.dir, Direction::Rtl);
        assert!(!snapshot.core_props.loop_focus);
        assert!(snapshot.core_props.disallow_empty_selection);
        assert!(snapshot.core_props.lazy_mount);
        assert!(snapshot.core_props.unmount_on_exit);
        assert!(snapshot.core_props.reorderable);
        assert_eq!(
            snapshot.core_props.disabled_keys,
            BTreeSet::from([key("first"), key("second")])
        );
        assert_eq!(snapshot.registrations.len(), 3);
        assert!(snapshot.registrations[2].closable);
        assert!(snapshot.config.tabs_meta[0].disabled);
        assert!(snapshot.config.tabs_meta[1].disabled);
        assert!(snapshot.config.reorderable);
    }

    #[test]
    fn pointer_type_mapping_matches_browser_tokens() {
        assert_eq!(pointer_type_from_dioxus("mouse"), PointerType::Mouse);
        assert_eq!(pointer_type_from_dioxus("touch"), PointerType::Touch);
        assert_eq!(pointer_type_from_dioxus("pen"), PointerType::Pen);
        assert_eq!(pointer_type_from_dioxus(""), PointerType::Virtual);
        assert_eq!(pointer_type_from_dioxus("unknown"), PointerType::Virtual);
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
