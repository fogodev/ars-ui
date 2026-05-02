use core::time::Duration;

use ars_components::data_display::avatar;
use ars_core::{Env, SafeUrl, Service};
use proptest::prelude::*;

fn arb_avatar_props() -> impl Strategy<Value = avatar::Props> {
    (
        prop::option::of(prop_oneof![
            Just(avatar::ImageSrc::from_safe_url(&SafeUrl::from_static(
                "/avatar.png"
            ))),
            Just(avatar::ImageSrc::from_safe_url(&SafeUrl::from_static(
                "https://example.com/avatar.png"
            ))),
        ]),
        prop::option::of(".*"),
        (0u64..=1_000).prop_map(Duration::from_millis),
    )
        .prop_map(|(src, name, fallback_delay)| {
            let mut props = avatar::Props::new()
                .id("avatar")
                .fallback_delay(fallback_delay);

            props.src = src;
            props.name = name;

            props
        })
}

fn arb_avatar_event() -> impl Strategy<Value = avatar::Event> {
    prop_oneof![
        Just(avatar::Event::ImageLoad),
        Just(avatar::Event::ImageError),
        Just(avatar::Event::FallbackDelayElapsed),
        Just(avatar::Event::SetSrc(None)),
        Just(avatar::Event::SetSrc(Some(
            avatar::ImageSrc::from_safe_url(&SafeUrl::from_static("/next.png"))
        ))),
    ]
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_avatar_event_sequences_preserve_invariants(
        props in arb_avatar_props(),
        events in prop::collection::vec(arb_avatar_event(), 0..64),
    ) {
        let mut service = Service::<avatar::Machine>::new(
            props,
            &Env::default(),
            &avatar::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let api = service.connect(&|_| {});

            match service.state() {
                avatar::State::Loading => {
                    prop_assert_eq!(service.context().loading_status, avatar::LoadingStatus::Loading);
                    prop_assert_eq!(api.is_image_visible(), false);
                    prop_assert_eq!(api.is_fallback_visible(), service.context().fallback_visible);
                }

                avatar::State::Loaded => {
                    prop_assert_eq!(service.context().loading_status, avatar::LoadingStatus::Loaded);
                    prop_assert!(api.is_image_visible());
                    prop_assert!(!api.is_fallback_visible());
                }

                avatar::State::Error => {
                    prop_assert_eq!(service.context().loading_status, avatar::LoadingStatus::Error);
                    prop_assert_eq!(api.is_image_visible(), false);
                    prop_assert!(api.is_fallback_visible());
                }

                avatar::State::Fallback => {
                    prop_assert_eq!(service.context().src.as_ref().map(avatar::ImageSrc::as_str), None);
                    prop_assert_eq!(service.context().loading_status, avatar::LoadingStatus::Error);
                    prop_assert_eq!(api.is_image_visible(), false);
                    prop_assert!(api.is_fallback_visible());
                }
            }
        }
    }
}
