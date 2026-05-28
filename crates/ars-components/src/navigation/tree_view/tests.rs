//! Unit, snapshot, keyboard, and drag-and-drop tests for the `tree_view`
//! machine. Mirrors the spec's "tests to add first" plus snapshot coverage of
//! every anatomy part across its output-affecting state/prop/context branches.

use alloc::{collections::BTreeSet, string::ToString, vec, vec::Vec};
use core::cell::RefCell;
use std::sync::{Arc, Mutex};

use ars_collections::{
    Collection as _, Key, TreeCollection, TreeItemConfig,
    dnd::{CollectionDropTarget, DropPosition},
    selection,
};
use ars_core::{AriaAttr, ConnectApi as _, Env, HtmlAttr, KeyboardKey, Service, StrongSend};
use ars_interactions::KeyboardEventData;
use insta::assert_snapshot;

use super::{
    Effect, Event, Machine, Messages, NodeLoadState, Part, Props, RenameEvent, ReorderEvent,
    TreeItem, snapshot_attrs,
};

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------

fn key(value: u64) -> Key {
    Key::int(value)
}

fn item(label: &str) -> TreeItem {
    TreeItem {
        label: label.to_string(),
        ..TreeItem::default()
    }
}

fn leaf(key: u64, label: &str) -> TreeItemConfig<TreeItem> {
    TreeItemConfig {
        key: Key::int(key),
        text_value: label.to_string(),
        value: item(label),
        children: Vec::new(),
        default_expanded: false,
    }
}

fn leaf_with(key: u64, label: &str, value: TreeItem) -> TreeItemConfig<TreeItem> {
    TreeItemConfig {
        key: Key::int(key),
        text_value: label.to_string(),
        value,
        children: Vec::new(),
        default_expanded: false,
    }
}

fn branch(
    key: u64,
    label: &str,
    expanded: bool,
    children: Vec<TreeItemConfig<TreeItem>>,
) -> TreeItemConfig<TreeItem> {
    TreeItemConfig {
        key: Key::int(key),
        text_value: label.to_string(),
        value: item(label),
        children,
        default_expanded: expanded,
    }
}

/// ```text
/// 1: Fruits (expanded)
///   2: Apple
///   3: Banana
/// 4: Vegetables (collapsed)
///   5: Carrot
///   6: Daikon
/// 7: Grains
/// ```
fn sample_items() -> TreeCollection<TreeItem> {
    TreeCollection::new(vec![
        branch(1, "Fruits", true, vec![leaf(2, "Apple"), leaf(3, "Banana")]),
        branch(
            4,
            "Vegetables",
            false,
            vec![leaf(5, "Carrot"), leaf(6, "Daikon")],
        ),
        leaf(7, "Grains"),
    ])
}

fn props() -> Props {
    Props::new().id("tree").items(sample_items())
}

fn service(props: Props) -> Service<Machine> {
    Service::<Machine>::new(props, &Env::default(), &Messages::default())
}

fn keyboard(key: KeyboardKey) -> KeyboardEventData {
    KeyboardEventData {
        key,
        character: None,
        code: key.as_w3c_str().to_owned(),
        shift_key: false,
        ctrl_key: false,
        alt_key: false,
        meta_key: false,
        repeat: false,
        is_composing: false,
    }
}

fn printable(ch: char) -> KeyboardEventData {
    KeyboardEventData {
        key: KeyboardKey::Unidentified,
        character: Some(ch),
        code: String::new(),
        shift_key: false,
        ctrl_key: false,
        alt_key: false,
        meta_key: false,
        repeat: false,
        is_composing: false,
    }
}

type Recorder = RefCell<Vec<Event>>;

fn record(recorder: &Recorder, event: Event) {
    recorder.borrow_mut().push(event);
}

// ----------------------------------------------------------------------------
// Init
// ----------------------------------------------------------------------------

#[test]
fn init_uncontrolled_uses_defaults() {
    let mut expanded = BTreeSet::new();

    expanded.insert(key(1));

    let service = service(
        props()
            .default_selected(selection::Set::Single(key(2)))
            .default_expanded(expanded.clone()),
    );

    assert!(!service.context().selected.is_controlled());
    assert!(!service.context().expanded.is_controlled());
    assert_eq!(service.context().expanded.get(), &expanded);
    assert!(service.context().selected.get().contains(&key(2)));
    assert_eq!(service.context().focused_node, None);
}

#[test]
fn init_controlled_uses_props() {
    let service = service(
        props()
            .selected(selection::Set::Single(key(3)))
            .expanded(BTreeSet::new()),
    );

    assert!(service.context().selected.is_controlled());
    assert!(service.context().expanded.is_controlled());
}

#[test]
fn init_seeds_selection_state_from_default_selection() {
    // Regression: the selection state machine must agree with the `selected`
    // binding at init, otherwise the first DeselectNode would operate on an
    // empty state and lose the seeded selection.
    let service = service(props().default_selected(selection::Set::Single(key(2))));

    assert!(
        service
            .context()
            .selection_state
            .selected_keys
            .contains(&key(2))
    );
}

// ----------------------------------------------------------------------------
// Controlled prop syncing (on_props_changed / set_props)
// ----------------------------------------------------------------------------

#[test]
fn on_props_changed_emits_sync_only_on_relevant_change() {
    let base = props();

    assert!(<Machine as ars_core::Machine>::on_props_changed(&base, &base).is_empty());

    let reselected = props().selected(selection::Set::Single(key(2)));

    assert_eq!(
        <Machine as ars_core::Machine>::on_props_changed(&base, &reselected),
        vec![Event::SyncProps]
    );
}

#[test]
fn set_props_syncs_controlled_selected() {
    let mut service = service(props().selected(selection::Set::Single(key(2))));

    assert!(service.connect(&|_| {}).is_node_selected(&key(2)));

    drop(service.set_props(props().selected(selection::Set::Single(key(3)))));

    let api = service.connect(&|_| {});

    assert!(!api.is_node_selected(&key(2)));
    assert!(api.is_node_selected(&key(3)));
}

#[test]
fn set_props_syncs_items_after_reorder() {
    // After a reorder the consumer re-supplies a new collection; ctx.items must
    // reflect it (the core never mutates items itself).
    let mut service = service(props());

    assert!(service.connect(&|_| {}).get_node(&key(7)).is_some());

    let reordered = TreeCollection::new(vec![branch(1, "Fruits", true, vec![leaf(2, "Apple")])]);

    drop(service.set_props(props().items(reordered)));

    let api = service.connect(&|_| {});

    assert!(api.get_node(&key(7)).is_none(), "Grains was removed");
    assert!(api.get_node(&key(2)).is_some());
}

// ----------------------------------------------------------------------------
// Expand / collapse / toggle
// ----------------------------------------------------------------------------

#[test]
fn expand_node_adds_to_expanded() {
    let mut service = service(props());

    drop(service.send(Event::ExpandNode(key(4))));

    assert!(service.context().expanded.get().contains(&key(4)));
}

#[test]
fn collapse_node_removes_from_expanded() {
    let mut service = service(props());

    drop(service.send(Event::ExpandNode(key(4))));
    drop(service.send(Event::CollapseNode(key(4))));

    assert!(!service.context().expanded.get().contains(&key(4)));
}

#[test]
fn toggle_node_flips_expansion() {
    let mut service = service(props());

    drop(service.send(Event::ToggleNode(key(4))));

    assert!(service.context().expanded.get().contains(&key(4)));

    drop(service.send(Event::ToggleNode(key(4))));

    assert!(!service.context().expanded.get().contains(&key(4)));
}

#[test]
fn expand_all_reaches_collapsed_subtrees() {
    let mut service = service(props());

    drop(service.send(Event::ExpandAll));

    // Both expandable branches (Fruits, Vegetables) must be expanded, even
    // though Vegetables started collapsed (its descendants were hidden).
    let expanded = service.context().expanded.get().clone();

    assert!(expanded.contains(&key(1)));
    assert!(expanded.contains(&key(4)));
}

#[test]
fn collapse_all_clears_expansion() {
    let mut service = service(props());

    drop(service.send(Event::ExpandAll));
    drop(service.send(Event::CollapseAll));

    assert!(service.context().expanded.get().is_empty());
}

// ----------------------------------------------------------------------------
// Selection
// ----------------------------------------------------------------------------

#[test]
fn select_node_selects() {
    let mut service = service(props());

    drop(service.send(Event::SelectNode(key(2))));

    assert!(service.connect(&|_| {}).is_node_selected(&key(2)));
}

#[test]
fn deselect_node_deselects() {
    let mut service = service(props());

    drop(service.send(Event::SelectNode(key(2))));
    drop(service.send(Event::DeselectNode(key(2))));

    assert!(!service.connect(&|_| {}).is_node_selected(&key(2)));
}

fn disabled_items() -> TreeCollection<TreeItem> {
    TreeCollection::new(vec![
        branch(
            1,
            "Fruits",
            true,
            vec![leaf_with(
                2,
                "Apple",
                TreeItem {
                    disabled: true,
                    ..item("Apple")
                },
            )],
        ),
        leaf(7, "Grains"),
    ])
}

#[test]
fn disabled_node_is_not_selectable() {
    let mut service = service(props().items(disabled_items()));

    drop(service.send(Event::SelectNode(key(2)))); // Apple is disabled

    assert!(!service.connect(&|_| {}).is_node_selected(&key(2)));

    // A non-disabled sibling still selects.
    drop(service.send(Event::SelectNode(key(7))));

    assert!(service.connect(&|_| {}).is_node_selected(&key(7)));
}

#[test]
fn disabled_node_cannot_be_dragged() {
    let mut service = service(props().items(disabled_items()).dnd_enabled(true));

    let result = service.send(Event::DragStart(key(2))); // Apple is disabled

    assert!(!result.context_changed);
    assert_eq!(service.context().dragging, None);
}

#[test]
fn select_ignored_when_selection_mode_none() {
    let mut service = service(props().selection_mode(selection::Mode::None));

    let result = service.send(Event::SelectNode(key(2)));

    assert!(!result.state_changed && !result.context_changed);
    assert!(!service.connect(&|_| {}).is_node_selected(&key(2)));
}

#[test]
fn single_mode_replaces_selection() {
    let mut service = service(props().selection_mode(selection::Mode::Single));

    drop(service.send(Event::SelectNode(key(2))));
    drop(service.send(Event::SelectNode(key(3))));

    let api = service.connect(&|_| {});

    assert!(!api.is_node_selected(&key(2)));
    assert!(api.is_node_selected(&key(3)));
}

#[test]
fn multiple_mode_keeps_selections() {
    let mut service = service(
        props()
            .multiple(true)
            .selection_mode(selection::Mode::Multiple),
    );

    drop(service.send(Event::SelectNode(key(2))));
    drop(service.send(Event::SelectNode(key(3))));

    let api = service.connect(&|_| {});

    assert!(api.is_node_selected(&key(2)));
    assert!(api.is_node_selected(&key(3)));
}

// ----------------------------------------------------------------------------
// Focus navigation
// ----------------------------------------------------------------------------

#[test]
fn focus_first_and_last() {
    let mut service = service(props());

    drop(service.send(Event::FocusFirst));

    assert_eq!(service.context().focused_node, Some(key(1)));

    drop(service.send(Event::FocusLast));

    // Visible (Vegetables collapsed): 1, 2, 3, 4, 7 -> last is Grains (7).
    assert_eq!(service.context().focused_node, Some(key(7)));
}

#[test]
fn focus_next_skips_collapsed_children_and_stops_at_end() {
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(4)))); // Vegetables (collapsed)
    drop(service.send(Event::FocusNext));

    // Carrot/Daikon are hidden; next visible after Vegetables is Grains.
    assert_eq!(service.context().focused_node, Some(key(7)));

    // Per the WAI-ARIA tree pattern, ArrowDown does not wrap: at the last node
    // it is a no-op (Home/End handle boundary jumps), so focus stays on Grains.
    drop(service.send(Event::FocusNext));

    assert_eq!(service.context().focused_node, Some(key(7)));
}

#[test]
fn focus_prev_at_first_node_does_not_wrap() {
    let mut service = service(props());

    drop(service.send(Event::FocusFirst));
    drop(service.send(Event::FocusPrev)); // ArrowUp at the first node is a no-op.

    assert_eq!(service.context().focused_node, Some(key(1)));
}

#[test]
fn focus_parent_moves_to_parent() {
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(2)))); // Apple
    drop(service.send(Event::FocusParent));

    assert_eq!(service.context().focused_node, Some(key(1))); // Fruits
}

#[test]
fn focus_parent_at_root_is_noop() {
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(1))));

    let result = service.send(Event::FocusParent);

    assert!(!result.state_changed && !result.context_changed);
}

#[test]
fn focus_navigation_emits_scroll_effect() {
    let mut service = service(props());

    let result = service.send(Event::FocusFirst);

    assert_eq!(result.pending_effects.len(), 1);
    assert_eq!(
        result.pending_effects[0].name,
        Effect::ScrollFocusedIntoView
    );
}

#[test]
fn focus_sets_focus_visible_and_blur_clears_it() {
    let mut service = service(props());

    drop(service.send(Event::Focus { is_keyboard: true }));

    assert!(service.context().focus_visible);

    drop(service.send(Event::Blur));

    assert!(!service.context().focus_visible);
}

// ----------------------------------------------------------------------------
// Typeahead
// ----------------------------------------------------------------------------

#[test]
fn typeahead_jumps_to_matching_node_case_insensitive() {
    let mut service = service(props());

    drop(service.send(Event::TypeaheadSearch('g', 1))); // Grains

    assert_eq!(service.context().focused_node, Some(key(7)));
}

#[test]
fn typeahead_searches_after_focused_and_wraps() {
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(1)))); // Fruits
    drop(service.send(Event::TypeaheadSearch('b', 1))); // Banana (after Fruits)

    assert_eq!(service.context().focused_node, Some(key(3)));
}

