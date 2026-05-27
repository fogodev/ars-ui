//! Property-based tests for the navigation/tabs state machine.
//!
//! Each `proptest!` block is `#[ignore]`d so the default `cargo test`
//! run skips them; the nightly `extended-proptest` job clears the
//! ignore filter and runs them with a higher case count via
//! `PROPTEST_CASES`.

use std::collections::BTreeSet;

use ars_collections::{
    Collection, Key, TreeCollection, TreeItemConfig,
    dnd::{CollectionDropTarget, DropPosition},
    selection,
};
use ars_components::navigation::{
    accordion, pagination, steps,
    tabs::{ActivationMode, Effect, Event, Machine, Messages, Props, State, TabRegistration},
    tree_view,
};
use ars_core::{AriaAttr, Direction, Env, HtmlAttr, Machine as MachineTrait, Orientation, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

// ────────────────────────────────────────────────────────────────────
// Strategies
// ────────────────────────────────────────────────────────────────────

fn arb_key() -> impl Strategy<Value = Key> {
    // Small key universe so collision/registration paths exercise.
    prop_oneof![
        Just(Key::str("a")),
        Just(Key::str("b")),
        Just(Key::str("c")),
        Just(Key::str("d")),
        Just(Key::Int(0)),
        Just(Key::Int(1)),
    ]
}

fn arb_orientation() -> impl Strategy<Value = Orientation> {
    prop_oneof![Just(Orientation::Horizontal), Just(Orientation::Vertical)]
}

fn arb_activation_mode() -> impl Strategy<Value = ActivationMode> {
    prop_oneof![
        Just(ActivationMode::Automatic),
        Just(ActivationMode::Manual),
    ]
}

fn arb_direction() -> impl Strategy<Value = Direction> {
    prop_oneof![
        Just(Direction::Ltr),
        Just(Direction::Rtl),
        Just(Direction::Auto),
    ]
}

fn arb_disabled_keys() -> impl Strategy<Value = BTreeSet<Key>> {
    prop::collection::vec(arb_key(), 0..3).prop_map(|keys| keys.into_iter().collect())
}

fn arb_props() -> impl Strategy<Value = Props> {
    (
        prop::option::of(prop::option::of(arb_key())),
        prop::option::of(arb_key()),
        arb_orientation(),
        arb_activation_mode(),
        arb_direction(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        arb_disabled_keys(),
        any::<bool>(),
    )
        .prop_map(
            |(
                value,
                default_value,
                orientation,
                activation_mode,
                dir,
                loop_focus,
                disallow_empty_selection,
                lazy_mount,
                unmount_on_exit,
                disabled_keys,
                reorderable,
            )| Props {
                id: "tabs".to_string(),
                value,
                default_value,
                orientation,
                activation_mode,
                dir,
                loop_focus,
                disallow_empty_selection,
                lazy_mount,
                unmount_on_exit,
                disabled_keys,
                reorderable,
            },
        )
}

fn arb_tab_registration() -> impl Strategy<Value = TabRegistration> {
    (arb_key(), any::<bool>()).prop_map(|(key, closable)| TabRegistration { key, closable })
}

fn arb_event() -> impl Strategy<Value = Event> {
    prop_oneof![
        arb_key().prop_map(Event::SelectTab),
        arb_key().prop_map(Event::Focus),
        Just(Event::Blur),
        Just(Event::FocusNext),
        Just(Event::FocusPrev),
        Just(Event::FocusFirst),
        Just(Event::FocusLast),
        arb_direction().prop_map(Event::SetDirection),
        prop::collection::vec(arb_tab_registration(), 0..5).prop_map(Event::SetTabs),
        arb_key().prop_map(Event::CloseTab),
        (arb_key(), 0usize..8).prop_map(|(tab, new_index)| Event::ReorderTab { tab, new_index }),
        Just(Event::SyncProps),
    ]
}

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

/// Combined step: either dispatch an event OR re-set props at runtime.
/// Lets the multi-prop interaction proptest exercise consumer-driven
/// prop changes mid-sequence.
#[derive(Clone, Debug)]
enum Step {
    Send(Event),
    SetProps(Props),
}

fn arb_step() -> impl Strategy<Value = Step> {
    prop_oneof![
        // Event dispatch is more common (8) than prop swap (2) so the
        // generated sequences exercise event-driven transitions more
        // densely than runtime prop churn.
        8 => arb_event().prop_map(Step::Send),
        2 => arb_props().prop_map(Step::SetProps),
    ]
}

// ────────────────────────────────────────────────────────────────────
// Invariants
// ────────────────────────────────────────────────────────────────────

/// Asserts every cross-cutting invariant the tabs machine must hold
/// after any reachable state. Returns a `TestCaseResult` so callers
/// can `?` propagate the `prop_assert` chain.
fn assert_invariants(service: &Service<Machine>) -> TestCaseResult {
    let ctx = service.context();

    // 1. State::Focused implies focused_tab == Some(tab).
    if let State::Focused { tab } = service.state() {
        prop_assert_eq!(
            ctx.focused_tab.as_ref(),
            Some(tab),
            "State::Focused must keep ctx.focused_tab in sync"
        );
    }

    // 2. value points at None or a key currently in `tabs`. The
    //    invariant only applies AFTER at least one `Event::SetTabs`
    //    populated the registered list — pre-registration `value` is
    //    still whatever `default_value` / `value` was at init time,
    //    and the snap doesn't run from init.
    //
    //    Controlled-Some(k) where k isn't registered is also ALLOWED
    //    even after registration — the consumer drives controlled
    //    values and the machine cannot override them via
    //    `Bindable::set`.
    if !ctx.tabs.is_empty()
        && !ctx.value.is_controlled()
        && let Some(selected) = ctx.value.get().as_ref()
    {
        prop_assert!(
            ctx.tabs.iter().any(|k| k == selected),
            "uncontrolled value {selected:?} not in ctx.tabs (registered: {:?})",
            ctx.tabs
        );
    }

    // 3. focused_tab points at a registered, non-disabled key (or None).
    if let Some(focused) = ctx.focused_tab.as_ref() {
        prop_assert!(
            ctx.tabs.iter().any(|k| k == focused),
            "focused_tab {focused:?} is not in ctx.tabs"
        );
        prop_assert!(
            !ctx.disabled_tabs.contains(focused),
            "focused_tab {focused:?} is disabled"
        );
    }

    // 4. closable_tabs ⊆ tabs.
    for closable_key in &ctx.closable_tabs {
        prop_assert!(
            ctx.tabs.iter().any(|k| k == closable_key),
            "closable_tabs contains unregistered key {closable_key:?}"
        );
    }

    // 5. tabs has no duplicate keys.
    let mut seen = BTreeSet::new();

    for key in &ctx.tabs {
        prop_assert!(
            seen.insert(key.clone()),
            "ctx.tabs contains duplicate key {key:?}"
        );
    }

    Ok(())
}

/// Asserts every tab trigger renders the canonical attribute set
/// regardless of mutation state.
fn assert_tab_attr_shape(service: &Service<Machine>) -> TestCaseResult {
    let api = service.connect(&|_| {});

    let tabs_snapshot = service.context().tabs.clone();

    for key in &tabs_snapshot {
        let attrs = api.tab_attrs(key, false);

        prop_assert_eq!(attrs.get(&HtmlAttr::Role), Some("tab"), "tab role missing");
        prop_assert!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Selected)).is_some(),
            "aria-selected missing"
        );
        prop_assert!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Controls)).is_some(),
            "aria-controls missing"
        );
        prop_assert!(attrs.get(&HtmlAttr::TabIndex).is_some(), "tabindex missing");
    }

    Ok(())
}

