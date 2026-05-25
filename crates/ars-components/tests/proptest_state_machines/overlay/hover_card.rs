use super::*;

fn arb_hover_card_event() -> impl Strategy<Value = core_hover_card::Event> {
    prop_oneof![
        Just(core_hover_card::Event::TriggerPointerEnter),
        Just(core_hover_card::Event::TriggerPointerLeave),
        Just(core_hover_card::Event::TriggerFocus),
        Just(core_hover_card::Event::TriggerBlur),
        Just(core_hover_card::Event::ContentPointerEnter),
        Just(core_hover_card::Event::ContentPointerLeave),
        Just(core_hover_card::Event::OpenTimerFired),
        Just(core_hover_card::Event::CloseTimerFired),
        Just(core_hover_card::Event::CloseOnEscape),
        Just(core_hover_card::Event::TitleMount),
        Just(core_hover_card::Event::Open),
        Just(core_hover_card::Event::Close),
        any::<bool>().prop_map(core_hover_card::Event::SetControlledOpen),
        (0..=4_000u32).prop_map(core_hover_card::Event::SetZIndex),
    ]
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_hover_card_closed_state_never_reports_open(
        events in prop::collection::vec(arb_hover_card_event(), 0..128),
    ) {
        let mut service = Service::<core_hover_card::Machine>::new(
            core_hover_card::Props {
                id: "hover-card-proptest".to_string(),
                ..core_hover_card::Props::default()
            },
            &Env::default(),
            &core_hover_card::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            if matches!(service.state(), core_hover_card::State::Closed) {
                let api = service.connect(&|_| ());

                prop_assert!(!service.context().open);
                prop_assert!(!api.is_open());
            }
        }
    }

}
