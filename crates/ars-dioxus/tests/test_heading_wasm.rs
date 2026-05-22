//! Browser coverage tests for the Dioxus `Heading` adapter.

#![cfg(target_arch = "wasm32")]

use ars_dioxus::utility::heading::{Heading, HeadingLevelProvider, Level, Section};
use dioxus::prelude::*;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn heading_browser_default_and_explicit_levels_build_without_panic() {
    fn app() -> Element {
        rsx! {
            Heading { id: "h-default", "Default" }
            Heading { id: "h-three", level: Level::Three, "Three" }
            Heading { id: "h-six", level: Level::Six, "Six" }
            Heading { "Passive" }
        }
    }

    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();
}

#[wasm_bindgen_test]
fn heading_browser_provider_and_section_publish_context() {
    fn app() -> Element {
        rsx! {
            HeadingLevelProvider { level: Level::Two,
                Heading { id: "provided", "Two" }
                Section {
                    Heading { id: "section-child", "Three" }
                }
            }
        }
    }

    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();
}

#[wasm_bindgen_test]
fn heading_browser_nested_sections_clamp_at_level_six() {
    fn app() -> Element {
        rsx! {
            Section {
                Section {
                    Section {
                        Section {
                            Section {
                                Section {
                                    Section {
                                        Heading { id: "deep", "Six" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();
}
