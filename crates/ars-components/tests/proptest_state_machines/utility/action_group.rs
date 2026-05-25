use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_action_group_event_sequences_preserve_invariants(
        props in arb_action_group_props(),
        events in prop::collection::vec(arb_action_group_event(), 0..128),
    ) {
        let mut service = Service::<utility_core::action_group::Machine>::new(
            props,
            &Env::default(),
            &utility_core::action_group::Messages::default(),
        );

        for event in events {
            let before_selected = service.context().selected_items.clone();
            let before_disabled = service.context().disabled;

            let value_event = matches!(
                event,
                utility_core::action_group::Event::ActivateItem(_) | utility_core::action_group::Event::SelectItem(_)
            );

            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            match state {
                utility_core::action_group::State::Idle => {
                    prop_assert!(ctx.focused_item.is_none());
                }

                utility_core::action_group::State::Focused { item } => {
                    prop_assert_eq!(ctx.focused_item.as_ref(), Some(item));
                    prop_assert!(
                        ctx.registered_items.iter().any(|registered| registered == item),
                        "focused item must be registered"
                    );
                    prop_assert!(
                        !service.props().disabled_items.contains(item),
                        "focused item must not be item-disabled"
                    );
                }
            }

            match service.props().selection_mode {
                selection::Mode::None => {
                    prop_assert!(ctx.selected_items.is_empty(), "none mode cannot select");
                }

                selection::Mode::Single => {
                    prop_assert!(
                        ctx.selected_items.len() <= 1,
                        "single mode selects at most one"
                    );
                }

                selection::Mode::Multiple => {}
            }

            if ctx.overflow_count <= ctx.registered_items.len() {
                prop_assert_eq!(
                    ctx.visible_count + ctx.overflow_count,
                    ctx.registered_items.len(),
                    "visible plus overflowed items should cover registered items"
                );
            } else {
                prop_assert_eq!(
                    ctx.visible_count,
                    0,
                    "overflow beyond the registered count saturates visible count at zero"
                );
            }

            let registered = ctx.registered_items.iter().collect::<BTreeSet<_>>();

            prop_assert_eq!(
                registered.len(),
                ctx.registered_items.len(),
                "registered item list must be deduplicated"
            );

            if before_disabled && value_event {
                prop_assert_eq!(
                    &ctx.selected_items,
                    &before_selected,
                    "disabled action group cannot change selection from value events"
                );
            }
        }
    }
}
