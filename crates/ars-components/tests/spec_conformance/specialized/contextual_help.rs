use super::{assert_anatomy, specialized_core};

#[test]
fn contextual_help_anatomy_matches_spec() {
    assert_anatomy(
        "contextual-help",
        &[
            (specialized_core::contextual_help::Part::Root, "root"),
            (specialized_core::contextual_help::Part::Trigger, "trigger"),
            (specialized_core::contextual_help::Part::Content, "content"),
            (specialized_core::contextual_help::Part::Heading, "heading"),
            (specialized_core::contextual_help::Part::Body, "body"),
            (specialized_core::contextual_help::Part::Footer, "footer"),
            (
                specialized_core::contextual_help::Part::DismissButton,
                "dismiss-button",
            ),
        ],
    );
}
