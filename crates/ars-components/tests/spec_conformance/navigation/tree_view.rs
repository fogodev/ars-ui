use ars_collections::Key;
use ars_components::navigation::tree_view;

use crate::helper::assert_anatomy;

#[test]
fn tree_view_anatomy_matches_spec() {
    // Spec references:
    // - `spec/components/navigation/tree-view.md` §2 base anatomy declares
    //   Root / Branch / BranchControl / BranchIndicator / BranchText /
    //   BranchContent / Leaf / LeafText.
    // - §4 (drag-and-drop reorder variant) layers the DragHandle and
    //   DropIndicator parts on top, appended last.
    // - §6 (renamable variant) layers the NodeRenameInput part, declared with
    //   the other node-text parts (after LeafText, before the DnD parts).
    assert_anatomy(
        "tree-view",
        &[
            (tree_view::Part::Root, "root"),
            (
                tree_view::Part::Branch {
                    node_id: Key::default(),
                },
                "branch",
            ),
            (
                tree_view::Part::BranchControl {
                    node_id: Key::default(),
                },
                "branch-control",
            ),
            (
                tree_view::Part::BranchIndicator {
                    node_id: Key::default(),
                },
                "branch-indicator",
            ),
            (tree_view::Part::BranchText, "branch-text"),
            (
                tree_view::Part::BranchContent {
                    node_id: Key::default(),
                },
                "branch-content",
            ),
            (
                tree_view::Part::Leaf {
                    node_id: Key::default(),
                },
                "leaf",
            ),
            (tree_view::Part::LeafText, "leaf-text"),
            (
                tree_view::Part::NodeRenameInput {
                    node_id: Key::default(),
                },
                "node-rename-input",
            ),
            (
                tree_view::Part::DragHandle {
                    node_id: Key::default(),
                },
                "drag-handle",
            ),
            (tree_view::Part::DropIndicator, "drop-indicator"),
        ],
    );
}
