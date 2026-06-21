//! CSS-class styled Leptos Tabs.

pub use ars_leptos::prelude::tabs::Tab;
use ars_leptos::prelude::*;

/// Stylesheet for the CSS Tabs variant.
pub const STYLES: &str = include_str!("tabs.css");

/// Leptos Tabs component styled with stable CSS classes.
#[component]
#[expect(
    clippy::too_many_arguments,
    reason = "styled source template mirrors the documented Tabs semantic prop surface"
)]
pub fn Tabs<K>(
    /// Controlled selected tab key.
    #[prop(optional, into)]
    value: Option<Signal<Option<K>>>,

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
    #[prop(optional)]
    on_reorder: Option<Callback<tabs::ReorderEvent<K>, bool>>,

    /// Consumer class tokens appended to the root.
    #[prop(optional, into)]
    class: Option<TextProp>,
) -> impl IntoView
where
    K: TabKey,
{
    macro_rules! tabs_view {
        ($($optional:tt)*) => {
            view! {
                <tabs::Root
                    $($optional)*
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
            .into_any()
        };
    }

    match (value, on_reorder) {
        (Some(value), Some(on_reorder)) => tabs_view!(value=value on_reorder=on_reorder),
        (Some(value), None) => tabs_view!(value = value),
        (None, Some(on_reorder)) => tabs_view!(on_reorder = on_reorder),
        (None, None) => tabs_view!(),
    }
}
