use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_button_event_sequences_preserve_invariants(
        props in arb_button_props(),
        events in prop::collection::vec(arb_button_event(), 0..128),
    ) {
        let mut service = Service::<utility_core::button::Machine>::new(
            props,
            &Env::default(),
            &utility_core::button::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            prop_assert_eq!(ctx.loading, matches!(state, utility_core::button::State::Loading));
            prop_assert_eq!(ctx.pressed, matches!(state, utility_core::button::State::Pressed));
            prop_assert!(
                !ctx.focus_visible || ctx.focused,
                "focus-visible cannot outlive focus"
            );

            if ctx.disabled {
                prop_assert!(!ctx.focused, "disabled button cannot stay focused");
                prop_assert!(
                    !ctx.focus_visible,
                    "disabled button cannot show focus-visible"
                );
                prop_assert!(!ctx.pressed, "disabled button cannot stay pressed");
            }

            if ctx.loading {
                prop_assert!(!ctx.pressed, "loading button cannot stay pressed");
            }

            match state {
                utility_core::button::State::Idle => {
                    prop_assert!(!ctx.focused, "idle button cannot stay focused");
                    prop_assert!(
                        !ctx.focus_visible,
                        "idle button cannot show focus-visible"
                    );
                    prop_assert!(!ctx.pressed, "idle button cannot stay pressed");
                }

                utility_core::button::State::Focused => {
                    prop_assert!(ctx.focused, "focused state requires focused context");
                    prop_assert!(!ctx.pressed, "focused button cannot stay pressed");
                }

                utility_core::button::State::Pressed => {
                    prop_assert!(ctx.pressed, "pressed state requires pressed context");
                    prop_assert!(!ctx.loading, "pressed button cannot be loading");
                }

                utility_core::button::State::Loading => {
                    prop_assert!(ctx.loading, "loading state requires loading context");
                    prop_assert!(!ctx.pressed, "loading button cannot be pressed");
                }
            }
        }
    }
}
