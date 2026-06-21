//! Tailwind styled Dioxus Tabs.

use std::collections::BTreeSet;

use ars_dioxus::prelude::*;

/// Props for the Tailwind-styled Dioxus [`Tabs`] component.
#[derive(Props, Clone, PartialEq)]
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

/// Dioxus Tabs component styled with Tailwind utility classes.
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
            attrs: root_class_attrs(props.attrs, "mt-6 grid gap-3 text-gray-900"),
            tabs::List::<K> {
                class: "relative mb-5 flex flex-wrap items-center gap-1.5 rounded-xl border border-slate-200 bg-slate-100 p-1.5 shadow-inner **:data-[ars-part=tab-indicator]:pointer-events-none **:data-[ars-part=tab-indicator]:absolute **:data-[ars-part=tab-indicator]:left-0 **:data-[ars-part=tab-indicator]:top-0 **:data-[ars-part=tab-indicator]:w-(--ars-indicator-width) **:data-[ars-part=tab-indicator]:h-(--ars-indicator-height) **:data-[ars-part=tab-indicator]:translate-x-(--ars-indicator-left) **:data-[ars-part=tab-indicator]:translate-y-(--ars-indicator-top) **:data-[ars-part=tab-indicator]:rounded-lg **:data-[ars-part=tab-indicator]:bg-white **:data-[ars-part=tab-indicator]:shadow-lg **:data-[ars-part=tab-indicator]:ring-1 **:data-[ars-part=tab-indicator]:ring-slate-300/40 **:data-[ars-part=tab-indicator]:transition-all **:data-[ars-part=tab-indicator]:duration-200 **:data-[ars-part=tab-indicator]:ease-out",

                tab_row: |item: tabs::TabRenderItem<K>| rsx! {
                    tabs::TabShell {
                        item,
                        class: "group relative z-10 inline-flex cursor-pointer items-center gap-1 rounded-md transition duration-150 ease-out active:cursor-grabbing data-ars-disabled:cursor-not-allowed data-ars-dragging:cursor-grabbing data-ars-closable:gap-0 data-ars-closable:pr-2 [&:not([data-ars-disabled]):not([data-ars-selected]):hover]:bg-white [&:not([data-ars-disabled]):not([data-ars-selected]):hover]:text-gray-900 [&:not([data-ars-disabled]):not([data-ars-selected]):hover]:shadow-sm [&:not([data-ars-disabled]):not([data-ars-selected]):hover]:ring-1 [&:not([data-ars-disabled]):not([data-ars-selected]):hover]:ring-gray-300/50 data-ars-selected:bg-blue-600 data-ars-selected:text-white data-ars-selected:shadow-lg data-ars-selected:shadow-blue-700/25 data-ars-selected:hover:bg-blue-700 data-ars-selected:hover:text-white data-ars-focus-visible:ring-2 data-ars-focus-visible:ring-blue-500 data-ars-focus-visible:ring-offset-2 data-ars-focus-visible:ring-offset-white",

                        tabs::Trigger::<K> { class: "relative z-10 cursor-pointer rounded-md px-3 py-2 text-sm font-medium hover:bg-transparent focus:outline-none focus:ring-0 [&:not([data-ars-selected])]:text-slate-700 data-ars-focus-visible:outline-none data-ars-focus-visible:ring-0 aria-disabled:cursor-not-allowed aria-disabled:opacity-50 data-ars-selected:text-white group-data-ars-dragging:cursor-grabbing group-data-ars-closable:pr-2" }

                        tabs::CloseTrigger::<K> { class: "grid size-5 cursor-pointer place-items-center rounded-full text-gray-500 transition group-hover:bg-gray-900/15 group-hover:text-gray-700 hover:bg-red-100 hover:text-red-700 hover:ring-1 hover:ring-red-200 group-data-ars-dragging:cursor-grabbing group-data-ars-selected:bg-white/20 group-data-ars-selected:text-white group-data-ars-selected:hover:bg-white/35 group-data-ars-selected:hover:ring-white/70" }
                    }
                },
            }
            tabs::Panels::<K> {
                class: "min-w-0",
                panel: |item: tabs::TabRenderItem<K>| rsx! {
                    tabs::Panel {
                        item,
                        class: "rounded-xl border border-slate-200 bg-white p-4 text-sm leading-6 text-slate-600 shadow-sm shadow-slate-900/5 outline-none",
                    }
                },
            }
            tabs::LiveRegion {}
        }
    }
}
