use super::*;

fn arb_resize_handle() -> impl Strategy<Value = core_floating_panel::ResizeHandle> {
    prop_oneof![
        Just(core_floating_panel::ResizeHandle::N),
        Just(core_floating_panel::ResizeHandle::S),
        Just(core_floating_panel::ResizeHandle::E),
        Just(core_floating_panel::ResizeHandle::W),
        Just(core_floating_panel::ResizeHandle::NE),
        Just(core_floating_panel::ResizeHandle::NW),
        Just(core_floating_panel::ResizeHandle::SE),
        Just(core_floating_panel::ResizeHandle::SW),
    ]
}

fn arb_floating_panel_event() -> impl Strategy<Value = core_floating_panel::Event> {
    prop_oneof![
        Just(core_floating_panel::Event::DragStart),
        (-500.0f64..=500.0, -500.0f64..=500.0)
            .prop_map(|(dx, dy)| { core_floating_panel::Event::DragMove { dx, dy } }),
        Just(core_floating_panel::Event::DragEnd),
        arb_resize_handle().prop_map(core_floating_panel::Event::ResizeStart),
        (-500.0f64..=500.0, -500.0f64..=500.0)
            .prop_map(|(dx, dy)| { core_floating_panel::Event::ResizeMove { dx, dy } }),
        Just(core_floating_panel::Event::ResizeEnd),
        Just(core_floating_panel::Event::Minimize),
        Just(core_floating_panel::Event::Maximize(
            core_floating_panel::MaximizeMetrics {
                viewport: core_floating_panel::ViewportRect {
                    x: 0.0,
                    y: 0.0,
                    width: 1024.0,
                    height: 768.0,
                },
            }
        )),
        Just(core_floating_panel::Event::Restore),
        Just(core_floating_panel::Event::Close),
        Just(core_floating_panel::Event::BringToFront),
        any::<bool>().prop_map(|is_keyboard| core_floating_panel::Event::Focus { is_keyboard }),
        Just(core_floating_panel::Event::Blur),
        Just(core_floating_panel::Event::CloseOnEscape),
        (0..=4_000u32).prop_map(core_floating_panel::Event::SetZIndex),
        any::<bool>().prop_map(core_floating_panel::Event::SetControlledOpen),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_floating_panel_stage_flags_are_mutually_exclusive_and_size_is_clamped(
        events in prop::collection::vec(arb_floating_panel_event(), 0..128),
    ) {
        let mut service = Service::<core_floating_panel::Machine>::new(
            core_floating_panel::Props {
                id: "floating-panel-proptest".to_string(),
                ..core_floating_panel::Props::default()
            },
            &Env::default(),
            &core_floating_panel::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let ctx = service.context();

            prop_assert!(
                !(ctx.minimized && ctx.maximized),
                "minimized and maximized flags must be mutually exclusive: {ctx:?}",
            );
            prop_assert!(ctx.size.0 >= ctx.min_size.0);
            prop_assert!(ctx.size.1 >= ctx.min_size.1);
            prop_assert!(ctx.size.0 <= ctx.max_size.0);
            prop_assert!(ctx.size.1 <= ctx.max_size.1);
        }
    }
}
