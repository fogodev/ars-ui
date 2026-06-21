//! CSS-class styled Leptos Tabs.

pub use ars_leptos::prelude::tabs::Tab;
use ars_leptos::prelude::*;

/// Stylesheet for the CSS Tabs variant.
pub const STYLES: &str = include_str!("tabs.css");

/// Leptos Tabs component styled with stable CSS classes.
#[component]
pub fn Tabs<K>(
    /// Initial selected tab key in uncontrolled mode.
    #[prop(into)]
    default_value: K,

    /// Per-tab render rows in DOM order.
    #[prop(into)]
    tabs: tabs::TabsSource<K>,

    /// Layout orientation.
    #[prop(optional, default = Orientation::Horizontal)]
    orientation: Orientation,

    /// How keyboard focus interacts with selection.
    #[prop(optional, default = tabs::ActivationMode::Automatic)]
    activation_mode: tabs::ActivationMode,

    /// Text direction.
    #[prop(optional, default = Direction::Ltr)]
    dir: Direction,

    /// Whether arrow-key focus wraps from last to first.
    #[prop(optional, default = true)]
    loop_focus: bool,

    /// Whether the final tab cannot be closed.
    #[prop(optional, default = false)]
    disallow_empty_selection: bool,

    /// Whether panels are mounted lazily.
    #[prop(optional, default = false)]
    lazy_mount: bool,

    /// Whether inactive panels are removed from the DOM.
    #[prop(optional, default = false)]
    unmount_on_exit: bool,

    /// Disabled tab keys.
    #[prop(optional)]
    disabled_keys: std::collections::BTreeSet<K>,

    /// Whether Ctrl+Arrow reorder is enabled.
    #[prop(optional, default = false)]
    reorderable: bool,

    /// Called after user intent requests a new selected key.
    #[prop(optional, default = Callback::new(|_| ()))]
    on_value_change: Callback<Option<K>>,

    /// Called when a tab close trigger is activated.
    #[prop(optional, default = Callback::new(|_| ()))]
    on_close_tab: Callback<K>,

    /// Called before a reorder request is emitted.
    #[prop(optional, default = Callback::new(|_| true))]
    on_reorder: Callback<tabs::ReorderEvent<K>, bool>,

    /// Consumer class tokens appended to the root.
    #[prop(optional, into)]
    class: Option<TextProp>,
) -> impl IntoView
where
    K: TabKey,
{
    view! {
        <tabs::Root
            default_value
            tabs
            orientation
            activation_mode
            dir
            loop_focus
            disallow_empty_selection
            lazy_mount
            unmount_on_exit
            disabled_keys
            reorderable
            on_value_change
            on_close_tab
            on_reorder
            class=root_class("ars-tabs", class)
        >
            <tabs::List<
            K,
        >
                class="ars-tabs__list"
                tab_row=|item| {
                    view! {
                        <tabs::TabShell item class="ars-tabs__tab-shell">
                            <tabs::Trigger<K> class="ars-tabs__tab" />
                            <tabs::CloseTrigger<K> class="ars-tabs__close-trigger" />
                        </tabs::TabShell>
                    }
                }
            />
            <tabs::Panels<K> class="ars-tabs__panels" />
            <tabs::LiveRegion />
        </tabs::Root>
    }
}
