//! Drag-and-drop interaction types, state machines, and snapshot attrs.
//!
//! This module defines the framework-agnostic drag payload types, source and
//! target configuration structs, state-machine helpers, and MIME-type
//! acceptance logic used by drag-and-drop-enabled components.

use std::{
    cell::RefCell,
    fmt::{self, Debug},
    rc::Rc,
    string::String,
    sync::Arc,
    vec::Vec,
};

use ars_core::{AttrMap, Callback, HtmlAttr, MessageFn};
use ars_i18n::Locale;

use crate::PointerType;

type DragItemsFn = Arc<dyn Fn() -> Vec<DragItem> + Send + Sync>;
type DragStartAnnouncementFn = MessageFn<dyn Fn(&[DragItem], &Locale) -> String + Send + Sync>;
type DragEnterAnnouncementFn = MessageFn<dyn Fn(&DropTargetEvent, &Locale) -> String + Send + Sync>;
type DropAnnouncementFn = MessageFn<dyn Fn(&DropEvent, &Locale) -> String + Send + Sync>;

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

    /// Localized screen reader message override when drag starts.
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

    /// Localized screen reader message override when dragged items enter this target.
    pub drag_enter_announcement: Option<DragEnterAnnouncementFn>,

    /// Localized screen reader message override when a drop succeeds.
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

/// The current state of the drag source state machine.
///
/// `DragState` intentionally does not implement `PartialEq` because dragged
/// payloads may contain opaque file-system handles that do not support value
/// equality. Callers should pattern-match on the variants they care about.
#[derive(Clone, Debug)]
pub enum DragState {
    /// No drag is active.
    Idle,
    /// A drag is active and no drop target is currently hovered.
    Dragging {
        /// Items included in the active drag payload.
        items: Vec<DragItem>,
        /// Input modality that initiated the drag.
        pointer_type: PointerType,
    },
    /// A drag is active and a valid drop target is currently hovered.
    DragOver {
        /// Items included in the active drag payload.
        items: Vec<DragItem>,
        /// Input modality that initiated the drag.
        pointer_type: PointerType,
        /// Identifier for the currently hovered drop target.
        target_id: String,
        /// Operation that would occur if dropped now.
        current_operation: DropOperation,
    },
    /// A drop was accepted and cleanup is pending.
    Dropped {
        /// Operation accepted by the drop target.
        operation: DropOperation,
    },
}

/// Snapshot output of [`use_drag`].
///
/// Unlike press, hover, and focus interactions, drag attrs are stored as a
/// stable snapshot and refreshed after each state-machine mutation.
#[derive(Debug)]
pub struct DragResult {
    /// Data attributes to spread onto the draggable element.
    pub attrs: AttrMap,
    /// Whether this element is currently being dragged.
    pub dragging: bool,

    state: Rc<RefCell<DragState>>,
    config: DragConfig,
}

impl DragResult {
    /// Returns the current drag-state snapshot.
    #[must_use]
    pub fn current_state(&self) -> DragState {
        self.state.borrow().clone()
    }

    /// Starts a drag and returns the emitted start event when the transition succeeds.
    ///
    /// This is the adapter-facing entry point for pointer-threshold and
    /// touch-long-press initiated drags.
    #[must_use]
    pub fn start_drag(&mut self, pointer_type: PointerType) -> Option<DragStartEvent> {
        if self.config.disabled || !matches!(self.current_state(), DragState::Idle) {
            self.refresh_snapshot();

            return None;
        }

        let items = collect_drag_items(&self.config);

        let event = DragStartEvent {
            items: items.clone(),
            pointer_type,
        };

        *self.state.borrow_mut() = DragState::Dragging {
            items,
            pointer_type,
        };

        if let Some(on_drag_start) = &self.config.on_drag_start {
            on_drag_start(event.clone());
        }

        self.refresh_snapshot();

        Some(event)
    }

    /// Transitions the active drag into the hovered-target state.
    ///
    /// Adapters should call this only after a drop target reports a valid
    /// target/operation for the current drag payload.
    pub fn enter_target(&mut self, target_id: impl Into<String>, current_operation: DropOperation) {
        if self.config.disabled {
            self.refresh_snapshot();

            return;
        }

        let next_state = match self.current_state() {
            DragState::Dragging {
                items,
                pointer_type,
            }
            | DragState::DragOver {
                items,
                pointer_type,
                ..
            } => Some(DragState::DragOver {
                items,
                pointer_type,
                target_id: target_id.into(),
                current_operation,
            }),

            DragState::Idle | DragState::Dropped { .. } => None,
        };

        if let Some(next_state) = next_state {
            *self.state.borrow_mut() = next_state;
        }

        self.refresh_snapshot();
    }

