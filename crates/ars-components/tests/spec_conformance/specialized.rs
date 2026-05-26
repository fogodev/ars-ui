//! Spec-conformance tests for `crates/ars-components/src/specialized/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §2 anatomy table and asserts the impl's `Part` enum matches.

use ars_components::specialized as specialized_core;

use super::helper::assert_anatomy;

#[test]
fn clipboard_anatomy_matches_spec() {
    assert_anatomy(
        "clipboard",
        &[
            (specialized_core::clipboard::Part::Root, "root"),
            (specialized_core::clipboard::Part::Label, "label"),
            (specialized_core::clipboard::Part::Trigger, "trigger"),
            (specialized_core::clipboard::Part::Indicator, "indicator"),
            (specialized_core::clipboard::Part::Status, "status"),
            (specialized_core::clipboard::Part::ValueText, "value-text"),
        ],
    );
}
