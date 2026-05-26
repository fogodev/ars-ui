//! Spec-conformance tests for `crates/ars-components/src/navigation/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 / §5 anatomy tables and asserts the impl's `Part` enum
//! matches the declared `(scope, part-name)` ordering.

use ars_collections::Key;
use ars_components::navigation::{
    accordion, breadcrumbs, link, navigation_menu, pagination, steps, tabs,
};
use ars_core::SafeUrl;

use super::helper::assert_anatomy;

#[test]
fn tabs_anatomy_matches_spec() {
    // Spec references:
    // - `spec/components/navigation/tabs.md` §2 base anatomy table
    //   declares Root / List / Tab / Indicator / Panel.
    // - §5.4 anatomy addition declares the Closable variant's
    //   `tab-close-trigger` part.
    //
    // Workspace convention (see `tests/spec_conformance/helper.rs`)
    // requires the first variant to be `Root`. The remaining variants
    // are listed in spec §2 declaration order, with `TabCloseTrigger`
    // appended last because §5 layers it on top of the base anatomy.
    assert_anatomy(
        "tabs",
        &[
            (tabs::Part::Root, "root"),
            (tabs::Part::List, "list"),
            (
                tabs::Part::Tab {
                    tab_key: Key::default(),
                },
                "tab",
            ),
            (tabs::Part::TabIndicator, "tab-indicator"),
            (
                tabs::Part::Panel {
                    tab_key: Key::default(),
                    tab_label: None,
                },
                "panel",
            ),
            (
                tabs::Part::TabCloseTrigger {
                    tab_label: String::new(),
                },
                "tab-close-trigger",
            ),
        ],
    );
}

#[test]
fn breadcrumbs_anatomy_matches_spec() {
    assert_anatomy(
        "breadcrumbs",
        &[
            (breadcrumbs::Part::Root, "root"),
            (breadcrumbs::Part::List, "list"),
            (breadcrumbs::Part::Item, "item"),
            (
                breadcrumbs::Part::Link {
                    href: SafeUrl::from_static("/"),
                },
                "link",
            ),
            (breadcrumbs::Part::CurrentPage, "current-page"),
            (breadcrumbs::Part::Separator, "separator"),
        ],
    );
}

#[test]
fn link_anatomy_matches_spec() {
    assert_anatomy("link", &[(link::Part::Root, "root")]);
}

#[test]
fn pagination_anatomy_matches_spec() {
    assert_anatomy(
        "pagination",
        &[
            (pagination::Part::Root, "root"),
            (pagination::Part::PrevTrigger, "prev-trigger"),
            (pagination::Part::NextTrigger, "next-trigger"),
            (
                pagination::Part::PageTrigger { page_number: 1 },
                "page-trigger",
            ),
            (pagination::Part::Ellipsis, "ellipsis"),
        ],
    );
}

#[test]
fn steps_anatomy_matches_spec() {
    assert_anatomy(
        "steps",
        &[
            (steps::Part::Root, "root"),
            (steps::Part::List, "list"),
            (steps::Part::Item { index: 0 }, "item"),
            (steps::Part::Indicator { index: 0 }, "indicator"),
            (steps::Part::Title { index: 0 }, "title"),
            (steps::Part::Description { index: 0 }, "description"),
            (steps::Part::Separator { after_index: 0 }, "separator"),
            (steps::Part::Content { index: 0 }, "content"),
            (steps::Part::PrevTrigger, "prev-trigger"),
            (steps::Part::NextTrigger, "next-trigger"),
        ],
    );
}

#[test]
fn accordion_anatomy_matches_spec() {
    assert_anatomy(
        "accordion",
        &[
            (accordion::Part::Root, "root"),
            (
                accordion::Part::Item {
                    item_key: Key::default(),
                },
                "item",
            ),
            (
                accordion::Part::ItemHeader {
                    item_key: Key::default(),
                },
                "item-header",
            ),
            (
                accordion::Part::ItemTrigger {
                    item_key: Key::default(),
                },
                "item-trigger",
            ),
            (
                accordion::Part::ItemIndicator {
                    item_key: Key::default(),
                },
                "item-indicator",
            ),
            (
                accordion::Part::ItemContent {
                    item_key: Key::default(),
                },
                "item-content",
            ),
        ],
    );
}

#[test]
fn navigation_menu_anatomy_matches_spec() {
    assert_anatomy(
        "navigation-menu",
        &[
            (navigation_menu::Part::Root, "root"),
            (navigation_menu::Part::List, "list"),
            (
                navigation_menu::Part::Item {
                    item_key: Key::default(),
                },
                "item",
            ),
            (
                navigation_menu::Part::Trigger {
                    item_key: Key::default(),
                    content_id: String::new(),
                },
                "trigger",
            ),
            (
                navigation_menu::Part::Content {
                    item_key: Key::default(),
                },
                "content",
            ),
            (navigation_menu::Part::Link { active: false }, "link"),
            (navigation_menu::Part::Indicator, "indicator"),
            (navigation_menu::Part::Viewport, "viewport"),
            (navigation_menu::Part::Sub, "sub"),
            (navigation_menu::Part::SubList, "sub-list"),
            (
                navigation_menu::Part::SubItem {
                    item_key: Key::default(),
                },
                "sub-item",
            ),
            (
                navigation_menu::Part::SubTrigger {
                    item_key: Key::default(),
                    content_id: String::new(),
                },
                "sub-trigger",
            ),
            (
                navigation_menu::Part::SubContent {
                    item_key: Key::default(),
                },
                "sub-content",
            ),
            (navigation_menu::Part::SubIndicator, "sub-indicator"),
            (navigation_menu::Part::SubViewport, "sub-viewport"),
        ],
    );
}