#[test]
fn typeahead_no_match_keeps_focus() {
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(1))));
    drop(service.send(Event::TypeaheadSearch('z', 1))); // no match

    // The buffer updates, but focus does not move.
    assert_eq!(service.context().focused_node, Some(key(1)));
}

#[test]
fn typeahead_skips_nodes_hidden_under_collapsed_parents() {
    // Carrot (5) is hidden under collapsed Vegetables; typing 'c' must not jump
    // to it (the shared matcher only scans visible nodes).
    let mut service = service(props());

    drop(service.send(Event::TypeaheadSearch('c', 1)));

    assert_eq!(service.context().focused_node, None);
}

#[test]
fn typeahead_clear_resets_buffer() {
    let mut service = service(props());

    drop(service.send(Event::TypeaheadSearch('g', 1)));

    assert!(!service.context().typeahead.search.is_empty());

    drop(service.send(Event::ClearTypeahead));

    assert!(service.context().typeahead.search.is_empty());
}

// ----------------------------------------------------------------------------
// Api queries
// ----------------------------------------------------------------------------

#[test]
fn sibling_info_reports_setsize_and_posinset() {
    let service = service(props());

    let api = service.connect(&|_| {});

    // Apple is child 1 of 2 under Fruits.
    assert_eq!(api.sibling_info(&key(2)), (2, 1));
    assert_eq!(api.sibling_info(&key(3)), (2, 2));

    // Root level has 3 nodes (Fruits, Vegetables, Grains).
    assert_eq!(api.sibling_info(&key(1)), (3, 1));
    assert_eq!(api.sibling_info(&key(7)), (3, 3));
}

#[test]
fn loading_label_uses_messages_default() {
    let service = service(props());

    assert_eq!(service.connect(&|_| {}).loading_label(), "Loading\u{2026}");
}

// ----------------------------------------------------------------------------
// Event-dispatch handlers
// ----------------------------------------------------------------------------

#[test]
fn branch_control_click_toggles_and_focuses() {
    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    let service = service(props());

    let api = service.connect(&send);

    api.on_branch_control_click(&key(4));

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::ToggleNode(key(4)), Event::FocusNode(key(4))]
    );
}

#[test]
fn leaf_click_selects_and_focuses() {
    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    let service = service(props());

    let api = service.connect(&send);

    api.on_leaf_click(&key(2));

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::SelectNode(key(2)), Event::FocusNode(key(2))]
    );
}

#[test]
fn api_command_methods_dispatch_events() {
    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    let service = service(props());

    let api = service.connect(&send);

    api.focus_node(&key(2));
    api.expand_all();
    api.collapse_all();

    assert_eq!(
        recorder.borrow().as_slice(),
        &[
            Event::FocusNode(key(2)),
            Event::ExpandAll,
            Event::CollapseAll,
        ]
    );
}

#[test]
fn root_focus_and_blur_dispatch_events() {
    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    let service = service(props());

    let api = service.connect(&send);

    api.on_root_focus(true); // keyboard tab-in
    api.on_root_blur();

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::Focus { is_keyboard: true }, Event::Blur]
    );
}

// ----------------------------------------------------------------------------
// Keyboard
// ----------------------------------------------------------------------------

fn dispatch_key(service: &Service<Machine>, node: &Key, data: &KeyboardEventData) -> Vec<Event> {
    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    service.connect(&send).on_node_keydown(node, data);

    recorder.into_inner()
}

#[test]
fn keydown_arrows_navigate() {
    let service = service(props());

    assert_eq!(
        dispatch_key(&service, &key(1), &keyboard(KeyboardKey::ArrowDown)),
        &[Event::FocusNext]
    );
    assert_eq!(
        dispatch_key(&service, &key(1), &keyboard(KeyboardKey::ArrowUp)),
        &[Event::FocusPrev]
    );
    assert_eq!(
        dispatch_key(&service, &key(1), &keyboard(KeyboardKey::Home)),
        &[Event::FocusFirst]
    );
    assert_eq!(
        dispatch_key(&service, &key(1), &keyboard(KeyboardKey::End)),
        &[Event::FocusLast]
    );
}

#[test]
fn keydown_arrow_right_expands_collapsed_branch_else_enters() {
    let service = service(props());

    // Vegetables (4) is a collapsed branch -> expand it.
    assert_eq!(
        dispatch_key(&service, &key(4), &keyboard(KeyboardKey::ArrowRight)),
        &[Event::ExpandNode(key(4))]
    );
    // Fruits (1) is already expanded -> enter (move to next visible).
    assert_eq!(
        dispatch_key(&service, &key(1), &keyboard(KeyboardKey::ArrowRight)),
        &[Event::FocusNext]
    );
    // A leaf -> inert (nothing to expand or enter; WAI-ARIA tree pattern).
    assert!(dispatch_key(&service, &key(7), &keyboard(KeyboardKey::ArrowRight)).is_empty());
}

#[test]
fn keydown_arrow_left_collapses_expanded_branch_else_moves_to_parent() {
    let service = service(props());

    // Fruits (1) is expanded -> collapse it.
    assert_eq!(
        dispatch_key(&service, &key(1), &keyboard(KeyboardKey::ArrowLeft)),
        &[Event::CollapseNode(key(1))]
    );
    // Vegetables (4) is collapsed -> move to parent.
    assert_eq!(
        dispatch_key(&service, &key(4), &keyboard(KeyboardKey::ArrowLeft)),
        &[Event::FocusParent]
    );
    // A leaf -> move to parent.
    assert_eq!(
        dispatch_key(&service, &key(2), &keyboard(KeyboardKey::ArrowLeft)),
        &[Event::FocusParent]
    );
}

#[test]
fn keydown_enter_selects_and_space_toggles() {
    let multi = || {
        service(
            props()
                .multiple(true)
                .selection_mode(selection::Mode::Multiple),
        )
    };

    // Enter always selects.
    assert_eq!(
        dispatch_key(&multi(), &key(2), &keyboard(KeyboardKey::Enter)),
        &[Event::SelectNode(key(2))]
    );

    // Space on an unselected node selects it.
    assert_eq!(
        dispatch_key(&multi(), &key(2), &keyboard(KeyboardKey::Space)),
        &[Event::SelectNode(key(2))]
    );

    // Space on an already-selected node deselects it (toggle contract).
    let mut service = multi();

    drop(service.send(Event::SelectNode(key(2))));

    assert!(service.connect(&|_| {}).is_node_selected(&key(2)));
    assert_eq!(
        dispatch_key(&service, &key(2), &keyboard(KeyboardKey::Space)),
        &[Event::DeselectNode(key(2))]
    );
}

#[test]
fn keydown_printable_triggers_typeahead() {
    let service = service(props());

    let events = dispatch_key(&service, &key(1), &printable('b'));

    assert!(
        matches!(events.as_slice(), [Event::TypeaheadSearch('b', now_ms)] if *now_ms > 0),
        "printable key dispatches TypeaheadSearch with a clock timestamp: {events:?}"
    );
}

#[test]
fn keydown_asterisk_expands_siblings() {
    let service = service(props());

    // Focused on Fruits (root level); '*' expands all expandable root siblings:
    // Fruits (1) and Vegetables (4) — Grains (7) is a leaf.
    let events = dispatch_key(&service, &key(1), &printable('*'));

    assert_eq!(
        events,
        &[Event::ExpandNode(key(1)), Event::ExpandNode(key(4))]
    );
}

// ----------------------------------------------------------------------------
// Drag and drop (agnostic surface)
// ----------------------------------------------------------------------------

fn dnd_props() -> Props {
    props().dnd_enabled(true)
}

#[test]
fn drag_start_ignored_when_dnd_disabled() {
    let mut service = service(props());

    let result = service.send(Event::DragStart(key(2)));

    assert!(!result.context_changed);
    assert_eq!(service.context().dragging, None);
}

#[test]
fn drag_start_sets_dragging() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));

    assert_eq!(service.context().dragging, Some(key(2)));
    assert_eq!(service.context().drop_target, None);
}

#[test]
fn drag_over_rejects_self_and_descendant_targets() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(1)))); // Fruits (parent of 2, 3)

    // Onto itself -> rejected.
    let result = service.send(Event::DragOver(CollectionDropTarget {
        key: key(1),
        position: DropPosition::On,
    }));

    assert!(!result.context_changed);

    // Onto a descendant (Apple) -> rejected (would create a cycle).
    let result = service.send(Event::DragOver(CollectionDropTarget {
        key: key(2),
        position: DropPosition::On,
    }));

    assert!(!result.context_changed);
    assert_eq!(service.context().drop_target, None);
}

#[test]
fn drag_over_accepts_valid_target() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));

    let target = CollectionDropTarget {
        key: key(7),
        position: DropPosition::After,
    };

    drop(service.send(Event::DragOver(target.clone())));

    assert_eq!(service.context().drop_target, Some(target));
}

#[test]
fn drag_move_steps_through_valid_slots() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2)))); // Apple

    // First valid slot: target 1 (Fruits), Before. (2 excluded as dragged.)
    drop(service.send(Event::DragMoveNext));

    assert_eq!(
        service.context().drop_target,
        Some(CollectionDropTarget {
            key: key(1),
            position: DropPosition::Before,
        })
    );

    // Prev from the first slot wraps to the last valid slot: target 7, After.
    drop(service.send(Event::DragStart(key(2))));
    drop(service.send(Event::DragMovePrev));

    assert_eq!(
        service.context().drop_target,
        Some(CollectionDropTarget {
            key: key(7),
            position: DropPosition::After,
        })
    );
}

#[test]
fn cancel_drag_clears_state() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));
    drop(service.send(Event::CancelDrag));

    assert_eq!(service.context().dragging, None);
    assert_eq!(service.context().drop_target, None);
}

#[test]
fn drop_without_target_is_noop() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));

    let result = service.send(Event::Drop);

    assert!(result.pending_effects.is_empty());
}

#[test]
fn drop_emits_reorder_effect_and_invokes_callback() {
    let captured: Arc<Mutex<Vec<ReorderEvent>>> = Arc::new(Mutex::new(Vec::new()));

    let sink = Arc::clone(&captured);

    let mut service = service(
        dnd_props().on_reorder(move |event: ReorderEvent| sink.lock().unwrap().push(event)),
    );

    let items_before = service.context().items.clone();

    drop(service.send(Event::DragStart(key(2)))); // Apple (child of Fruits)
    drop(service.send(Event::DragOver(CollectionDropTarget {
        key: key(7),
        position: DropPosition::After,
    })));

    let mut result = service.send(Event::Drop);

    assert_eq!(result.pending_effects.len(), 1);
    assert_eq!(result.pending_effects[0].name, Effect::Reorder);

    // The core never mutates its collection on drop — the consumer applies it.
    assert_eq!(&items_before, &service.context().items);
    assert_eq!(service.context().dragging, None);
    assert_eq!(service.context().drop_target, None);

    // Running the effect invokes `on_reorder` with the resolved paths.
    let effect = result.pending_effects.pop().expect("reorder effect");

    let noop_send: StrongSend<Event> = Arc::new(|_| {});

    drop(effect.run(service.context(), service.props(), noop_send));

    assert_eq!(
        captured.lock().unwrap().as_slice(),
        &[ReorderEvent {
            source_path: vec![key(1), key(2)], // Fruits -> Apple
            target_path: vec![key(7)],         // Grains (root)
            position: DropPosition::After,
        }]
    );
}

#[test]
fn drag_handle_keydown_pickup_move_and_drop() {
    let service = service(dnd_props());

    // Not dragging yet: Enter picks up.
    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    service
        .connect(&send)
        .on_drag_handle_keydown(&key(2), &keyboard(KeyboardKey::Enter));

    assert_eq!(recorder.into_inner(), &[Event::DragStart(key(2))]);
}

#[test]
fn drag_handle_keydown_while_dragging_confirms_and_cancels() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));

    // Enter confirms the drop.
    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    service
        .connect(&send)
        .on_drag_handle_keydown(&key(2), &keyboard(KeyboardKey::Enter));

    assert_eq!(recorder.into_inner(), &[Event::Drop]);

    // Escape cancels; arrows step the target.
    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    let api = service.connect(&send);

    api.on_drag_handle_keydown(&key(2), &keyboard(KeyboardKey::Escape));
    api.on_drag_handle_keydown(&key(2), &keyboard(KeyboardKey::ArrowDown));
    api.on_drag_handle_keydown(&key(2), &keyboard(KeyboardKey::ArrowUp));

    assert_eq!(
        recorder.into_inner(),
        &[Event::CancelDrag, Event::DragMoveNext, Event::DragMovePrev]
    );
}

#[test]
fn drag_handle_keydown_noop_when_dnd_disabled() {
    let service = service(props());

    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    service
        .connect(&send)
        .on_drag_handle_keydown(&key(2), &keyboard(KeyboardKey::Enter));

    assert!(recorder.into_inner().is_empty());
}

// ----------------------------------------------------------------------------
// Edge cases, accessors, and ConnectApi dispatch
// ----------------------------------------------------------------------------

fn empty_service() -> Service<Machine> {
    service(
        Props::new()
            .id("tree")
            .items(TreeCollection::new(Vec::new())),
    )
}

#[test]
fn focus_navigation_on_empty_tree_is_noop() {
    let mut service = empty_service();

    for event in [
        Event::FocusFirst,
        Event::FocusLast,
        Event::FocusNext,
        Event::FocusPrev,
    ] {
        assert!(!service.send(event).context_changed);
    }

    assert_eq!(service.context().focused_node, None);
}

#[test]
fn focus_prev_from_middle_moves_up_one() {
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(3)))); // Banana (not first)
    drop(service.send(Event::FocusPrev));

    assert_eq!(service.context().focused_node, Some(key(2))); // Apple
}

