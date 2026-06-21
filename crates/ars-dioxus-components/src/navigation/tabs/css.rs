//! CSS-class styled Dioxus Tabs.

use std::collections::BTreeSet;

use ars_dioxus::prelude::*;

/// Stylesheet for the CSS Tabs variant.
pub const STYLES: &str = include_str!("tabs.css");

/// Props for the CSS-styled Dioxus [`Tabs`] component.
#[derive(Props, Clone, PartialEq, Debug)]
pub struct TabsProps<K: TabKey> {
    /// Controlled selected tab key.
    #[props(optional)]
    pub value: Option<Option<K>>,

    /// Initial selected tab key in uncontrolled mode.
    #[props(into)]
    pub default_value: K,

    /// Per-tab render rows in DOM order.
    #[props(into)]
    pub tabs: tabs::TabsSource<K>,

    /// Layout orientation.
    #[props(default)]
    pub orientation: Orientation,

    /// How keyboard focus interacts with selection.
    #[props(default)]
    pub activation_mode: tabs::ActivationMode,

    /// Text direction.
    #[props(default = Direction::Ltr)]
    pub dir: Direction,

    /// Whether arrow-key focus wraps from last to first.
    #[props(default = true)]
    pub loop_focus: bool,

    /// Whether the final tab cannot be closed.
    #[props(default = false)]
    pub disallow_empty_selection: bool,

    /// Whether panels are mounted lazily.
    #[props(default = false)]
    pub lazy_mount: bool,

    /// Whether inactive panels are removed from the DOM.
    #[props(default = false)]
    pub unmount_on_exit: bool,

    /// Disabled tab keys.
    #[props(default)]
    pub disabled_keys: BTreeSet<K>,

    /// Whether Ctrl+Arrow reorder is enabled.
    #[props(default = false)]
    pub reorderable: bool,

    /// Called after user intent requests a new selected key.
    #[props(optional)]
    pub on_value_change: Option<EventHandler<Option<K>>>,

    /// Called when a tab close trigger is activated.
    #[props(optional)]
    pub on_close_tab: Option<EventHandler<K>>,

    /// Called before a reorder request is emitted.
    #[props(optional)]
    pub on_reorder: Option<Callback<tabs::ReorderEvent<K>, bool>>,

    /// Global HTML attributes forwarded onto the rendered root.
    #[props(extends = GlobalAttributes)]
    pub attrs: Vec<Attribute>,
}

/// Dioxus Tabs component styled with stable CSS classes.
#[component]
pub fn Tabs<K: TabKey>(props: TabsProps<K>) -> Element {
    rsx! {
        tabs::Root {
            value: props.value,
            default_value: props.default_value,
            tabs: props.tabs,
            orientation: props.orientation,
            activation_mode: props.activation_mode,
            dir: props.dir,
            loop_focus: props.loop_focus,
            disallow_empty_selection: props.disallow_empty_selection,
            lazy_mount: props.lazy_mount,
            unmount_on_exit: props.unmount_on_exit,
            disabled_keys: props.disabled_keys,
            reorderable: props.reorderable,
            on_value_change: props.on_value_change,
            on_close_tab: props.on_close_tab,
            on_reorder: props.on_reorder,
            attrs: root_class_attrs(props.attrs, "ars-tabs"),
            tabs::List::<K> {
                class: "ars-tabs__list",
                tab_row: |item: tabs::TabRenderItem<K>| rsx! {
                    tabs::TabShell {
                        item: item.clone(),
                        class: "ars-tabs__tab-shell",
                        tabs::Trigger::<K> { class: "ars-tabs__tab" }
                        tabs::CloseTrigger::<K> { class: "ars-tabs__close-trigger" }
                    }
                }
            }
            tabs::Panels::<K> { class: "ars-tabs__panels" }
            tabs::LiveRegion {}
        }
    }
}
