use ars_components::specialized::signature_pad::{Event, Machine, Props, State};
use ars_core::{Env, Service};
use proptest::prelude::*;

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (0.0f64..1000.0, 0.0f64..1000.0, 0.0f64..=1.0)
            .prop_map(|(x, y, pressure)| Event::DrawStart { x, y, pressure }),
        (0.0f64..1000.0, 0.0f64..1000.0, 0.0f64..=1.0)
            .prop_map(|(x, y, pressure)| Event::DrawMove { x, y, pressure }),
        Just(Event::DrawEnd),
        Just(Event::Undo),
        Just(Event::Clear),
        Just(Event::Focus),
        Just(Event::Blur),
        Just(Event::SyncData),
        Just(Event::SyncProps),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn signature_event_sequences_keep_invariants(
        events in prop::collection::vec(arb_event(), 0..96),
    ) {
        let mut svc = Service::<Machine>::new(
            Props::new().id("sig").min_distance(0.0),
            &Env::default(),
            &Default::default(),
        );

        for ev in events {
            drop(svc.send(ev));
        }

        let state = *svc.state();
        let ctx = svc.context();

        prop_assert_eq!(ctx.current_stroke.is_some(), state == State::Drawing);

        if state == State::Idle {
            prop_assert!(ctx.data.get().is_empty());
        }

        if state == State::Completed {
            prop_assert!(!ctx.data.get().is_empty());
        }

        for stroke in &ctx.data.get().strokes {
            prop_assert!(stroke.points.len() >= 2);
        }

        let data = ctx.data.get();

        prop_assert_eq!(data.is_empty(), data.point_count() == 0);

        drop(data.to_svg_path());

        let api = svc.connect(&|_| {});

        drop(api.root_attrs());
        drop(api.canvas_attrs());
        drop(api.clear_trigger_attrs());
        drop(api.undo_trigger_attrs());
        drop(api.guide_attrs());
        drop(api.hidden_input_attrs());
    }
}
