//! Drag-and-drop interaction core types.
//!
//! This module defines the framework-agnostic drag payload types, source and
//! target configuration structs, and the MIME-type acceptance helpers used by
//! later drag state-machine work.

use std::{
    fmt::{self, Debug},
    string::String,
    sync::Arc,
    vec::Vec,
};

use ars_core::Callback;

use crate::PointerType;

type DragItemsFn = Arc<dyn Fn() -> Vec<DragItem> + Send + Sync>;
type DragStartAnnouncementFn = Callback<dyn Fn(&[DragItem]) -> String>;
type DragEnterAnnouncementFn = Callback<dyn Fn(&DropTargetEvent) -> String>;
type DropAnnouncementFn = Callback<dyn Fn(&DropEvent) -> String>;

/// The data associated with a drag operation.
///
/// Multiple representations may be present for cross-application
/// interoperability, such as exposing both plain text and HTML.
#[derive(Clone, Debug)]
pub enum DragItem {
    /// Plain text content.
    Text(String),

    /// URI/URL string content.
    Uri(String),

    /// HTML-formatted text content.
    Html(String),

    /// A file reference from the browser file system or drag payload.
    File {
        /// Display name of the file.
        name: String,
        /// File MIME type.
        mime_type: String,
        /// File size in bytes.
        size: u64,
        /// Opaque file handle resolved by `ars-dom`.
        handle: FileHandle,
    },

    /// A directory reference from the browser file system or drag payload.
    Directory {
        /// Display name of the directory.
        name: String,
        /// Opaque directory handle resolved by `ars-dom`.
        handle: DirectoryHandle,
    },

    /// Custom application-defined data.
    Custom {
        /// MIME type string used for transfer interoperability.
        mime_type: String,
        /// Serialized custom payload data.
        data: String,
    },
}

/// Opaque file-system handle resolved by `ars-dom` against browser APIs.
#[derive(Clone, Debug)]
pub struct FileHandle(());

/// Opaque directory handle resolved by `ars-dom` against browser APIs.
#[derive(Clone, Debug)]
pub struct DirectoryHandle(());

/// The type of operation that will occur when items are dropped.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DropOperation {
    /// Move items from source to target.
    Move,
    /// Copy items to the target.
    Copy,
    /// Create a link or shortcut to the source items.
    Link,
    /// Reject the drop.
    Cancel,
}

impl DropOperation {
    /// Returns the HTML5 `DataTransfer.dropEffect` value for this operation.
    #[must_use]
    pub const fn as_drop_effect(&self) -> &'static str {
        match self {
            DropOperation::Move => "move",
            DropOperation::Copy => "copy",
            DropOperation::Link => "link",
            DropOperation::Cancel => "none",
        }
    }
}

/// Configuration for a draggable element.
#[derive(Clone, Default)]
pub struct DragConfig {
    /// Whether dragging is disabled.
    pub disabled: bool,

    /// Items provided by the primary dragged element.
    pub items: Option<DragItemsFn>,

    /// The set of drop operations this source allows.
    pub allowed_operations: Option<Vec<DropOperation>>,

    /// Called when dragging begins.
    pub on_drag_start: Option<Callback<dyn Fn(DragStartEvent)>>,

    /// Called when dragging ends, regardless of outcome.
    pub on_drag_end: Option<Callback<dyn Fn(DragEndEvent)>>,

    /// Additional selected items to include for multi-item drag.
    pub get_items: Option<DragItemsFn>,

    /// Screen reader announcement when drag starts.
    pub drag_start_announcement: Option<DragStartAnnouncementFn>,
}

impl DragConfig {
    /// Convenience builder for multi-item drag.
    #[must_use]
    pub fn with_selection(
        mut self,
        get_selected_items: impl Fn() -> Vec<DragItem> + Send + Sync + 'static,
    ) -> Self {
        self.get_items = Some(Arc::new(get_selected_items));
        self
    }
}

impl Debug for DragConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragConfig")
            .field("disabled", &self.disabled)
            .field("items", &self.items.as_ref().map(|_| "<closure>"))
            .field("allowed_operations", &self.allowed_operations)
            .field("on_drag_start", &self.on_drag_start)
            .field("on_drag_end", &self.on_drag_end)
            .field("get_items", &self.get_items.as_ref().map(|_| "<closure>"))
            .field(
                "drag_start_announcement",
                &self.drag_start_announcement.as_ref().map(|_| "<closure>"),
            )
            .finish()
    }
}