    /// Transitions the active drag back to the non-hovered dragging state.
    ///
    /// Adapters should call this when the active target's enter/leave nesting
    /// count reaches zero.
    pub fn leave_target(&mut self) {
        if self.config.disabled {
            self.refresh_snapshot();

            return;
        }

        let next_state = match self.current_state() {
            DragState::DragOver {
                items,
                pointer_type,
                ..
            } => Some(DragState::Dragging {
                items,
                pointer_type,
            }),

            DragState::Idle | DragState::Dragging { .. } | DragState::Dropped { .. } => None,
        };

        if let Some(next_state) = next_state {
            *self.state.borrow_mut() = next_state;
        }

        self.refresh_snapshot();
    }

    /// Completes the drag with an accepted drop and returns the emitted end event.
    #[must_use]
    pub fn complete_drop(&mut self) -> Option<DragEndEvent> {
        if self.config.disabled {
            self.refresh_snapshot();

            return None;
        }

        let (items, pointer_type, operation) = match self.current_state() {
            DragState::DragOver {
                items,
                pointer_type,
                current_operation,
                ..
            } => (items, pointer_type, current_operation),

            DragState::Idle | DragState::Dragging { .. } | DragState::Dropped { .. } => {
                self.refresh_snapshot();
                return None;
            }
        };

        let event = DragEndEvent {
            items,
            operation,
            pointer_type,
            was_dropped: true,
        };

        *self.state.borrow_mut() = DragState::Dropped { operation };

        if let Some(on_drag_end) = &self.config.on_drag_end {
            on_drag_end(event.clone());
        }

        self.refresh_snapshot();

        Some(event)
    }

    /// Cancels the active drag and returns the emitted end event when one existed.
    ///
    /// This covers drag-end-without-drop, explicit cancel, and adapter-level
    /// recovery after pointer-capture or drag setup failures.
    #[must_use]
    pub fn cancel_drag(&mut self) -> Option<DragEndEvent> {
        if self.config.disabled {
            self.refresh_snapshot();

            return None;
        }

        let (items, pointer_type) = match self.current_state() {
            DragState::Dragging {
                items,
                pointer_type,
            }
            | DragState::DragOver {
                items,
                pointer_type,
                ..
            } => (items, pointer_type),

            DragState::Idle | DragState::Dropped { .. } => {
                self.refresh_snapshot();
                return None;
            }
        };

        let event = DragEndEvent {
            items,
            operation: DropOperation::Cancel,
            pointer_type,
            was_dropped: false,
        };

        *self.state.borrow_mut() = DragState::Idle;

        if let Some(on_drag_end) = &self.config.on_drag_end {
            on_drag_end(event.clone());
        }

        self.refresh_snapshot();

        Some(event)
    }

    /// Resets the drag source to `Idle` without emitting callbacks.
    ///
    /// Adapters use this after post-drop cleanup or forced error recovery.
    pub fn reset(&mut self) {
        *self.state.borrow_mut() = DragState::Idle;

        self.refresh_snapshot();
    }

    fn refresh_snapshot(&mut self) {
        self.dragging = !matches!(self.current_state(), DragState::Idle);
        self.attrs = build_drag_attrs(self.dragging);
    }
}

/// Snapshot output of [`use_drop`].
///
/// `DropResult` keeps the latest drop-target snapshot attrs alongside minimal
/// adapter-facing state for nested enter/leave tracking.
#[derive(Debug)]
pub struct DropResult {
    /// Data attributes to spread onto the drop target element.
    pub attrs: AttrMap,
    /// Whether a dragged item is currently over this target.
    pub drag_over: bool,
    /// The operation that will occur if dropped now.
    pub drop_operation: Option<DropOperation>,
    /// Where the drop indicator line should appear.
    pub indicator_position: Option<DropIndicatorPosition>,

    config: DropConfig,
    enter_count: i32,
}

impl DropResult {
    /// Handles a drag-enter event and returns the operation currently accepted.
    ///
    /// Nested child enters increment the internal enter count but only the
    /// initial `0 -> 1` transition fires `on_drag_enter`.
    #[must_use]
    pub fn drag_enter(
        &mut self,
        items: Vec<DragItemPreview>,
        offered_operation: DropOperation,
        pointer_type: PointerType,
    ) -> DropOperation {
        if self.config.disabled {
            self.refresh_snapshot();

            return DropOperation::Cancel;
        }

        let operation =
            resolve_drop_operation(&self.config, &items, offered_operation, pointer_type);

        if operation == DropOperation::Cancel {
            self.refresh_snapshot();

            return DropOperation::Cancel;
        }

        self.enter_count = self.enter_count.saturating_add(1);

        let was_inactive = !self.drag_over;

        self.drag_over = true;
        self.drop_operation = Some(operation);
        self.indicator_position = Some(self.config.drop_indicator_position);

        if was_inactive && let Some(on_drag_enter) = &self.config.on_drag_enter {
            on_drag_enter(DropTargetEvent {
                items,
                operation,
                pointer_type,
            });
        }

        self.refresh_snapshot();

        operation
    }

