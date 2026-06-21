//! Tailwind styled Leptos Tabs.

pub use ars_leptos::prelude::tabs::Tab;
use ars_leptos::prelude::*;

/// Leptos Tabs component styled with Tailwind utility classes.
#[component]
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
                    class=root_class("mt-6 grid gap-3 text-gray-900", class)
                >
                    <tabs::List<
                    K,
                >
                        class="flex relative flex-wrap gap-1.5 items-center p-1.5 mb-5 rounded-xl border shadow-inner border-slate-200 bg-slate-100 **:data-[ars-part=tab-indicator]:pointer-events-none **:data-[ars-part=tab-indicator]:absolute **:data-[ars-part=tab-indicator]:left-0 **:data-[ars-part=tab-indicator]:top-0 **:data-[ars-part=tab-indicator]:w-(--ars-indicator-width) **:data-[ars-part=tab-indicator]:h-(--ars-indicator-height) **:data-[ars-part=tab-indicator]:translate-x-(--ars-indicator-left) **:data-[ars-part=tab-indicator]:translate-y-(--ars-indicator-top) **:data-[ars-part=tab-indicator]:rounded-lg **:data-[ars-part=tab-indicator]:bg-white **:data-[ars-part=tab-indicator]:shadow-lg **:data-[ars-part=tab-indicator]:ring-1 **:data-[ars-part=tab-indicator]:ring-slate-300/40 **:data-[ars-part=tab-indicator]:transition-all **:data-[ars-part=tab-indicator]:duration-200 **:data-[ars-part=tab-indicator]:ease-out"
                        tab_row=move |item| {
                            view! {
                                <tabs::TabShell
                                    item
                                    class="inline-flex relative z-10 gap-1 items-center rounded-md transition duration-150 ease-out cursor-pointer group data-ars-disabled:cursor-not-allowed data-ars-dragging:cursor-grabbing data-ars-closable:gap-0 data-ars-closable:pr-2 [&:not([data-ars-disabled]):not([data-ars-selected]):hover]:bg-white [&:not([data-ars-disabled]):not([data-ars-selected]):hover]:text-gray-900 [&:not([data-ars-disabled]):not([data-ars-selected]):hover]:shadow-sm [&:not([data-ars-disabled]):not([data-ars-selected]):hover]:ring-1 [&:not([data-ars-disabled]):not([data-ars-selected]):hover]:ring-gray-300/50 data-ars-selected:bg-blue-600 data-ars-selected:text-white data-ars-selected:shadow-lg data-ars-selected:shadow-blue-700/25 data-ars-selected:hover:bg-blue-700 data-ars-selected:hover:text-white data-ars-focus-visible:ring-2 data-ars-focus-visible:ring-blue-500 data-ars-focus-visible:ring-offset-2 data-ars-focus-visible:ring-offset-white active:cursor-grabbing"
                                >
                                    <tabs::Trigger<
                                    K,
                                > class="relative z-10 py-2 px-3 text-sm font-medium rounded-md cursor-pointer hover:bg-transparent focus:ring-0 focus:outline-none [&:not([data-ars-selected])]:text-slate-700 data-ars-focus-visible:outline-none data-ars-focus-visible:ring-0 aria-disabled:cursor-not-allowed aria-disabled:opacity-50 data-ars-selected:text-white group-data-ars-dragging:cursor-grabbing group-data-ars-closable:pr-2" />
                                    <tabs::CloseTrigger<
                                    K,
                                > class="grid place-items-center text-gray-500 rounded-full transition cursor-pointer group-hover:text-gray-700 hover:text-red-700 hover:bg-red-100 hover:ring-1 hover:ring-red-200 size-5 group-data-ars-dragging:cursor-grabbing group-data-ars-selected:bg-white/20 group-data-ars-selected:text-white group-data-ars-selected:hover:bg-white/35 group-data-ars-selected:hover:ring-white/70 group-hover:bg-gray-900/15" />
                                </tabs::TabShell>
                            }
                        }
                    />
                    <tabs::Panels<
                    K,
                >
                        class="min-w-0"
                        panel=|item| {
                            view! {
                                <tabs::Panel
                                    item
                                    class="p-4 text-sm leading-6 bg-white rounded-xl border shadow-sm outline-none text-slate-600 border-slate-200 shadow-slate-900/5"
                                />
                            }
                        }
                    />
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
