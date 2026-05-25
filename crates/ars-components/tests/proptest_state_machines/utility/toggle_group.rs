use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_toggle_group_event_sequences_preserve_invariants(
        props in arb_toggle_group_props(),
        events in prop::collection::vec(arb_toggle_group_event(), 0..128),
    ) {
        let mut service = Service::<utility_core::toggle_group::Machine>::new(
            props,
            &Env::default(),
            &utility_core::toggle_group::Messages::default(),
        );

        for event in events {
            let before_value = service.context().value.get().clone();
            let before_disabled = service.context().disabled;
            let before_read_only = service.props().read_only;

            let value_item_event = matches!(
                event,
                utility_core::toggle_group::Event::SelectItem(_)
                    | utility_core::toggle_group::Event::DeselectItem(_)
                    | utility_core::toggle_group::Event::ToggleItem(_)
            );

            drop(service.send(event));

            let state = service.state();
            let ctx = service.context();

            match state {
                utility_core::toggle_group::State::Idle => {
                    prop_assert!(ctx.focused_item.is_none());
                    prop_assert!(!ctx.focus_visible);
                }

                utility_core::toggle_group::State::Focused { item } => {
                    prop_assert_eq!(ctx.focused_item.as_ref(), Some(item));
                    prop_assert!(
                        ctx.registered_items.iter().any(|registered| registered == item),
                        "focused item must be registered"
                    );
                    prop_assert!(
                        !ctx.disabled_items.contains(item),
                        "focused item must not be item-disabled"
                    );
                }
            }

            match ctx.selection_mode {
                utility_core::toggle_group::SelectionMode::None => {
                    prop_assert!(ctx.value.get().is_empty(), "none mode cannot select");
                }

                utility_core::toggle_group::SelectionMode::Single => {
                    prop_assert!(ctx.value.get().len() <= 1, "single mode selects at most one");
                }

                utility_core::toggle_group::SelectionMode::Multiple => {}
            }

            if before_disabled && value_item_event {
                prop_assert_eq!(
                    ctx.value.get(),
                    &before_value,
                    "disabled group cannot change selection from item value events"
                );
            }

            if before_read_only && value_item_event {
                prop_assert_eq!(
                    ctx.value.get(),
                    &before_value,
                    "read-only group cannot change selection from item events"
                );
            }

            let registered = ctx.registered_items.iter().collect::<BTreeSet<_>>();

            prop_assert_eq!(
                registered.len(),
                ctx.registered_items.len(),
                "registered item list must be deduplicated"
            );

            if let Some(focused) = &ctx.focused_item {
                prop_assert!(ctx.registered_items.iter().any(|item| item == focused));
                prop_assert!(!ctx.disabled_items.contains(focused));
            }

            if ctx.roving_focus {
                let enabled = ctx
                    .registered_items
                    .iter()
                    .filter(|item| !ctx.disabled_items.contains(*item))
                    .collect::<Vec<_>>();

                if !enabled.is_empty() {
                    let api = service.connect(&|_| {});

                    let zero_count = enabled
                        .iter()
                        .filter(|item| {
                            api.item_attrs(item)
                                .get(&HtmlAttr::TabIndex)
                                .is_some_and(|value| value == "0")
                        })
                        .count();

                    prop_assert_eq!(
                        zero_count,
                        1,
                        "exactly one enabled item anchors roving tabindex"
                    );
                }
            }
        }
    }
}