    /// Handles a drag-over update and returns the operation currently accepted.
    #[must_use]
    pub fn drag_over(
        &mut self,
        items: &[DragItemPreview],
        offered_operation: DropOperation,
        pointer_type: PointerType,
    ) -> DropOperation {
        if self.config.disabled {
            self.refresh_snapshot();

            return DropOperation::Cancel;
        }

        let operation =
            resolve_drop_operation(&self.config, items, offered_operation, pointer_type);

        if operation == DropOperation::Cancel {
            self.drag_over = false;
            self.drop_operation = None;
            self.indicator_position = None;

            self.refresh_snapshot();

            return DropOperation::Cancel;
        }

        if self.enter_count == 0 {
            self.enter_count = 1;
        }

        self.drag_over = true;
        self.drop_operation = Some(operation);
        self.indicator_position = Some(self.config.drop_indicator_position);

        self.refresh_snapshot();

        operation
    }

    /// Handles a drag-leave event, clearing state only when nesting reaches zero.
    pub fn drag_leave(&mut self, items: &[DragItemPreview], pointer_type: PointerType) {
        if self.config.disabled {
            self.refresh_snapshot();

            return;
        }

        if self.enter_count <= 0 {
            self.enter_count = 0;
            self.refresh_snapshot();

            return;
        }

        self.enter_count -= 1;

        if self.enter_count == 0 {
            if let Some(on_drag_leave) = &self.config.on_drag_leave {
                on_drag_leave(DropTargetEvent {
                    items: items.to_vec(),
                    operation: self.drop_operation.unwrap_or(DropOperation::Cancel),
                    pointer_type,
                });
            }

            self.drag_over = false;
            self.drop_operation = None;
            self.indicator_position = None;
        }

        self.refresh_snapshot();
    }

    /// Handles a completed drop and returns the emitted drop event when accepted.
    #[must_use]
    pub fn drop(&mut self, items: Vec<DragItem>, pointer_type: PointerType) -> Option<DropEvent> {
        if self.config.disabled || !self.drag_over {
            self.reset();

            return None;
        }

        let operation = self.drop_operation.unwrap_or(DropOperation::Cancel);

        let event = (operation != DropOperation::Cancel).then_some(DropEvent {
            items,
            operation,
            pointer_type,
            drop_position: self.config.drop_indicator_position,
        });

        if let Some(on_drop) = &self.config.on_drop {
            if let Some(event) = &event {
                on_drop(event.clone());
            }
        }

        self.reset();

        event
    }

    /// Clears all drop-target state without emitting callbacks.
    pub fn reset(&mut self) {
        self.enter_count = 0;
        self.drag_over = false;
        self.drop_operation = None;
        self.indicator_position = None;

        self.refresh_snapshot();
    }

    fn refresh_snapshot(&mut self) {
        self.attrs = build_drop_attrs(self.drag_over, self.drop_operation, self.indicator_position);
    }
}

/// Creates a drag-source state container with snapshot attrs.
#[must_use]
pub fn use_drag(config: DragConfig) -> DragResult {
    let mut result = DragResult {
        attrs: AttrMap::new(),
        dragging: false,
        state: Rc::new(RefCell::new(DragState::Idle)),
        config,
    };

    result.refresh_snapshot();

    result
}

/// Creates a drop-target state container with snapshot attrs.
#[must_use]
pub fn use_drop(config: DropConfig) -> DropResult {
    let mut result = DropResult {
        attrs: AttrMap::new(),
        drag_over: false,
        drop_operation: None,
        indicator_position: None,
        config,
        enter_count: 0,
    };

    result.refresh_snapshot();

    result
}

fn collect_drag_items(config: &DragConfig) -> Vec<DragItem> {
    let mut items = Vec::new();

    if let Some(primary_items) = &config.items {
        items.extend(primary_items());
    }

    if let Some(selected_items) = &config.get_items {
        items.extend(selected_items());
    }

    items
}

fn build_drag_attrs(dragging: bool) -> AttrMap {
    let mut attrs = AttrMap::new();

    attrs.set(HtmlAttr::Draggable, "true");

    if dragging {
        attrs.set_bool(HtmlAttr::Data("ars-dragging"), true);
    }

    attrs
}

fn build_drop_attrs(
    drag_over: bool,
    drop_operation: Option<DropOperation>,
    indicator_position: Option<DropIndicatorPosition>,
) -> AttrMap {
    let mut attrs = AttrMap::new();

    if drag_over {
        attrs.set_bool(HtmlAttr::Data("ars-drag-over"), true);

        if let Some(operation) = drop_operation {
            attrs.set(
                HtmlAttr::Data("ars-drop-operation"),
                operation.as_drop_effect(),
            );
        }

        if let Some(position) = indicator_position {
            attrs.set(
                HtmlAttr::Data("ars-drop-position"),
                match position {
                    DropIndicatorPosition::Before => "before",
                    DropIndicatorPosition::After => "after",
                    DropIndicatorPosition::OnTarget => "on",
                },
            );
        }
    }
    attrs
}