/// Payload sent when a drag operation starts.
#[derive(Clone, Debug)]
pub struct DragStartEvent {
    /// Items included in the drag payload.
    pub items: Vec<DragItem>,
    /// Input modality that initiated the drag.
    pub pointer_type: PointerType,
}

/// Payload sent when a drag operation ends.
#[derive(Clone, Debug)]
pub struct DragEndEvent {
    /// Items included in the drag payload.
    pub items: Vec<DragItem>,
    /// Final drop operation.
    pub operation: DropOperation,
    /// Input modality that initiated the drag.
    pub pointer_type: PointerType,
    /// Whether a drop target accepted the payload.
    pub was_dropped: bool,
}

/// Configuration for a drop target element.
#[derive(Clone, Debug, Default)]
pub struct DropConfig {
    /// Whether dropping is disabled.
    pub disabled: bool,

    /// Called when dragged items enter this target.
    pub on_drag_enter: Option<Callback<dyn Fn(DropTargetEvent)>>,

    /// Called when dragged items leave this target.
    pub on_drag_leave: Option<Callback<dyn Fn(DropTargetEvent)>>,

    /// Called during drag-over to determine the accepted operation.
    pub on_drag_over: Option<Callback<dyn Fn(DropTargetEvent) -> DropOperation>>,

    /// Called when items are dropped onto this target.
    pub on_drop: Option<Callback<dyn Fn(DropEvent)>>,

    /// Drop operations accepted by this target.
    pub accepted_operations: Option<Vec<DropOperation>>,

    /// MIME types and wildcard patterns accepted by this target.
    pub accepted_types: Option<Vec<String>>,

    /// Placement of the drop indicator within the target.
    pub drop_indicator_position: DropIndicatorPosition,

    /// Screen reader announcement when dragged items enter this target.
    pub drag_enter_announcement: Option<DragEnterAnnouncementFn>,

    /// Screen reader announcement when a drop succeeds.
    pub drop_announcement: Option<DropAnnouncementFn>,
}

#[cfg(test)]
impl DropConfig {
    /// Returns whether any preview item matches this target's accepted MIME types.
    #[must_use]
    fn accepts_preview_items(&self, items: &[DragItemPreview]) -> bool {
        match &self.accepted_types {
            None => true,
            Some(accepted_types) => items
                .iter()
                .any(|item| preview_matches_accepted_types(item, accepted_types)),
        }
    }
}

/// Placement of the drop indicator within the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DropIndicatorPosition {
    /// Show the indicator over the target itself.
    #[default]
    OnTarget,
    /// Show the indicator before the target in reading order.
    Before,
    /// Show the indicator after the target in reading order.
    After,
}

/// Preview payload exposed during drag-enter and drag-over.
#[derive(Clone, Debug)]
pub struct DropTargetEvent {
    /// Preview items for the active drag payload.
    pub items: Vec<DragItemPreview>,
    /// Operation currently being offered.
    pub operation: DropOperation,
    /// Input modality that initiated the drag.
    pub pointer_type: PointerType,
}

/// Preview of a drag item used during hover feedback.
#[derive(Clone, Debug)]
pub struct DragItemPreview {
    /// High-level item category.
    pub kind: DragItemKind,
    /// MIME types exposed by the item.
    pub mime_types: Vec<String>,
}

/// High-level category of a dragged item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragItemKind {
    /// Plain text content.
    Text,
    /// URI/URL content.
    Uri,
    /// HTML content.
    Html,
    /// A file payload.
    File,
    /// A directory payload.
    Directory,
    /// Custom application data.
    Custom,
}

impl From<&DragItem> for DragItemKind {
    fn from(value: &DragItem) -> Self {
        match value {
            DragItem::Text(_) => Self::Text,
            DragItem::Uri(_) => Self::Uri,
            DragItem::Html(_) => Self::Html,
            DragItem::File { .. } => Self::File,
            DragItem::Directory { .. } => Self::Directory,
            DragItem::Custom { .. } => Self::Custom,
        }
    }
}

/// Payload delivered to a successful drop handler.
#[derive(Clone, Debug)]
pub struct DropEvent {
    /// Fully resolved dropped items.
    pub items: Vec<DragItem>,
    /// Accepted drop operation.
    pub operation: DropOperation,
    /// Input modality that initiated the drag.
    pub pointer_type: PointerType,
    /// Position of the accepted drop within the target.
    pub drop_position: DropIndicatorPosition,
}