// ────────────────────────────────────────────────────────────────────
// proptest! blocks
// ────────────────────────────────────────────────────────────────────

proptest! {
    #![proptest_config(super::common::proptest_config())]

    /// Drive the machine with arbitrary event sequences and assert
    /// none of the cross-cutting invariants break.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_tabs_invariants_hold_after_arbitrary_events(
        props in arb_props(),
        events in prop::collection::vec(arb_event(), 0..32),
    ) {
        let mut service = Service::<Machine>::new(
            props,
            &Env::default(),
            &Messages::default(),
        );

        // Initial state must satisfy invariants.
        assert_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_invariants(&service)?;
        }
    }

    /// `tab_attrs` for every registered tab always renders the
    /// canonical ARIA set (id / role / aria-selected / aria-controls /
    /// tabindex), regardless of selection, focus, disabled, or
    /// reorderable state.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_tab_attrs_always_render_canonical_attrs(
        props in arb_props(),
        events in prop::collection::vec(arb_event(), 0..32),
    ) {
        let mut service = Service::<Machine>::new(
            props,
            &Env::default(),
            &Messages::default(),
        );

        for event in events {
            drop(service.send(event));
        }

        assert_tab_attr_shape(&service)?;
    }

    /// FocusNext / FocusPrev / FocusFirst / FocusLast that produce a
    /// transition always emit `Effect::FocusFocusedTab` and leave
    /// `focused_tab` at a registered, non-disabled key.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_focus_transitions_emit_focus_effect(
        props in arb_props(),
        registrations in prop::collection::vec(arb_tab_registration(), 1..6),
        focus_event in prop_oneof![
            Just(Event::FocusNext),
            Just(Event::FocusPrev),
            Just(Event::FocusFirst),
            Just(Event::FocusLast),
        ],
    ) {
        let mut service = Service::<Machine>::new(
            props,
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::SetTabs(registrations)));

        let result = service.send(focus_event);

        if result.state_changed || result.context_changed {
            // When a focus-movement event causes a transition, the
            // machine must request live focus from the adapter.
            let names = result.pending_effects.iter().map(|e| e.name).collect::<Vec<_>>();

            prop_assert!(
                names.contains(&Effect::FocusFocusedTab),
                "focus transition without FocusFocusedTab effect: {names:?}"
            );

            let ctx = service.context();

            if let Some(focused) = ctx.focused_tab.as_ref() {
                prop_assert!(
                    ctx.tabs.iter().any(|k| k == focused),
                    "focused tab not in registered list"
                );
                prop_assert!(
                    !ctx.disabled_tabs.contains(focused),
                    "focus moved to a disabled tab"
                );
            }
        }
    }

    /// Drive the machine with a mix of events AND runtime prop changes
    /// (`Service::set_props` followed by the `SyncProps` event the
    /// adapter would dispatch via `on_props_changed`). Asserts the same
    /// invariants hold across the more chaotic event-prop interleaving
    /// — covers consumer flows that swap `disabled_keys`,
    /// `orientation`, `dir`, etc. while the user is interacting.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_invariants_hold_across_event_and_prop_interleavings(
        initial_props in arb_props(),
        steps in prop::collection::vec(arb_step(), 0..32),
    ) {
        let mut service = Service::<Machine>::new(
            initial_props,
            &Env::default(),
            &Messages::default(),
        );

        assert_invariants(&service)?;

        for step in steps {
            match step {
                Step::Send(event) => {
                    drop(service.send(event));
                }

                Step::SetProps(new_props) => {
                    let old_props = service.props().clone();

                    let triggered = <Machine as MachineTrait>::on_props_changed(&old_props, &new_props);

                    drop(service.set_props(new_props));

                    // Consumer adapters are expected to forward the
                    // events `on_props_changed` returns. Replay them
                    // here so the machine sees the same sequence the
                    // adapter would dispatch.
                    for event in triggered {
                        drop(service.send(event));
                    }
                }
            }

            assert_invariants(&service)?;
        }
    }

    /// `Api::successor_for_close(k)` and `Api::can_close_tab(k)` are
    /// mutually consistent: when `can_close_tab` returns `false`,
    /// `successor_for_close` returns `None` and the machine still
    /// holds invariants if the consumer ignores the close.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_successor_and_can_close_are_consistent(
        registrations in prop::collection::vec(arb_tab_registration(), 1..6),
        disallow_empty in any::<bool>(),
        target in arb_key(),
    ) {
        let mut service = Service::<Machine>::new(
            Props {
                id: "tabs".to_string(),
                disallow_empty_selection: disallow_empty,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::SetTabs(registrations.clone())));

        let api = service.connect(&|_| {});

        let registered_unique =
            registrations.iter().map(|r| r.key.clone()).collect::<BTreeSet<_>>();

        let in_list = registered_unique.contains(&target);

        let can_close = api.can_close_tab(&target);

        let successor = api.successor_for_close(&target);

        if !in_list {
            prop_assert!(!can_close, "can_close_tab({target:?}) must be false for unregistered key");
            prop_assert!(successor.is_none(), "successor_for_close({target:?}) must be None for unregistered key");
        } else if disallow_empty && registered_unique.len() == 1 {
            prop_assert!(!can_close, "disallow_empty + only-tab must refuse close");
        }

        // Successor (when present) is always a valid registered key
        // distinct from the closing key.
        if let Some(next) = successor.as_ref() {
            prop_assert!(registered_unique.contains(next));
            prop_assert_ne!(next, &target);
        }
    }

    /// Reorder index arithmetic is single-sourced in the agnostic API
    /// used by both keyboard handlers and drag/drop adapters: unregistered
    /// and disabled tabs cannot move, edge moves clamp to `None`, and all
    /// valid moves advance by exactly one DOM index.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_next_reorder_index_matches_registered_non_disabled_bounds(
        registrations in prop::collection::vec(arb_tab_registration(), 1..6),
        disabled_keys in arb_disabled_keys(),
        target in arb_key(),
    ) {
        let mut service = Service::<Machine>::new(
            Props {
                id: "tabs".to_string(),
                reorderable: true,
                disabled_keys,
                ..Props::default()
            },
            &Env::default(),
            &Messages::default(),
        );

        drop(service.send(Event::SetTabs(registrations)));

        let ctx = service.context();

        let index = ctx.tabs.iter().position(|key| key == &target);

        let is_disabled = ctx.disabled_tabs.contains(&target);

        let expected_next = index
            .filter(|_| !is_disabled)
            .and_then(|position| (position + 1 < ctx.tabs.len()).then_some(position + 1));

        let expected_prev = index
            .filter(|_| !is_disabled)
            .and_then(|position| position.checked_sub(1));

        let api = service.connect(&|_| {});

        prop_assert_eq!(api.next_reorder_index(&target, true), expected_next);
        prop_assert_eq!(api.next_reorder_index(&target, false), expected_prev);
    }

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

