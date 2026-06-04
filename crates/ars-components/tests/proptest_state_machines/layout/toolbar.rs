use ars_components::layout::toolbar;
use ars_core::{AriaAttr, Direction, Env, HtmlAttr, Orientation, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

#[derive(Clone, Debug)]
enum ToolbarStep {
    Send(toolbar::Event),
    SetProps(toolbar::Props),
}

fn arb_toolbar_props() -> impl Strategy<Value = toolbar::Props> {
    (
        prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical),],
        prop_oneof![Just(Direction::Ltr), Just(Direction::Rtl)],
        any::<bool>(),
    )
        .prop_map(|(orientation, dir, disabled)| {
            toolbar::Props::new()
                .id("toolbar")
                .orientation(orientation)
                .dir(dir)
                .disabled(disabled)
        })
}

fn arb_toolbar_event() -> impl Strategy<Value = toolbar::Event> {
    prop_oneof![
        (0usize..8).prop_map(toolbar::Event::FocusItem),
        Just(toolbar::Event::FocusNext),
        Just(toolbar::Event::FocusPrev),
        Just(toolbar::Event::FocusFirst),
        Just(toolbar::Event::FocusLast),
        any::<bool>().prop_map(|is_keyboard| toolbar::Event::Focus { is_keyboard }),
        Just(toolbar::Event::Blur),
        (0usize..8, prop::collection::vec(0usize..8, 0..8)).prop_map(|(count, disabled_items)| {
            toolbar::Event::SetItems {
                count,
                disabled_items,
            }
        },),
    ]
}

fn arb_toolbar_step() -> impl Strategy<Value = ToolbarStep> {
    prop_oneof![
        arb_toolbar_event().prop_map(ToolbarStep::Send),
        arb_toolbar_props().prop_map(ToolbarStep::SetProps),
    ]
}

fn assert_toolbar_invariants(service: &Service<toolbar::Machine>) -> TestCaseResult {
    let ctx = service.context();

    if let Some(index) = ctx.focused_index {
        prop_assert!(index < ctx.item_count);
        prop_assert!(!ctx.disabled_items.contains(&index));
    }

    if !ctx.disabled {
        let first_enabled = (0..ctx.item_count).find(|index| !ctx.disabled_items.contains(index));

        if first_enabled.is_some() {
            prop_assert!(ctx.focused_index.is_some());
        }
    }

    let api = service.connect(&|_| {});

    let root = api.root_attrs();

    prop_assert_eq!(root.get(&HtmlAttr::Role), Some("toolbar"));
    prop_assert_eq!(
        root.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
        Some(match ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical => "vertical",
        })
    );

    for index in 0..ctx.item_count {
        let attrs = api.item_attrs(index);

        let expected_tabindex = if ctx.focused_index == Some(index) {
            "0"
        } else {
            "-1"
        };

        prop_assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some(expected_tabindex));

        if ctx.disabled || ctx.disabled_items.contains(&index) {
            prop_assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        }
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_toolbar_event_sequences_preserve_invariants(
        props in arb_toolbar_props(),
        steps in prop::collection::vec(arb_toolbar_step(), 0..128),
    ) {
        let mut service = Service::<toolbar::Machine>::new(
            props,
            &Env::default(),
            &toolbar::Messages,
        );

        assert_toolbar_invariants(&service)?;

        for step in steps {
            let result = match step {
                ToolbarStep::Send(event) => service.send(event),
                ToolbarStep::SetProps(props) => service.set_props(props),
            };

            prop_assert!(result.pending_effects.is_empty());
            prop_assert!(result.cancel_effects.is_empty());

            assert_toolbar_invariants(&service)?;
        }
    }
}
