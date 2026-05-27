//! Unit, snapshot, keyboard, and drag-and-drop tests for the `tree_view`
//! machine. Mirrors the spec's "tests to add first" plus snapshot coverage of
//! every anatomy part across its output-affecting state/prop/context branches.

use alloc::{collections::BTreeSet, string::ToString, vec, vec::Vec};
use core::cell::RefCell;
use std::sync::{Arc, Mutex};

use ars_collections::{
    Key, TreeCollection, TreeItemConfig,
    dnd::{CollectionDropTarget, DropPosition},
    selection,
};
use ars_core::{AriaAttr, ConnectApi as _, Env, HtmlAttr, KeyboardKey, Service, StrongSend};
use ars_interactions::KeyboardEventData;
use insta::assert_snapshot;

use super::{
    Effect, Event, Machine, Messages, Part, Props, ReorderEvent, TreeItem, snapshot_attrs,
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
fn focus_next_skips_collapsed_children_and_wraps() {
    let mut service = service(props());

    drop(service.send(Event::FocusNode(key(4)))); // Vegetables (collapsed)
    drop(service.send(Event::FocusNext));

    // Carrot/Daikon are hidden; next visible after Vegetables is Grains.
    assert_eq!(service.context().focused_node, Some(key(7)));

    drop(service.send(Event::FocusNext)); // wraps to first

    assert_eq!(service.context().focused_node, Some(key(1)));
}

#[test]
fn focus_prev_wraps_to_last() {
    let mut service = service(props());

    drop(service.send(Event::FocusFirst));
    drop(service.send(Event::FocusPrev));

    assert_eq!(service.context().focused_node, Some(key(7)));
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

    api.on_root_focus();
    api.on_root_blur();

    assert_eq!(
        recorder.borrow().as_slice(),
        &[Event::Focus { is_keyboard: false }, Event::Blur]
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
