use ars_components::layout::splitter;
use ars_core::{Direction, Env, KeyboardKey, Orientation, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

#[derive(Clone, Debug)]
enum SplitterStep {
    Send(splitter::Event),
    SetProps(splitter::Props),
}

fn splitter_panel(id: &'static str, default_size: f64) -> splitter::Panel {
    splitter::Panel {
        id: id.into(),
        min_size: 10.0,
        max_size: Some(90.0),
        default_size,
        collapsible: true,
        collapsed_size: 0.0,
        collapse_threshold: 0.5,
    }
}

fn arb_splitter_keyboard_event() -> impl Strategy<Value = splitter::KeyboardEvent> {
    prop_oneof![
        Just(KeyboardKey::ArrowLeft),
        Just(KeyboardKey::ArrowRight),
        Just(KeyboardKey::ArrowUp),
        Just(KeyboardKey::ArrowDown),
        Just(KeyboardKey::Home),
        Just(KeyboardKey::End),
        Just(KeyboardKey::Enter),
        Just(KeyboardKey::Space),
        Just(KeyboardKey::Escape),
    ]
    .prop_flat_map(|key| {
        any::<bool>().prop_map(move |shift| splitter::KeyboardEvent {
            key,
            shift,
            alt: false,
            ctrl: false,
            meta: false,
        })
    })
}

fn arb_splitter_props() -> impl Strategy<Value = splitter::Props> {
    (
        prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)],
        prop_oneof![Just(Direction::Ltr), Just(Direction::Rtl)],
        prop_oneof![
            Just(splitter::SizeUnit::Percent),
            Just(splitter::SizeUnit::Pixels)
        ],
    )
        .prop_map(|(orientation, dir, size_unit)| {
            splitter::Props::new()
                .id("split")
                .panels(vec![
                    splitter_panel("left", 40.0),
                    splitter_panel("middle", 30.0),
                    splitter_panel("right", 30.0),
                ])
                .orientation(orientation)
                .dir(dir)
                .size_unit(size_unit)
                .default_sizes(vec![40.0, 30.0, 30.0])
        })
}

fn arb_splitter_event() -> impl Strategy<Value = splitter::Event> {
    prop_oneof![
        (0usize..4, -200.0f64..200.0)
            .prop_map(|(handle_index, pos)| { splitter::Event::DragStart { handle_index, pos } }),
        (-200.0f64..200.0).prop_map(|pos| splitter::Event::DragMove { pos }),
        Just(splitter::Event::DragEnd),
        (0usize..4, arb_splitter_keyboard_event()).prop_map(|(handle_index, event)| {
            splitter::Event::KeyDown {
                handle_index,
                event,
            }
        }),
        (0usize..4).prop_map(|handle_index| splitter::Event::HandleFocus { handle_index }),
        Just(splitter::Event::HandleBlur),
        (0usize..4).prop_map(|panel_index| splitter::Event::CollapsePanel { panel_index }),
        (0usize..4).prop_map(|panel_index| splitter::Event::ExpandPanel { panel_index }),
        prop::collection::vec(0.0f64..100.0, 0..5)
            .prop_map(|sizes| splitter::Event::SetSizes { sizes }),
    ]
}

fn arb_splitter_step() -> impl Strategy<Value = SplitterStep> {
    prop_oneof![
        arb_splitter_event().prop_map(SplitterStep::Send),
        arb_splitter_props().prop_map(SplitterStep::SetProps),
    ]
}

fn assert_splitter_invariants(service: &Service<splitter::Machine>) -> TestCaseResult {
    let sizes = service.context().sizes.get();

    prop_assert_eq!(sizes.len(), service.context().panels.len());
    prop_assert!(sizes.iter().all(|size| size.is_finite()));
    prop_assert!(service.context().keyboard_step.is_finite());
    prop_assert!(service.context().drag_scale_factor.is_finite());

    if let Some(handle_index) = service.context().focused_handle {
        prop_assert!(handle_index + 1 < sizes.len());
    }

    if let splitter::State::Dragging { handle_index } = service.state() {
        prop_assert!(*handle_index + 1 < sizes.len());
        prop_assert_eq!(service.context().drag_start_sizes.len(), sizes.len());
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_splitter_event_sequences_preserve_invariants(
        props in arb_splitter_props(),
        steps in prop::collection::vec(arb_splitter_step(), 0..128),
    ) {
        let mut service = Service::<splitter::Machine>::new(
            props,
            &Env::default(),
            &splitter::Messages::default(),
        );

        assert_splitter_invariants(&service)?;

        for step in steps {
            match step {
                SplitterStep::Send(event) => {
                    drop(service.send(event));
                }

                SplitterStep::SetProps(props) => {
                    drop(service.set_props(props));
                }
            }

            assert_splitter_invariants(&service)?;
        }
    }
}
