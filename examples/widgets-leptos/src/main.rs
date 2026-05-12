mod locale;
mod messages;
mod panels;
mod text;

use ars_leptos::{
    ArsProvider,
    navigation::tabs::{Tab, Tabs},
    prelude::t,
};
use leptos::{mount::mount_to_body, prelude::*};

use crate::{
    locale::{LocaleSwitcher, parse_locale},
    panels::{EmptyPanel, NavigationPanel, UtilityPanel},
    text::{CategoryTab, WidgetsText},
};

fn main() {
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let locale = RwSignal::new(parse_locale("en-US"));

    view! {
        <ArsProvider locale=locale i18n_registries=messages::i18n_registries()>
            <main class="widgets-page">
                <h1>{t(WidgetsText::LeptosTitle)}</h1>
                <p>{t(WidgetsText::PageSummary)}</p>
                <LocaleSwitcher locale />
                <Tabs
                    default_value=CategoryTab::Utility
                    tabs=[
                        Tab::new(
                            CategoryTab::Input,
                            || {
                                view! { <EmptyPanel message=WidgetsText::InputPanel /> }
                            },
                        ),
                        Tab::new(
                            CategoryTab::Selection,
                            || {
                                view! { <EmptyPanel message=WidgetsText::SelectionPanel /> }
                            },
                        ),
                        Tab::new(
                            CategoryTab::Overlay,
                            || {
                                view! { <EmptyPanel message=WidgetsText::OverlayPanel /> }
                            },
                        ),
                        Tab::new(CategoryTab::Navigation, NavigationPanel),
                        Tab::new(
                            CategoryTab::DateTime,
                            || {
                                view! { <EmptyPanel message=WidgetsText::DateTimePanel /> }
                            },
                        ),
                        Tab::new(
                            CategoryTab::DataDisplay,
                            || {
                                view! { <EmptyPanel message=WidgetsText::DataDisplayPanel /> }
                            },
                        ),
                        Tab::new(
                            CategoryTab::Layout,
                            || {
                                view! { <EmptyPanel message=WidgetsText::LayoutPanel /> }
                            },
                        ),
                        Tab::new(
                            CategoryTab::Specialized,
                            || {
                                view! { <EmptyPanel message=WidgetsText::SpecializedPanel /> }
                            },
                        ),
                        Tab::new(CategoryTab::Utility, UtilityPanel),
                    ]
                />
            </main>
        </ArsProvider>
    }
}
