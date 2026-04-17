//! Collection-level drag-and-drop types and extension traits.
//!
//! This module extends [`Collection`](crate::Collection) with collection-aware
//! drag-source and drop-target behavior for listbox, grid-list, table, and
//! tree-view style widgets.

use alloc::{format, string::String, sync::Arc, vec::Vec};
use core::fmt::{self, Debug, Display};

use ars_core::{Locale, MessageFn};
use ars_interactions::DragItem;

use crate::{Collection, Key, Node, selection};

type LabelLocaleMessage = dyn Fn(&str, &Locale) -> String + Send + Sync;
type CountLocaleMessage = dyn Fn(usize, &Locale) -> String + Send + Sync;
type LocaleMessage = dyn Fn(&Locale) -> String + Send + Sync;
type DropTargetEnterMessage = dyn Fn(&str, DropPosition, &Locale) -> String + Send + Sync;
type DropCompleteMessage = dyn Fn(&str, &str, DropPosition, &Locale) -> String + Send + Sync;
type ReorderCompleteMessage = dyn Fn(&str, usize, usize, &Locale) -> String + Send + Sync;
type DndAnnouncementMessage = dyn Fn(DndAnnouncementData, &Locale) -> String + Send + Sync;

/// Where an item is being dropped relative to a target item in reading order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropPosition {
    /// Before the target item in reading order.
    ///
    /// In a vertical list this is above the target. In a horizontal list this
    /// is the inline-start side of the target.
    Before,

    /// After the target item in reading order.
    ///
    /// In a vertical list this is below the target. In a horizontal list this
    /// is the inline-end side of the target.
    After,

    /// On top of the target item.
    ///
    /// This is primarily used for tree-style reparenting operations.
    On,
}

impl Display for DropPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Before => f.write_str("before"),
            Self::After => f.write_str("after"),
            Self::On => f.write_str("on"),
        }
    }
}

/// A resolved drop target within a collection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionDropTarget {
    /// The key of the item nearest to the drop point.
    pub key: Key,

    /// Where relative to that item the drop will occur.
    pub position: DropPosition,
}

/// Events fired by collection-level drag-and-drop operations.
#[derive(Clone, Debug)]
pub enum CollectionDndEvent {
    /// Items within the same collection are being reordered.
    Reorder {
        /// The keys being moved.
        keys: Vec<Key>,

        /// The destination item key.
        target: Key,

        /// Where relative to `target` the items should be placed.
        position: DropPosition,
    },

    /// Items are being moved from one collection into another.
    Move {
        /// The source collection keys being moved.
        keys: Vec<Key>,

        /// The destination item key in this collection.
        target: Key,

        /// Where relative to `target` the items should be placed.
        position: DropPosition,
    },

    /// External or cross-data-source items are being inserted.
    Insert {
        /// Serialized drag data for each dragged item.
        items: Vec<DragItem>,

        /// The destination item key.
        target: Key,

        /// Where relative to `target` the items should be placed.
        position: DropPosition,
    },

    /// A drag operation has started on the source collection.
    DragStart {
        /// The keys participating in the drag.
        keys: Vec<Key>,
    },

    /// A drag operation has ended on the source collection.
    DragEnd {
        /// The keys that participated in the drag.
        keys: Vec<Key>,

        /// Whether the drag completed successfully.
        success: bool,
    },
}

