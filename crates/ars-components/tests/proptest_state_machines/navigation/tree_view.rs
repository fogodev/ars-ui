//! Property-based tests for the `navigation/tree_view` state machine.

use std::{collections::BTreeSet, time::Duration};

use ars_collections::{
    Collection, Key, TreeCollection, TreeItemConfig,
    dnd::{CollectionDropTarget, DropPosition},
    selection,
};
use ars_components::navigation::tree_view;
use ars_core::{Env, HtmlAttr, Service};
use proptest::{prelude::*, test_runner::TestCaseResult};

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
    (
        tv_mode(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(|(mode, multiple, dnd, expand_alpha, renamable)| {
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
                .renamable(renamable)
                .default_expanded(expanded)
        })
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
            .prop_map(|(ch, now)| {
                tree_view::Event::TypeaheadSearch(ch, Duration::from_millis(now))
            }),
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
        // §5 lazy loading: deliver one synthetic child config under a parent,
        // or report a load error for a key.
        (tv_key(), 100u64..200).prop_map(|(parent, child_key)| {
            tree_view::Event::ChildrenLoaded {
                parent,
                children: vec![TreeItemConfig {
                    key: Key::int(child_key),
                    text_value: "Loaded".to_string(),
                    value: tv_item("Loaded"),
                    children: Vec::new(),
                    default_expanded: false,
                }],
            }
        }),
        tv_key().prop_map(tree_view::Event::LoadError),
        // §6 renamable nodes.
        tv_key().prop_map(tree_view::Event::RenameStart),
        (tv_key(), prop_oneof![Just("X"), Just("Renamed")]).prop_map(|(key, new_name)| {
            tree_view::Event::RenameCommit {
                key,
                new_name: new_name.to_string(),
            }
        }),
        tv_key().prop_map(tree_view::Event::RenameCancel),
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
    #![proptest_config(crate::common::proptest_config())]

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
