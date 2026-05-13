//! SSR tests for the Leptos Tabs adapter.

#![cfg(all(not(target_arch = "wasm32"), feature = "ssr"))]

use ars_collections::TabKey;
use ars_i18n::{IntlBackend, Locale, Translate};
use ars_leptos::{
    navigation::tabs::{ActivationMode, Field, Tab, Tabs},
    reactive_stores::Store,
};
use leptos::prelude::*;

type TestTab = Tab<&'static str>;

#[derive(Store)]
struct TabsTestState {
    tabs: Vec<TestTab>,
}

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

/// Render the supplied view-builder closure under a fresh reactive
/// `Owner` so the `<For>` component (and the `Store::new` allocation
/// backing the `tabs` field) have an arena to live in.
fn render<V: IntoView + 'static>(build: impl FnOnce() -> V) -> String {
    use leptos::reactive::owner::Owner;

    let owner = Owner::new();

    let html = owner.with(|| build().into_view().to_html());

    drop(owner);

    html
}

fn store_tabs(tabs: Vec<TestTab>) -> Field<Vec<TestTab>> {
    // The `prop(into)` on `Tabs::tabs` accepts a `Subfield` directly,
    // but the test helper materializes the `Field` once so call-sites
    // are free to pass it without further conversions.
    Store::new(TabsTestState { tabs }).tabs().into()
}

fn three_tabs() -> Vec<TestTab> {
    vec![
        Tab::new_with_label(
            "first",
            "First",
            ViewFn::from(|| view! { "First" }),
            ViewFn::from(|| view! { <p>"Panel one"</p> }),
        ),
        Tab::new_with_label(
            "second",
            "Second",
            ViewFn::from(|| view! { "Second" }),
            ViewFn::from(|| view! { <p>"Panel two"</p> }),
        ),
        Tab::new_with_label(
            "third",
            "Third",
            ViewFn::from(|| view! { "Third" }),
            ViewFn::from(|| view! { <p>"Panel three"</p> }),
        ),
    ]
}

fn typed_tabs() -> [Tab<TypedTab>; 2] {
    [
        Tab::new_with_label(
            TypedTab::Alpha,
            "Alpha",
            ViewFn::from(|| view! { "Alpha" }),
            ViewFn::from(|| view! { <p>"Alpha panel"</p> }),
        ),
        Tab::new_with_label(
            TypedTab::Beta,
            "Beta",
            ViewFn::from(|| view! { "Beta" }),
            ViewFn::from(|| view! { <p>"Beta panel"</p> }),
        ),
    ]
}

#[test]
fn tab_new_uses_translated_key_as_default_label() {
    let html = render(|| {
        view! {
            <Tabs
                default_value=TypedTab::Alpha
                tabs=[
                    Tab::new(TypedTab::Alpha, ViewFn::from(|| view! { <p>"Alpha panel"</p> })),
                    Tab::new(TypedTab::Beta, ViewFn::from(|| view! { <p>"Beta panel"</p> })),
                ]
            />
        }
    });

    assert!(html.contains("Translated alpha"));
    assert!(html.contains("Translated beta"));
}

#[test]
fn tab_new_static_uses_static_text_for_default_label() {
    let html = render(|| {
        view! {
            <Tabs
                default_value="first"
                tabs=[
                    Tab::new_static(
                        "first",
                        "First static",
                        ViewFn::from(|| view! { <p>"First panel"</p> }),
                    ),
                    Tab::new_static(
                        "second",
                        "Second static",
                        ViewFn::from(|| view! { <p>"Second panel"</p> }),
                    ),
                ]
            />
        }
    });

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
            ViewFn::from(|| view! { "Inbox" }),
            ViewFn::from(|| view! { <p data-test="panel-inbox">"Inbox panel"</p> }),
        )
        .closable(true),
        Tab::new_with_label(
            "docs",
            "Docs",
            ViewFn::from(|| view! { "Docs" }),
            ViewFn::from(|| view! { <p data-test="panel-docs">"Docs panel"</p> }),
        )
        .link(SafeUrl::from_static("/docs")),
        Tab::new_with_label(
            "settings",
            "Settings",
            ViewFn::from(|| view! { "Settings" }),
            ViewFn::from(|| view! { <p data-test="panel-settings">"Settings panel"</p> }),
        )
        .disabled(true),
    ]
}