impl PartialEq for CollectionDndEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Reorder {
                    keys: left_keys,
                    target: left_target,
                    position: left_position,
                },
                Self::Reorder {
                    keys: right_keys,
                    target: right_target,
                    position: right_position,
                },
            )
            | (
                Self::Move {
                    keys: left_keys,
                    target: left_target,
                    position: left_position,
                },
                Self::Move {
                    keys: right_keys,
                    target: right_target,
                    position: right_position,
                },
            ) => {
                left_keys == right_keys
                    && left_target == right_target
                    && left_position == right_position
            }

            (
                Self::Insert {
                    items: left_items,
                    target: left_target,
                    position: left_position,
                },
                Self::Insert {
                    items: right_items,
                    target: right_target,
                    position: right_position,
                },
            ) => {
                left_target == right_target
                    && left_position == right_position
                    && drag_items_eq(left_items, right_items)
            }

            (Self::DragStart { keys: left_keys }, Self::DragStart { keys: right_keys }) => {
                left_keys == right_keys
            }

            (
                Self::DragEnd {
                    keys: left_keys,
                    success: left_success,
                },
                Self::DragEnd {
                    keys: right_keys,
                    success: right_success,
                },
            ) => left_keys == right_keys && left_success == right_success,

            _ => false,
        }
    }
}

/// Extends a collection with drag-source behavior.
pub trait DraggableCollection<T>: Collection<T> {
    /// Returns whether the item with the given key can be dragged.
    ///
    /// By default, all focusable items are draggable.
    fn is_draggable(&self, key: &Key) -> bool {
        self.get(key).is_some_and(Node::is_focusable)
    }

    /// Returns the drag data for the given keys.
    ///
    /// By default, each key contributes a [`DragItem::Text`] payload derived
    /// from [`Collection::text_value_of`]. Unknown keys are skipped.
    fn drag_data(&self, keys: &[Key]) -> Vec<DragItem> {
        keys.iter()
            .filter_map(|key| {
                self.text_value_of(key)
                    .map(|text| DragItem::Text(String::from(text)))
            })
            .collect()
    }

    /// Returns the current selection state.
    fn selection(&self) -> &selection::State;

    /// Returns the set of keys that participate in the current drag.
    ///
    /// When selection is [`selection::Set::All`], this resolves to all item
    /// keys in the concrete collection. Otherwise it returns the explicit
    /// selected keys.
    fn drag_keys(&self) -> Vec<Key> {
        match &self.selection().selected_keys {
            selection::Set::All => self.item_keys().cloned().collect(),
            other => other.keys().cloned().collect(),
        }
    }
}

/// Extends a collection with drop-target policy hooks.
///
/// Adapter layers perform pointer hit-testing and resolve concrete drop targets.
/// This trait only controls which drops the collection accepts once a target has
/// been resolved.
pub trait DroppableCollection<T>: Collection<T> {
    /// The set of MIME types this collection accepts for external drops.
    ///
    /// An empty slice means the collection only supports internal reorder.
    fn accepted_types(&self) -> &[&str] {
        &[]
    }

    /// Returns whether an item can receive an "on" drop.
    ///
    /// By default, collections only support between-item drops.
    fn allows_drop_on(&self, _key: &Key) -> bool {
        false
    }

    /// Returns whether a proposed drop is valid.
    ///
    /// The default implementation accepts all proposed drops.
    fn is_drop_valid(&self, _target: &CollectionDropTarget, _items: &[DragItem]) -> bool {
        true
    }
}

/// Localizable messages for collection-level drag-and-drop operations.
#[derive(Clone)]
pub struct CollectionDndMessages {
    /// Announced when a single-item drag starts.
    pub drag_start: MessageFn<LabelLocaleMessage>,

    /// Announced when a multi-item drag starts.
    pub drag_start_multi: MessageFn<CountLocaleMessage>,

    /// Announced when the drag enters a drop target.
    pub drop_target_enter: MessageFn<DropTargetEnterMessage>,

    /// Announced when a drop completes successfully.
    pub drop_complete: MessageFn<DropCompleteMessage>,

    /// Announced when a pointer-based reorder completes.
    pub reorder_complete: MessageFn<ReorderCompleteMessage>,

    /// Announced when drag is cancelled.
    pub drop_cancelled: MessageFn<LocaleMessage>,

    /// Role description for draggable items.
    pub draggable: MessageFn<LocaleMessage>,

    /// Template for a drag handle aria-label.
    pub drag_handle: MessageFn<LabelLocaleMessage>,
}