fn resolve_drop_operation(
    config: &DropConfig,
    items: &[DragItemPreview],
    offered_operation: DropOperation,
    pointer_type: PointerType,
) -> DropOperation {
    if config.disabled || !accepts_preview_items(config, items) {
        return DropOperation::Cancel;
    }

    let operation = config
        .on_drag_over
        .as_ref()
        .map_or(offered_operation, |on_drag_over| {
            on_drag_over(DropTargetEvent {
                items: items.to_vec(),
                operation: offered_operation,
                pointer_type,
            })
        });

    if operation == DropOperation::Cancel {
        return DropOperation::Cancel;
    }

    if let Some(accepted_operations) = &config.accepted_operations {
        if !accepted_operations.contains(&operation) {
            return DropOperation::Cancel;
        }
    }

    operation
}

fn preview_matches_accepted_types(item: &DragItemPreview, accepted_types: &[String]) -> bool {
    item.mime_types.iter().any(|mime_type| {
        accepted_types
            .iter()
            .map(String::as_str)
            .map(normalize_mime_type)
            .any(|accepted| mime_type_matches(&accepted, &normalize_mime_type(mime_type)))
    })
}

fn mime_type_matches(accepted: &str, actual: &str) -> bool {
    if let Some(prefix) = accepted.strip_suffix("/*") {
        actual
            .split_once('/')
            .is_some_and(|(actual_prefix, _)| actual_prefix == prefix)
    } else {
        accepted == actual
    }
}

fn normalize_mime_type(mime_type: &str) -> String {
    let normalized = mime_type.trim().to_ascii_lowercase();
    if normalized == "image/jpg" {
        "image/jpeg".to_owned()
    } else {
        normalized
    }
}

