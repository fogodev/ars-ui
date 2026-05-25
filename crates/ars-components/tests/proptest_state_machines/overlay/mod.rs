use core::time::Duration;

use ars_a11y::FocusTarget;
use ars_components::overlay::{
    alert_dialog as core_alert_dialog, dialog as core_dialog, drawer as core_drawer,
    floating_panel as core_floating_panel, hover_card as core_hover_card, popover as core_popover,
    positioning::{ArrowOffset, Offset, Placement, PositioningOptions, PositioningSnapshot},
    presence as core_presence,
    toast::{manager as core_toast_manager, single as core_toast_single},
    tooltip as core_tooltip, tour as core_tour,
};
use ars_core::{Direction, Env, SendResult, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

const MIN_TOUCH_AUTO_HIDE: Duration = Duration::from_secs(5);

mod alert_dialog;
mod dialog;
mod drawer;
mod floating_panel;
mod hover_card;
mod popover;
mod presence;
mod toast_manager;
mod toast_single;
mod tooltip;
mod tour;

fn arb_direction() -> impl Strategy<Value = Direction> {
    prop_oneof![Just(Direction::Ltr), Just(Direction::Rtl)]
}

fn arb_placement() -> impl Strategy<Value = Placement> {
    prop_oneof![
        Just(Placement::Bottom),
        Just(Placement::BottomStart),
        Just(Placement::BottomEnd),
        Just(Placement::Top),
        Just(Placement::TopStart),
        Just(Placement::TopEnd),
        Just(Placement::Left),
        Just(Placement::LeftStart),
        Just(Placement::LeftEnd),
        Just(Placement::Right),
        Just(Placement::RightStart),
        Just(Placement::RightEnd),
        Just(Placement::Auto),
        Just(Placement::AutoStart),
        Just(Placement::AutoEnd),
        Just(Placement::Start),
        Just(Placement::End),
        Just(Placement::StartTop),
        Just(Placement::StartBottom),
        Just(Placement::EndTop),
        Just(Placement::EndBottom),
    ]
}

fn arb_duration(max_millis: u64) -> impl Strategy<Value = Duration> {
    (0..=max_millis).prop_map(Duration::from_millis)
}

fn arb_positioning_options() -> impl Strategy<Value = PositioningOptions> {
    (
        arb_placement(),
        -16.0f64..=16.0,
        -16.0f64..=16.0,
        any::<bool>(),
        any::<bool>(),
        0.0f64..=32.0,
        0.0f64..=32.0,
        any::<bool>(),
        prop::collection::vec(arb_placement(), 0..4),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(
                placement,
                main_axis,
                cross_axis,
                flip,
                shift,
                shift_padding,
                arrow_padding,
                auto_max_size,
                fallback_placements,
                keyboard_aware,
                auto_placement,
            )| PositioningOptions {
                placement,
                offset: Offset {
                    main_axis,
                    cross_axis,
                },
                flip,
                shift,
                shift_padding,
                arrow_padding,
                auto_max_size,
                fallback_placements,
                keyboard_aware,
                auto_placement,
            },
        )
}

fn arb_arrow_offset() -> impl Strategy<Value = Option<ArrowOffset>> {
    prop_oneof![
        Just(None),
        (-32.0f64..=32.0, -32.0f64..=32.0).prop_map(|(main_axis, cross_axis)| Some(ArrowOffset {
            main_axis,
            cross_axis,
        })),
    ]
}

fn arb_positioning_snapshot() -> impl Strategy<Value = PositioningSnapshot> {
    (arb_placement(), arb_arrow_offset())
        .prop_map(|(placement, arrow)| PositioningSnapshot { placement, arrow })
}

fn arb_focus_target() -> impl Strategy<Value = Option<FocusTarget>> {
    prop_oneof![
        Just(None),
        Just(Some(FocusTarget::First)),
        Just(Some(FocusTarget::Last)),
        Just(Some(FocusTarget::AutofocusMarked)),
        Just(Some(FocusTarget::PreviouslyActive)),
    ]
}

fn arb_dialog_role() -> impl Strategy<Value = core_dialog::Role> {
    prop_oneof![
        Just(core_dialog::Role::Dialog),
        Just(core_dialog::Role::AlertDialog)
    ]
}

fn arb_toast_kind() -> impl Strategy<Value = core_toast_single::Kind> {
    prop_oneof![
        Just(core_toast_single::Kind::Info),
        Just(core_toast_single::Kind::Success),
        Just(core_toast_single::Kind::Warning),
        Just(core_toast_single::Kind::Error),
        Just(core_toast_single::Kind::Loading),
    ]
}