fn arb_pagination_event() -> impl Strategy<Value = pagination::Event> {
    prop_oneof![
        (0u32..64).prop_map(pagination::Event::GoToPage),
        Just(pagination::Event::NextPage),
        Just(pagination::Event::PrevPage),
        Just(pagination::Event::GoToFirstPage),
        Just(pagination::Event::GoToLastPage),
        (1u32..32).prop_map(|size| pagination::Event::SetPageSize(
            core::num::NonZeroU32::new(size).expect("range starts at one")
        )),
    ]
}

fn arb_pagination_props() -> impl Strategy<Value = pagination::Props> {
    (
        prop::option::of(0u32..64),
        0u32..64,
        1u32..32,
        0u32..256,
        0u32..4,
        1u32..3,
    )
        .prop_map(
            |(page, default_page, page_size, total_items, sibling_count, boundary_count)| {
                let mut props = pagination::Props::new()
                    .id("pagination")
                    .default_page(default_page)
                    .page_size(core::num::NonZeroU32::new(page_size).expect("range starts at one"))
                    .total_items(total_items)
                    .sibling_count(sibling_count)
                    .boundary_count(boundary_count);

                if let Some(page) = page {
                    props = props.page(page);
                }

                props
            },
        )
}

fn arb_step_status() -> impl Strategy<Value = steps::Status> {
    prop_oneof![
        Just(steps::Status::Incomplete),
        Just(steps::Status::Current),
        Just(steps::Status::Complete),
        Just(steps::Status::Error),
    ]
}