#[test]
fn props_builder_sets_every_field() {
    let props = Props::new()
        .id("t")
        .items(sample_items())
        .selected(selection::Set::Single(key(2)))
        .default_selected(selection::Set::Single(key(3)))
        .expanded(BTreeSet::new())
        .default_expanded(BTreeSet::new())
        .multiple(true)
        .selection_mode(selection::Mode::Multiple)
        .selection_behavior(selection::Behavior::Replace)
        .dnd_enabled(true)
        .on_reorder(|_event: ReorderEvent| {});

    assert_eq!(props.id, "t");
    assert_eq!(props.selected, Some(selection::Set::Single(key(2))));
    assert_eq!(props.default_selected, selection::Set::Single(key(3)));
    assert_eq!(props.expanded, Some(BTreeSet::new()));
    assert!(props.multiple);
    assert_eq!(props.selection_mode, selection::Mode::Multiple);
    assert_eq!(props.selection_behavior, selection::Behavior::Replace);
    assert!(props.dnd_enabled);
    assert!(props.on_reorder.is_some());
}

#[test]
fn drag_move_steps_from_existing_target() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));
    drop(service.send(Event::DragMoveNext)); // None -> slot 0: (1, Before)
    drop(service.send(Event::DragMoveNext)); // Some(0) -> slot 1: (1, On)

    assert_eq!(
        service.context().drop_target,
        Some(CollectionDropTarget {
            key: key(1),
            position: DropPosition::On,
        })
    );
}

#[test]
fn drag_move_prev_from_existing_target() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));
    drop(service.send(Event::DragMoveNext)); // None -> slot 0: (1, Before)
    drop(service.send(Event::DragMovePrev)); // Some(0) -> wraps to last slot

    assert_eq!(
        service.context().drop_target,
        Some(CollectionDropTarget {
            key: key(7),
            position: DropPosition::After,
        })
    );
}

#[test]
fn drag_over_and_drop_ignored_without_active_drag() {
    let mut service = service(dnd_props());

    // No DragStart yet: DragOver and Drop are no-ops.
    let over = service.send(Event::DragOver(CollectionDropTarget {
        key: key(7),
        position: DropPosition::On,
    }));

    assert!(!over.context_changed);

    let drop = service.send(Event::Drop);

    assert!(drop.pending_effects.is_empty());
}

#[test]
fn drop_ignored_when_dnd_disabled() {
    let mut service = service(props()); // dnd disabled

    assert!(service.send(Event::Drop).pending_effects.is_empty());
}

#[test]
fn typeahead_on_empty_tree_does_not_focus() {
    let mut service = empty_service();

    drop(service.send(Event::TypeaheadSearch('a', 1)));
    assert_eq!(service.context().focused_node, None);
}

#[test]
fn focus_next_and_prev_without_prior_focus() {
    let mut next = service(props());

    drop(next.send(Event::FocusNext));

    assert_eq!(next.context().focused_node, Some(key(1))); // first visible

    let mut prev = service(props());

    drop(prev.send(Event::FocusPrev));

    assert_eq!(prev.context().focused_node, Some(key(7))); // last visible
}

#[test]
fn collapsing_ancestor_clamps_focus_to_that_ancestor() {
    // Focus a descendant, then collapse its parent: focus must move up to the
    // now-collapsed ancestor so aria-activedescendant stays on a rendered node.
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(2)))); // Apple (under Fruits)
    drop(service.send(Event::CollapseNode(key(1)))); // hides Apple

    assert_eq!(service.context().focused_node, Some(key(1))); // clamped to Fruits

    // And navigation proceeds correctly from the clamped position.
    drop(service.send(Event::FocusNext));

    assert_eq!(service.context().focused_node, Some(key(4))); // Vegetables
}

#[test]
fn collapse_all_clamps_focus_to_root_ancestor() {
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(3)))); // Banana (under Fruits)
    drop(service.send(Event::CollapseAll));

    assert_eq!(service.context().focused_node, Some(key(1))); // Fruits (root)
}

#[test]
fn sync_props_clears_focus_when_focused_node_removed() {
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(7)))); // Grains

    // New collection without Grains.
    let trimmed = TreeCollection::new(vec![branch(1, "Fruits", true, vec![leaf(2, "Apple")])]);

    drop(service.set_props(props().items(trimmed)));

    assert_eq!(service.context().focused_node, None);
}

#[test]
fn drag_step_noop_without_active_drag_or_dnd() {
    let mut dnd = service(dnd_props()); // dnd enabled, no drag started

    assert!(!dnd.send(Event::DragMoveNext).context_changed);

    let mut plain = service(props()); // dnd disabled

    assert!(!plain.send(Event::DragMoveNext).context_changed);
}

#[test]
fn drop_target_accessor_reflects_state() {
    let mut service = service(dnd_props());

    assert!(service.connect(&|_| {}).drop_target().is_none());

    drop(service.send(Event::DragStart(key(2))));
    drop(service.send(Event::DragOver(CollectionDropTarget {
        key: key(7),
        position: DropPosition::Before,
    })));

    assert_eq!(
        service.connect(&|_| {}).drop_target().map(|t| t.position),
        Some(DropPosition::Before)
    );
}

#[test]
fn sibling_info_unknown_key_defaults() {
    let service = service(props());

    assert_eq!(service.connect(&|_| {}).sibling_info(&key(999)), (1, 1));
}

#[test]
fn connect_api_part_attrs_dispatches_all_parts() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));
    drop(service.send(Event::DragOver(CollectionDropTarget {
        key: key(7),
        position: DropPosition::On,
    })));

    let api = service.connect(&|_| {});

    for part in [
        Part::Root,
        Part::Branch { node_id: key(1) },
        Part::BranchControl { node_id: key(1) },
        Part::BranchIndicator { node_id: key(1) },
        Part::BranchText,
        Part::BranchContent { node_id: key(1) },
        Part::Leaf { node_id: key(2) },
        Part::LeafText,
        Part::DragHandle { node_id: key(2) },
        Part::DropIndicator,
    ] {
        let attrs = api.part_attrs(part);

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("tree-view"),
            "every part carries the scope attribute"
        );
    }
}

#[test]
fn drop_indicator_part_without_active_target_is_bare() {
    let service = service(dnd_props());

    let attrs = service.connect(&|_| {}).part_attrs(Part::DropIndicator);

    assert!(attrs.get(&HtmlAttr::Data("ars-drop-position")).is_none());
    assert_eq!(
        attrs.get(&HtmlAttr::Data("ars-part")),
        Some("drop-indicator")
    );
}

#[test]
fn asterisk_on_child_expands_child_level_siblings() {
    let items = TreeCollection::new(vec![branch(
        1,
        "Root",
        true,
        vec![
            branch(2, "A", false, vec![leaf(20, "x")]),
            branch(3, "B", false, vec![leaf(30, "y")]),
            leaf(4, "C"),
        ],
    )]);

    let service = service(props().items(items));

    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    // Focused on child A: '*' expands its expandable siblings (A and B).
    service
        .connect(&send)
        .on_node_keydown(&key(2), &printable('*'));

    assert_eq!(
        recorder.into_inner(),
        &[Event::ExpandNode(key(2)), Event::ExpandNode(key(3))]
    );
}

#[test]
fn dragging_node_branch_attrs_include_data_ars_dragging() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(1))));

    let attrs = service.connect(&|_| {}).branch_attrs(&key(1));

    assert_eq!(attrs.get(&HtmlAttr::Data("ars-dragging")), Some("true"));
}

#[test]
fn drag_handle_keydown_ignores_other_keys_while_dragging() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));

    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    service
        .connect(&send)
        .on_drag_handle_keydown(&key(2), &printable('x'));

    assert!(recorder.into_inner().is_empty());
}

#[test]
fn typeahead_starts_strictly_after_focused_node() {
    // Two siblings share an initial letter; typing it from the first must move
    // to the second (search starts *after* the focused node), pinning the
    // match comparison and the `start = pos + 1` offset.
    let items = TreeCollection::new(vec![
        leaf(1, "Apple"),
        leaf(2, "Avocado"),
        leaf(3, "Cherry"),
    ]);

    let mut service = service(props().items(items));

    drop(service.send(Event::FocusNode(key(1)))); // Apple
    drop(service.send(Event::TypeaheadSearch('a', 1)));

    assert_eq!(service.context().focused_node, Some(key(2))); // Avocado, not Apple
}

#[test]
fn valid_drop_slots_exclude_self_and_descendants() {
    // Dragging Fruits (1, parent of 2 & 3): the first valid keyboard slot must
    // skip 1 (self) and 2/3 (descendants), landing on Vegetables (4).
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(1))));
    drop(service.send(Event::DragMoveNext));

    assert_eq!(
        service.context().drop_target,
        Some(CollectionDropTarget {
            key: key(4),
            position: DropPosition::Before,
        })
    );
}

#[test]
fn focus_visible_marker_only_on_the_focused_node() {
    let mut service = service(props());

    drop(service.send(Event::FocusFirst)); // keyboard nav -> focus_visible = true, node 1

    let api = service.connect(&|_| {});

    assert_eq!(
        api.branch_attrs(&key(1))
            .get(&HtmlAttr::Data("ars-focus-visible")),
        Some("true"),
        "the focused node carries focus-visible"
    );
    assert!(
        api.leaf_attrs(&key(7))
            .get(&HtmlAttr::Data("ars-focus-visible"))
            .is_none(),
        "a non-focused node never carries focus-visible"
    );
}

#[test]
fn focus_visible_marker_cleared_after_blur() {
    let mut service = service(props());

    drop(service.send(Event::FocusFirst)); // keyboard focus on node 1
    drop(service.send(Event::Blur)); // focus_visible = false, focused_node retained

    assert!(
        service
            .connect(&|_| {})
            .branch_attrs(&key(1))
            .get(&HtmlAttr::Data("ars-focus-visible"))
            .is_none(),
        "focus-visible requires BOTH focused and focus_visible"
    );
}

#[test]
fn pointer_focus_does_not_set_focus_visible() {
    // FocusNode (used by on_leaf_click / on_branch_control_click and the public
    // focus_node API) is pointer/programmatic — it must not render the keyboard
    // focus ring.
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(1))));

    assert_eq!(service.context().focused_node, Some(key(1)));
    assert!(!service.context().focus_visible);
    assert!(
        service
            .connect(&|_| {})
            .branch_attrs(&key(1))
            .get(&HtmlAttr::Data("ars-focus-visible"))
            .is_none(),
        "pointer/programmatic focus must not show keyboard focus styling"
    );
}

#[test]
fn drag_handle_arrows_and_escape_are_inert_when_not_dragging() {
    let service = service(dnd_props()); // dnd enabled, no active drag

    for code in [
        KeyboardKey::ArrowDown,
        KeyboardKey::ArrowUp,
        KeyboardKey::Escape,
    ] {
        let recorder: Recorder = RefCell::new(Vec::new());
        let send = |event| record(&recorder, event);

        service
            .connect(&send)
            .on_drag_handle_keydown(&key(2), &keyboard(code));

        assert!(
            recorder.into_inner().is_empty(),
            "{code:?} on a drag handle does nothing until a drag is picked up"
        );
    }
}

#[test]
fn api_debug_impl_renders() {
    let service = service(props());

    let api = service.connect(&|_| {});

    assert!(format!("{api:?}").contains("Api"));
}

// ----------------------------------------------------------------------------
// Codex review regressions (PR #695)
// ----------------------------------------------------------------------------

#[test]
fn multiple_prop_enables_real_multi_selection() {
    // Props::multiple(true) alone (selection_mode left at default Single) must
    // enable real multi-selection, not just aria-multiselectable.
    let mut service = service(props().multiple(true));

    drop(service.send(Event::SelectNode(key(2))));
    drop(service.send(Event::SelectNode(key(3))));

    let api = service.connect(&|_| {});

    assert!(api.is_node_selected(&key(2)));
    assert!(
        api.is_node_selected(&key(3)),
        "multiple=true must allow more than one selected node"
    );
}

#[test]
fn set_props_switch_to_multiple_reconfigures_selection() {
    let mut service = service(props()); // single mode

    drop(service.send(Event::SelectNode(key(2))));
    drop(service.set_props(props().multiple(true))); // switch to multiple
    drop(service.send(Event::SelectNode(key(3))));

    let api = service.connect(&|_| {});

    assert!(
        api.is_node_selected(&key(2)) && api.is_node_selected(&key(3)),
        "after switching to multiple at runtime, both nodes stay selected"
    );
}

#[test]
fn set_props_switch_to_none_stops_selection() {
    let mut service = service(props().selected(selection::Set::Single(key(2))));

    drop(
        service.set_props(
            props()
                .selected(selection::Set::Single(key(2)))
                .selection_mode(selection::Mode::None),
        ),
    );

    // In None mode further selection is rejected.
    let result = service.send(Event::SelectNode(key(3)));

    assert!(!result.context_changed);
    assert!(!service.connect(&|_| {}).is_node_selected(&key(3)));
}

#[test]
fn expand_all_expands_lazy_branches() {
    // A node with the has_children flag but no loaded children is a lazy
    // branch; ExpandAll must expand it so consumers can trigger lazy loading.
    let items = TreeCollection::new(vec![leaf_with(
        9,
        "Lazy",
        TreeItem {
            has_children: true,
            ..item("Lazy")
        },
    )]);

    let mut service = service(props().items(items));

    drop(service.send(Event::ExpandAll));

    assert!(service.context().expanded.get().contains(&key(9)));
}

#[test]
fn drag_over_rejects_unknown_target_key() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));

    let result = service.send(Event::DragOver(CollectionDropTarget {
        key: key(999), // not a node in the collection
        position: DropPosition::On,
    }));

    assert!(!result.context_changed);
    assert!(service.context().drop_target.is_none());
}

