//! Spec-conformance tests for `crates/ars-components/src/overlay/*`.
//!
//! Each test pulls the expected anatomy from the corresponding component
//! spec's §3 anatomy table and asserts the impl's `Part` enum matches.

use ars_components::overlay::{
    alert_dialog, drawer, floating_panel, hover_card,
    toast::{manager as toast_manager, single as toast_single},
    tour,
};
use ars_core::{ComponentPart, Env, HtmlAttr, Service};

use super::helper::assert_anatomy;

#[test]
fn alert_dialog_anatomy_matches_spec() {
    assert_anatomy(
        "alert-dialog",
        &[
            (alert_dialog::Part::Root, "root"),
            (alert_dialog::Part::Trigger, "trigger"),
            (alert_dialog::Part::Backdrop, "backdrop"),
            (alert_dialog::Part::Positioner, "positioner"),
            (alert_dialog::Part::Content, "content"),
            (alert_dialog::Part::Title, "title"),
            (alert_dialog::Part::Description, "description"),
            (alert_dialog::Part::CancelTrigger, "cancel-trigger"),
            (alert_dialog::Part::ActionTrigger, "action-trigger"),
            (alert_dialog::Part::CloseTrigger, "close-trigger"),
        ],
    );
}

#[test]
fn toast_single_anatomy_matches_spec() {
    assert_anatomy(
        "toast",
        &[
            (toast_single::Part::Root, "root"),
            (toast_single::Part::Title, "title"),
            (toast_single::Part::Description, "description"),
            (
                toast_single::Part::ActionTrigger {
                    alt_text: String::new(),
                },
                "action-trigger",
            ),
            (toast_single::Part::CloseTrigger, "close-trigger"),
            (toast_single::Part::ProgressBar, "progress-bar"),
        ],
    );
}

#[test]
fn toast_manager_anatomy_matches_spec() {
    // Manager's enumerable Part is just `Root`. The polite/assertive
    // `aria-live` region shells stamp `data-ars-scope="toast"` (NOT
    // `toast-provider`) — see spec §3 and `manager::region_attrs` —
    // because they belong to the per-toast surface conceptually. The
    // region attrs are exercised by the `manager::tests` snapshot
    // suite, not enumerated through `Part::all()`.
    assert_anatomy("toast-provider", &[(toast_manager::Part::Root, "root")]);
}

#[test]
fn hover_card_anatomy_matches_spec() {
    assert_anatomy(
        "hover-card",
        &[
            (hover_card::Part::Root, "root"),
            (hover_card::Part::Trigger, "trigger"),
            (hover_card::Part::Positioner, "positioner"),
            (hover_card::Part::Content, "content"),
            (hover_card::Part::Arrow, "arrow"),
            (hover_card::Part::Title, "title"),
            (hover_card::Part::DismissButton, "dismiss-button"),
        ],
    );
}

#[test]
fn drawer_anatomy_matches_spec() {
    assert_anatomy(
        "drawer",
        &[
            (drawer::Part::Root, "root"),
            (drawer::Part::Trigger, "trigger"),
            (drawer::Part::Backdrop, "backdrop"),
            (drawer::Part::Positioner, "positioner"),
            (drawer::Part::Content, "content"),
            (drawer::Part::Title, "title"),
            (drawer::Part::Description, "description"),
            (drawer::Part::Header, "header"),
            (drawer::Part::Body, "body"),
            (drawer::Part::Footer, "footer"),
            (drawer::Part::CloseTrigger, "close-trigger"),
            (drawer::Part::DragHandle, "drag-handle"),
        ],
    );
}

#[test]
fn floating_panel_anatomy_matches_spec() {
    assert_anatomy(
        "floating-panel",
        &[
            (floating_panel::Part::Root, "root"),
            (floating_panel::Part::Header, "header"),
            (floating_panel::Part::DragHandle, "drag-handle"),
            (floating_panel::Part::Title, "title"),
            (floating_panel::Part::Content, "content"),
            (floating_panel::Part::Footer, "footer"),
            (
                floating_panel::Part::ResizeHandle {
                    handle: floating_panel::ResizeHandle::N,
                },
                "resize-handle",
            ),
            (floating_panel::Part::CloseTrigger, "close-trigger"),
            (floating_panel::Part::MinimizeTrigger, "minimize-trigger"),
            (floating_panel::Part::MaximizeTrigger, "maximize-trigger"),
            (floating_panel::Part::StageTrigger, "stage-trigger"),
        ],
    );
}

#[test]
fn floating_panel_all_resize_handles_emit_handle_attrs() {
    let service = Service::<floating_panel::Machine>::new(
        floating_panel::Props {
            id: "floating-panel".to_string(),
            ..floating_panel::Props::default()
        },
        &Env::default(),
        &floating_panel::Messages::default(),
    );
    let api = service.connect(&|_| {});

    for handle in floating_panel::ResizeHandle::ALL {
        let part = floating_panel::Part::ResizeHandle { handle };

        let attrs = api.resize_handle_attrs(handle);

        assert_eq!(part.name(), "resize-handle");
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-handle")),
            Some(handle.as_str())
        );
    }
}

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
