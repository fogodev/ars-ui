use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_live_region_event_sequences_preserve_invariants(
        props in arb_live_region_props(),
        events in prop::collection::vec(arb_live_region_event(), 0..128),
    ) {
        let mut service = Service::<utility_core::live_region::Machine>::new(
            props,
            &Env::default(),
            &utility_core::live_region::Messages,
        );

        for event in events {
            let was_clear = matches!(event, utility_core::live_region::Event::Clear);

            let was_rendered = matches!(event, utility_core::live_region::Event::Rendered);

            let was_announcing = matches!(service.state(), utility_core::live_region::State::Announcing);

            let queued_has_urgent = service
                .context()
                .queue
                .iter()
                .any(|queued| queued.priority == utility_core::live_region::AnnouncePriority::Urgent);

            let queued_has_normal = service
                .context()
                .queue
                .iter()
                .any(|queued| queued.priority == utility_core::live_region::AnnouncePriority::Normal);

            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            prop_assert!(
                ctx.pending_message.is_none() || matches!(state, utility_core::live_region::State::Announcing),
                "pending message requires Announcing state"
            );
            prop_assert!(
                ctx.messages.len() <= 1,
                "only one rendered announcement may be present"
            );
            prop_assert!(
                ctx.queue
                    .windows(2)
                    .all(|window| window[0].sequence < window[1].sequence),
                "queue sequence must preserve insertion order"
            );

            if was_clear {
                prop_assert_eq!(state, &utility_core::live_region::State::Idle);
                prop_assert!(ctx.messages.is_empty(), "Clear empties rendered messages");
                prop_assert!(ctx.queue.is_empty(), "Clear empties queued messages");
                prop_assert_eq!(&ctx.pending_message, &None, "Clear drops pending message");
            }

            if was_rendered && was_announcing && queued_has_urgent && queued_has_normal {
                prop_assert_eq!(
                    ctx.current_priority,
                    utility_core::live_region::AnnouncePriority::Urgent,
                    "urgent queued messages are selected before normal messages"
                );
            }
        }
    }
}
