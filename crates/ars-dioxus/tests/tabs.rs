//! SSR tests for the Dioxus Tabs adapter.

#![cfg(not(target_arch = "wasm32"))]

use ars_collections::TabKey;
use ars_dioxus::{
    dioxus_stores::use_store,
    navigation::tabs::{ActivationMode, ReadStore, Tab, Tabs},
    prelude::{Direction, Orientation},
};
use ars_i18n::{IntlBackend, Locale, Translate};
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, TabKey)]
#[tab_key(ordinal)]
enum TypedTab {
    Alpha,
    Beta,
}

impl Translate for TypedTab {
    fn translate(&self, _locale: &Locale, _intl: &dyn IntlBackend) -> String {
        match self {
            Self::Alpha => String::from("Translated alpha"),
            Self::Beta => String::from("Translated beta"),
        }
    }
}

type TestTab = Tab<&'static str>;

fn render_app(app: fn() -> Element) -> String {
    let mut vdom = VirtualDom::new(app);

    vdom.rebuild_in_place();

    dioxus_ssr::render(&vdom)
}

fn three_tabs() -> Vec<TestTab> {
    vec![
        Tab::new_with_label(
            "first",
            "First",
            rsx! { "First" },
            rsx! { p { "Panel one" } },
        ),
        Tab::new_with_label(
            "second",
            "Second",
            rsx! { "Second" },
            rsx! { p { "Panel two" } },
        ),
        Tab::new_with_label(
            "third",
            "Third",
            rsx! { "Third" },
            rsx! { p { "Panel three" } },
        ),
    ]
}

fn typed_tabs() -> [Tab<TypedTab>; 2] {
    [
        Tab::new_with_label(
            TypedTab::Alpha,
            "Alpha",
            rsx! { "Alpha" },
            rsx! {
                p { "Alpha panel" }
            },
        ),
        Tab::new_with_label(
            TypedTab::Beta,
            "Beta",
            rsx! { "Beta" },
            rsx! {
                p { "Beta panel" }
            },
        ),
    ]
}

#[test]
fn tab_new_uses_translated_key_as_default_label() {
    fn translated_tabs_app() -> Element {
        rsx! {
            Tabs {
                default_value: TypedTab::Alpha,
                tabs: [
                    Tab::new(TypedTab::Alpha, rsx! {
                        p { "Alpha panel" }
                    }),
                    Tab::new(TypedTab::Beta, rsx! {
                        p { "Beta panel" }
                    }),
                ],
            }
        }
    }

    let html = render_app(translated_tabs_app);

    assert!(html.contains("Translated alpha"));
    assert!(html.contains("Translated beta"));
}

#[test]
fn tab_new_static_uses_static_text_for_default_label() {
    fn static_tabs_app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: [
                    Tab::new_static("first", "First static", rsx! {
                        p { "First panel" }
                    }),
                    Tab::new_static("second", "Second static", rsx! {
                        p { "Second panel" }
                    }),
                ],
            }
        }
    }

    let html = render_app(static_tabs_app);

    assert!(html.contains("First static"));
    assert!(html.contains("Second static"));
}