fn accepts_preview_items(config: &DropConfig, items: &[DragItemPreview]) -> bool {
    match &config.accepted_types {
        None => true,
        Some(accepted_types) => items
            .iter()
            .any(|item| preview_matches_accepted_types(item, accepted_types)),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fmt::Write as _,
        sync::{Arc, Mutex},
    };

    use ars_core::{AttrValue, Callback, HtmlAttr, MessageFn};
    use ars_i18n::{Locale, locales};

    use super::{
        DirectoryHandle, DragConfig, DragEndEvent, DragItem, DragItemKind, DragItemPreview,
        DragStartEvent, DragState, DropConfig, DropEvent, DropIndicatorPosition, DropOperation,
        DropTargetEvent, FileHandle, use_drag, use_drop,
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
    fn drag_start_announcement_is_constructible_and_invokable() {
        type AnnouncementFn = dyn Fn(&[DragItem], &Locale) -> String + Send + Sync;

        let announcement: Arc<AnnouncementFn> = Arc::new(|items: &[DragItem], locale: &Locale| {
            format!("{} @ {}", items.len(), locale.to_bcp47())
        });

        let config = DragConfig {
            drag_start_announcement: Some(MessageFn::new(announcement)),
            ..DragConfig::default()
        };

        let message = config
            .drag_start_announcement
            .as_ref()
            .expect("announcement should be set")(
            &[
                DragItem::Text("payload".into()),
                DragItem::Uri("urn:test".into()),
            ],
            &locales::en_us(),
        );

        assert_eq!(message, "2 @ en-US");
    }

    #[test]
    fn drag_enter_announcement_is_constructible_and_invokable() {
        type AnnouncementFn = dyn Fn(&DropTargetEvent, &Locale) -> String + Send + Sync;

        let announcement: Arc<AnnouncementFn> =
            Arc::new(|event: &DropTargetEvent, locale: &Locale| {
                format!("{:?} @ {}", event.operation, locale.to_bcp47())
            });

        let config = DropConfig {
            drag_enter_announcement: Some(MessageFn::new(announcement)),
            ..DropConfig::default()
        };

        let message = config
            .drag_enter_announcement
            .as_ref()
            .expect("announcement should be set")(
            &DropTargetEvent {
                items: vec![preview(DragItemKind::File, &["image/png"])],
                operation: DropOperation::Copy,
                pointer_type: PointerType::Mouse,
            },
            &locales::de_de(),
        );

        assert_eq!(message, "Copy @ de-DE");
    }

    #[test]
    fn drop_announcement_is_constructible_and_invokable() {
        type AnnouncementFn = dyn Fn(&DropEvent, &Locale) -> String + Send + Sync;

        let announcement: Arc<AnnouncementFn> = Arc::new(|event: &DropEvent, locale: &Locale| {
            format!("{:?} @ {}", event.drop_position, locale.to_bcp47())
        });

        let config = DropConfig {
            drop_announcement: Some(MessageFn::new(announcement)),
            ..DropConfig::default()
        };

        let message = config
            .drop_announcement
            .as_ref()
            .expect("announcement should be set")(
            &DropEvent {
                items: vec![DragItem::Text("payload".into())],
                operation: DropOperation::Move,
                pointer_type: PointerType::Mouse,
                drop_position: DropIndicatorPosition::After,
            },
            &locales::en_us(),
        );

        assert_eq!(message, "After @ en-US");
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

    #[test]
    fn drag_result_start_drag_transitions_idle_to_dragging() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        let event = result
            .start_drag(PointerType::Mouse)
            .expect("idle drag should start");

        assert_eq!(event.pointer_type, PointerType::Mouse);
        assert_eq!(event.items.len(), 1);
        assert!(result.dragging);
        assert!(result.attrs.contains(&HtmlAttr::Data("ars-dragging")));

        match result.current_state() {
            DragState::Dragging {
                items,
                pointer_type,
            } => {
                assert_eq!(items.len(), 1);
                assert_eq!(pointer_type, PointerType::Mouse);
            }
            state => panic!("unexpected state after start_drag: {state:?}"),
        }
    }

    #[test]
    fn drag_result_start_drag_fires_callback_with_payload() {
        let start_events = Arc::new(Mutex::new(Vec::<DragStartEvent>::new()));

        let observed_events = Arc::clone(&start_events);

        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            on_drag_start: Some(Callback::new(move |event: DragStartEvent| {
                observed_events
                    .lock()
                    .expect("start events lock should succeed")
                    .push(event);
            })),
            ..DragConfig::default()
        });

        let event = result
            .start_drag(PointerType::Pen)
            .expect("idle drag should start");

        let start_events = start_events
            .lock()
            .expect("start events lock should succeed");

        assert_eq!(start_events.len(), 1);
        assert_eq!(start_events[0].pointer_type, event.pointer_type);
        assert_eq!(start_events[0].items.len(), event.items.len());
    }

    #[test]
    fn drag_result_enter_target_transitions_dragging_to_drag_over() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Mouse));

        result.enter_target("target-1", DropOperation::Copy);

        match result.current_state() {
            DragState::DragOver {
                items,
                pointer_type,
                target_id,
                current_operation,
            } => {
                assert_eq!(items.len(), 1);
                assert_eq!(pointer_type, PointerType::Mouse);
                assert_eq!(target_id, "target-1");
                assert_eq!(current_operation, DropOperation::Copy);
            }
            state => panic!("unexpected state after enter_target: {state:?}"),
        }
    }

    #[test]
    fn drag_result_invalid_source_transitions_are_noops() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        result.enter_target("target-1", DropOperation::Move);
        result.leave_target();

        assert!(result.complete_drop().is_none());
        assert!(result.cancel_drag().is_none());
        assert!(matches!(result.current_state(), DragState::Idle));

        drop(result.start_drag(PointerType::Mouse));

        assert!(result.complete_drop().is_none());
        assert!(matches!(result.current_state(), DragState::Dragging { .. }));

        result.enter_target("target-1", DropOperation::Copy);

        drop(result.complete_drop());

        result.enter_target("target-2", DropOperation::Move);
        result.leave_target();

        assert!(result.complete_drop().is_none());
        assert!(result.cancel_drag().is_none());
        assert!(matches!(
            result.current_state(),
            DragState::Dropped {
                operation: DropOperation::Copy
            }
        ));
    }

    #[test]
    fn drag_result_leave_target_transitions_drag_over_to_dragging() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Mouse));

        result.enter_target("target-1", DropOperation::Move);

        result.leave_target();

        match result.current_state() {
            DragState::Dragging {
                items,
                pointer_type,
            } => {
                assert_eq!(items.len(), 1);
                assert_eq!(pointer_type, PointerType::Mouse);
            }
            state => panic!("unexpected state after leave_target: {state:?}"),
        }
    }

    #[test]
    fn drag_result_complete_drop_fires_end_callback_with_current_operation() {
        let end_events = Arc::new(Mutex::new(Vec::<DragEndEvent>::new()));

        let observed_events = Arc::clone(&end_events);

        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            on_drag_end: Some(Callback::new(move |event: DragEndEvent| {
                observed_events
                    .lock()
                    .expect("end events lock should succeed")
                    .push(event);
            })),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Pen));

        result.enter_target("target-1", DropOperation::Link);

        let event = result
            .complete_drop()
            .expect("drag over should complete drop");

        let end_events = end_events.lock().expect("end events lock should succeed");

        assert_eq!(end_events.len(), 1);
        assert_eq!(end_events[0].operation, event.operation);
        assert_eq!(end_events[0].pointer_type, event.pointer_type);
        assert_eq!(end_events[0].items.len(), event.items.len());
        assert!(end_events[0].was_dropped);
    }

    #[test]
    fn drag_result_complete_drop_transitions_drag_over_to_dropped() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Pen));

        result.enter_target("target-1", DropOperation::Link);

        let event = result
            .complete_drop()
            .expect("drag over should complete drop");

        assert_eq!(event.operation, DropOperation::Link);
        assert_eq!(event.pointer_type, PointerType::Pen);
        assert!(event.was_dropped);

        match result.current_state() {
            DragState::Dropped { operation } => assert_eq!(operation, DropOperation::Link),
            state => panic!("unexpected state after complete_drop: {state:?}"),
        }
    }

    #[test]
    fn drag_result_cancel_drag_transitions_to_idle_and_fires_cancel_event() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Touch));

        let event = result.cancel_drag().expect("active drag should cancel");

        assert_eq!(event.operation, DropOperation::Cancel);
        assert_eq!(event.pointer_type, PointerType::Touch);
        assert!(!event.was_dropped);
        assert!(matches!(result.current_state(), DragState::Idle));
        assert!(!result.dragging);
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-dragging")));
    }

    #[test]
    fn drag_result_cancel_drag_fires_end_callback_once() {
        let end_events = Arc::new(Mutex::new(Vec::<DragEndEvent>::new()));

        let observed_events = Arc::clone(&end_events);

        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            on_drag_end: Some(Callback::new(move |event: DragEndEvent| {
                observed_events
                    .lock()
                    .expect("end events lock should succeed")
                    .push(event);
            })),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Touch));

        let event = result.cancel_drag().expect("active drag should cancel");

        let end_events = end_events.lock().expect("end events lock should succeed");

        assert_eq!(end_events.len(), 1);
        assert_eq!(end_events[0].operation, event.operation);
        assert_eq!(end_events[0].pointer_type, event.pointer_type);
        assert_eq!(end_events[0].items.len(), event.items.len());
        assert!(!end_events[0].was_dropped);
    }

    #[test]
    fn drag_result_reset_transitions_dropped_to_idle() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Mouse));

        result.enter_target("target-1", DropOperation::Copy);

        drop(result.complete_drop());

        result.reset();

        assert!(matches!(result.current_state(), DragState::Idle));
        assert!(!result.dragging);
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-dragging")));
    }

    #[test]
    fn drag_result_attrs_include_draggable_even_when_idle() {
        let result = use_drag(DragConfig::default());

        assert!(result.attrs.contains(&HtmlAttr::Draggable));
        assert!(matches!(
            result.attrs.get_value(&HtmlAttr::Draggable),
            Some(AttrValue::String(value)) if value == "true"
        ));
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-dragging")));
    }

    #[test]
    fn drag_result_multi_item_drag_concatenates_primary_and_selected_items() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("primary".into())])),
            get_items: Some(Arc::new(|| vec![DragItem::Text("selected".into())])),
            ..DragConfig::default()
        });

        let event = result
            .start_drag(PointerType::Mouse)
            .expect("drag should start");

        assert_eq!(event.items.len(), 2);

        match &event.items[0] {
            DragItem::Text(text) => assert_eq!(text, "primary"),
            other => panic!("unexpected primary drag item: {other:?}"),
        }

        match &event.items[1] {
            DragItem::Text(text) => assert_eq!(text, "selected"),
            other => panic!("unexpected selected drag item: {other:?}"),
        }
    }

    #[test]
    fn drag_result_start_drag_uses_only_primary_items_when_selection_is_none() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("primary".into())])),
            ..DragConfig::default()
        });

        let event = result
            .start_drag(PointerType::Mouse)
            .expect("drag should start");

        assert_eq!(event.items.len(), 1);

        match &event.items[0] {
            DragItem::Text(text) => assert_eq!(text, "primary"),
            other => panic!("unexpected drag item: {other:?}"),
        }
    }

    #[test]
    fn drag_result_disabled_noops_all_transition_helpers() {
        let mut result = use_drag(DragConfig {
            disabled: true,
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        assert!(result.start_drag(PointerType::Mouse).is_none());

        result.enter_target("target-1", DropOperation::Move);
        result.leave_target();

        assert!(result.complete_drop().is_none());
        assert!(result.cancel_drag().is_none());

        assert!(matches!(result.current_state(), DragState::Idle));
        assert!(!result.dragging);
        assert!(result.attrs.contains(&HtmlAttr::Draggable));
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-dragging")));
    }

    #[test]
    fn drop_result_drag_enter_fires_callback_only_on_initial_activation() {
        let enter_events = Arc::new(Mutex::new(Vec::<DropTargetEvent>::new()));

        let observed_events = Arc::clone(&enter_events);

        let mut result = use_drop(DropConfig {
            on_drag_enter: Some(Callback::new(move |event: DropTargetEvent| {
                observed_events
                    .lock()
                    .expect("enter events lock should succeed")
                    .push(event);
            })),
            ..DropConfig::default()
        });

        let first = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        let second = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        let enter_events = enter_events
            .lock()
            .expect("enter events lock should succeed");

        assert_eq!(first, DropOperation::Move);
        assert_eq!(second, DropOperation::Move);
        assert_eq!(result.enter_count, 2);
        assert_eq!(enter_events.len(), 1);
        assert_eq!(enter_events[0].operation, DropOperation::Move);
        assert_eq!(enter_events[0].pointer_type, PointerType::Mouse);
    }

    #[test]
    fn drop_result_drag_enter_sets_drag_over_and_indicator_snapshot() {
        let mut result = use_drop(DropConfig {
            drop_indicator_position: DropIndicatorPosition::Before,
            ..DropConfig::default()
        });

        let operation = result.drag_enter(
            vec![preview(DragItemKind::File, &["image/png"])],
            DropOperation::Copy,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Copy);
        assert!(result.drag_over);
        assert_eq!(result.drop_operation, Some(DropOperation::Copy));
        assert_eq!(
            result.indicator_position,
            Some(DropIndicatorPosition::Before)
        );
        assert!(result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
        assert!(matches!(
            result.attrs.get_value(&HtmlAttr::Data("ars-drop-operation")),
            Some(AttrValue::String(value)) if value == "copy"
        ));
        assert!(matches!(
            result.attrs.get_value(&HtmlAttr::Data("ars-drop-position")),
            Some(AttrValue::String(value)) if value == "before"
        ));
    }

    #[test]
    fn drop_result_drag_over_recovers_missing_drag_enter() {
        let mut result = use_drop(DropConfig {
            drop_indicator_position: DropIndicatorPosition::After,
            ..DropConfig::default()
        });

        let operation = result.drag_over(
            &[preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Copy,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Copy);
        assert_eq!(result.enter_count, 1);
        assert!(result.drag_over);
        assert_eq!(result.drop_operation, Some(DropOperation::Copy));
        assert_eq!(
            result.indicator_position,
            Some(DropIndicatorPosition::After)
        );
        assert!(result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
    }

    #[test]
    fn drop_result_drag_over_updates_operation_and_position_snapshot() {
        let mut result = use_drop(DropConfig {
            on_drag_over: Some(Callback::new(|_: DropTargetEvent| DropOperation::Link)),
            drop_indicator_position: DropIndicatorPosition::After,
            ..DropConfig::default()
        });

        let _ = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Copy,
            PointerType::Mouse,
        );

        let operation = result.drag_over(
            &[preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Copy,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Link);
        assert_eq!(result.drop_operation, Some(DropOperation::Link));
        assert_eq!(
            result.indicator_position,
            Some(DropIndicatorPosition::After)
        );
        assert!(matches!(
            result.attrs.get_value(&HtmlAttr::Data("ars-drop-operation")),
            Some(AttrValue::String(value)) if value == "link"
        ));
        assert!(matches!(
            result.attrs.get_value(&HtmlAttr::Data("ars-drop-position")),
            Some(AttrValue::String(value)) if value == "after"
        ));
    }

    #[test]
    fn drop_result_rejects_operations_outside_accepted_list() {
        let mut result = use_drop(DropConfig {
            accepted_operations: Some(vec![DropOperation::Copy]),
            ..DropConfig::default()
        });

        let operation = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Cancel);
        assert_eq!(result.enter_count, 0);
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
    }

    #[test]
    fn drop_result_nested_enter_leave_only_clears_on_final_leave() {
        let mut result = use_drop(DropConfig::default());

        let _ = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        let _ = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        assert_eq!(result.enter_count, 2);
        assert!(result.drag_over);

        result.drag_leave(
            &[preview(DragItemKind::Text, &["text/plain"])],
            PointerType::Mouse,
        );

        assert_eq!(result.enter_count, 1);
        assert!(result.drag_over);
        assert!(result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));

        result.drag_leave(
            &[preview(DragItemKind::Text, &["text/plain"])],
            PointerType::Mouse,
        );

        assert_eq!(result.enter_count, 0);
        assert!(!result.drag_over);
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
    }

    #[test]
    fn drop_result_drag_leave_saturates_at_zero() {
        let mut result = use_drop(DropConfig::default());

        result.drag_leave(
            &[preview(DragItemKind::Text, &["text/plain"])],
            PointerType::Mouse,
        );

        assert_eq!(result.enter_count, 0);
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
    }

    #[test]
    fn drop_result_drag_leave_fires_callback_on_final_leave() {
        let leave_events = Arc::new(Mutex::new(Vec::<DropTargetEvent>::new()));

        let observed_events = Arc::clone(&leave_events);

        let mut result = use_drop(DropConfig {
            on_drag_leave: Some(Callback::new(move |event: DropTargetEvent| {
                observed_events
                    .lock()
                    .expect("leave events lock should succeed")
                    .push(event);
            })),
            ..DropConfig::default()
        });

        let _ = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Link,
            PointerType::Pen,
        );

        let _ = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Link,
            PointerType::Pen,
        );

        result.drag_leave(
            &[preview(DragItemKind::Text, &["text/plain"])],
            PointerType::Pen,
        );

        assert!(
            leave_events
                .lock()
                .expect("leave events lock should succeed")
                .is_empty()
        );

        result.drag_leave(
            &[preview(DragItemKind::Text, &["text/plain"])],
            PointerType::Pen,
        );

        let leave_events = leave_events
            .lock()
            .expect("leave events lock should succeed");

        assert_eq!(leave_events.len(), 1);
        assert_eq!(leave_events[0].operation, DropOperation::Link);
        assert_eq!(leave_events[0].pointer_type, PointerType::Pen);
    }

    #[test]
    fn drop_result_drop_returns_event_and_resets_state() {
        let mut result = use_drop(DropConfig {
            drop_indicator_position: DropIndicatorPosition::OnTarget,
            ..DropConfig::default()
        });

        let _ = result.drag_enter(
            vec![preview(DragItemKind::File, &["image/png"])],
            DropOperation::Move,
            PointerType::Pen,
        );

        let event = result
            .drop(
                vec![DragItem::File {
                    name: "image.png".into(),
                    mime_type: "image/png".into(),
                    size: 12,
                    handle: FileHandle(()),
                }],
                PointerType::Pen,
            )
            .expect("accepted drop should produce event");

        assert_eq!(event.operation, DropOperation::Move);
        assert_eq!(event.pointer_type, PointerType::Pen);
        assert_eq!(event.drop_position, DropIndicatorPosition::OnTarget);
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
    }

    #[test]
    fn drop_result_drop_fires_callback_and_inactive_drop_is_none() {
        let drop_events = Arc::new(Mutex::new(Vec::<DropEvent>::new()));

        let observed_events = Arc::clone(&drop_events);

        let mut result = use_drop(DropConfig {
            on_drop: Some(Callback::new(move |event: DropEvent| {
                observed_events
                    .lock()
                    .expect("drop events lock should succeed")
                    .push(event);
            })),
            ..DropConfig::default()
        });

        assert!(
            result
                .drop(vec![DragItem::Text("payload".into())], PointerType::Mouse)
                .is_none()
        );
        assert!(
            drop_events
                .lock()
                .expect("drop events lock should succeed")
                .is_empty()
        );

        let _ = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        let event = result
            .drop(vec![DragItem::Text("payload".into())], PointerType::Mouse)
            .expect("accepted drop should produce event");

        let drop_events = drop_events.lock().expect("drop events lock should succeed");

        assert_eq!(drop_events.len(), 1);
        assert_eq!(drop_events[0].operation, event.operation);
        assert_eq!(drop_events[0].pointer_type, event.pointer_type);
        assert_eq!(drop_events[0].drop_position, event.drop_position);
        assert_eq!(drop_events[0].items.len(), event.items.len());
    }

    #[test]
    fn drop_result_rejected_types_return_cancel_operation() {
        let mut result = use_drop(DropConfig {
            accepted_types: Some(vec!["image/*".into()]),
            ..DropConfig::default()
        });

        let operation = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Copy,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Cancel);
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
        assert!(result.indicator_position.is_none());
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drop-operation")));
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drop-position")));
    }

    #[test]
    fn drop_result_drag_over_cancel_clears_active_snapshot() {
        let mut result = use_drop(DropConfig {
            on_drag_over: Some(Callback::new(|_: DropTargetEvent| DropOperation::Cancel)),
            ..DropConfig::default()
        });

        let _ = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        let operation = result.drag_over(
            &[preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Cancel);
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
        assert!(result.indicator_position.is_none());
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
    }

    #[test]
    fn drop_result_disabled_noops_all_transition_helpers() {
        let mut result = use_drop(DropConfig {
            disabled: true,
            ..DropConfig::default()
        });

        let operation = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Cancel);
        assert_eq!(
            result.drag_over(
                &[preview(DragItemKind::Text, &["text/plain"])],
                DropOperation::Move,
                PointerType::Mouse,
            ),
            DropOperation::Cancel
        );

        result.drag_leave(
            &[preview(DragItemKind::Text, &["text/plain"])],
            PointerType::Mouse,
        );

        assert!(
            result
                .drop(vec![DragItem::Text("payload".into())], PointerType::Mouse)
                .is_none()
        );

        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
    }
}
