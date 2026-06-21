use ars_collections::Key;
use ars_components::navigation::tabs;

use crate::helper::assert_anatomy;

#[test]
fn tabs_anatomy_matches_spec() {
    // Spec references:
    // - `spec/components/navigation/tabs.md` §2 base anatomy table
    //   declares Root / List / Panels / TabShell / Tab / Indicator / Panel.
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
            (tabs::Part::Panels, "panels"),
            (
                tabs::Part::TabShell {
                    tab_key: Key::default(),
                },
                "tab-shell",
            ),
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