#[test]
fn typeahead_matches_collection_text_value() {
    // Typeahead uses the canonical shared matcher, which searches the
    // collection's `text_value` (the node's designated searchable text).
    let searchable = TreeItemConfig {
        key: key(1),
        text_value: "Apricot".to_string(),
        value: item("Apricot"),
        children: Vec::new(),
        default_expanded: false,
    };

    let items = TreeCollection::new(vec![searchable, leaf(2, "Banana")]);

    let mut service = service(props().items(items));

    drop(service.send(Event::TypeaheadSearch('a', 1))); // matches "Apricot"

    assert_eq!(service.context().focused_node, Some(key(1)));
}

// ----------------------------------------------------------------------------
// Codex review regressions, round 2 (PR #695)
// ----------------------------------------------------------------------------

#[test]
fn focus_node_ignores_non_visible_and_unknown_keys() {
    let mut service = service(props()); // Vegetables(4) collapsed -> Carrot(5) hidden

    assert!(!service.send(Event::FocusNode(key(5))).context_changed); // hidden
    assert_eq!(service.context().focused_node, None);
    assert!(!service.send(Event::FocusNode(key(999))).context_changed); // unknown
    assert_eq!(service.context().focused_node, None);
}

#[test]
fn asterisk_expands_lazy_siblings() {
    let items = TreeCollection::new(vec![branch(
        1,
        "Root",
        true,
        vec![
            leaf_with(
                2,
                "A",
                TreeItem {
                    has_children: true, // lazy branch (no loaded children)
                    ..item("A")
                },
            ),
            leaf(3, "B"),
        ],
    )]);

    let service = service(props().items(items));

    let recorder: Recorder = RefCell::new(Vec::new());
    let send = |event| record(&recorder, event);

    service
        .connect(&send)
        .on_node_keydown(&key(2), &printable('*'));

    assert_eq!(recorder.into_inner(), &[Event::ExpandNode(key(2))]);
}

#[test]
fn focus_on_container_initializes_active_node() {
    // No prior focus -> first visible node.
    let mut first = service(props());

    drop(first.send(Event::Focus { is_keyboard: true }));

    assert_eq!(first.context().focused_node, Some(key(1)));

    // With a selection -> the (visible) selected node.
    let mut selected = service(props().selected(selection::Set::Single(key(4))));

    drop(selected.send(Event::Focus { is_keyboard: true }));

    assert_eq!(selected.context().focused_node, Some(key(4)));

    // Existing active node is not overridden.
    let mut existing = service(props());

    drop(existing.send(Event::FocusNode(key(2))));
    drop(existing.send(Event::Focus { is_keyboard: false }));

    assert_eq!(existing.context().focused_node, Some(key(2)));
}

#[test]
fn switching_to_single_normalizes_multi_selection() {
    let mut service = service(
        props()
            .multiple(true)
            .selection_mode(selection::Mode::Multiple),
    );

    drop(service.send(Event::SelectNode(key(2))));
    drop(service.send(Event::SelectNode(key(3))));
    drop(service.set_props(props().selection_mode(selection::Mode::Single))); // tighten

    let api = service.connect(&|_| {});

    let count = [key(1), key(2), key(3), key(4), key(7)]
        .iter()
        .filter(|k| api.is_node_selected(k))
        .count();

    assert!(
        count <= 1,
        "single mode keeps at most one selected after switch"
    );
}

#[test]
fn set_props_resyncs_generated_ids_on_id_change() {
    let mut service = service(props().id("old"));

    assert_eq!(
        service
            .connect(&|_| {})
            .branch_attrs(&key(1))
            .get(&HtmlAttr::Id),
        Some("old-node-i-1")
    );

    drop(service.set_props(props().id("new")));

    assert_eq!(
        service
            .connect(&|_| {})
            .branch_attrs(&key(1))
            .get(&HtmlAttr::Id),
        Some("new-node-i-1")
    );
}

#[test]
fn drag_handle_is_keyboard_focusable() {
    let service = service(dnd_props());

    assert_eq!(
        service
            .connect(&|_| {})
            .drag_handle_attrs(&key(2))
            .get(&HtmlAttr::TabIndex),
        Some("0")
    );
}

#[test]
fn disabled_node_is_not_announced_draggable() {
    let service = service(props().items(disabled_items()).dnd_enabled(true));

    let api = service.connect(&|_| {});

    assert!(
        api.leaf_attrs(&key(2)) // Apple is disabled
            .get(&HtmlAttr::Aria(AriaAttr::RoleDescription))
            .is_none()
    );
    assert_eq!(
        api.leaf_attrs(&key(7)) // Grains is enabled
            .get(&HtmlAttr::Aria(AriaAttr::RoleDescription)),
        Some("draggable")
    );
}

#[test]
fn controlled_selection_state_tracks_binding_not_optimistic() {
    let mut service = service(
        props()
            .multiple(true)
            .selection_mode(selection::Mode::Multiple)
            .selected(selection::Set::Single(key(2))),
    );

    drop(service.send(Event::SelectNode(key(3)))); // optimistic; parent owns selection

    let api = service.connect(&|_| {});

    assert!(api.is_node_selected(&key(2)));
    assert!(
        !api.is_node_selected(&key(3)),
        "controlled selection does not change until the parent echoes it"
    );
    assert_eq!(
        &service.context().selection_state.selected_keys,
        service.context().selected.get(),
        "selection_state stays consistent with the controlled binding"
    );
}

#[test]
fn disabled_node_suppresses_href() {
    let items = TreeCollection::new(vec![leaf_with(
        9,
        "Docs",
        TreeItem {
            disabled: true,
            href: Some("/docs".to_string()),
            ..TreeItem::default()
        },
    )]);

    let service = service(props().items(items));

    assert!(
        service
            .connect(&|_| {})
            .leaf_attrs(&key(9))
            .get(&HtmlAttr::Href)
            .is_none(),
        "a disabled node must not expose a live href"
    );
}

// ----------------------------------------------------------------------------
// Codex review regressions, round 3 (PR #695)
// ----------------------------------------------------------------------------

fn disabled_branch_tree() -> TreeCollection<TreeItem> {
    TreeCollection::new(vec![TreeItemConfig {
        key: key(1),
        text_value: "Fruits".to_string(),
        value: TreeItem {
            disabled: true,
            ..item("Fruits")
        },
        children: vec![leaf(2, "Apple")],
        default_expanded: false,
    }])
}

#[test]
fn disabled_branch_cannot_be_toggled_or_expanded() {
    let mut service = service(props().items(disabled_branch_tree()));

    assert!(!service.send(Event::ToggleNode(key(1))).context_changed);
    assert!(!service.send(Event::ExpandNode(key(1))).context_changed);
    assert!(!service.context().expanded.get().contains(&key(1)));
}

#[test]
fn expand_events_reject_leaves_and_unknown_keys() {
    let mut service = service(props());

    assert!(!service.send(Event::ExpandNode(key(2))).context_changed); // Apple is a leaf
    assert!(!service.send(Event::ToggleNode(key(2))).context_changed);
    assert!(!service.send(Event::ExpandNode(key(999))).context_changed); // unknown
    assert!(!service.context().expanded.get().contains(&key(2)));
    assert!(!service.context().expanded.get().contains(&key(999)));
}

#[test]
fn select_node_rejects_unknown_keys() {
    let mut service = service(props());

    assert!(!service.send(Event::SelectNode(key(999))).context_changed);
    assert!(!service.connect(&|_| {}).is_node_selected(&key(999)));
}

#[test]
fn arrow_right_on_expanded_lazy_branch_is_inert() {
    // node 9 is a lazy branch (has_children flag, no loaded children) and is
    // expanded; there is no rendered child to enter, so ArrowRight does nothing.
    let items = TreeCollection::new(vec![
        leaf_with(
            9,
            "Lazy",
            TreeItem {
                has_children: true,
                ..item("Lazy")
            },
        ),
        leaf(8, "After"),
    ]);

    let mut expanded = BTreeSet::new();

    expanded.insert(key(9));

    let service = service(props().items(items).default_expanded(expanded));

    assert!(dispatch_key(&service, &key(9), &keyboard(KeyboardKey::ArrowRight)).is_empty());
}

#[test]
fn set_props_clears_active_drag_on_items_change() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));
    drop(service.send(Event::DragOver(CollectionDropTarget {
        key: key(7),
        position: DropPosition::After,
    })));

    let new_items = TreeCollection::new(vec![leaf(1, "Solo")]);

    drop(service.set_props(dnd_props().items(new_items)));

    assert_eq!(service.context().dragging, None);
    assert_eq!(service.context().drop_target, None);
}

#[test]
fn controlled_multi_selection_is_clamped_under_single_mode() {
    // A controlled binding that violates the mode (two keys under Single) renders
    // a mode-valid selection without mutating the parent-owned value.
    let mut set = BTreeSet::new();

    set.insert(key(2));
    set.insert(key(3));

    let service = service(
        props()
            .selection_mode(selection::Mode::Single)
            .selected(selection::Set::Multiple(set)),
    );

    let api = service.connect(&|_| {});

    let count = [key(2), key(3)]
        .iter()
        .filter(|k| api.is_node_selected(k))
        .count();

    assert_eq!(
        count, 1,
        "single mode clamps a multi-key controlled binding"
    );
}

// ----------------------------------------------------------------------------
// Codex review regressions, round 4 (PR #695)
// ----------------------------------------------------------------------------

#[test]
fn blur_resets_typeahead_buffer() {
    let mut service = service(props());

    drop(service.send(Event::TypeaheadSearch('g', 1)));

    assert!(!service.context().typeahead.search.is_empty());

    drop(service.send(Event::Blur));

    assert!(
        service.context().typeahead.search.is_empty(),
        "blur clears the typeahead buffer so a refocus starts fresh"
    );
}

#[test]
fn keyboard_root_focus_shows_focus_visible_on_active_node() {
    let mut service = service(props());

    drop(service.send(Event::Focus { is_keyboard: true }));

    let node = service.context().focused_node.clone().expect("active node");

    assert_eq!(
        service
            .connect(&|_| {})
            .branch_attrs(&node)
            .get(&HtmlAttr::Data("ars-focus-visible")),
        Some("true"),
        "a keyboard tab-in shows the focus ring on the initialized active node"
    );
}

#[test]
fn typeahead_reaches_disabled_nodes() {
    // Apple (2) is disabled but focusable; typing 'a' must still reach it
    // (FocusOnly), even though it remains non-selectable.
    let mut service = service(props().items(disabled_items()));

    drop(service.send(Event::TypeaheadSearch('a', 1)));

    assert_eq!(service.context().focused_node, Some(key(2)));
}

#[test]
fn disabling_dnd_at_runtime_clears_active_drag() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));

    assert_eq!(service.context().dragging, Some(key(2)));

    drop(service.set_props(props())); // dnd_enabled = false

    assert_eq!(service.context().dragging, None);
    assert_eq!(service.context().drop_target, None);
}

#[test]
fn disabled_node_drag_handle_is_not_operable() {
    let service = service(props().items(disabled_items()).dnd_enabled(true));

    let attrs = service.connect(&|_| {}).drag_handle_attrs(&key(2)); // Apple disabled

    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
    assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("-1"));
}

#[test]
fn typeahead_respects_live_expansion_state() {
    // Vegetables (4) starts collapsed (Carrot hidden). After expanding it via an
    // event, typeahead must see the newly-visible Carrot (5).
    let mut service = service(props());

    drop(service.send(Event::ExpandNode(key(4))));
    drop(service.send(Event::TypeaheadSearch('c', 1)));

    assert_eq!(service.context().focused_node, Some(key(5)));
}

#[test]
fn nested_drag_start_is_rejected() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2))));

    let result = service.send(Event::DragStart(key(3)));

    assert!(!result.context_changed);
    assert_eq!(
        service.context().dragging,
        Some(key(2)),
        "an in-flight drag cannot be retargeted by a second DragStart"
    );
}

// ----------------------------------------------------------------------------
// Codex review regressions, round 5 (PR #695)
// ----------------------------------------------------------------------------

#[test]
fn disabled_initial_selection_is_dropped() {
    // Apple (2) is disabled; a `default_selected` containing it must not init as
    // selected (it can never be selected via `SelectNode`).
    let service = service(
        props()
            .items(disabled_items())
            .default_selected(selection::Set::Single(key(2))),
    );

    let api = service.connect(&|_| {});

    assert!(
        !api.is_node_selected(&key(2)),
        "a disabled node cannot initialize as selected"
    );
    assert!(service.context().selection_state.selected_keys.is_empty());
}

#[test]
fn non_selectable_nodes_omit_aria_selected() {
    // selection_mode None: no node is selectable, so none exposes aria-selected.
    let none = service(props().selection_mode(selection::Mode::None));

    let none_api = none.connect(&|_| {});

    assert_eq!(
        none_api
            .branch_attrs(&key(1))
            .get(&HtmlAttr::Aria(AriaAttr::Selected)),
        None,
        "a branch in a non-selectable tree omits aria-selected"
    );
    assert_eq!(
        none_api
            .leaf_attrs(&key(2))
            .get(&HtmlAttr::Aria(AriaAttr::Selected)),
        None,
        "a leaf in a non-selectable tree omits aria-selected"
    );

    // A disabled node in a selectable tree is also non-selectable.
    let disabled = service(props().items(disabled_items()));

    assert_eq!(
        disabled
            .connect(&|_| {})
            .leaf_attrs(&key(2))
            .get(&HtmlAttr::Aria(AriaAttr::Selected)),
        None,
        "a disabled node omits aria-selected"
    );

    // A selectable, enabled node still advertises selection state.
    assert_eq!(
        service(props())
            .connect(&|_| {})
            .leaf_attrs(&key(2))
            .get(&HtmlAttr::Aria(AriaAttr::Selected)),
        Some("false"),
        "a selectable enabled node still exposes aria-selected"
    );
}