impl Default for CollectionDndMessages {
    fn default() -> Self {
        Self {
            drag_start: MessageFn::new(|label: &str, _locale: &Locale| {
                format!("{label}. Press Tab to move to a drop target, Escape to cancel.")
            }),

            drag_start_multi: MessageFn::new(|count: usize, _locale: &Locale| {
                format!("Dragging {count} items.")
            }),

            drop_target_enter: MessageFn::new(Arc::new(
                |target: &str, position: DropPosition, _locale: &Locale| {
                    format!("Drop available: {position} {target}")
                },
            ) as Arc<DropTargetEnterMessage>),

            drop_complete: MessageFn::new(Arc::new(
                |item: &str, target: &str, position: DropPosition, _locale: &Locale| {
                    format!("Dropped {item} {position} {target}.")
                },
            ) as Arc<DropCompleteMessage>),

            reorder_complete: MessageFn::new(Arc::new(
                |item: &str, position: usize, total: usize, _locale: &Locale| {
                    format!("Reordered: {item} moved to position {position} of {total}.")
                },
            ) as Arc<ReorderCompleteMessage>),

            drop_cancelled: MessageFn::new(|_locale: &Locale| String::from("Drop cancelled.")),

            draggable: MessageFn::new(|_locale: &Locale| String::from("draggable")),

            drag_handle: MessageFn::new(|item: &str, _locale: &Locale| format!("Drag {item}")),
        }
    }
}

impl Debug for CollectionDndMessages {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CollectionDndMessages { .. }")
    }
}

/// Accessible announcement templates for drag-and-drop lifecycle updates.
pub struct DndAnnouncements {
    /// Announced when drag starts.
    pub drag_start: MessageFn<DndAnnouncementMessage>,

    /// Announced while dragging over a drop target.
    pub drag_over: MessageFn<DndAnnouncementMessage>,

    /// Announced after drop completion.
    pub drop: MessageFn<DndAnnouncementMessage>,

    /// Announced when drag is cancelled.
    pub drag_cancel: MessageFn<DndAnnouncementMessage>,
}

/// Data passed to `DnD` announcement templates.
#[derive(Debug)]
pub struct DndAnnouncementData {
    /// Label of the item being dragged.
    pub item_label: String,

    /// Label of the drop target, if any.
    pub target_label: Option<String>,

    /// Position hint announced during drag.
    pub position_hint: Option<String>,

    /// Human-readable action description.
    pub action: Option<String>,

    /// Human-readable result description.
    pub result: Option<String>,
}

impl Debug for DndAnnouncements {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DndAnnouncements { .. }")
    }
}

fn drag_items_eq(left: &[DragItem], right: &[DragItem]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(left_item, right_item)| drag_item_eq(left_item, right_item))
}