fn arb_steps_event() -> impl Strategy<Value = steps::Event> {
    prop_oneof![
        (0u32..12).prop_map(steps::Event::GoToStep),
        Just(steps::Event::NextStep),
        Just(steps::Event::PrevStep),
        (0u32..12).prop_map(steps::Event::CompleteStep),
        (0u32..12, arb_step_status())
            .prop_map(|(step, status)| steps::Event::SetStatus { step, status }),
    ]
}

fn arb_steps_props() -> impl Strategy<Value = steps::Props> {
    (
        prop::option::of(0u32..12),
        0u32..12,
        1u32..12,
        any::<bool>(),
        arb_orientation(),
        prop::collection::vec(arb_step_status(), 0..12),
    )
        .prop_map(
            |(step, default_step, count, linear, orientation, statuses)| {
                let mut props = steps::Props::new()
                    .id("steps")
                    .default_step(default_step)
                    .count(core::num::NonZeroU32::new(count).expect("range starts at one"))
                    .linear(linear)
                    .orientation(orientation)
                    .statuses(statuses)
                    .is_step_skippable(|_| true)
                    .is_step_valid(|_| true);

                if let Some(step) = step {
                    props = props.step(step);
                }

                props
            },
        )
}

fn assert_pagination_invariants(service: &Service<pagination::Machine>) -> TestCaseResult {
    let ctx = service.context();
    let page = *ctx.page.get();

    prop_assert!(page >= 1);
    prop_assert!(page <= ctx.page_count);

    let mut previous = None;

    for entry in ctx.page_range().into_iter().flatten() {
        if let Some(previous) = previous {
            prop_assert!(entry > previous, "page range must be strictly increasing");
        }

        previous = Some(entry);
    }

    Ok(())
}

