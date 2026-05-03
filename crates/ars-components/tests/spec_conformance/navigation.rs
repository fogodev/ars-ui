//! Spec-conformance tests for `crates/ars-components/src/navigation/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 / §5 anatomy tables and asserts the impl's `Part` enum
//! matches the declared `(scope, part-name)` ordering.

use ars_collections::Key;
use ars_components::navigation::tabs;

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
