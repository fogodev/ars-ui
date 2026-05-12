mod locale;
mod messages;
mod panels;
mod text;

use ars_dioxus::{
    ArsProvider,
    navigation::tabs::{Tab, Tabs},
    prelude::t,
};
use dioxus::prelude::*;

use crate::{
    locale::{LocaleSwitcher, parse_locale},
    panels::{EmptyCategoryPanel, NavigationPanel, UtilityPanel},
    text::{CategoryTab, WidgetsText},
};

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let locale = use_signal(|| parse_locale("en-US"));

    rsx! {
        ArsProvider { locale, i18n_registries: messages::i18n_registries(),
            main { class: "widgets-page",
                h1 { {t(WidgetsText::DioxusTitle)} }
                p { {t(WidgetsText::PageSummary)} }
                LocaleSwitcher { locale }
                Tabs {
                    default_value: CategoryTab::Utility,
                    tabs: [
                        Tab::new(CategoryTab::Input, rsx! {
                            EmptyCategoryPanel { text: WidgetsText::InputPanel }
                        }),
                        Tab::new(CategoryTab::Selection, rsx! {
                            EmptyCategoryPanel { text: WidgetsText::SelectionPanel }
                        }),
                        Tab::new(CategoryTab::Overlay, rsx! {
                            EmptyCategoryPanel { text: WidgetsText::OverlayPanel }
                        }),
                        Tab::new(CategoryTab::Navigation, NavigationPanel()),
                        Tab::new(CategoryTab::DateTime, rsx! {
                            EmptyCategoryPanel { text: WidgetsText::DateTimePanel }
                        }),
                        Tab::new(CategoryTab::DataDisplay, rsx! {
                            EmptyCategoryPanel { text: WidgetsText::DataDisplayPanel }
                        }),
                        Tab::new(CategoryTab::Layout, rsx! {
                            EmptyCategoryPanel { text: WidgetsText::LayoutPanel }
                        }),
                        Tab::new(CategoryTab::Specialized, rsx! {
                            EmptyCategoryPanel { text: WidgetsText::SpecializedPanel }
                        }),
                        Tab::new(CategoryTab::Utility, UtilityPanel()),
                    ],
                }
            }
        }
    }
}