#[test]
fn drag_over_invalid_target_clears_stale_drop_target() {
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(2)))); // drag Apple
    drop(service.send(Event::DragOver(CollectionDropTarget {
        key: key(7),
        position: DropPosition::Before,
    })));

    assert!(service.context().drop_target.is_some());

    // Hover an invalid slot (the dragged node itself): the stale target clears.
    let result = service.send(Event::DragOver(CollectionDropTarget {
        key: key(2),
        position: DropPosition::On,
    }));

    assert!(
        result.context_changed,
        "dropping the stale target is a context change"
    );
    assert_eq!(
        service.context().drop_target,
        None,
        "hovering an invalid slot drops the stale target so Drop can't reuse it"
    );
}

#[test]
fn drag_handle_is_inert_when_dnd_disabled() {
    // DnD off, but a consumer still renders a handle on an enabled node: it must
    // not present an operable-looking control.
    let service = service(props()); // dnd_enabled = false

    let attrs = service.connect(&|_| {}).drag_handle_attrs(&key(2));

    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
    assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("-1"));
}

#[test]
fn expansion_prop_echo_preserves_active_drag() {
    // A controlled-expanded tree echoing a new `expanded` prop during a drag
    // (e.g. adapter hover-expand) must not cancel the in-flight reorder.
    let mut expanded = BTreeSet::new();

    expanded.insert(key(1));

    let mut service = service(dnd_props().expanded(expanded.clone()));

    drop(service.send(Event::DragStart(key(2))));

    assert_eq!(service.context().dragging, Some(key(2)));

    expanded.insert(key(4)); // echo: also expand Vegetables; items unchanged.

    drop(service.set_props(dnd_props().expanded(expanded)));

    assert_eq!(
        service.context().dragging,
        Some(key(2)),
        "an expanded-prop echo must preserve an in-flight drag"
    );
}

#[test]
fn drag_over_hidden_target_is_rejected() {
    // Vegetables (4) is collapsed, so Carrot (5) is hidden. A pointer hit-test
    // that sends the hidden row as a target must be rejected.
    let mut service = service(dnd_props());

    drop(service.send(Event::DragStart(key(7)))); // drag Grains

    let result = service.send(Event::DragOver(CollectionDropTarget {
        key: key(5),
        position: DropPosition::On,
    }));

    assert!(!result.context_changed);
    assert_eq!(
        service.context().drop_target,
        None,
        "a target hidden under a collapsed parent is not a valid drop slot"
    );
}

#[test]
fn selection_drops_keys_removed_from_collection() {
    // Select Grains (7), then supply a collection that no longer contains it.
    let mut service = service(props());

    drop(service.send(Event::SelectNode(key(7))));

    assert!(service.connect(&|_| {}).is_node_selected(&key(7)));

    let smaller = TreeCollection::new(vec![branch(1, "Fruits", true, vec![leaf(2, "Apple")])]);

    drop(service.set_props(props().items(smaller)));

    assert!(
        !service.connect(&|_| {}).is_node_selected(&key(7)),
        "a selection key removed from the collection is dropped on resync"
    );
    assert!(service.context().selection_state.selected_keys.is_empty());
}

#[test]
fn multiple_selection_drops_all_removed_keys() {
    let mut service = service(
        props()
            .multiple(true)
            .selection_mode(selection::Mode::Multiple),
    );

    drop(service.send(Event::SelectNode(key(2))));
    drop(service.send(Event::SelectNode(key(7))));

    assert!(service.connect(&|_| {}).is_node_selected(&key(2)));
    assert!(service.connect(&|_| {}).is_node_selected(&key(7)));

    // New collection contains neither selected key.
    let smaller = TreeCollection::new(vec![branch(1, "Fruits", true, vec![leaf(3, "Banana")])]);

    drop(
        service.set_props(
            props()
                .items(smaller)
                .multiple(true)
                .selection_mode(selection::Mode::Multiple),
        ),
    );

    assert!(
        service.context().selection_state.selected_keys.is_empty(),
        "every multi-selected key removed from the collection is dropped"
    );
}

#[test]
fn stale_drop_target_cleared_when_echo_hides_it() {
    // Vegetables (4) starts expanded so Carrot (5) is a visible drop slot.
    let mut expanded = BTreeSet::new();

    expanded.insert(key(1));
    expanded.insert(key(4));

    let mut service = service(dnd_props().expanded(expanded));

    drop(service.send(Event::DragStart(key(7)))); // drag Grains
    drop(service.send(Event::DragOver(CollectionDropTarget {
        key: key(5),
        position: DropPosition::Before,
    })));

    assert!(service.context().drop_target.is_some());

    // Echo a collapse of Vegetables: Carrot (5) is no longer visible. The drag
    // survives but the now-hidden target is dropped.
    let collapsed: BTreeSet<Key> = [key(1)].into_iter().collect();

    drop(service.set_props(dnd_props().expanded(collapsed)));

    assert_eq!(
        service.context().dragging,
        Some(key(7)),
        "the drag itself survives an expanded-prop echo"
    );
    assert_eq!(
        service.context().drop_target,
        None,
        "a drop target hidden by the echoed collapse is dropped"
    );
}

// ----------------------------------------------------------------------------
// Codex review regressions, round 6 (PR #695)
// ----------------------------------------------------------------------------

#[test]
fn typeahead_ignored_during_ime_composition() {
    // A key event with is_composing=true carries a transient IME character that
    // must not drive typeahead until composition completes.
    let service = service(props());

    let mut data = printable('g');

    data.is_composing = true;

    assert!(
        dispatch_key(&service, &key(1), &data).is_empty(),
        "no typeahead is dispatched while an IME composition is active"
    );
}

#[test]
fn ime_asterisk_does_not_expand_siblings() {
    // The `*` expand-siblings shortcut is also character input: suppress it mid
    // composition.
    let recorder = Recorder::default();

    let service = service(props());

    let mut data = printable('*');

    data.is_composing = true;

    service
        .connect(&|event| record(&recorder, event))
        .on_node_keydown(&key(2), &data);

    assert!(recorder.into_inner().is_empty());
}

#[test]
fn init_sanitizes_uncontrolled_selected_binding() {
    // Apple (2) is disabled; the public uncontrolled binding must agree with
    // `is_node_selected` (both empty), not retain the stale key.
    let service = service(
        props()
            .items(disabled_items())
            .default_selected(selection::Set::Single(key(2))),
    );

    assert!(
        service.context().selected.get().is_empty(),
        "the uncontrolled selected binding is sanitized at init"
    );
}

#[test]
fn expand_all_skips_disabled_branches() {
    // disabled_branch_tree: node 1 is a disabled, collapsed branch.
    let mut service = service(props().items(disabled_branch_tree()));

    drop(service.send(Event::ExpandAll));

    assert!(
        !service.context().expanded.get().contains(&key(1)),
        "ExpandAll honors the disabled guard and does not expand a disabled branch"
    );
}

#[test]
fn drag_over_disabled_target_is_rejected() {
    // disabled_items: Apple (2) disabled, Grains (7) enabled. Drag 7 over 2.
    let mut service = service(props().items(disabled_items()).dnd_enabled(true));

    drop(service.send(Event::DragStart(key(7))));

    let result = service.send(Event::DragOver(CollectionDropTarget {
        key: key(2),
        position: DropPosition::On,
    }));

    assert!(!result.context_changed);
    assert_eq!(
        service.context().drop_target,
        None,
        "a disabled node is never a valid drop target"
    );
}

#[test]
fn select_all_resolves_to_selectable_keys() {
    // `Set::All` under multiple mode must resolve to the concrete selectable
    // keys, never reporting a disabled node as selected.
    let service = service(
        props()
            .items(disabled_items())
            .multiple(true)
            .selection_mode(selection::Mode::Multiple)
            .default_selected(selection::Set::All),
    );

    let api = service.connect(&|_| {});

    assert!(
        !api.is_node_selected(&key(2)),
        "All does not select the disabled node"
    );
    assert!(api.is_node_selected(&key(7)), "All selects an enabled leaf");
    assert!(
        api.is_node_selected(&key(1)),
        "All selects an enabled branch"
    );
}

#[test]
fn drag_start_from_hidden_node_is_rejected() {
    // Vegetables (4) is collapsed, so Carrot (5) is not in the rendered tree.
    let mut service = service(dnd_props());

    let result = service.send(Event::DragStart(key(5)));

    assert!(!result.context_changed);
    assert_eq!(
        service.context().dragging,
        None,
        "a node hidden under a collapsed parent cannot be a drag source"
    );
}

// ----------------------------------------------------------------------------
// Codex review regressions, round 7 (PR #695)
// ----------------------------------------------------------------------------

#[test]
fn controlled_selection_change_notifies_without_mutating() {
    // A controlled tree must not change its rendered selection optimistically;
    // instead it emits `Effect::SelectionChange` so the parent can echo back.
    let captured: Arc<Mutex<Vec<selection::Set>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&captured);
    let mut service = service(
        props()
            .selected(selection::Set::Empty)
            .on_selection_change(move |set: selection::Set| sink.lock().unwrap().push(set)),
    );

    let mut result = service.send(Event::SelectNode(key(2)));

    assert!(
        service.context().selected.get().is_empty(),
        "controlled selection does not change until the parent echoes it"
    );
    assert!(!service.connect(&|_| {}).is_node_selected(&key(2)));
    assert_eq!(result.pending_effects.len(), 1);
    assert_eq!(result.pending_effects[0].name, Effect::SelectionChange);

    let effect = result
        .pending_effects
        .pop()
        .expect("selection-change effect");

    let noop_send: StrongSend<Event> = Arc::new(|_| {});

    drop(effect.run(service.context(), service.props(), noop_send));

    assert_eq!(
        captured.lock().unwrap().as_slice(),
        &[selection::Set::Single(key(2))],
        "the parent is notified of the requested selection"
    );
}

#[test]
fn controlled_expansion_change_notifies_without_mutating() {
    let captured: Arc<Mutex<Vec<BTreeSet<Key>>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&captured);
    let mut service = service(
        props()
            .expanded(BTreeSet::new()) // controlled, nothing expanded
            .on_expanded_change(move |set: BTreeSet<Key>| sink.lock().unwrap().push(set)),
    );

    let mut result = service.send(Event::ExpandNode(key(4))); // Vegetables

    assert!(
        service.context().expanded.get().is_empty(),
        "controlled expansion does not change until the parent echoes it"
    );
    assert!(!service.connect(&|_| {}).is_node_expanded(&key(4)));
    assert_eq!(result.pending_effects.len(), 1);
    assert_eq!(result.pending_effects[0].name, Effect::ExpandedChange);

    let effect = result
        .pending_effects
        .pop()
        .expect("expanded-change effect");

    let noop_send: StrongSend<Event> = Arc::new(|_| {});

    drop(effect.run(service.context(), service.props(), noop_send));

    let expected: BTreeSet<Key> = [key(4)].into_iter().collect();

    assert_eq!(captured.lock().unwrap().as_slice(), &[expected]);
}

#[test]
fn uncontrolled_changes_mutate_and_emit_effects() {
    // Uncontrolled trees render the change immediately AND emit the effect.
    let mut service = service(props());

    let result = service.send(Event::SelectNode(key(2)));

    assert!(service.connect(&|_| {}).is_node_selected(&key(2)));
    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::SelectionChange),
        "an uncontrolled selection change still notifies the parent"
    );

    let result = service.send(Event::ExpandNode(key(4)));

    assert!(service.connect(&|_| {}).is_node_expanded(&key(4)));
    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::ExpandedChange)
    );
}

#[test]
fn drag_cancelled_when_source_hidden_by_echo() {
    // Fruits (1) is controlled-expanded so Apple (2) is a visible drag source.
    let mut expanded = BTreeSet::new();

    expanded.insert(key(1));

    let mut service = service(dnd_props().expanded(expanded));

    drop(service.send(Event::DragStart(key(2)))); // Apple

    assert_eq!(service.context().dragging, Some(key(2)));

    // Echo a collapse of Fruits: Apple (2) is no longer rendered. The whole drag
    // is cancelled, matching DragStart's hidden-source rejection.
    drop(service.set_props(dnd_props().expanded(BTreeSet::new())));

    assert_eq!(
        service.context().dragging,
        None,
        "a drag whose source becomes hidden is cancelled"
    );
    assert_eq!(service.context().drop_target, None);
}

#[test]
fn toggle_leaf_click_deselects_already_selected_leaf() {
    let mut service = service(
        props()
            .multiple(true)
            .selection_mode(selection::Mode::Multiple),
    );

    drop(service.send(Event::SelectNode(key(2))));

    assert!(service.connect(&|_| {}).is_node_selected(&key(2)));

    let recorder = Recorder::default();

    service
        .connect(&|event| record(&recorder, event))
        .on_leaf_click(&key(2));

    let events = recorder.into_inner();

    assert!(
        events.contains(&Event::DeselectNode(key(2))),
        "under toggle behavior, clicking a selected leaf deselects it"
    );
    assert!(!events.contains(&Event::SelectNode(key(2))));
}

#[test]
fn replace_leaf_click_always_selects() {
    let mut service = service(
        props()
            .multiple(true)
            .selection_mode(selection::Mode::Multiple)
            .selection_behavior(selection::Behavior::Replace),
    );

    drop(service.send(Event::SelectNode(key(2))));

    let recorder = Recorder::default();

    service
        .connect(&|event| record(&recorder, event))
        .on_leaf_click(&key(2));

    let events = recorder.into_inner();

    assert!(
        events.contains(&Event::SelectNode(key(2))),
        "under replace behavior, a click (re)selects rather than deselecting"
    );
    assert!(!events.contains(&Event::DeselectNode(key(2))));
}

// ----------------------------------------------------------------------------
// Snapshots — every anatomy part across output-affecting branches
// ----------------------------------------------------------------------------

