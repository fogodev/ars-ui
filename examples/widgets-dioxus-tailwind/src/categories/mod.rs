mod data_display;
mod date_time;
mod input;
mod layout;
mod navigation;
mod overlay;
mod selection;
mod specialized;
mod utility;

use ars_dioxus::prelude::*;
use ars_dioxus_components::navigation::tabs::tailwind::Tabs;

use crate::text::CategoryTab;

#[component]
pub(crate) fn CategoryTabs() -> Element {
    rsx! {
        Tabs {
            default_value: CategoryTab::Utility,
            tabs: [
                Tab::new(CategoryTab::Input, input::InputPanel()),
                Tab::new(CategoryTab::Selection, selection::SelectionPanel()),
                Tab::new(CategoryTab::Overlay, overlay::OverlayPanel()),
                Tab::new(CategoryTab::Navigation, navigation::NavigationPanel()),
                Tab::new(CategoryTab::DateTime, date_time::DateTimePanel()),
                Tab::new(CategoryTab::DataDisplay, data_display::DataDisplayPanel()),
                Tab::new(CategoryTab::Layout, layout::LayoutPanel()),
                Tab::new(CategoryTab::Specialized, specialized::SpecializedPanel()),
                Tab::new(CategoryTab::Utility, utility::UtilityPanel()),
            ],
        }
    }
}