#[cfg(test)]
fn preview_matches_accepted_types(item: &DragItemPreview, accepted_types: &[String]) -> bool {
    item.mime_types.iter().any(|mime_type| {
        let normalized_item = normalize_mime_type(mime_type);
        accepted_types
            .iter()
            .map(String::as_str)
            .map(normalize_mime_type)
            .any(|accepted| mime_type_matches(&accepted, &normalized_item))
    })
}

#[cfg(test)]
fn mime_type_matches(accepted: &str, actual: &str) -> bool {
    if let Some(prefix) = accepted.strip_suffix("/*") {
        actual
            .split_once('/')
            .is_some_and(|(actual_prefix, _)| actual_prefix == prefix)
    } else {
        accepted == actual
    }
}

#[cfg(test)]
fn normalize_mime_type(mime_type: &str) -> String {
    let normalized = mime_type.trim().to_ascii_lowercase();
    if normalized == "image/jpg" {
        "image/jpeg".to_owned()
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use std::{fmt::Write as _, sync::Arc};

    use ars_core::Callback;

    use super::{
        DirectoryHandle, DragConfig, DragEndEvent, DragItem, DragItemKind, DragItemPreview,
        DragStartEvent, DropConfig, DropEvent, DropIndicatorPosition, DropOperation, FileHandle,
    };
    use crate::PointerType;

    fn preview(kind: DragItemKind, mime_types: &[&str]) -> DragItemPreview {
        DragItemPreview {
            kind,
            mime_types: mime_types.iter().map(|mime| (*mime).to_owned()).collect(),
        }
    }

    #[test]
    fn drop_operation_as_drop_effect_returns_html5_values() {
        assert_eq!(DropOperation::Move.as_drop_effect(), "move");
        assert_eq!(DropOperation::Copy.as_drop_effect(), "copy");
        assert_eq!(DropOperation::Link.as_drop_effect(), "link");
        assert_eq!(DropOperation::Cancel.as_drop_effect(), "none");
    }

    #[test]
    fn drop_indicator_position_default_is_on_target() {
        assert_eq!(
            DropIndicatorPosition::default(),
            DropIndicatorPosition::OnTarget
        );
    }

    #[test]
    fn drag_config_default_has_expected_empty_state() {
        let config = DragConfig::default();

        assert!(!config.disabled);
        assert!(config.items.is_none());
        assert!(config.allowed_operations.is_none());
        assert!(config.on_drag_start.is_none());
        assert!(config.on_drag_end.is_none());
        assert!(config.get_items.is_none());
        assert!(config.drag_start_announcement.is_none());
    }

    #[test]
    fn drop_config_default_has_expected_empty_state() {
        let config = DropConfig::default();

        assert!(!config.disabled);
        assert!(config.on_drag_enter.is_none());
        assert!(config.on_drag_leave.is_none());
        assert!(config.on_drag_over.is_none());
        assert!(config.on_drop.is_none());
        assert!(config.accepted_operations.is_none());
        assert!(config.accepted_types.is_none());
        assert_eq!(
            config.drop_indicator_position,
            DropIndicatorPosition::OnTarget
        );
        assert!(config.drag_enter_announcement.is_none());
        assert!(config.drop_announcement.is_none());
    }

    #[test]
    fn drag_config_with_selection_sets_get_items() {
        let config =
            DragConfig::default().with_selection(|| vec![DragItem::Text("selected".into())]);

        let items = config
            .get_items
            .as_ref()
            .expect("selection closure should be set")();

        assert_eq!(items.len(), 1);
        match &items[0] {
            DragItem::Text(text) => assert_eq!(text, "selected"),
            other => panic!("unexpected drag item: {other:?}"),
        }
    }

    #[test]
    fn drag_config_debug_redacts_closures() {
        let config = DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("primary".into())])),
            allowed_operations: Some(vec![DropOperation::Copy, DropOperation::Move]),
            on_drag_start: Some(Callback::new(|_: DragStartEvent| {})),
            on_drag_end: Some(Callback::new(|_: DragEndEvent| {})),
            get_items: Some(Arc::new(|| vec![DragItem::Text("selection".into())])),
            ..DragConfig::default()
        };

        let mut debug = String::new();
        write!(&mut debug, "{config:?}").expect("debug write should succeed");

        assert!(debug.contains("disabled: false"));
        assert!(debug.contains("items: Some(\"<closure>\")"));
        assert!(debug.contains("allowed_operations: Some([Copy, Move])"));
        assert!(debug.contains("on_drag_start: Some(Callback(..))"));
        assert!(debug.contains("on_drag_end: Some(Callback(..))"));
        assert!(debug.contains("get_items: Some(\"<closure>\")"));
        assert!(debug.contains("drag_start_announcement: None"));
    }

    #[test]
    fn drop_config_without_accepted_types_accepts_any_preview_items() {
        let config = DropConfig::default();

        assert!(
            config.accepts_preview_items(&[preview(
                DragItemKind::Custom,
                &["application/x-ars-demo"],
            )])
        );
    }

    #[test]
    fn mime_type_matching_is_case_insensitive_and_normalizes_aliases() {
        let config = DropConfig {
            accepted_types: Some(vec!["image/jpeg".to_owned()]),
            ..DropConfig::default()
        };

        assert!(config.accepts_preview_items(&[preview(DragItemKind::File, &["IMAGE/JPEG"],)]));
        assert!(config.accepts_preview_items(&[preview(DragItemKind::File, &["image/jpg"],)]));
        assert!(!config.accepts_preview_items(&[preview(DragItemKind::File, &["text/plain"],)]));
    }

    #[test]
    fn mime_type_matching_supports_wildcards() {
        let config = DropConfig {
            accepted_types: Some(vec!["image/*".to_owned()]),
            ..DropConfig::default()
        };

        assert!(config.accepts_preview_items(&[preview(DragItemKind::File, &["image/png"],)]));
        assert!(config.accepts_preview_items(&[preview(DragItemKind::File, &["image/jpeg"],)]));
        assert!(!config.accepts_preview_items(&[preview(DragItemKind::File, &["text/plain"],)]));
    }

    #[test]
    fn mime_type_matching_accepts_when_one_of_multiple_item_mime_types_matches() {
        let config = DropConfig {
            accepted_types: Some(vec!["image/jpeg".to_owned()]),
            ..DropConfig::default()
        };

        assert!(
            config.accepts_preview_items(&[preview(
                DragItemKind::File,
                &["text/plain", "image/jpeg"],
            )])
        );
    }

    #[test]
    fn mime_type_matching_accepts_when_any_preview_item_matches() {
        let config = DropConfig {
            accepted_types: Some(vec!["image/jpeg".to_owned()]),
            ..DropConfig::default()
        };

        assert!(config.accepts_preview_items(&[
            preview(DragItemKind::File, &["text/plain"]),
            preview(DragItemKind::File, &["image/jpeg"]),
        ]));
    }

    #[test]
    fn drag_item_kind_round_trips_from_variants() {
        let cases = [
            (DragItem::Text("text".into()), DragItemKind::Text),
            (
                DragItem::Uri("https://example.com".into()),
                DragItemKind::Uri,
            ),
            (DragItem::Html("<p>hi</p>".into()), DragItemKind::Html),
            (
                DragItem::File {
                    name: "photo.jpg".into(),
                    mime_type: "image/jpeg".into(),
                    size: 42,
                    handle: FileHandle(()),
                },
                DragItemKind::File,
            ),
            (
                DragItem::Directory {
                    name: "assets".into(),
                    handle: DirectoryHandle(()),
                },
                DragItemKind::Directory,
            ),
            (
                DragItem::Custom {
                    mime_type: "application/x-ars-demo".into(),
                    data: "payload".into(),
                },
                DragItemKind::Custom,
            ),
        ];

        for (item, expected_kind) in cases {
            assert_eq!(DragItemKind::from(&item), expected_kind);
        }
    }

    #[test]
    fn drop_event_is_constructible_with_pointer_type_and_position() {
        let event = DropEvent {
            items: vec![DragItem::Text("payload".into())],
            operation: DropOperation::Move,
            pointer_type: PointerType::Mouse,
            drop_position: DropIndicatorPosition::Before,
        };

        assert_eq!(event.operation, DropOperation::Move);
        assert_eq!(event.pointer_type, PointerType::Mouse);
        assert_eq!(event.drop_position, DropIndicatorPosition::Before);
        assert_eq!(event.items.len(), 1);
    }
}