fn tabs_snapshot_summary(html: &str) -> String {
    let rows = [
        ("scope", html.matches(r#"data-ars-scope="tabs""#).count()),
        ("root", html.matches(r#"data-ars-part="root""#).count()),
        ("list", html.matches(r#"data-ars-part="list""#).count()),
        ("tabs", html.matches(r#"role="tab""#).count()),
        ("panels", html.matches(r#"role="tabpanel""#).count()),
        ("selected", html.matches(r#"aria-selected="true""#).count()),
        (
            "unselected",
            html.matches(r#"aria-selected="false""#).count(),
        ),
        ("disabled", html.matches(r#"aria-disabled="true""#).count()),
        (
            "close_triggers",
            html.matches(r#"data-ars-part="tab-close-trigger""#).count(),
        ),
        ("links", html.matches(r#"href="/docs""#).count()),
        (
            "reorderable",
            html.matches(r#"aria-roledescription="draggable tab""#)
                .count(),
        ),
        (
            "live_regions",
            html.matches(r#"aria-live="polite""#).count(),
        ),
        (
            "vertical",
            html.matches(r#"aria-orientation="vertical""#).count(),
        ),
        ("rtl", html.matches(r#"dir="rtl""#).count()),
        (
            "docs_panel_body",
            html.matches(r#"data-test="panel-docs""#).count(),
        ),
        (
            "settings_panel_body",
            html.matches(r#"data-test="panel-settings""#).count(),
        ),
    ];

    rows.into_iter()
        .map(|(name, count)| format!("{name}={count}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn rich_tabs() -> Vec<TestTab> {
    use ars_core::SafeUrl;

    vec![
        Tab::new_with_label(
            "inbox",
            "Inbox",
            rsx! { "Inbox" },
            rsx! { p { "data-test": "panel-inbox", "Inbox panel" } },
        )
        .closable(true),
        Tab::new_with_label(
            "docs",
            "Docs",
            rsx! { "Docs" },
            rsx! { p { "data-test": "panel-docs", "Docs panel" } },
        )
        .link(SafeUrl::from_static("/docs")),
        Tab::new_with_label(
            "settings",
            "Settings",
            rsx! { "Settings" },
            rsx! { p { "data-test": "panel-settings", "Settings panel" } },
        )
        .disabled(true),
    ]
}

#[test]
fn typed_enum_tab_keys_render_without_string_keys_at_call_site() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: TypedTab::Beta,
                tabs: typed_tabs(),
                on_value_change: move |_value: Option<TypedTab>| {},
                on_close_tab: move |_key: TypedTab| {},
            }
        }
    }

    let html = render_app(app);

    assert!(html.contains("Beta panel"));
    assert!(html.contains(r#"aria-selected="true""#));
    assert!(html.contains("-panel-i-1"));
}

#[test]
fn renders_root_list_and_tab_data_attributes() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"data-ars-scope="tabs""#),
        "missing tabs scope: {html}"
    );
    assert!(
        html.contains(r#"data-ars-part="root""#),
        "missing root part: {html}"
    );
    assert!(
        html.contains(r#"data-ars-part="list""#),
        "missing list part: {html}"
    );
    assert!(
        html.contains(r#"role="tablist""#),
        "missing tablist role: {html}"
    );
    assert!(
        html.contains(r#"data-ars-part="tab""#),
        "missing tab part: {html}"
    );
    assert!(html.contains(r#"role="tab""#), "missing tab role: {html}");
    assert!(
        html.contains(r#"data-ars-part="tab-indicator""#),
        "missing indicator part: {html}"
    );
}

#[test]
fn rich_tabs_ssr_structural_snapshot() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "docs",
                tabs: ReadStore::from(use_store(rich_tabs)),
                orientation: Orientation::Vertical,
                dir: Direction::Rtl,
                activation_mode: ActivationMode::Manual,
                reorderable: true,
                lazy_mount: true,
            }
        }
    }

    let html = render_app(app);

    insta::assert_snapshot!(tabs_snapshot_summary(&html), @r"
scope=10
root=1
list=1
tabs=3
panels=3
selected=1
unselected=2
disabled=1
close_triggers=1
links=1
reorderable=3
live_regions=1
vertical=1
rtl=1
docs_panel_body=1
settings_panel_body=0");
}

#[test]
fn first_tab_is_selected_by_default() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }

    let html = render_app(app);

    let selected_count = html.matches(r#"aria-selected="true""#).count();
    let unselected_count = html.matches(r#"aria-selected="false""#).count();

    assert_eq!(
        selected_count, 1,
        "exactly one selected tab expected, got {selected_count}: {html}"
    );
    assert_eq!(
        unselected_count, 2,
        "exactly two unselected tabs expected, got {unselected_count}: {html}"
    );
}

#[test]
fn inline_array_tabs_render_without_consumer_store() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: [
                    Tab::new_with_label("first", "First", rsx! { "First" }, rsx! {
                        p { "Panel one" }
                    }),
                    Tab::new_with_label("second", "Second", rsx! { "Second" }, rsx! {
                        p { "Panel two" }
                    }),
                ],
            }
        }
    }

    let html = render_app(app);

    assert_eq!(html.matches(r#"role="tab""#).count(), 2, "{html}");
    assert!(html.contains("Panel one"), "{html}");
}

#[test]
fn panels_render_with_aria_labelledby_and_hidden_for_unselected() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "second",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"role="tabpanel""#),
        "missing tabpanel role: {html}"
    );
    assert!(
        html.contains(r#"aria-labelledby="#),
        "missing aria-labelledby on panel: {html}"
    );

    let hidden_count = html.matches("hidden").count();

    assert!(
        hidden_count >= 2,
        "expected at least two hidden attributes (unselected panels), got {hidden_count}: {html}"
    );
}

#[test]
fn aria_controls_links_tab_to_panel_via_component_ids() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"aria-controls="#),
        "missing aria-controls: {html}"
    );

    assert!(
        html.contains("-panel-s-6669727374"),
        "expected panel id ending with the encoded first key: {html}"
    );
}

#[test]
fn vertical_orientation_propagates_to_aria_and_data_attrs() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
                orientation: Orientation::Vertical,
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"aria-orientation="vertical""#),
        "missing aria-orientation=vertical: {html}"
    );
    assert!(
        html.contains(r#"data-ars-orientation="vertical""#),
        "missing data-ars-orientation=vertical: {html}"
    );
}

#[test]
fn rtl_direction_propagates_to_root_dir_attribute() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
                dir: Direction::Rtl,
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"dir="rtl""#),
        "missing dir=rtl on root: {html}"
    );
}

#[test]
fn disabled_tab_renders_aria_disabled() {
    fn disabled_tabs() -> Vec<TestTab> {
        vec![
            Tab::new_with_label("ok", "OK", rsx! { "OK" }, rsx! { p { "OK panel" } }),
            Tab::new_with_label("nope", "Nope", rsx! { "Nope" }, rsx! { p { "Nope panel" } })
                .disabled(true),
        ]
    }

    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "ok",
                tabs: ReadStore::from(use_store(disabled_tabs)),
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"aria-disabled="true""#),
        "missing aria-disabled=true on disabled tab: {html}"
    );
    assert!(
        html.contains(r#"data-ars-disabled"#),
        "missing data-ars-disabled marker: {html}"
    );
}

#[test]
fn link_tab_renders_anchor_with_href_and_tab_role() {
    use ars_core::SafeUrl;

    fn link_tabs() -> Vec<TestTab> {
        vec![
            Tab::new_with_label("home", "Home", rsx! { "Home" }, rsx! { p { "Home panel" } })
                .link(SafeUrl::from_static("/home")),
            Tab::new_with_label("docs", "Docs", rsx! { "Docs" }, rsx! { p { "Docs panel" } }),
        ]
    }

    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "home",
                tabs: ReadStore::from(use_store(link_tabs)),
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"<a"#),
        "expected anchor element for link tab: {html}"
    );
    assert!(
        html.contains(r#"href="/home""#),
        "expected href on link tab: {html}"
    );
    assert!(
        html.contains(r#"role="tab""#),
        "link tab should still carry role=tab: {html}"
    );
}

#[test]
fn closable_tab_renders_close_trigger_with_label() {
    fn closable_tabs() -> Vec<TestTab> {
        vec![
            Tab::new_with_label(
                "inbox",
                "Inbox",
                rsx! { "Inbox" },
                rsx! { p { "Inbox content" } },
            )
            .closable(true),
        ]
    }

    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "inbox",
                tabs: ReadStore::from(use_store(closable_tabs)),
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"data-ars-part="tab-close-trigger""#),
        "missing close-trigger part: {html}"
    );
    assert!(
        html.contains(r#"aria-label="Close Inbox""#),
        "missing accessible close label: {html}"
    );
}

#[test]
fn reorderable_tabs_get_role_description_and_live_region() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
                reorderable: true,
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"aria-roledescription="draggable tab""#),
        "missing draggable tab roledescription: {html}"
    );
    assert!(
        html.contains(r#"aria-live="polite""#),
        "missing reorder live region: {html}"
    );
}

#[test]
fn manual_activation_mode_does_not_change_default_aria_selected() {
    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(three_tabs)),
                activation_mode: ActivationMode::Manual,
            }
        }
    }

    let html = render_app(app);

    let selected_count = html.matches(r#"aria-selected="true""#).count();

    assert_eq!(
        selected_count, 1,
        "manual activation mode should still show the default selection: {html}"
    );
}

#[test]
fn lazy_mount_omits_panel_body_for_inactive_tabs_on_initial_render() {
    fn lazy_tabs() -> Vec<TestTab> {
        vec![
            Tab::new_with_label(
                "first",
                "First",
                rsx! { "First" },
                rsx! { p { "data-test": "panel-first", "First panel" } },
            ),
            Tab::new_with_label(
                "second",
                "Second",
                rsx! { "Second" },
                rsx! { p { "data-test": "panel-second", "Second panel" } },
            ),
        ]
    }

    fn app() -> Element {
        rsx! {
            Tabs {
                default_value: "first",
                tabs: ReadStore::from(use_store(lazy_tabs)),
                lazy_mount: true,
            }
        }
    }

    let html = render_app(app);

    assert!(
        html.contains(r#"data-test="panel-first""#),
        "selected panel body should render: {html}"
    );

    assert!(
        !html.contains(r#"data-test="panel-second""#),
        "lazy-mounted unselected panel body should NOT render initially: {html}"
    );

    let panel_count = html.matches(r#"role="tabpanel""#).count();

    assert_eq!(
        panel_count, 2,
        "both panel containers must render for ARIA stability: {html}"
    );
}