fn assert_steps_invariants(service: &Service<steps::Machine>) -> TestCaseResult {
    let ctx = service.context();
    let step = *ctx.step.get();

    prop_assert!(step < ctx.count.get());
    prop_assert_eq!(ctx.statuses.len(), ctx.count.get() as usize);
    let current_positions = ctx
        .statuses
        .iter()
        .enumerate()
        .filter_map(|(index, status)| (*status == steps::Status::Current).then_some(index as u32))
        .collect::<Vec<_>>();

    prop_assert!(
        current_positions.len() <= 1,
        "steps must not keep multiple current statuses"
    );

    if let Some(current_position) = current_positions.first() {
        prop_assert_eq!(*current_position, step);
    }

    Ok(())
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    /// Pagination keeps page state within the derived one-based bounds.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_pagination_page_bounds_and_ranges_hold(
        props in arb_pagination_props(),
        events in prop::collection::vec(arb_pagination_event(), 0..64),
    ) {
        let mut service = Service::<pagination::Machine>::new(
            props,
            &Env::default(),
            &pagination::Messages::default(),
        );

        assert_pagination_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_pagination_invariants(&service)?;
        }
    }

    /// Steps keeps the current index in range and exactly one current status.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_steps_current_status_invariants_hold(
        props in arb_steps_props(),
        events in prop::collection::vec(arb_steps_event(), 0..64),
    ) {
        let mut service = Service::<steps::Machine>::new(
            props,
            &Env::default(),
            &steps::Messages::default(),
        );

        assert_steps_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_steps_invariants(&service)?;
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// TreeView
// ────────────────────────────────────────────────────────────────────

fn tv_item(label: &str) -> tree_view::TreeItem {
    tree_view::TreeItem {
        label: label.to_string(),
        ..tree_view::TreeItem::default()
    }
}

/// Fixed shape so event keys map to known nodes:
/// ```text
/// 1: Alpha (branch)
///   2: Beta
///   3: Gamma
/// 4: Delta
/// ```
fn tv_items() -> TreeCollection<tree_view::TreeItem> {
    TreeCollection::new(vec![
        TreeItemConfig {
            key: Key::int(1),
            text_value: "Alpha".to_string(),
            value: tv_item("Alpha"),
            children: vec![
                TreeItemConfig {
                    key: Key::int(2),
                    text_value: "Beta".to_string(),
                    value: tv_item("Beta"),
                    children: Vec::new(),
                    default_expanded: false,
                },
                TreeItemConfig {
                    key: Key::int(3),
                    text_value: "Gamma".to_string(),
                    value: tv_item("Gamma"),
                    children: Vec::new(),
                    default_expanded: false,
                },
            ],
            default_expanded: false,
        },
        TreeItemConfig {
            key: Key::int(4),
            text_value: "Delta".to_string(),
            value: tv_item("Delta"),
            children: Vec::new(),
            default_expanded: false,
        },
    ])
}

fn tv_key() -> impl Strategy<Value = Key> {
    prop_oneof![
        Just(Key::int(1)),
        Just(Key::int(2)),
        Just(Key::int(3)),
        Just(Key::int(4)),
        Just(Key::int(99)), // unknown key — exercises missing-node paths
    ]
}

fn tv_mode() -> impl Strategy<Value = selection::Mode> {
    prop_oneof![
        Just(selection::Mode::None),
        Just(selection::Mode::Single),
        Just(selection::Mode::Multiple),
    ]
}

fn tv_position() -> impl Strategy<Value = DropPosition> {
    prop_oneof![
        Just(DropPosition::Before),
        Just(DropPosition::On),
        Just(DropPosition::After),
    ]
}

fn arb_tree_view_props() -> impl Strategy<Value = tree_view::Props> {
    (tv_mode(), any::<bool>(), any::<bool>(), any::<bool>()).prop_map(
        |(mode, multiple, dnd, expand_alpha)| {
            let mut expanded = BTreeSet::new();

            if expand_alpha {
                expanded.insert(Key::int(1));
            }

            tree_view::Props::new()
                .id("tree")
                .items(tv_items())
                .selection_mode(mode)
                .multiple(multiple)
                .dnd_enabled(dnd)
                .default_expanded(expanded)
        },
    )
}

fn arb_tree_view_event() -> impl Strategy<Value = tree_view::Event> {
    prop_oneof![
        tv_key().prop_map(tree_view::Event::ExpandNode),
        tv_key().prop_map(tree_view::Event::CollapseNode),
        tv_key().prop_map(tree_view::Event::ToggleNode),
        tv_key().prop_map(tree_view::Event::SelectNode),
        tv_key().prop_map(tree_view::Event::DeselectNode),
        tv_key().prop_map(tree_view::Event::FocusNode),
        Just(tree_view::Event::FocusNext),
        Just(tree_view::Event::FocusPrev),
        Just(tree_view::Event::FocusFirst),
        Just(tree_view::Event::FocusLast),
        Just(tree_view::Event::FocusParent),
        any::<bool>().prop_map(|is_keyboard| tree_view::Event::Focus { is_keyboard }),
        Just(tree_view::Event::Blur),
        (
            prop_oneof![Just('a'), Just('b'), Just('d'), Just('z')],
            0u64..4000
        )
            .prop_map(|(ch, now_ms)| tree_view::Event::TypeaheadSearch(ch, now_ms)),
        Just(tree_view::Event::ClearTypeahead),
        Just(tree_view::Event::ExpandAll),
        Just(tree_view::Event::CollapseAll),
        tv_key().prop_map(tree_view::Event::DragStart),
        (tv_key(), tv_position()).prop_map(|(key, position)| tree_view::Event::DragOver(
            CollectionDropTarget { key, position }
        )),
        Just(tree_view::Event::DragMoveNext),
        Just(tree_view::Event::DragMovePrev),
        Just(tree_view::Event::Drop),
        Just(tree_view::Event::CancelDrag),
        Just(tree_view::Event::SyncProps),
    ]
}

fn tv_is_descendant(
    items: &TreeCollection<tree_view::TreeItem>,
    ancestor: &Key,
    candidate: &Key,
) -> bool {
    let mut current = items
        .get(candidate)
        .and_then(|node| node.parent_key.clone());

    while let Some(parent) = current {
        if &parent == ancestor {
            return true;
        }

        current = items.get(&parent).and_then(|node| node.parent_key.clone());
    }

    false
}

fn assert_tree_view_invariants(service: &Service<tree_view::Machine>) -> TestCaseResult {
    let ctx = service.context();

    // The `selected` binding and the selection state machine never diverge.
    prop_assert_eq!(ctx.selected.get(), &ctx.selection_state.selected_keys);

    match ctx.selection_mode {
        selection::Mode::None => {
            prop_assert!(
                ctx.selected.get().is_empty(),
                "selection mode None must never accumulate selection"
            );
        }

        selection::Mode::Single => {
            prop_assert!(
                ctx.selected.get().len() <= 1,
                "single selection mode must keep at most one selected key"
            );
        }

        selection::Mode::Multiple => {}
    }

    // A drop target only exists during an active, dnd-enabled drag, and is
    // always a cycle-free target.
    if let Some(target) = &ctx.drop_target {
        let dragging = ctx
            .dragging
            .as_ref()
            .expect("drop target requires an active drag");

        prop_assert!(&target.key != dragging, "cannot drop a node onto itself");
        prop_assert!(
            !tv_is_descendant(&ctx.items, dragging, &target.key),
            "cannot drop a node into its own descendant"
        );
    }

    if ctx.dragging.is_some() {
        prop_assert!(
            service.props().dnd_enabled,
            "a drag can only begin when dnd is enabled"
        );
    }

    // ARIA shape: the root is always a tree, every visible node a treeitem.
    let api = service.connect(&|_| {});

    let root_attrs = api.root_attrs();

    prop_assert_eq!(root_attrs.get(&HtmlAttr::Role), Some("tree"));

    for key in ctx.items.visible_keys_with_expanded(ctx.expanded.get()) {
        let is_branch = ctx.items.get(&key).is_some_and(|node| node.has_children);

        let attrs = if is_branch {
            api.branch_attrs(&key)
        } else {
            api.leaf_attrs(&key)
        };

        prop_assert_eq!(attrs.get(&HtmlAttr::Role), Some("treeitem"));
    }

    Ok(())
}

proptest! {
    #![proptest_config(super::common::proptest_config())]

    /// TreeView keeps selection state consistent and drag/drop invariants
    /// (cycle-free, dnd-gated) under arbitrary event sequences.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_tree_view_invariants_hold(
        props in arb_tree_view_props(),
        events in prop::collection::vec(arb_tree_view_event(), 0..64),
    ) {
        let mut service = Service::<tree_view::Machine>::new(
            props,
            &Env::default(),
            &tree_view::Messages::default(),
        );

        assert_tree_view_invariants(&service)?;

        for event in events {
            drop(service.send(event));

            assert_tree_view_invariants(&service)?;
        }
    }
}
