use ars_components::overlay::tour;
use ars_core::{ComponentPart, Env, HtmlAttr, Service};

use crate::helper::assert_anatomy;

#[test]
fn tour_anatomy_matches_spec() {
    assert_anatomy(
        "tour",
        &[
            (tour::Part::Root, "root"),
            (tour::Part::Overlay, "overlay"),
            (tour::Part::Highlight, "highlight"),
            (tour::Part::StepContent, "step-content"),
            (tour::Part::StepTitle, "step-title"),
            (tour::Part::StepDescription, "step-description"),
            (tour::Part::NextTrigger, "next-trigger"),
            (tour::Part::PrevTrigger, "prev-trigger"),
            (tour::Part::SkipTrigger, "skip-trigger"),
            (tour::Part::CloseTrigger, "close-trigger"),
            (tour::Part::Progress, "progress"),
            (tour::Part::StepIndicator { index: 0 }, "step-indicator"),
        ],
    );
}

#[test]
fn tour_step_indicator_attrs_carry_step_index() {
    let service = Service::<tour::Machine>::new(
        tour::Props {
            id: "tour".to_string(),
            steps: vec![tour::Step::default()],
            default_open: true,
            ..tour::Props::default()
        },
        &Env::default(),
        &tour::Messages::default(),
    );

    let api = service.connect(&|_| {});

    let attrs = api.step_indicator_attrs(0);

    assert_eq!(
        tour::Part::StepIndicator { index: 0 }.name(),
        "step-indicator"
    );
    assert_eq!(attrs.get(&HtmlAttr::Data("ars-step")), Some("0"));
}
