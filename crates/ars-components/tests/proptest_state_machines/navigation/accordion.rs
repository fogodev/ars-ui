//! Property-based tests for the `navigation/accordion` state machine.

use std::collections::BTreeSet;

use ars_components::navigation::accordion;
use ars_core::{AriaAttr, Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

use super::{arb_direction, arb_key, arb_orientation};

fn arb_accordion_registration() -> impl Strategy<Value = accordion::ItemRegistration> {
    (arb_key(), any::<bool>())
        .prop_map(|(key, disabled)| accordion::ItemRegistration { key, disabled })
}

fn arb_accordion_props() -> impl Strategy<Value = accordion::Props> {
    (
        prop::collection::vec(arb_key(), 0..4),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        arb_orientation(),
        arb_direction(),
    )
        .prop_map(
            |(default_keys, multiple, collapsible, disabled, orientation, dir)| {
                accordion::Props::new()
                    .id("accordion")
                    .default_value(default_keys.into_iter().collect())
                    .multiple(multiple)
                    .collapsible(collapsible)
                    .disabled(disabled)
                    .orientation(orientation)
                    .dir(dir)
            },
        )
}

fn arb_accordion_event() -> impl Strategy<Value = accordion::Event> {
    prop_oneof![
        arb_key().prop_map(accordion::Event::ExpandItem),
        arb_key().prop_map(accordion::Event::CollapseItem),
        arb_key().prop_map(accordion::Event::ToggleItem),
        Just(accordion::Event::ExpandAll),
        Just(accordion::Event::CollapseAll),
        arb_key().prop_map(accordion::Event::Focus),
        Just(accordion::Event::Blur),
        Just(accordion::Event::FocusNext),
        Just(accordion::Event::FocusPrev),
        Just(accordion::Event::FocusFirst),
        Just(accordion::Event::FocusLast),
        prop::collection::vec(arb_accordion_registration(), 0..6)
            .prop_map(accordion::Event::SetItems),
        Just(accordion::Event::SyncProps),
    ]
}

fn assert_accordion_invariants(service: &Service<accordion::Machine>) -> TestCaseResult {
    let ctx = service.context();

    let mut seen = BTreeSet::new();

    for item in &ctx.items {
        prop_assert!(
            seen.insert(item.clone()),
            "duplicate registered item {item:?}"
        );
    }

    if !ctx.multiple {
        prop_assert!(
            ctx.value.get().len() <= 1,
            "single accordion has multiple open values: {:?}",
            ctx.value.get()
        );
    }

    if let Some(focused) = &ctx.focused_item {
        prop_assert!(ctx.items.iter().any(|item| item == focused));
        prop_assert!(
            !ctx.disabled_items.get(focused).copied().unwrap_or(false),
            "focused item {focused:?} is disabled"
        );
    }

    Ok(())
}

fn assert_accordion_trigger_attr_shape(service: &Service<accordion::Machine>) -> TestCaseResult {
    let api = service.connect(&|_| {});

    for item in &service.context().items {
        let attrs = api.item_trigger_attrs(item, false);

        prop_assert!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Expanded)).is_some(),
            "aria-expanded missing"
        );
        prop_assert!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Controls)).is_some(),
            "aria-controls missing"
        );
    }

    Ok(())
}

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// Accordion keeps its open/focus invariants across arbitrary event
    /// sequences, including item registration changes.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_accordion_invariants_hold_after_arbitrary_events(
        props in arb_accordion_props(),
        events in prop::collection::vec(arb_accordion_event(), 0..32),
    ) {
        let mut service = Service::<accordion::Machine>::new(
            props,
            &Env::default(),
            &accordion::Messages,
        );

        assert_accordion_invariants(&service)?;

        for event in events {
            drop(service.send(event));
            assert_accordion_invariants(&service)?;
        }
    }

    /// Every registered Accordion trigger exposes the required disclosure
    /// ARIA pair regardless of open, closed, disabled, or focused state.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_accordion_trigger_attrs_always_render_canonical_attrs(
        props in arb_accordion_props(),
        registrations in prop::collection::vec(arb_accordion_registration(), 0..6),
        events in prop::collection::vec(arb_accordion_event(), 0..32),
    ) {
        let mut service = Service::<accordion::Machine>::new(
            props,
            &Env::default(),
            &accordion::Messages,
        );

        drop(service.send(accordion::Event::SetItems(registrations)));

        for event in events {
            drop(service.send(event));
        }

        assert_accordion_trigger_attr_shape(&service)?;
    }
}