#[test]
fn root_snapshots() {
    assert_snapshot!(
        "tree_view_root_default",
        snapshot_attrs(&service(props()).connect(&|_| {}).root_attrs())
    );

    assert_snapshot!(
        "tree_view_root_multiselectable",
        snapshot_attrs(
            &service(props().multiple(true))
                .connect(&|_| {})
                .root_attrs()
        )
    );

    let mut focused = service(props());

    drop(focused.send(Event::FocusNode(key(2))));

    assert_snapshot!(
        "tree_view_root_active_descendant",
        snapshot_attrs(&focused.connect(&|_| {}).root_attrs())
    );
}

#[test]
fn branch_snapshots() {
    assert_snapshot!(
        "tree_view_branch_expanded",
        snapshot_attrs(&service(props()).connect(&|_| {}).branch_attrs(&key(1)))
    );

    assert_snapshot!(
        "tree_view_branch_collapsed",
        snapshot_attrs(&service(props()).connect(&|_| {}).branch_attrs(&key(4)))
    );

    let mut selected = service(props());

    drop(selected.send(Event::SelectNode(key(1))));

    assert_snapshot!(
        "tree_view_branch_selected",
        snapshot_attrs(&selected.connect(&|_| {}).branch_attrs(&key(1)))
    );

    let mut focused = service(props());

    drop(focused.send(Event::FocusFirst)); // keyboard focus on node 1 -> focus-visible

    assert_snapshot!(
        "tree_view_branch_focus_visible",
        snapshot_attrs(&focused.connect(&|_| {}).branch_attrs(&key(1)))
    );

    // Disabled branch.
    let disabled_items = TreeCollection::new(vec![branch(
        1,
        "Fruits",
        true,
        vec![leaf_with(
            2,
            "Apple",
            TreeItem {
                disabled: true,
                ..item("Apple")
            },
        )],
    )]);

    let disabled = service(props().items(disabled_items));

    assert_snapshot!(
        "tree_view_branch_with_disabled_child_leaf",
        snapshot_attrs(&disabled.connect(&|_| {}).leaf_attrs(&key(2)))
    );

    // dnd_enabled branch gains aria-roledescription.
    assert_snapshot!(
        "tree_view_branch_dnd_enabled",
        snapshot_attrs(&service(dnd_props()).connect(&|_| {}).branch_attrs(&key(1)))
    );
}

#[test]
fn branch_control_snapshots() {
    assert_snapshot!(
        "tree_view_branch_control_plain",
        snapshot_attrs(
            &service(props())
                .connect(&|_| {})
                .branch_control_attrs(&key(1))
        )
    );

    // href on the branch itself renders BranchControl as an `<a>`.
    let linked_branch = TreeCollection::new(vec![branch_with_href()]);

    assert_snapshot!(
        "tree_view_branch_control_href",
        snapshot_attrs(
            &service(props().items(linked_branch))
                .connect(&|_| {})
                .branch_control_attrs(&key(1))
        )
    );
}

fn branch_with_href() -> TreeItemConfig<TreeItem> {
    TreeItemConfig {
        key: key(1),
        text_value: "Fruits".to_string(),
        value: TreeItem {
            href: Some("/fruits".to_string()),
            ..item("Fruits")
        },
        children: vec![leaf(2, "Apple")],
        default_expanded: true,
    }
}

#[test]
fn branch_indicator_snapshots() {
    assert_snapshot!(
        "tree_view_branch_indicator_expanded",
        snapshot_attrs(
            &service(props())
                .connect(&|_| {})
                .branch_indicator_attrs(&key(1))
        )
    );
    assert_snapshot!(
        "tree_view_branch_indicator_collapsed",
        snapshot_attrs(
            &service(props())
                .connect(&|_| {})
                .branch_indicator_attrs(&key(4))
        )
    );
}

#[test]
fn branch_text_and_leaf_text_snapshots() {
    assert_snapshot!(
        "tree_view_branch_text",
        snapshot_attrs(&service(props()).connect(&|_| {}).branch_text_attrs())
    );
    assert_snapshot!(
        "tree_view_leaf_text",
        snapshot_attrs(&service(props()).connect(&|_| {}).leaf_text_attrs())
    );
}

#[test]
fn branch_content_snapshots() {
    assert_snapshot!(
        "tree_view_branch_content_expanded",
        snapshot_attrs(
            &service(props())
                .connect(&|_| {})
                .branch_content_attrs(&key(1))
        )
    );
    assert_snapshot!(
        "tree_view_branch_content_collapsed_hidden",
        snapshot_attrs(
            &service(props())
                .connect(&|_| {})
                .branch_content_attrs(&key(4))
        )
    );
}

#[test]
fn leaf_snapshots() {
    assert_snapshot!(
        "tree_view_leaf_default",
        snapshot_attrs(&service(props()).connect(&|_| {}).leaf_attrs(&key(2)))
    );

    let mut selected = service(props());

    drop(selected.send(Event::SelectNode(key(2))));

    assert_snapshot!(
        "tree_view_leaf_selected",
        snapshot_attrs(&selected.connect(&|_| {}).leaf_attrs(&key(2)))
    );

    // Lazy leaf: has_children flag set but no real children -> aria-expanded.
    let lazy = TreeCollection::new(vec![leaf_with(
        9,
        "Lazy",
        TreeItem {
            has_children: true,
            ..item("Lazy")
        },
    )]);

    assert_snapshot!(
        "tree_view_leaf_has_children_flag",
        snapshot_attrs(
            &service(props().items(lazy))
                .connect(&|_| {})
                .leaf_attrs(&key(9))
        )
    );

    // Leaf with href.
    let linked = TreeCollection::new(vec![leaf_with(
        9,
        "Docs",
        TreeItem {
            href: Some("/docs".to_string()),
            ..item("Docs")
        },
    )]);

    assert_snapshot!(
        "tree_view_leaf_href",
        snapshot_attrs(
            &service(props().items(linked))
                .connect(&|_| {})
                .leaf_attrs(&key(9))
        )
    );
}

#[test]
fn drag_handle_snapshots() {
    assert_snapshot!(
        "tree_view_drag_handle_default",
        snapshot_attrs(
            &service(dnd_props())
                .connect(&|_| {})
                .drag_handle_attrs(&key(2))
        )
    );

    let mut dragging = service(dnd_props());

    drop(dragging.send(Event::DragStart(key(2))));

    assert_snapshot!(
        "tree_view_drag_handle_grabbed",
        snapshot_attrs(&dragging.connect(&|_| {}).drag_handle_attrs(&key(2)))
    );
}

#[test]
fn drop_indicator_snapshots() {
    let api_service = service(dnd_props());

    let api = api_service.connect(&|_| {});

    for (name, position) in [
        ("tree_view_drop_indicator_before", DropPosition::Before),
        ("tree_view_drop_indicator_on", DropPosition::On),
        ("tree_view_drop_indicator_after", DropPosition::After),
    ] {
        let attrs = api.drop_indicator_attrs(&CollectionDropTarget {
            key: key(7),
            position,
        });

        assert_snapshot!(name, snapshot_attrs(&attrs));
    }
}

// ----------------------------------------------------------------------------
// §5 Lazy loading
// ----------------------------------------------------------------------------

/// A tree whose branch (1, "Fruits") has the `has_children` affordance but no
/// loaded children — the canonical `NotLoaded` lazy branch. Node 4 ("Vegetables")
/// is a real, loaded branch.
fn lazy_items() -> TreeCollection<TreeItem> {
    TreeCollection::new(vec![
        leaf_with(
            1,
            "Fruits",
            TreeItem {
                has_children: true,
                ..item("Fruits")
            },
        ),
        branch(4, "Vegetables", false, vec![leaf(5, "Carrot")]),
        leaf(7, "Grains"),
    ])
}

fn lazy_props() -> Props {
    Props::new().id("tree").items(lazy_items())
}

/// Children configs delivered by a lazy load under "Fruits" (1).
fn loaded_children() -> Vec<TreeItemConfig<TreeItem>> {
    vec![leaf(2, "Apple"), leaf(3, "Banana")]
}

#[test]
fn init_seeds_load_state_not_loaded_for_lazy_branch() {
    let service = service(lazy_props());

    let api = service.connect(&|_| {});

    // Lazy branch (has_children flag, no real children) starts NotLoaded.
    assert_eq!(api.node_load_state(&key(1)), NodeLoadState::NotLoaded);
    // A real, loaded branch starts Loaded.
    assert_eq!(api.node_load_state(&key(4)), NodeLoadState::Loaded);
    // A plain leaf starts Loaded.
    assert_eq!(api.node_load_state(&key(7)), NodeLoadState::Loaded);
}

#[test]
fn expanding_lazy_branch_emits_load_children_and_marks_loading() {
    let requested: Arc<Mutex<Vec<Key>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&requested);

    let mut service =
        service(lazy_props().on_load_children(move |key: Key| sink.lock().unwrap().push(key)));

    let mut result = service.send(Event::ExpandNode(key(1)));

    // The branch is marked Loading immediately.
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loading
    );
    assert!(service.connect(&|_| {}).is_loading(&key(1)));

    // A LoadChildren effect is emitted; running it invokes `on_load_children`
    // with the branch key so the app can fetch the children.
    let index = result
        .pending_effects
        .iter()
        .position(|effect| effect.name == Effect::LoadChildren)
        .expect("expanding a NotLoaded branch emits Effect::LoadChildren");

    let effect = result.pending_effects.remove(index);

    let noop_send: StrongSend<Event> = Arc::new(|_| {});

    drop(effect.run(service.context(), service.props(), noop_send));

    assert_eq!(requested.lock().unwrap().as_slice(), &[key(1)]);
}

#[test]
fn expanding_loaded_branch_does_not_emit_load_children() {
    let mut service = service(lazy_props());

    let result = service.send(Event::ExpandNode(key(4)));

    assert!(
        result
            .pending_effects
            .iter()
            .all(|effect| effect.name != Effect::LoadChildren),
        "expanding an already-loaded branch must not trigger a lazy load"
    );
}

#[test]
fn toggle_lazy_branch_emits_load_children() {
    let mut service = service(lazy_props());

    let result = service.send(Event::ToggleNode(key(1)));

    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::LoadChildren)
    );
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loading
    );
}

#[test]
fn expand_all_triggers_lazy_load_for_unloaded_branch() {
    let mut service = service(lazy_props());

    let result = service.send(Event::ExpandAll);

    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::LoadChildren),
        "ExpandAll triggers lazy loading for NotLoaded branches"
    );
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loading
    );
}

#[test]
fn children_loaded_inserts_and_marks_loaded() {
    let mut service = service(lazy_props());

    drop(service.send(Event::ExpandNode(key(1))));

    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loading
    );

    drop(service.send(Event::ChildrenLoaded {
        parent: key(1),
        children: loaded_children(),
    }));

    let ctx = service.context();
    // Children are now present in the collection under "Fruits".
    assert!(
        ctx.items.get(&key(2)).is_some(),
        "Apple inserted under Fruits"
    );
    assert!(
        ctx.items.get(&key(3)).is_some(),
        "Banana inserted under Fruits"
    );
    assert_eq!(
        ctx.items.get(&key(2)).and_then(|n| n.parent_key.clone()),
        Some(key(1))
    );
    // The parent transitions to Loaded.
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loaded
    );
}

#[test]
fn reexpanding_already_expanded_unloaded_branch_triggers_load() {
    // A branch that booted expanded (via default_expanded) but lazy stays
    // NotLoaded until the user expands it; re-sending ExpandNode (the expansion
    // set is unchanged) still fires the lazy load.
    let mut expanded = BTreeSet::new();

    expanded.insert(key(1));

    let mut service = service(lazy_props().default_expanded(expanded));

    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::NotLoaded,
        "a default-expanded lazy branch does not auto-load at init"
    );

    let result = service.send(Event::ExpandNode(key(1)));

    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::LoadChildren),
        "re-expanding an already-expanded NotLoaded branch retries the load"
    );
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loading
    );
}

#[test]
fn controlled_expansion_still_fires_lazy_load_on_first_expand() {
    // A controlled tree does not optimistically change `expanded`, but the lazy
    // load must still fire so children can be fetched before the echo.
    let mut service = service(lazy_props().expanded(BTreeSet::new()));

    let result = service.send(Event::ExpandNode(key(1)));

    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::LoadChildren)
    );
    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::ExpandedChange),
        "the expanded-change notification still fires for the controlled echo"
    );
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loading
    );
    // Controlled: the rendered expansion is unchanged until the parent echoes.
    assert!(!service.connect(&|_| {}).is_node_expanded(&key(1)));
}

#[test]
fn load_error_then_reexpand_retries() {
    let mut service = service(lazy_props());

    drop(service.send(Event::ExpandNode(key(1))));
    drop(service.send(Event::LoadError(key(1))));

    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Error
    );

    // Re-expanding an `Error` branch retries the lazy load: the failed state
    // would otherwise strand any retry affordance the adapter exposes, since
    // there is no separate retry event. The branch flips back to `Loading`
    // and a fresh `LoadChildren` effect fires for the consumer.
    let result = service.send(Event::ExpandNode(key(1)));

    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::LoadChildren),
        "an Error branch must re-trigger LoadChildren on re-expand (retry)"
    );
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loading
    );
}

#[test]
fn loading_branch_advertises_aria_busy() {
    let mut service = service(lazy_props());

    drop(service.send(Event::ExpandNode(key(1))));

    let api = service.connect(&|_| {});

    let attrs = api.leaf_attrs(&key(1)); // node 1 is a lazy leaf-shaped branch

    assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Busy)), Some("true"));
    assert_eq!(attrs.get(&HtmlAttr::Data("ars-loading")), Some("true"));
}

#[test]
fn children_loaded_ignored_for_unknown_parent() {
    let mut service = service(lazy_props());

    let result = service.send(Event::ChildrenLoaded {
        parent: key(999),
        children: loaded_children(),
    });

    assert!(
        !result.context_changed,
        "children for an absent parent are ignored, not spliced under a dangling key"
    );
}

#[test]
fn load_error_ignored_for_unknown_node() {
    let mut service = service(lazy_props());

    let result = service.send(Event::LoadError(key(999)));

    assert!(!result.context_changed);
}