fn drag_item_eq(left: &DragItem, right: &DragItem) -> bool {
    match (left, right) {
        (DragItem::Text(left_text), DragItem::Text(right_text))
        | (DragItem::Uri(left_text), DragItem::Uri(right_text))
        | (DragItem::Html(left_text), DragItem::Html(right_text)) => left_text == right_text,

        (
            DragItem::File {
                name: left_name,
                mime_type: left_mime_type,
                size: left_size,
                handle: _,
            },
            DragItem::File {
                name: right_name,
                mime_type: right_mime_type,
                size: right_size,
                handle: _,
            },
        ) => {
            left_name == right_name && left_mime_type == right_mime_type && left_size == right_size
        }

        (
            DragItem::Directory {
                name: left_name,
                handle: _,
            },
            DragItem::Directory {
                name: right_name,
                handle: _,
            },
        ) => left_name == right_name,

        (
            DragItem::Custom {
                mime_type: left_mime_type,
                data: left_data,
            },
            DragItem::Custom {
                mime_type: right_mime_type,
                data: right_data,
            },
        ) => left_mime_type == right_mime_type && left_data == right_data,

        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use alloc::{collections::BTreeSet, format, string::String, sync::Arc, vec};

    use ars_interactions::test_support::{directory_drag_item, file_drag_item};

    use super::*;
    use crate::{Collection, StaticCollection, selection};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestItem {
        key: Key,
        label: String,
    }

    impl crate::CollectionItem for TestItem {
        fn key(&self) -> &Key {
            &self.key
        }

        fn text_value(&self) -> &str {
            &self.label
        }
    }

    struct TestDraggableCollection {
        inner: StaticCollection<TestItem>,
        selection: selection::State,
    }

    impl TestDraggableCollection {
        fn new(items: &[(Key, &str)]) -> Self {
            Self {
                inner: items
                    .iter()
                    .map(|(key, label)| {
                        (
                            key.clone(),
                            (*label).to_owned(),
                            TestItem {
                                key: key.clone(),
                                label: (*label).to_owned(),
                            },
                        )
                    })
                    .collect(),
                selection: selection::State::new(
                    selection::Mode::Multiple,
                    selection::Behavior::Toggle,
                ),
            }
        }
    }

    impl Collection<TestItem> for TestDraggableCollection {
        fn size(&self) -> usize {
            self.inner.size()
        }

        fn get(&self, key: &Key) -> Option<&Node<TestItem>> {
            self.inner.get(key)
        }

        fn get_by_index(&self, index: usize) -> Option<&Node<TestItem>> {
            self.inner.get_by_index(index)
        }

        fn first_key(&self) -> Option<&Key> {
            self.inner.first_key()
        }

        fn last_key(&self) -> Option<&Key> {
            self.inner.last_key()
        }

        fn key_after(&self, key: &Key) -> Option<&Key> {
            self.inner.key_after(key)
        }

        fn key_before(&self, key: &Key) -> Option<&Key> {
            self.inner.key_before(key)
        }

        fn key_after_no_wrap(&self, key: &Key) -> Option<&Key> {
            self.inner.key_after_no_wrap(key)
        }

        fn key_before_no_wrap(&self, key: &Key) -> Option<&Key> {
            self.inner.key_before_no_wrap(key)
        }

        fn keys<'a>(&'a self) -> impl Iterator<Item = &'a Key>
        where
            TestItem: 'a,
        {
            self.inner.keys()
        }

        fn nodes<'a>(&'a self) -> impl Iterator<Item = &'a Node<TestItem>>
        where
            TestItem: 'a,
        {
            self.inner.nodes()
        }

        fn children_of<'a>(&'a self, parent_key: &Key) -> impl Iterator<Item = &'a Node<TestItem>>
        where
            TestItem: 'a,
        {
            self.inner.children_of(parent_key)
        }
    }

    impl DraggableCollection<TestItem> for TestDraggableCollection {
        fn selection(&self) -> &selection::State {
            &self.selection
        }
    }

    impl DroppableCollection<TestItem> for TestDraggableCollection {}

    fn locale() -> Locale {
        Locale::parse("en-US").expect("en-US must parse")
    }

    fn test_announcements() -> DndAnnouncements {
        DndAnnouncements {
            drag_start: MessageFn::new(Arc::new(|data: DndAnnouncementData, _locale: &Locale| {
                data.item_label
            }) as Arc<DndAnnouncementMessage>),
            drag_over: MessageFn::new(Arc::new(|data: DndAnnouncementData, _locale: &Locale| {
                data.target_label.unwrap_or_default()
            }) as Arc<DndAnnouncementMessage>),
            drop: MessageFn::new(Arc::new(|data: DndAnnouncementData, _locale: &Locale| {
                data.result.unwrap_or_default()
            }) as Arc<DndAnnouncementMessage>),
            drag_cancel: MessageFn::new(Arc::new(|data: DndAnnouncementData, _locale: &Locale| {
                data.action.unwrap_or_default()
            }) as Arc<DndAnnouncementMessage>),
        }
    }

    fn test_announcement_data() -> DndAnnouncementData {
        DndAnnouncementData {
            item_label: String::from("Alpha"),
            target_label: Some(String::from("Beta")),
            position_hint: Some(String::from("position 2 of 3")),
            action: Some(String::from("cancelled")),
            result: Some(String::from("dropped")),
        }
    }

    #[test]
    fn drop_position_display_matches_spec() {
        assert_eq!(DropPosition::Before.to_string(), "before");
        assert_eq!(DropPosition::After.to_string(), "after");
        assert_eq!(DropPosition::On.to_string(), "on");
    }

    #[test]
    fn drop_position_derives_clone_copy_debug_partial_eq_and_eq() {
        let position = DropPosition::Before;
        let copied = position;
        let cloned = position;

        assert_eq!(copied, cloned);
        assert_eq!(format!("{position:?}"), "Before");
    }

    #[test]
    fn collection_drop_target_construction_and_derives() {
        let target = CollectionDropTarget {
            key: Key::int(7),
            position: DropPosition::After,
        };

        assert_eq!(target.clone(), target);
        assert_eq!(
            format!("{target:?}"),
            "CollectionDropTarget { key: Int(7), position: After }"
        );
        assert_eq!(target.key, Key::int(7));
        assert_eq!(target.position, DropPosition::After);
    }

    #[test]
    fn collection_dnd_event_reorder_construction_and_field_access() {
        let event = CollectionDndEvent::Reorder {
            keys: vec![Key::int(1), Key::int(2)],
            target: Key::int(3),
            position: DropPosition::Before,
        };

        assert_eq!(
            event,
            CollectionDndEvent::Reorder {
                keys: vec![Key::int(1), Key::int(2)],
                target: Key::int(3),
                position: DropPosition::Before,
            }
        );
    }

    #[test]
    fn collection_dnd_event_move_construction_and_field_access() {
        let event = CollectionDndEvent::Move {
            keys: vec![Key::str("a")],
            target: Key::str("b"),
            position: DropPosition::After,
        };

        assert_eq!(
            event,
            CollectionDndEvent::Move {
                keys: vec![Key::str("a")],
                target: Key::str("b"),
                position: DropPosition::After,
            }
        );
    }

    #[test]
    fn collection_dnd_event_insert_construction_and_field_access() {
        let event = CollectionDndEvent::Insert {
            items: vec![DragItem::Text(String::from("payload"))],
            target: Key::int(9),
            position: DropPosition::On,
        };

        assert_eq!(
            event,
            CollectionDndEvent::Insert {
                items: vec![DragItem::Text(String::from("payload"))],
                target: Key::int(9),
                position: DropPosition::On,
            }
        );
    }

    #[test]
    fn collection_dnd_event_drag_start_construction_and_field_access() {
        let event = CollectionDndEvent::DragStart {
            keys: vec![Key::int(1), Key::int(2)],
        };

        assert_eq!(
            event,
            CollectionDndEvent::DragStart {
                keys: vec![Key::int(1), Key::int(2)],
            }
        );
    }

    #[test]
    fn collection_dnd_event_drag_end_construction_and_field_access() {
        let event = CollectionDndEvent::DragEnd {
            keys: vec![Key::str("row-1")],
            success: true,
        };

        assert_eq!(
            event,
            CollectionDndEvent::DragEnd {
                keys: vec![Key::str("row-1")],
                success: true,
            }
        );
    }

    #[test]
    fn collection_dnd_event_supports_clone_debug_and_manual_partial_eq() {
        let event = CollectionDndEvent::Insert {
            items: vec![DragItem::Text(String::from("payload"))],
            target: Key::str("target"),
            position: DropPosition::Before,
        };

        assert_eq!(event.clone(), event);
        assert_eq!(
            format!("{event:?}"),
            "Insert { items: [Text(\"payload\")], target: String(\"target\"), position: Before }"
        );
    }

    #[test]
    fn collection_dnd_event_manual_partial_eq_covers_non_insert_variants_and_mismatches() {
        assert_eq!(
            CollectionDndEvent::Reorder {
                keys: vec![Key::int(1), Key::int(2)],
                target: Key::int(3),
                position: DropPosition::Before,
            },
            CollectionDndEvent::Reorder {
                keys: vec![Key::int(1), Key::int(2)],
                target: Key::int(3),
                position: DropPosition::Before,
            }
        );
        assert_ne!(
            CollectionDndEvent::Reorder {
                keys: vec![Key::int(1), Key::int(2)],
                target: Key::int(3),
                position: DropPosition::Before,
            },
            CollectionDndEvent::Reorder {
                keys: vec![Key::int(1), Key::int(2)],
                target: Key::int(3),
                position: DropPosition::After,
            }
        );

        assert_eq!(
            CollectionDndEvent::Move {
                keys: vec![Key::str("a")],
                target: Key::str("b"),
                position: DropPosition::On,
            },
            CollectionDndEvent::Move {
                keys: vec![Key::str("a")],
                target: Key::str("b"),
                position: DropPosition::On,
            }
        );
        assert_ne!(
            CollectionDndEvent::Move {
                keys: vec![Key::str("a")],
                target: Key::str("b"),
                position: DropPosition::On,
            },
            CollectionDndEvent::Move {
                keys: vec![Key::str("a")],
                target: Key::str("c"),
                position: DropPosition::On,
            }
        );

        assert_eq!(
            CollectionDndEvent::DragStart {
                keys: vec![Key::int(1), Key::int(2)],
            },
            CollectionDndEvent::DragStart {
                keys: vec![Key::int(1), Key::int(2)],
            }
        );
        assert_ne!(
            CollectionDndEvent::DragStart {
                keys: vec![Key::int(1), Key::int(2)],
            },
            CollectionDndEvent::DragStart {
                keys: vec![Key::int(2), Key::int(1)],
            }
        );

        assert_eq!(
            CollectionDndEvent::DragEnd {
                keys: vec![Key::str("row-1")],
                success: true,
            },
            CollectionDndEvent::DragEnd {
                keys: vec![Key::str("row-1")],
                success: true,
            }
        );
        assert_ne!(
            CollectionDndEvent::DragEnd {
                keys: vec![Key::str("row-1")],
                success: true,
            },
            CollectionDndEvent::DragEnd {
                keys: vec![Key::str("row-1")],
                success: false,
            }
        );

        assert_ne!(
            CollectionDndEvent::DragStart {
                keys: vec![Key::int(1)],
            },
            CollectionDndEvent::DragEnd {
                keys: vec![Key::int(1)],
                success: true,
            }
        );
    }

    #[test]
    fn collection_dnd_event_insert_partial_eq_covers_public_drag_item_variants() {
        let uri = CollectionDndEvent::Insert {
            items: vec![DragItem::Uri(String::from("https://example.com/item"))],
            target: Key::str("target"),
            position: DropPosition::After,
        };

        assert_eq!(uri.clone(), uri);
        assert_ne!(
            uri,
            CollectionDndEvent::Insert {
                items: vec![DragItem::Uri(String::from("https://example.com/other"))],
                target: Key::str("target"),
                position: DropPosition::After,
            }
        );

        let html = CollectionDndEvent::Insert {
            items: vec![DragItem::Html(String::from("<p>payload</p>"))],
            target: Key::str("target"),
            position: DropPosition::On,
        };

        assert_eq!(html.clone(), html);
        assert_ne!(
            html,
            CollectionDndEvent::Insert {
                items: vec![DragItem::Html(String::from("<p>different</p>"))],
                target: Key::str("target"),
                position: DropPosition::On,
            }
        );

        let custom = CollectionDndEvent::Insert {
            items: vec![DragItem::Custom {
                mime_type: String::from("application/x-ars-demo"),
                data: String::from("payload"),
            }],
            target: Key::str("custom"),
            position: DropPosition::After,
        };

        assert_eq!(custom.clone(), custom);
        assert_ne!(
            custom,
            CollectionDndEvent::Insert {
                items: vec![DragItem::Custom {
                    mime_type: String::from("application/x-ars-demo"),
                    data: String::from("other"),
                }],
                target: Key::str("custom"),
                position: DropPosition::After,
            }
        );

        assert_ne!(
            CollectionDndEvent::Insert {
                items: vec![DragItem::Text(String::from("payload"))],
                target: Key::str("target"),
                position: DropPosition::Before,
            },
            CollectionDndEvent::Insert {
                items: vec![DragItem::Uri(String::from("payload"))],
                target: Key::str("target"),
                position: DropPosition::Before,
            }
        );
        assert_ne!(
            CollectionDndEvent::Insert {
                items: vec![
                    DragItem::Text(String::from("alpha")),
                    DragItem::Text(String::from("beta")),
                ],
                target: Key::str("target"),
                position: DropPosition::Before,
            },
            CollectionDndEvent::Insert {
                items: vec![DragItem::Text(String::from("alpha"))],
                target: Key::str("target"),
                position: DropPosition::Before,
            }
        );
    }

    #[test]
    fn drag_item_eq_covers_file_and_directory_metadata_comparisons() {
        assert!(drag_item_eq(
            &file_drag_item("report.csv", "text/csv", 1024),
            &file_drag_item("report.csv", "text/csv", 1024),
        ));
        assert!(!drag_item_eq(
            &file_drag_item("report.csv", "text/csv", 1024),
            &file_drag_item("other.csv", "text/csv", 1024),
        ));
        assert!(!drag_item_eq(
            &file_drag_item("report.csv", "text/csv", 1024),
            &file_drag_item("report.csv", "application/json", 1024),
        ));
        assert!(!drag_item_eq(
            &file_drag_item("report.csv", "text/csv", 1024),
            &file_drag_item("report.csv", "text/csv", 2048),
        ));

        assert!(drag_item_eq(
            &directory_drag_item("Documents"),
            &directory_drag_item("Documents"),
        ));
        assert!(!drag_item_eq(
            &directory_drag_item("Documents"),
            &directory_drag_item("Archives"),
        ));
    }

    #[test]
    fn drag_items_eq_returns_false_for_length_mismatches() {
        assert!(!drag_items_eq(
            &[DragItem::Text(String::from("alpha"))],
            &[
                DragItem::Text(String::from("alpha")),
                DragItem::Text(String::from("beta")),
            ],
        ));
    }

    #[test]
    fn draggable_collection_is_draggable_defaults_to_focusable_items() {
        let collection = TestDraggableCollection::new(&[(Key::int(1), "Alpha")]);

        assert!(collection.is_draggable(&Key::int(1)));
        assert!(!collection.is_draggable(&Key::int(99)));
    }

    #[test]
    fn draggable_collection_drag_data_defaults_to_text_items() {
        let collection =
            TestDraggableCollection::new(&[(Key::int(1), "Alpha"), (Key::int(2), "Beta")]);

        let data = collection.drag_data(&[Key::int(1), Key::int(2), Key::int(99)]);

        assert_eq!(data.len(), 2);
        assert!(matches!(&data[0], DragItem::Text(text) if text == "Alpha"));
        assert!(matches!(&data[1], DragItem::Text(text) if text == "Beta"));
    }

    #[test]
    fn draggable_collection_drag_keys_resolves_all_selection_to_all_item_keys() {
        let mut collection =
            TestDraggableCollection::new(&[(Key::int(1), "Alpha"), (Key::int(2), "Beta")]);

        collection.selection = selection::State {
            selected_keys: selection::Set::All,
            ..selection::State::new(selection::Mode::Multiple, selection::Behavior::Toggle)
        };

        assert_eq!(collection.drag_keys(), vec![Key::int(1), Key::int(2)]);
    }

    #[test]
    fn draggable_collection_drag_keys_returns_selected_keys_for_concrete_selection() {
        let mut selected = BTreeSet::new();

        selected.insert(Key::int(1));
        selected.insert(Key::int(3));

        let mut collection = TestDraggableCollection::new(&[
            (Key::int(1), "Alpha"),
            (Key::int(2), "Beta"),
            (Key::int(3), "Gamma"),
        ]);

        collection.selection = selection::State {
            selected_keys: selection::Set::Multiple(selected),
            ..selection::State::new(selection::Mode::Multiple, selection::Behavior::Toggle)
        };

        assert_eq!(collection.drag_keys(), vec![Key::int(1), Key::int(3)]);
    }

    #[test]
    fn draggable_collection_drag_keys_returns_empty_when_selection_is_empty() {
        let collection = TestDraggableCollection::new(&[(Key::int(1), "Alpha")]);

        assert!(collection.drag_keys().is_empty());
    }

    #[test]
    fn droppable_collection_policy_defaults_match_spec() {
        let collection = TestDraggableCollection::new(&[(Key::int(1), "Alpha")]);

        let target = CollectionDropTarget {
            key: Key::int(1),
            position: DropPosition::Before,
        };

        assert!(collection.accepted_types().is_empty());
        assert!(!collection.allows_drop_on(&Key::int(1)));
        assert!(collection.is_drop_valid(&target, &[DragItem::Text(String::from("payload"))]));
    }

    #[test]
    fn collection_dnd_messages_default_strings_match_spec() {
        let locale = locale();

        let messages = CollectionDndMessages::default();

        assert_eq!(
            (messages.drag_start)("Alpha", &locale),
            "Alpha. Press Tab to move to a drop target, Escape to cancel."
        );
        assert_eq!((messages.drag_start_multi)(3, &locale), "Dragging 3 items.");
        assert_eq!(
            (messages.drop_target_enter)("Beta", DropPosition::Before, &locale),
            "Drop available: before Beta"
        );
        assert_eq!(
            (messages.drop_complete)("Alpha", "Beta", DropPosition::After, &locale),
            "Dropped Alpha after Beta."
        );
        assert_eq!(
            (messages.reorder_complete)("Alpha", 2, 5, &locale),
            "Reordered: Alpha moved to position 2 of 5."
        );
        assert_eq!((messages.drop_cancelled)(&locale), "Drop cancelled.");
        assert_eq!((messages.draggable)(&locale), "draggable");
        assert_eq!((messages.drag_handle)("Alpha", &locale), "Drag Alpha");
    }

    #[test]
    fn collection_dnd_messages_debug_is_redacted() {
        assert_eq!(
            format!("{:?}", CollectionDndMessages::default()),
            "CollectionDndMessages { .. }"
        );
    }

    #[test]
    fn dnd_announcements_construct_with_closure_fields() {
        let announcements = test_announcements();

        let locale = locale();

        assert_eq!(
            (announcements.drag_start)(test_announcement_data(), &locale),
            "Alpha"
        );
        assert_eq!(
            (announcements.drag_over)(test_announcement_data(), &locale),
            "Beta"
        );
        assert_eq!(
            (announcements.drop)(test_announcement_data(), &locale),
            "dropped"
        );
        assert_eq!(
            (announcements.drag_cancel)(test_announcement_data(), &locale),
            "cancelled"
        );
    }

    #[test]
    fn dnd_announcements_debug_is_redacted() {
        let announcements = test_announcements();

        assert_eq!(format!("{announcements:?}"), "DndAnnouncements { .. }");
    }

    #[test]
    fn dnd_announcement_data_constructs_with_all_fields() {
        let data = DndAnnouncementData {
            item_label: String::from("Alpha"),
            target_label: Some(String::from("Beta")),
            position_hint: Some(String::from("position 2 of 3")),
            action: Some(String::from("move")),
            result: Some(String::from("before Beta")),
        };

        assert_eq!(data.item_label, "Alpha");
        assert_eq!(data.target_label.as_deref(), Some("Beta"));
        assert_eq!(data.position_hint.as_deref(), Some("position 2 of 3"));
        assert_eq!(data.action.as_deref(), Some("move"));
        assert_eq!(data.result.as_deref(), Some("before Beta"));
    }
}