#[test]
fn typed_enum_tab_keys_render_without_string_keys_at_call_site() {
    let html = render(|| {
        view! {
            <Tabs
                default_value=TypedTab::Beta
                tabs=typed_tabs()
                on_value_change=Callback::new(|_value: Option<TypedTab>| {})
                on_close_tab=Callback::new(|_key: TypedTab| {})
            />
        }
    });

    assert!(html.contains("Beta panel"));
    assert!(html.contains(r#"aria-selected="true""#));
    assert!(html.contains("-panel-i-1"));
}

#[test]
fn renders_root_list_and_tab_data_attributes() {
    let html = render(|| view! { <Tabs default_value="first" tabs=store_tabs(three_tabs()) /> });

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
    let html = render(|| {
        view! {
            <Tabs
                default_value="docs"
                tabs=store_tabs(rich_tabs())
                orientation=ars_leptos::prelude::Orientation::Vertical
                dir=ars_leptos::prelude::Direction::Rtl
                activation_mode=ActivationMode::Manual
                reorderable=true
                lazy_mount=true
            />
        }
    });

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
fn first_tab_is_selected_by_default_with_roving_tabindex() {
    let html = render(|| view! { <Tabs default_value="first" tabs=store_tabs(three_tabs()) /> });

    // Selected tab should have aria-selected="true" and tabindex="0".
    assert!(
        html.contains(r#"aria-selected="true""#),
        "missing aria-selected=true on default-selected tab: {html}"
    );

    // Unselected tabs should have aria-selected="false" and tabindex="-1".
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

    let roving_zero = html.matches(r#"tabindex="0""#).count();
    let roving_neg_one = html.matches(r#"tabindex="-1""#).count();

    // Each panel ALSO has tabindex="0" (programmatically focusable when
    // visible). Three panels + one selected tab = at least 1 zero on tabs.
    assert!(
        roving_zero >= 1,
        "expected at least one tabindex=\"0\": {html}"
    );

    assert!(
        roving_neg_one >= 2,
        "expected at least two tabindex=\"-1\" entries (the unselected tabs): {html}"
    );
}

#[test]
fn inline_array_tabs_render_without_consumer_store() {
    let html = render(|| {
        view! {
            <Tabs
                default_value="first"
                tabs=[
                    Tab::new_with_label(
                        "first",
                        "First",
                        ViewFn::from(|| view! { "First" }),
                        ViewFn::from(|| view! { <p>"Panel one"</p> }),
                    ),
                    Tab::new_with_label(
                        "second",
                        "Second",
                        ViewFn::from(|| view! { "Second" }),
                        ViewFn::from(|| view! { <p>"Panel two"</p> }),
                    ),
                ]
            />
        }
    });

    assert_eq!(html.matches(r#"role="tab""#).count(), 2, "{html}");
    assert!(html.contains("Panel one"), "{html}");
}

#[test]
fn panels_render_with_aria_labelledby_and_hidden_for_unselected() {
    let html = render(|| view! { <Tabs default_value="second" tabs=store_tabs(three_tabs()) /> });

    assert!(
        html.contains(r#"role="tabpanel""#),
        "missing tabpanel role: {html}"
    );
    assert!(
        html.contains(r#"aria-labelledby="#),
        "missing aria-labelledby on panel: {html}"
    );

    // Two panels should be hidden (the unselected ones).
    let hidden_count = html.matches("hidden").count();

    assert!(
        hidden_count >= 2,
        "expected at least two hidden attributes (unselected panels), got {hidden_count}: {html}"
    );
}

#[test]
fn aria_controls_links_tab_to_panel_via_component_ids() {
    let html = render(|| view! { <Tabs default_value="first" tabs=store_tabs(three_tabs()) /> });

    // Each tab has aria-controls referencing the panel id.
    assert!(
        html.contains(r#"aria-controls="#),
        "missing aria-controls: {html}"
    );

    // Verify the panel id pattern. ComponentIds derives panel ids
    // from the DOM-safe key token.
    assert!(
        html.contains("-panel-s-6669727374"),
        "expected panel id ending with the encoded first key: {html}"
    );
}

#[test]
fn vertical_orientation_propagates_to_aria_and_data_attrs() {
    let html = render(|| {
        view! {
            <Tabs
                default_value="first"
                tabs=store_tabs(three_tabs())
                orientation=ars_leptos::prelude::Orientation::Vertical
            />
        }
    });

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
    let html = render(|| {
        view! {
            <Tabs
                default_value="first"
                tabs=store_tabs(three_tabs())
                dir=ars_leptos::prelude::Direction::Rtl
            />
        }
    });

    assert!(
        html.contains(r#"dir="rtl""#),
        "missing dir=rtl on root: {html}"
    );
}

#[test]
fn disabled_tab_renders_aria_disabled() {
    let disabled_tabs = vec![
        Tab::new_with_label(
            "ok",
            "OK",
            ViewFn::from(|| view! { "OK" }),
            ViewFn::from(|| view! { <p>"OK panel"</p> }),
        ),
        Tab::new_with_label(
            "nope",
            "Nope",
            ViewFn::from(|| view! { "Nope" }),
            ViewFn::from(|| view! { <p>"Nope panel"</p> }),
        )
        .disabled(true),
    ];

    let html = render(|| view! { <Tabs default_value="ok" tabs=store_tabs(disabled_tabs) /> });

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

    let link_tabs = vec![
        Tab::new_with_label(
            "home",
            "Home",
            ViewFn::from(|| view! { "Home" }),
            ViewFn::from(|| view! { <p>"Home panel"</p> }),
        )
        .link(SafeUrl::from_static("/home")),
        Tab::new_with_label(
            "docs",
            "Docs",
            ViewFn::from(|| view! { "Docs" }),
            ViewFn::from(|| view! { <p>"Docs panel"</p> }),
        ),
    ];

    let html = render(|| view! { <Tabs default_value="home" tabs=store_tabs(link_tabs) /> });

    assert!(
        html.contains("<a "),
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
    let closable_tabs = vec![
        Tab::new_with_label(
            "inbox",
            "Inbox",
            ViewFn::from(|| view! { "Inbox" }),
            ViewFn::from(|| view! { <p>"Inbox content"</p> }),
        )
        .closable(true),
    ];

    let html = render(|| view! { <Tabs default_value="inbox" tabs=store_tabs(closable_tabs) /> });

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
    let html = render(|| {
        view! { <Tabs default_value="first" tabs=store_tabs(three_tabs()) reorderable=true /> }
    });

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
    let html = render(|| {
        view! {
            <Tabs
                default_value="first"
                tabs=store_tabs(three_tabs())
                activation_mode=ActivationMode::Manual
            />
        }
    });

    let selected_count = html.matches(r#"aria-selected="true""#).count();

    assert_eq!(
        selected_count, 1,
        "manual activation mode should still show the default selection: {html}"
    );
}

#[test]
fn ssr_render_is_deterministic_for_identical_input() {
    let render_once =
        || render(|| view! { <Tabs default_value="first" tabs=store_tabs(three_tabs()) /> });

    let first = render_once();
    let second = render_once();

    // Component IDs are generated from a global counter, so two
    // renders won't be byte-identical, but the structural attributes
    // (data-ars-*, ARIA roles) should match.
    fn structural(html: &str) -> Vec<&str> {
        html.match_indices("data-ars-")
            .map(|(idx, _)| {
                let end = html[idx..]
                    .find(['"', ' '])
                    .map_or(html.len(), |off| idx + off);

                &html[idx..end]
            })
            .collect()
    }

    assert_eq!(
        structural(&first),
        structural(&second),
        "structural data-ars-* attributes should be deterministic across renders"
    );
}

#[test]
fn empty_children_does_not_break_render() {
    let html = render(|| view! { <Tabs default_value="first" tabs=store_tabs(three_tabs()) /> });

    assert!(
        html.contains(r#"data-ars-scope="tabs""#),
        "render must succeed without consumer children: {html}"
    );
}

#[test]
fn lazy_mount_omits_panel_body_for_inactive_tabs_on_initial_render() {
    let tabs = vec![
        Tab::new_with_label(
            "first",
            "First",
            ViewFn::from(|| view! { "First" }),
            ViewFn::from(|| view! { <p data-test="panel-first">"First panel"</p> }),
        ),
        Tab::new_with_label(
            "second",
            "Second",
            ViewFn::from(|| view! { "Second" }),
            ViewFn::from(|| view! { <p data-test="panel-second">"Second panel"</p> }),
        ),
    ];

    let html = render(|| {
        view! { <Tabs default_value="first" tabs=store_tabs(tabs) lazy_mount=true /> }
    });

    // Selected panel should render its body.
    assert!(
        html.contains(r#"data-test="panel-first""#),
        "selected panel body should render: {html}"
    );

    // Unselected panel body should be omitted under lazy_mount.
    assert!(
        !html.contains(r#"data-test="panel-second""#),
        "lazy-mounted unselected panel body should NOT render initially: {html}"
    );

    // Both panel containers (with role=tabpanel) should still render so
    // ARIA wiring stays stable.
    let panel_count = html.matches(r#"role="tabpanel""#).count();

    assert_eq!(
        panel_count, 2,
        "both panel containers must render for ARIA stability: {html}"
    );
}
