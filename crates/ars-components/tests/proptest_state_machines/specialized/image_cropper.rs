use ars_components::specialized::image_cropper::{CropArea, CropHandle, Event, Machine, Props};
use ars_core::{Env, Service};
use proptest::prelude::*;

fn arb_crop_area() -> impl Strategy<Value = CropArea> {
    (0.0f64..0.9, 0.0f64..0.9, -180.0f64..180.0).prop_flat_map(|(x, y, rotation)| {
        (0.05f64..=(1.0 - x), 0.05f64..=(1.0 - y)).prop_map(move |(width, height)| CropArea {
            x,
            y,
            width,
            height,
            rotation,
        })
    })
}

fn arb_handle() -> impl Strategy<Value = CropHandle> {
    prop::sample::select(CropHandle::all().to_vec())
}

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        (0.0f64..1.0, 0.0f64..1.0).prop_map(|(x, y)| Event::DragStart { x, y }),
        (0.0f64..1.0, 0.0f64..1.0).prop_map(|(x, y)| Event::DragMove { x, y }),
        Just(Event::DragEnd),
        (arb_handle(), 0.0f64..1.0, 0.0f64..1.0).prop_map(|(handle, x, y)| Event::ResizeStart {
            handle,
            x,
            y
        }),
        (0.0f64..1.0, 0.0f64..1.0).prop_map(|(x, y)| Event::ResizeMove { x, y }),
        Just(Event::ResizeEnd),
        arb_crop_area().prop_map(Event::SetCropArea),
        (-0.2f64..0.2, -0.2f64..0.2).prop_map(|(dx, dy)| Event::NudgeCrop { dx, dy }),
        (0.5f64..5.0).prop_map(Event::SetZoom),
        (-360.0f64..360.0).prop_map(Event::SetRotation),
        Just(Event::FlipHorizontal),
        Just(Event::FlipVertical),
        Just(Event::Reset),
    ]
}

proptest! {
    #![proptest_config(super::super::common::proptest_config())]

    #[test]
    #[ignore = "proptest - nightly extended-proptest job"]
    fn cropper_event_sequences_keep_geometry_valid(
        events in prop::collection::vec(arb_event(), 0..64),
    ) {
        let mut svc = Service::<Machine>::new(
            Props { id: "ic".into(), ..Props::default() },
            &Env::default(),
            &Default::default(),
        );

        for ev in events {
            drop(svc.send(ev));
        }

        let crop = *svc.context().crop.get();

        const EPS: f64 = 1e-9;

        prop_assert!((0.0..=1.0).contains(&crop.x));
        prop_assert!((0.0..=1.0).contains(&crop.y));
        prop_assert!(crop.width >= 0.05 - EPS && crop.width <= 1.0 + EPS);
        prop_assert!(crop.height >= 0.05 - EPS && crop.height <= 1.0 + EPS);
        prop_assert!(crop.x + crop.width <= 1.0 + EPS);
        prop_assert!(crop.y + crop.height <= 1.0 + EPS);
        prop_assert!((-180.0 - EPS..=180.0 + EPS).contains(&crop.rotation));
        prop_assert!((Props::default().min_zoom..=Props::default().max_zoom)
            .contains(&svc.context().zoom));

        let api = svc.connect(&|_| {});

        drop(api.root_attrs());
        drop(api.crop_area_attrs());
        drop(api.image_attrs());
        drop(api.handle_attrs(CropHandle::TopLeft));
        drop(api.zoom_slider_attrs());
        drop(api.rotation_slider_attrs());
    }
}
