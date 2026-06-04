use ars_components::layout::collapsible;
use ars_core::{AriaAttr, Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

#[derive(Clone, Debug)]
enum CollapsibleStep {
    Send(collapsible::Event),
    SetProps(collapsible::Props),
}

fn arb_collapsible_props() -> impl Strategy<Value = collapsible::Props> {
    (
        prop::option::of(any::<bool>()),
        any::<bool>(),
        any::<bool>(),
        prop::option::of("[1-9][0-9]{0,2}px".prop_map(String::from)),
        prop::option::of("[1-9][0-9]{0,2}px".prop_map(String::from)),
    )
        .prop_map(
            |(open, default_open, disabled, collapsed_height, collapsed_width)| {
                let mut props = collapsible::Props::new()
                    .id("collapsible")
                    .default_open(default_open)
                    .disabled(disabled);

                if let Some(open) = open {
                    props = props.open(open);
                }

                if let Some(collapsed_height) = collapsed_height {
                    props = props.collapsed_height(collapsed_height);
                }

                if let Some(collapsed_width) = collapsed_width {
                    props = props.collapsed_width(collapsed_width);
                }

                props
            },
        )
}

fn arb_collapsible_event() -> impl Strategy<Value = collapsible::Event> {
    prop_oneof![
        Just(collapsible::Event::Toggle),
        any::<bool>().prop_map(collapsible::Event::SetOpen),
        any::<bool>().prop_map(|is_keyboard| collapsible::Event::Focus { is_keyboard }),
        Just(collapsible::Event::Blur),
    ]
}

fn arb_collapsible_step() -> impl Strategy<Value = CollapsibleStep> {
    prop_oneof![
        arb_collapsible_event().prop_map(CollapsibleStep::Send),
        arb_collapsible_props().prop_map(CollapsibleStep::SetProps),
    ]
}

fn assert_collapsible_invariants(service: &Service<collapsible::Machine>) -> TestCaseResult {
    prop_assert_eq!(
        matches!(service.state(), collapsible::State::Open),
        *service.context().open.get()
    );
    prop_assert_eq!(service.context().ids.id(), "collapsible");
    prop_assert!(!service.context().focus_visible || service.context().focused);

    let api = service.connect(&|_| {});

    let root = api.root_attrs();
    let trigger = api.trigger_attrs();
    let content = api.content_attrs();

    prop_assert_eq!(
        root.get(&HtmlAttr::Data("ars-state")),
        Some(if api.is_open() { "open" } else { "closed" })
    );
    prop_assert_eq!(
        trigger.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
        Some(if api.is_open() { "true" } else { "false" })
    );
    prop_assert_eq!(content.get(&HtmlAttr::Role), Some("region"));

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_collapsible_event_sequences_preserve_invariants(
        props in arb_collapsible_props(),
        steps in prop::collection::vec(arb_collapsible_step(), 0..128),
    ) {
        let mut service = Service::<collapsible::Machine>::new(
            props,
            &Env::default(),
            &collapsible::Messages::default(),
        );

        assert_collapsible_invariants(&service)?;

        for step in steps {
            match step {
                CollapsibleStep::Send(event) => {
                    let before_state = *service.state();
                    let before_context = service.context().clone();

                    let result = service.send(event);

                    prop_assert!(result.pending_effects.is_empty());
                    prop_assert!(result.cancel_effects.is_empty());

                    if before_context.disabled
                        && matches!(event, collapsible::Event::Toggle | collapsible::Event::SetOpen(_))
                    {
                        prop_assert_eq!(service.state(), &before_state);
                        prop_assert_eq!(service.context().open.get(), before_context.open.get());
                    }
                }

                CollapsibleStep::SetProps(props) => {
                    let old_id = service.context().ids.id().to_owned();

                    drop(service.set_props(props));

                    prop_assert_eq!(service.context().ids.id(), old_id);
                }
            }

            assert_collapsible_invariants(&service)?;
        }
    }
}
