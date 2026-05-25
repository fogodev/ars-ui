use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Arbitrary DropZone event sequences must keep the public state and
    /// connect API internally consistent: drag-over state owns the drop-target
    /// marker, named enabled instances expose stored accepted form data, and
    /// disabled/read-only instances never enter drag/drop states.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_drop_zone_event_sequences_preserve_invariants(
        props in arb_drop_zone_props(),
        events in prop::collection::vec(arb_drop_zone_event(), 0..128),
    ) {
        let initially_disabled_or_readonly = props.disabled || props.read_only;

        let mut service = Service::<utility_core::drop_zone::Machine>::new(
            props,
            &Env::default(),
            &utility_core::drop_zone::Messages::default(),
        );

        for event in events {
            drop(service.send(event));

            let attrs = service.connect(&|_| {}).root_attrs();

            let drag_over_attr = attrs.get(&HtmlAttr::Data("ars-drag-over"));

            prop_assert_eq!(
                matches!(service.state(), utility_core::drop_zone::State::DragOver),
                service.context().is_drop_target,
                "DragOver state and is_drop_target must agree"
            );
            prop_assert_eq!(
                matches!(service.state(), utility_core::drop_zone::State::DragOver),
                drag_over_attr == Some("true"),
                "root data-ars-drag-over must track DragOver state"
            );

            if service.state() != &utility_core::drop_zone::State::DragOver {
                prop_assert!(
                    !service.context().valid_drag,
                    "valid_drag must clear outside DragOver"
                );
                prop_assert!(
                    service.context().drag_types.is_empty(),
                    "drag_types must clear outside DragOver"
                );
            }

            if service.props().name.is_none() || service.context().disabled {
                let form_data = service.connect(&|_| {}).form_data().to_vec();

                prop_assert!(
                    form_data.is_empty(),
                    "form_data must be empty for unnamed or disabled instances"
                );
            } else {
                let form_data = service.connect(&|_| {}).form_data().to_vec();

                prop_assert_eq!(
                    form_data.as_slice(),
                    service.context().dropped_items.as_slice(),
                    "form_data must expose stored accepted items for named enabled instances"
                );
            }

            if initially_disabled_or_readonly {
                prop_assert!(
                    !matches!(
                        service.state(),
                        utility_core::drop_zone::State::DragOver
                            | utility_core::drop_zone::State::DropAccepted
                            | utility_core::drop_zone::State::DropRejected
                    ),
                    "disabled/read-only DropZone must ignore drag/drop transitions"
                );
            }
        }
    }
}
