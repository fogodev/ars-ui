use super::{assert_anatomy, specialized_core};

#[test]
fn timer_anatomy_matches_spec() {
    assert_anatomy(
        "timer",
        &[
            (specialized_core::timer::Part::Root, "root"),
            (specialized_core::timer::Part::Label, "label"),
            (specialized_core::timer::Part::Display, "display"),
            (specialized_core::timer::Part::Progress, "progress"),
            (specialized_core::timer::Part::StartTrigger, "start-trigger"),
            (specialized_core::timer::Part::PauseTrigger, "pause-trigger"),
            (specialized_core::timer::Part::ResetTrigger, "reset-trigger"),
            (specialized_core::timer::Part::Separator, "separator"),
        ],
    );
}