#[test]
fn rename_start_ignored_for_unknown_node() {
    let mut service = service(renamable_props());

    let result = service.send(Event::RenameStart(key(999)));

    assert!(!result.context_changed);
    assert_eq!(service.context().renaming_key, None);
}

#[test]
fn load_error_marks_error_state() {
    let mut service = service(lazy_props());

    drop(service.send(Event::ExpandNode(key(1))));
    drop(service.send(Event::LoadError(key(1))));

    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Error
    );
}

#[test]
fn sync_props_reseeds_load_state() {
    // After loading children and re-deriving props, the now-loaded branch must
    // report Loaded (its load_state is reseeded from the new collection).
    let mut service = service(lazy_props());

    drop(service.send(Event::ExpandNode(key(1))));
    drop(service.send(Event::ChildrenLoaded {
        parent: key(1),
        children: loaded_children(),
    }));

    // Re-derive props from the now-populated collection.
    let new_items = service.context().items.clone();

    drop(service.set_props(Props::new().id("tree").items(new_items)));

    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loaded
    );
}

// ----------------------------------------------------------------------------
// §6 Renamable nodes
// ----------------------------------------------------------------------------

fn renamable_props() -> Props {
    props().renamable(true)
}

#[test]
fn renamable_defaults_false() {
    assert!(!Props::new().renamable);
    assert!(Props::new().renamable(true).renamable);
}

#[test]
fn init_renaming_key_none() {
    assert_eq!(service(renamable_props()).context().renaming_key, None);
}

#[test]
fn rename_start_sets_renaming_key_when_enabled() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));

    assert_eq!(service.context().renaming_key, Some(key(2)));
    assert_eq!(service.context().focused_node, Some(key(2)));
    assert!(service.connect(&|_| {}).is_renaming(&key(2)));
}

#[test]
fn rename_start_ignored_when_not_renamable() {
    let mut service = service(props()); // renamable = false

    let result = service.send(Event::RenameStart(key(2)));

    assert!(!result.context_changed);
    assert_eq!(service.context().renaming_key, None);
}

#[test]
fn rename_start_ignored_for_disabled_node() {
    let disabled_items = TreeCollection::new(vec![branch(
        1,
        "Fruits",
        true,
        vec![leaf_with(
            2,
            "Apple",
            TreeItem {
                disabled: true,
                ..item("Apple")
            },
        )],
    )]);

    let mut service = service(renamable_props().items(disabled_items));

    let result = service.send(Event::RenameStart(key(2)));

    assert!(!result.context_changed);
    assert_eq!(service.context().renaming_key, None);
}

#[test]
fn rename_commit_clears_renaming_key() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));
    drop(service.send(Event::RenameCommit {
        key: key(2),
        new_name: "Apricot".to_string(),
    }));

    assert_eq!(service.context().renaming_key, None);
}

#[test]
fn rename_commit_for_outgoing_key_still_fires_effect() {
    // During a rename retarget the outgoing input's blur fires
    // `RenameCommit` for the previous key while `renaming_key` already
    // points at the new node. The transition must still emit
    // `Effect::Rename` so the outgoing edit reaches the consumer; only the
    // `renaming_key` clear is gated on the key actually being the active
    // target.
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));

    let result = service.send(Event::RenameCommit {
        key: key(3),
        new_name: "x".to_string(),
    });

    // `renaming_key` is unchanged — the new active target stays.
    assert_eq!(service.context().renaming_key, Some(key(2)));

    // ...but `Effect::Rename` did fire for the committed (outgoing) key, so
    // the consumer's `on_rename` receives the outgoing edit.
    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::Rename),
        "RenameCommit for an outgoing key still emits Effect::Rename"
    );
}

#[test]
fn rename_commit_no_op_when_no_rename_active() {
    // A stray `RenameCommit` when nothing is renaming anywhere must be a
    // pure no-op — no context change, no effect fired. Distinct from the
    // retarget hand-off above (where a rename IS active on another key).
    let mut service = service(renamable_props());

    let result = service.send(Event::RenameCommit {
        key: key(2),
        new_name: "x".to_string(),
    });

    assert!(!result.context_changed);
    assert!(result.pending_effects.is_empty());
    assert_eq!(service.context().renaming_key, None);
}

#[test]
fn rename_cancel_clears_renaming_key() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));
    drop(service.send(Event::RenameCancel(key(2))));

    assert_eq!(service.context().renaming_key, None);
}

#[test]
fn rename_cancel_ignored_for_non_renaming_key() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));

    let result = service.send(Event::RenameCancel(key(3)));

    assert!(!result.context_changed);
    assert_eq!(service.context().renaming_key, Some(key(2)));
}

#[test]
fn rename_start_while_renaming_commits_previous() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));

    assert_eq!(service.context().renaming_key, Some(key(2)));

    // Starting rename on a second node commits the first, then activates the new.
    drop(service.send(Event::RenameStart(key(3))));

    assert_eq!(
        service.context().renaming_key,
        Some(key(3)),
        "the new node becomes the active rename target"
    );
}

#[test]
fn f2_dispatches_rename_start_on_focused_node() {
    let mut service = service(renamable_props());

    drop(service.send(Event::FocusNode(key(2))));

    let recorder = Recorder::default();

    service
        .connect(&|event| record(&recorder, event))
        .on_node_keydown(&key(2), &keyboard(KeyboardKey::F2));

    assert!(
        recorder.into_inner().contains(&Event::RenameStart(key(2))),
        "F2 starts rename on the focused node"
    );
}

#[test]
fn sync_props_clears_renaming_key_when_node_removed() {
    // Renaming a node, then a data update that removes it, must not leave a
    // dangling rename target.
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2)))); // Apple

    assert_eq!(service.context().renaming_key, Some(key(2)));

    // New collection without node 2.
    let without_apple = TreeCollection::new(vec![
        branch(1, "Fruits", true, vec![leaf(3, "Banana")]),
        leaf(7, "Grains"),
    ]);

    drop(service.set_props(renamable_props().items(without_apple)));

    assert_eq!(
        service.context().renaming_key,
        None,
        "a rename target removed by a data update is cleared"
    );
}

#[test]
fn sync_props_keeps_renaming_key_when_node_survives() {
    // A data update that keeps the renaming node must preserve the rename.
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));

    // Same shape, different instance (forces an items_changed SyncProps).
    let same_with_extra = TreeCollection::new(vec![
        branch(1, "Fruits", true, vec![leaf(2, "Apple"), leaf(3, "Banana")]),
        branch(
            4,
            "Vegetables",
            false,
            vec![leaf(5, "Carrot"), leaf(6, "Daikon")],
        ),
        leaf(7, "Grains"),
        leaf(8, "Extra"),
    ]);

    drop(service.set_props(renamable_props().items(same_with_extra)));

    assert_eq!(
        service.context().renaming_key,
        Some(key(2)),
        "a surviving rename target is preserved across a data update"
    );
}

#[test]
fn part_attrs_dispatches_node_rename_input() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));

    let api = service.connect(&|_| {});

    let attrs = api.part_attrs(Part::NodeRenameInput { node_id: key(2) });

    assert_eq!(attrs.get(&HtmlAttr::Type), Some("text"));
    assert_eq!(
        attrs.get(&HtmlAttr::Data("ars-part")),
        Some("node-rename-input")
    );
}

#[test]
fn node_rename_input_attrs_shape() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2)))); // Apple

    let api = service.connect(&|_| {});

    let attrs = api.node_rename_input_attrs(&key(2));

    assert_eq!(attrs.get(&HtmlAttr::Type), Some("text"));
    assert_eq!(attrs.get(&HtmlAttr::Value), Some("Apple"));
    assert_eq!(
        attrs.get(&HtmlAttr::Aria(AriaAttr::Label)),
        Some("Rename Apple")
    );
}

#[test]
fn on_rename_input_keydown_enter_commits() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));

    let recorder = Recorder::default();

    service
        .connect(&|event| record(&recorder, event))
        .on_rename_input_keydown(&key(2), "Enter", "Apricot");

    assert!(recorder.into_inner().contains(&Event::RenameCommit {
        key: key(2),
        new_name: "Apricot".to_string(),
    }));
}

#[test]
fn on_rename_input_keydown_escape_cancels() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));

    let recorder = Recorder::default();

    service
        .connect(&|event| record(&recorder, event))
        .on_rename_input_keydown(&key(2), "Escape", "ignored");

    assert!(recorder.into_inner().contains(&Event::RenameCancel(key(2))));
}

#[test]
fn on_rename_input_blur_commits_when_renaming() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));

    let recorder = Recorder::default();

    service
        .connect(&|event| record(&recorder, event))
        .on_rename_input_blur(&key(2), "Apricot");

    assert!(recorder.into_inner().contains(&Event::RenameCommit {
        key: key(2),
        new_name: "Apricot".to_string(),
    }));
}

#[test]
fn on_rename_input_blur_noop_when_not_renaming() {
    let service = service(renamable_props()); // no active rename

    let recorder = Recorder::default();

    service
        .connect(&|event| record(&recorder, event))
        .on_rename_input_blur(&key(2), "Apricot");

    assert!(recorder.into_inner().is_empty());
}

#[test]
fn expand_all_loads_every_lazy_branch() {
    // Two lazy branches — `Fruits` (1) and `Beverages` (8). `ExpandAll` must
    // fan one `LoadChildren` effect per `NotLoaded` branch and mark every one
    // `Loading`, otherwise the bulk path strands the second branch as
    // expanded-but-empty without ever calling `on_load_children`.
    let items = TreeCollection::new(vec![
        leaf_with(
            1,
            "Fruits",
            TreeItem {
                has_children: true,
                ..item("Fruits")
            },
        ),
        leaf_with(
            8,
            "Beverages",
            TreeItem {
                has_children: true,
                ..item("Beverages")
            },
        ),
        leaf(7, "Grains"),
    ]);

    let requested: Arc<Mutex<Vec<Key>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&requested);

    let mut service = service(
        Props::new()
            .id("tree")
            .items(items)
            .on_load_children(move |k: Key| sink.lock().unwrap().push(k)),
    );

    let result = service.send(Event::ExpandAll);

    let api = service.connect(&|_| {});
    assert_eq!(api.node_load_state(&key(1)), NodeLoadState::Loading);
    assert_eq!(api.node_load_state(&key(8)), NodeLoadState::Loading);

    // Two `LoadChildren` effects emitted — one per lazy branch.
    let load_effects: Vec<_> = result
        .pending_effects
        .into_iter()
        .filter(|effect| effect.name == Effect::LoadChildren)
        .collect();
    assert_eq!(
        load_effects.len(),
        2,
        "one LoadChildren effect per lazy branch"
    );

    // Running each effect invokes `on_load_children` with that branch's key.
    let noop_send: StrongSend<Event> = Arc::new(|_| {});
    for effect in load_effects {
        drop(effect.run(service.context(), service.props(), Arc::clone(&noop_send)));
    }

    let mut got = requested.lock().unwrap().clone();
    got.sort();
    let mut want = vec![key(1), key(8)];
    want.sort();
    assert_eq!(got, want);
}

#[test]
fn rename_commit_emits_rename_event_to_callback() {
    // `RenameCommit { key, new_name }` is the only path that surfaces the
    // edited label to the consumer. Without an `on_rename` callback fired by
    // `Effect::Rename`, the new value would be silently discarded.
    let captured: Arc<Mutex<Vec<RenameEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&captured);

    let mut service = service(
        renamable_props().on_rename(move |event: RenameEvent| sink.lock().unwrap().push(event)),
    );

    drop(service.send(Event::RenameStart(key(2))));
    let mut result = service.send(Event::RenameCommit {
        key: key(2),
        new_name: "Apricot".to_string(),
    });

    // Renaming clears the rename_key and emits a single Rename effect.
    assert_eq!(service.context().renaming_key, None);

    let index = result
        .pending_effects
        .iter()
        .position(|e| e.name == Effect::Rename)
        .expect("RenameCommit must emit Effect::Rename");
    let effect = result.pending_effects.remove(index);

    let noop_send: StrongSend<Event> = Arc::new(|_| {});
    drop(effect.run(service.context(), service.props(), noop_send));

    assert_eq!(
        captured.lock().unwrap().as_slice(),
        &[RenameEvent {
            key: key(2),
            new_name: "Apricot".to_string(),
        }]
    );
}

#[test]
fn disabling_renamable_via_sync_props_cancels_active_rename() {
    // Flipping `renamable` false mid-edit must cancel the active rename so
    // adapters do not keep rendering `NodeRenameInput` against a tree whose
    // props now say renaming is off. The gate has two parts: `on_props_changed`
    // emits `SyncProps` on the toggle, and `SyncProps` itself clears
    // `renaming_key` when the live prop is false.
    let mut service = service(renamable_props());
    drop(service.send(Event::RenameStart(key(2))));
    assert_eq!(service.context().renaming_key, Some(key(2)));

    let new_props = renamable_props().renamable(false);
    let triggered = <Machine as ars_core::Machine>::on_props_changed(service.props(), &new_props);
    assert!(
        triggered.contains(&Event::SyncProps),
        "on_props_changed must emit SyncProps when `renamable` flips"
    );

    drop(service.set_props(new_props));
    for event in triggered {
        drop(service.send(event));
    }

    assert_eq!(service.context().renaming_key, None);
    assert!(!service.connect(&|_| {}).is_renaming(&key(2)));
}

#[test]
fn children_loaded_ignored_when_parent_already_loaded() {
    // A duplicate or late `ChildrenLoaded` delivery must not re-insert the
    // children: `TreeCollection::new` accepts duplicate nodes, so a second
    // insertion would leave duplicated visible rows whose `get()` only points
    // at the last copy. Only an in-flight `Loading` parent accepts the
    // delivery.
    let mut service = service(lazy_props());

    drop(service.send(Event::ExpandNode(key(1)))); // Loading
    drop(service.send(Event::ChildrenLoaded {
        parent: key(1),
        children: loaded_children(),
    })); // Loaded

    let after_first = service.context().items.clone();
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loaded
    );

    // Second delivery while the parent is already `Loaded` must be a no-op.
    let result = service.send(Event::ChildrenLoaded {
        parent: key(1),
        children: loaded_children(),
    });

    assert!(
        !result.context_changed,
        "duplicate ChildrenLoaded for a Loaded parent must be ignored"
    );
    assert_eq!(&after_first, &service.context().items);
}

#[test]
fn lazy_loaded_children_survive_unrelated_sync_props_echo() {
    // `ctx.items` carries lazy-spliced children that the consumer's
    // `Props::items` may not have echoed back. A `SyncProps` for an unrelated
    // prop change (e.g., toggling `renamable`) must not wipe that subtree:
    // the comparison key is the last-seen `Props::items` baseline, not
    // `ctx.items` itself.
    let mut service = service(lazy_props().renamable(true));

    drop(service.send(Event::ExpandNode(key(1))));
    drop(service.send(Event::ChildrenLoaded {
        parent: key(1),
        children: loaded_children(),
    }));
    let items_after_load = service.context().items.clone();
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loaded
    );
    assert!(items_after_load.get(&key(2)).is_some());

    // Toggle `renamable` (true -> false); items prop stays the same.
    let new_props = lazy_props().renamable(false);
    let triggered = <Machine as ars_core::Machine>::on_props_changed(service.props(), &new_props);
    assert!(triggered.contains(&Event::SyncProps));

    drop(service.set_props(new_props));
    for event in triggered {
        drop(service.send(event));
    }

    // The lazy-loaded subtree survives the unrelated echo, and load_state
    // for the parent is still Loaded (not reset to NotLoaded).
    assert_eq!(&items_after_load, &service.context().items);
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loaded
    );
    assert!(service.context().items.get(&key(2)).is_some());
    assert!(service.context().items.get(&key(3)).is_some());
}

#[test]
fn rename_cancelled_when_target_node_becomes_disabled() {
    // A consumer-driven prop update can flip a renaming node to `disabled`.
    // The rest of the machine treats disabled nodes as blocking all
    // interaction, so an in-flight rename must not survive the transition.
    let mut service = service(renamable_props());
    drop(service.send(Event::RenameStart(key(2))));
    assert_eq!(service.context().renaming_key, Some(key(2)));

    // Build a new items tree where the same key (`2`) is now disabled.
    let disabled_items = TreeCollection::new(vec![branch(
        1,
        "Fruits",
        true,
        vec![
            leaf_with(
                2,
                "Banana",
                TreeItem {
                    disabled: true,
                    ..item("Banana")
                },
            ),
            leaf(3, "Cherry"),
        ],
    )]);
    let new_props = Props::new()
        .id("tree")
        .items(disabled_items)
        .renamable(true);

    let triggered = <Machine as ars_core::Machine>::on_props_changed(service.props(), &new_props);
    assert!(triggered.contains(&Event::SyncProps));

    drop(service.set_props(new_props));
    for event in triggered {
        drop(service.send(event));
    }

    assert_eq!(service.context().renaming_key, None);
    assert!(!service.connect(&|_| {}).is_renaming(&key(2)));
}

#[test]
fn load_error_ignored_when_parent_already_loaded() {
    // A late `LoadError` arriving after the same async request has already
    // delivered `ChildrenLoaded` must not flip an already-`Loaded` branch
    // back to `Error`; later retries would otherwise request loading again
    // for a populated subtree. Only an in-flight `Loading` parent accepts
    // the failure (mirrors the `ChildrenLoaded` guard).
    let mut service = service(lazy_props());

    drop(service.send(Event::ExpandNode(key(1)))); // Loading
    drop(service.send(Event::ChildrenLoaded {
        parent: key(1),
        children: loaded_children(),
    })); // Loaded

    let items_after = service.context().items.clone();
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loaded
    );

    let result = service.send(Event::LoadError(key(1)));

    assert!(
        !result.context_changed,
        "stale LoadError after success must be ignored"
    );
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loaded
    );
    assert_eq!(&items_after, &service.context().items);
}

#[test]
fn children_loaded_recomputes_disabled_keys_for_loaded_disabled_child() {
    // A lazy load can deliver children carrying `TreeItem { disabled: true }`.
    // `ChildrenLoaded` must rebuild `selection_state.disabled_keys` from the
    // updated collection — otherwise `SelectNode` happily admits a key whose
    // node is now disabled (the selection machine still trusts a stale
    // disabled set).
    let mut service = service(lazy_props());

    drop(service.send(Event::ExpandNode(key(1))));
    drop(service.send(Event::ChildrenLoaded {
        parent: key(1),
        children: vec![
            leaf_with(
                2,
                "Apple",
                TreeItem {
                    disabled: true,
                    ..item("Apple")
                },
            ),
            leaf(3, "Banana"),
        ],
    }));

    // The disabled-key set now contains the newly-loaded disabled child.
    assert!(
        service
            .context()
            .selection_state
            .disabled_keys
            .contains(&key(2))
    );

    // SelectNode on the disabled-but-loaded child is a no-op.
    let before = service.context().selected.get().clone();
    drop(service.send(Event::SelectNode(key(2))));
    assert_eq!(&before, service.context().selected.get());

    // The enabled sibling can still be selected.
    drop(service.send(Event::SelectNode(key(3))));
    assert!(service.context().selected.get().contains(&key(3)));
}

#[test]
fn lazy_loaded_disabled_child_stays_unselectable_across_unrelated_sync_props() {
    // After `ChildrenLoaded` inserts a disabled child and `disabled_keys` is
    // recomputed (R3-T1), an unrelated `SyncProps` echo that preserves
    // `ctx.items` (the lazy subtree survives unchanged `Props::items`) must
    // also preserve the recomputed disabled-key set — otherwise the closure
    // would silently restore the stale `props.items`-derived set and
    // re-admit the disabled child to `SelectNode`.
    let mut service = service(lazy_props().renamable(true));

    drop(service.send(Event::ExpandNode(key(1))));
    drop(service.send(Event::ChildrenLoaded {
        parent: key(1),
        children: vec![leaf_with(
            2,
            "Apple",
            TreeItem {
                disabled: true,
                ..item("Apple")
            },
        )],
    }));
    assert!(
        service
            .context()
            .selection_state
            .disabled_keys
            .contains(&key(2))
    );

    // Trigger an unrelated SyncProps: toggle `renamable` (items prop unchanged).
    let new_props = lazy_props().renamable(false);
    let triggered = <Machine as ars_core::Machine>::on_props_changed(service.props(), &new_props);
    assert!(triggered.contains(&Event::SyncProps));
    drop(service.set_props(new_props));
    for event in triggered {
        drop(service.send(event));
    }

    // The disabled-key set still includes the lazily-loaded disabled child.
    assert!(
        service
            .context()
            .selection_state
            .disabled_keys
            .contains(&key(2))
    );

    // SelectNode on the disabled-but-loaded child remains a no-op.
    let before = service.context().selected.get().clone();
    drop(service.send(Event::SelectNode(key(2))));
    assert_eq!(&before, service.context().selected.get());
}

#[test]
fn children_loaded_rejects_payload_with_duplicate_keys() {
    // `TreeCollection::new` silently accepts duplicate nodes while resolving
    // `get()` to the last occurrence only, so a lazy-load payload with a
    // duplicated key would leave focus / selection / ARIA ids pointing at a
    // different row than the visible duplicate. The `ChildrenLoaded` arm
    // rejects such payloads — both keys that collide with the existing
    // collection AND keys that duplicate each other within the payload.

    // Case A: loaded child collides with a key that already exists in the
    // tree (`key(7)` "Grains" is a root sibling in `lazy_items`).
    let mut service_a = service(lazy_props());
    drop(service_a.send(Event::ExpandNode(key(1))));

    let result_a = service_a.send(Event::ChildrenLoaded {
        parent: key(1),
        children: vec![leaf(7, "Duplicate Grains")],
    });

    assert!(
        !result_a.context_changed,
        "ChildrenLoaded must reject a payload whose key already exists in items"
    );
    // Parent stays `Loading` — the delivery was rejected, not consumed, so
    // a retry / a corrected payload can still settle the load.
    assert_eq!(
        service_a.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loading
    );

    // Case B: loaded children duplicate each other within the payload.
    let mut service_b = service(lazy_props());
    drop(service_b.send(Event::ExpandNode(key(1))));

    let result_b = service_b.send(Event::ChildrenLoaded {
        parent: key(1),
        children: vec![leaf(2, "Apple"), leaf(2, "Apple Twin")],
    });

    assert!(
        !result_b.context_changed,
        "ChildrenLoaded must reject a payload whose own keys duplicate each other"
    );
    assert_eq!(
        service_b.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loading
    );
}

#[test]
fn children_loaded_seeds_default_expanded_for_loaded_subtree() {
    // `TreeItemConfig::default_expanded: true` is honored at init via the
    // initial `ctx.expanded` seeding. Lazy-load deliveries must honor it
    // too — `TreeCollection::new` records the marker internally, but
    // rendering is driven by `ctx.expanded`, so the descendant would
    // render collapsed without an explicit merge.
    let mut service = service(lazy_props());

    drop(service.send(Event::ExpandNode(key(1))));
    drop(service.send(Event::ChildrenLoaded {
        parent: key(1),
        children: vec![TreeItemConfig {
            key: key(2),
            text_value: "Apple".to_string(),
            value: item("Apple"),
            children: vec![leaf(20, "Apple Pie")],
            default_expanded: true,
        }],
    }));

    let expanded = service.context().expanded.get();
    assert!(
        expanded.contains(&key(2)),
        "loaded child with default_expanded must be merged into ctx.expanded"
    );
    assert!(service.connect(&|_| {}).is_node_expanded(&key(2)));
}

#[test]
fn blur_during_retarget_fires_rename_effect_for_outgoing_input() {
    // Adapter contract: `on_rename_input_blur` fires `RenameCommit` for the
    // outgoing input after a retarget, so the user's edit reaches
    // `Props::on_rename` via `Effect::Rename`. The new active rename
    // (`renaming_key = Some(other)`) is unaffected.
    let captured: Arc<Mutex<Vec<RenameEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&captured);

    let mut service = service(
        renamable_props().on_rename(move |event: RenameEvent| sink.lock().unwrap().push(event)),
    );

    // Start renaming node 2, then retarget to node 3 — `renaming_key` is now
    // Some(key(3)) and node 2's input is the outgoing surface.
    drop(service.send(Event::RenameStart(key(2))));
    drop(service.send(Event::RenameStart(key(3))));
    assert_eq!(service.context().renaming_key, Some(key(3)));

    // Adapter dispatches the outgoing input's blur — through the API to
    // verify the gate (must fire because `renaming_key.is_some()`).
    let sent: Arc<Mutex<Vec<Event>>> = Arc::new(Mutex::new(Vec::new()));
    let sent_clone = Arc::clone(&sent);
    service
        .connect(&move |event| sent_clone.lock().unwrap().push(event))
        .on_rename_input_blur(&key(2), "Apricot");

    let outgoing_event = sent
        .lock()
        .unwrap()
        .iter()
        .find_map(|event| match event {
            Event::RenameCommit { key, new_name } => Some((key.clone(), new_name.clone())),
            _ => None,
        })
        .expect("blur during retarget must dispatch RenameCommit for the outgoing key");
    assert_eq!(outgoing_event, (key(2), "Apricot".to_string()));

    // Replay the event through the service so the effect surface materializes.
    let mut result = service.send(Event::RenameCommit {
        key: key(2),
        new_name: "Apricot".to_string(),
    });

    // Active rename target is unchanged.
    assert_eq!(service.context().renaming_key, Some(key(3)));

    // ...but Effect::Rename fired; running it delivers the outgoing edit.
    let index = result
        .pending_effects
        .iter()
        .position(|effect| effect.name == Effect::Rename)
        .expect("retargeted RenameCommit emits Effect::Rename");
    let effect = result.pending_effects.remove(index);
    let noop_send: StrongSend<Event> = Arc::new(|_| {});
    drop(effect.run(service.context(), service.props(), noop_send));

    assert_eq!(
        captured.lock().unwrap().as_slice(),
        &[RenameEvent {
            key: key(2),
            new_name: "Apricot".to_string(),
        }]
    );
}

#[test]
fn toggle_node_on_error_branch_retries_load() {
    // After a lazy load fails, the branch sits expanded with `Error` state.
    // Adapter retry affordances re-dispatch `ToggleNode` (the default
    // branch-control click path); the toggle must treat an `Error` branch
    // as a retry rather than collapsing it, otherwise retrying requires a
    // collapse + re-expand instead of one click.
    let mut service = service(lazy_props());

    drop(service.send(Event::ExpandNode(key(1))));
    drop(service.send(Event::LoadError(key(1))));
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Error
    );
    assert!(service.connect(&|_| {}).is_node_expanded(&key(1)));

    let result = service.send(Event::ToggleNode(key(1)));

    // Branch stays expanded (retry, not collapse) and re-fires LoadChildren.
    assert!(service.connect(&|_| {}).is_node_expanded(&key(1)));
    assert_eq!(
        service.connect(&|_| {}).node_load_state(&key(1)),
        NodeLoadState::Loading
    );
    assert!(
        result
            .pending_effects
            .iter()
            .any(|effect| effect.name == Effect::LoadChildren),
        "ToggleNode on an Error branch must re-fire LoadChildren"
    );
}

#[test]
fn rename_input_snapshot() {
    let mut service = service(renamable_props());

    drop(service.send(Event::RenameStart(key(2))));

    assert_snapshot!(
        "tree_view_node_rename_input",
        snapshot_attrs(&service.connect(&|_| {}).node_rename_input_attrs(&key(2)))
    );
}
