//! Drag-and-drop interaction types, state machines, and snapshot attrs.
//!
//! This module defines the framework-agnostic drag payload types, source and
//! target configuration structs, state-machine helpers, and MIME-type
//! acceptance logic used by drag-and-drop-enabled components.

use alloc::{borrow::ToOwned, format, rc::Rc, string::String, sync::Arc, vec::Vec};
use core::{
    cell::RefCell,
    fmt::{self, Debug},
};

use ars_a11y::{AnnouncementPriority, LiveAnnouncer};
use ars_core::{AttrMap, Callback, HtmlAttr, MessageFn};
use ars_i18n::Locale;

use crate::PointerType;

type DragItemsFn = Arc<dyn Fn() -> Vec<DragItem> + Send + Sync>;
type DragStartAnnouncementFn = MessageFn<dyn Fn(&[DragItem], &Locale) -> String + Send + Sync>;
type DragEnterAnnouncementFn = MessageFn<dyn Fn(&DropTargetEvent, &Locale) -> String + Send + Sync>;
type DropAnnouncementFn = MessageFn<dyn Fn(&DropEvent, &Locale) -> String + Send + Sync>;
type DragStartDefaultAnnouncementFn = MessageFn<dyn Fn(usize, &Locale) -> String + Send + Sync>;
type TargetAnnouncementFn = MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>;
type DropDefaultAnnouncementFn = MessageFn<dyn Fn(usize, &str, &Locale) -> String + Send + Sync>;

/// Screen reader announcements for keyboard and pointer drag-and-drop flows.
#[derive(Clone)]
pub struct DragAnnouncements {
    /// Announces that a drag session has started.
    pub drag_start: DragStartDefaultAnnouncementFn,

    /// Announces that focus has moved onto a drop target.
    pub drag_enter: TargetAnnouncementFn,

    /// Announces that focus has moved off a drop target.
    pub drag_leave: TargetAnnouncementFn,

    /// Announces that a drop completed successfully.
    pub drop: DropDefaultAnnouncementFn,

    /// Announces that the drag session was cancelled.
    pub drag_cancel: MessageFn<dyn Fn(&Locale) -> String + Send + Sync>,
}

impl Default for DragAnnouncements {
    fn default() -> Self {
        Self {
            drag_start: MessageFn::new(|count: usize, _locale: &Locale| {
                if count == 1 {
                    String::from(
                        "Started dragging 1 item. Press Tab or Shift+Tab to move between drop targets, Escape to cancel.",
                    )
                } else {
                    format!(
                        "Started dragging {count} items. Press Tab or Shift+Tab to move between drop targets, Escape to cancel."
                    )
                }
            }),

            drag_enter: MessageFn::new(|target: &str, _locale: &Locale| {
                format!("Drop target: {target}. Press Enter to drop here, Escape to cancel.")
            }),

            drag_leave: MessageFn::new(|target: &str, _locale: &Locale| {
                format!("Left drop target: {target}.")
            }),

            drop: MessageFn::new(|count: usize, target: &str, _locale: &Locale| {
                if count == 1 {
                    format!("Dropped 1 item into {target}.")
                } else {
                    format!("Dropped {count} items into {target}.")
                }
            }),

            drag_cancel: MessageFn::new(|_locale: &Locale| String::from("Drag cancelled.")),
        }
    }
}

impl Debug for DragAnnouncements {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DragAnnouncements").finish_non_exhaustive()
    }
}

/// Metadata for a drop target that participates in keyboard drag-and-drop.
#[derive(Clone, Debug)]
pub struct KeyboardDropTarget {
    /// The DOM id or framework-stable identifier for the target element.
    pub element_id: String,

    /// Human-readable label announced to assistive technology users.
    pub label: String,

    /// Drop-target configuration cloned from the mounted target.
    pub config: DropConfig,
}

/// Registry of active drop targets used for keyboard drag-and-drop navigation.
#[derive(Clone, Debug, Default)]
pub struct KeyboardDragRegistry {
    targets: Vec<KeyboardDropTarget>,
    current_index: Option<usize>,
}

impl KeyboardDragRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a live drop target in document order.
    pub fn register(&mut self, target: KeyboardDropTarget) {
        self.targets.push(target);
    }

    /// Unregisters a live drop target by element id.
    pub fn unregister(&mut self, element_id: &str) {
        let Some(index) = self
            .targets
            .iter()
            .position(|target| target.element_id == element_id)
        else {
            return;
        };

        self.targets.remove(index);

        self.current_index = match self.current_index {
            None => None,
            Some(current) if self.targets.is_empty() || current == index => None,
            Some(current) if index < current => Some(current - 1),
            Some(current) => Some(current.min(self.targets.len() - 1)),
        };
    }

    /// Advances to the next registered target, wrapping to the beginning.
    #[must_use]
    #[expect(
        clippy::should_implement_trait,
        reason = "The registry is not an iterator; next() is the spec-defined target-cycling API."
    )]
    pub fn next(&mut self) -> Option<&KeyboardDropTarget> {
        if self.targets.is_empty() {
            self.current_index = None;

            return None;
        }

        self.current_index = Some(if let Some(index) = self.current_index {
            (index + 1) % self.targets.len()
        } else {
            0
        });

        self.current()
    }

    /// Moves to the previous registered target, wrapping to the end.
    #[must_use]
    pub fn prev(&mut self) -> Option<&KeyboardDropTarget> {
        if self.targets.is_empty() {
            self.current_index = None;

            return None;
        }

        self.current_index = Some(match self.current_index {
            Some(0) | None => self.targets.len() - 1,
            Some(index) => index - 1,
        });

        self.current()
    }

    /// Returns the currently selected target, if any.
    #[must_use]
    pub fn current(&self) -> Option<&KeyboardDropTarget> {
        self.current_index.and_then(|index| self.targets.get(index))
    }

    /// Clears the active keyboard target selection while preserving registrations.
    pub const fn clear(&mut self) {
        self.current_index = None;
    }
}

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

/// Test-only helpers for constructing opaque drag payloads in downstream crates.
#[cfg(feature = "test-support")]
pub mod test_support {
    use alloc::string::String;

    use super::{DirectoryHandle, DragItem, FileHandle};

    /// Constructs a file drag item with a placeholder opaque handle for tests.
    #[must_use]
    pub fn file_drag_item(
        name: impl Into<String>,
        mime_type: impl Into<String>,
        size: u64,
    ) -> DragItem {
        DragItem::File {
            name: name.into(),
            mime_type: mime_type.into(),
            size,
            handle: FileHandle(()),
        }
    }

    /// Constructs a directory drag item with a placeholder opaque handle for tests.
    #[must_use]
    pub fn directory_drag_item(name: impl Into<String>) -> DragItem {
        DragItem::Directory {
            name: name.into(),
            handle: DirectoryHandle(()),
        }
    }
}

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
    pub on_drag_start: Option<Callback<dyn Fn(DragStartEvent) + Send + Sync>>,

    /// Called when dragging ends, regardless of outcome.
    pub on_drag_end: Option<Callback<dyn Fn(DragEndEvent) + Send + Sync>>,

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
    pub on_drag_enter: Option<Callback<dyn Fn(DropTargetEvent) + Send + Sync>>,

    /// Called when dragged items leave this target.
    pub on_drag_leave: Option<Callback<dyn Fn(DropTargetEvent) + Send + Sync>>,

    /// Called during drag-over to determine the accepted operation.
    pub on_drag_over: Option<Callback<dyn Fn(DropTargetEvent) -> DropOperation + Send + Sync>>,

    /// Called when items are dropped onto this target.
    pub on_drop: Option<Callback<dyn Fn(DropEvent) + Send + Sync>>,

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
        if let Some(accepted_types) = &self.accepted_types {
            items
                .iter()
                .any(|item| preview_matches_accepted_types(item, accepted_types))
        } else {
            true
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

    /// Starts a keyboard drag session and dispatches the corresponding announcement.
    #[must_use]
    pub fn start_keyboard_drag(
        &mut self,
        locale: &Locale,
        announcer: &mut LiveAnnouncer,
        announcements: &DragAnnouncements,
    ) -> Option<DragStartEvent> {
        let event = self.start_drag(PointerType::Keyboard)?;

        let message = self.config.drag_start_announcement.as_ref().map_or_else(
            || (announcements.drag_start)(event.items.len(), locale),
            |announcement| announcement(&event.items, locale),
        );

        announcer.announce_with_priority(message, AnnouncementPriority::Assertive);

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
            } => {
                if source_allows_operation(&self.config, current_operation) {
                    Some(DragState::DragOver {
                        items,
                        pointer_type,
                        target_id: target_id.into(),
                        current_operation,
                    })
                } else {
                    Some(DragState::Dragging {
                        items,
                        pointer_type,
                    })
                }
            }

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

    /// Completes a keyboard-driven drop and resets keyboard target selection.
    #[must_use]
    pub fn complete_keyboard_drop(
        &mut self,
        registry: &mut KeyboardDragRegistry,
    ) -> Option<DragEndEvent> {
        let event = self.complete_drop()?;

        registry.clear();

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

    /// Cancels a keyboard drag session, clears the active target snapshot, resets
    /// keyboard target selection, and announces the cancellation.
    #[must_use]
    pub fn cancel_keyboard_drag(
        &mut self,
        drop_target: &mut DropResult,
        registry: &mut KeyboardDragRegistry,
        locale: &Locale,
        announcer: &mut LiveAnnouncer,
        announcements: &DragAnnouncements,
    ) -> Option<DragEndEvent> {
        let event = self.cancel_drag()?;

        drop_target.reset();
        registry.clear();

        announcer.announce_with_priority(
            (announcements.drag_cancel)(locale),
            AnnouncementPriority::Assertive,
        );

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

        let operation = resolve_enter_operation(&self.config, &items, offered_operation);

        if operation == DropOperation::Cancel {
            self.enter_count = 0;
            self.drag_over = false;
            self.drop_operation = None;
            self.indicator_position = None;

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

    /// Handles keyboard focus moving onto this target and dispatches announcements.
    #[must_use]
    pub fn keyboard_drag_enter(
        &mut self,
        items: Vec<DragItemPreview>,
        offered_operation: DropOperation,
        target_label: &str,
        locale: &Locale,
        announcer: &mut LiveAnnouncer,
        announcements: &DragAnnouncements,
    ) -> DropOperation {
        let preview_items = items.clone();

        let operation = self.drag_enter(items, offered_operation, PointerType::Keyboard);

        if operation != DropOperation::Cancel {
            let message = self.config.drag_enter_announcement.as_ref().map_or_else(
                || (announcements.drag_enter)(target_label, locale),
                |announcement| {
                    announcement(
                        &DropTargetEvent {
                            items: preview_items,
                            operation,
                            pointer_type: PointerType::Keyboard,
                        },
                        locale,
                    )
                },
            );

            announcer.announce_with_priority(message, AnnouncementPriority::Polite);
        }

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
            resolve_drag_over_operation(&self.config, items, offered_operation, pointer_type);

        if operation == DropOperation::Cancel {
            self.enter_count = 0;
            self.drag_over = false;
            self.drop_operation = None;
            self.indicator_position = None;

            self.refresh_snapshot();

            return DropOperation::Cancel;
        }

        let was_inactive = self.enter_count == 0 || !self.drag_over;

        if self.enter_count == 0 {
            self.enter_count = 1;
        }

        self.drag_over = true;
        self.drop_operation = Some(operation);
        self.indicator_position = Some(self.config.drop_indicator_position);

        if was_inactive && let Some(on_drag_enter) = &self.config.on_drag_enter {
            on_drag_enter(DropTargetEvent {
                items: items.to_vec(),
                operation,
                pointer_type,
            });
        }

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

    /// Handles keyboard focus leaving this target and dispatches the leave announcement.
    pub fn keyboard_drag_leave(
        &mut self,
        items: &[DragItemPreview],
        target_label: &str,
        locale: &Locale,
        announcer: &mut LiveAnnouncer,
        announcements: &DragAnnouncements,
    ) {
        let should_announce = !self.config.disabled && self.enter_count == 1;

        self.drag_leave(items, PointerType::Keyboard);

        if should_announce {
            announcer.announce_with_priority(
                (announcements.drag_leave)(target_label, locale),
                AnnouncementPriority::Polite,
            );
        }
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

        if let Some(on_drop) = &self.config.on_drop
            && let Some(event) = &event
        {
            on_drop(event.clone());
        }

        self.reset();

        event
    }

    /// Handles a keyboard drop and dispatches the corresponding announcement.
    #[must_use]
    pub fn keyboard_drop(
        &mut self,
        items: Vec<DragItem>,
        target_label: &str,
        locale: &Locale,
        announcer: &mut LiveAnnouncer,
        announcements: &DragAnnouncements,
    ) -> Option<DropEvent> {
        let event = self.drop(items, PointerType::Keyboard)?;

        let message = self.config.drop_announcement.as_ref().map_or_else(
            || (announcements.drop)(event.items.len(), target_label, locale),
            |announcement| announcement(&event, locale),
        );

        announcer.announce_with_priority(message, AnnouncementPriority::Assertive);

        Some(event)
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

fn resolve_enter_operation(
    config: &DropConfig,
    items: &[DragItemPreview],
    offered_operation: DropOperation,
) -> DropOperation {
    if config.disabled || !accepts_preview_items(config, items) {
        return DropOperation::Cancel;
    }

    if offered_operation == DropOperation::Cancel {
        return DropOperation::Cancel;
    }

    if let Some(accepted_operations) = &config.accepted_operations
        && !accepted_operations.contains(&offered_operation)
    {
        return DropOperation::Cancel;
    }

    offered_operation
}

fn resolve_drag_over_operation(
    config: &DropConfig,
    items: &[DragItemPreview],
    offered_operation: DropOperation,
    pointer_type: PointerType,
) -> DropOperation {
    if config.disabled || !accepts_preview_items(config, items) {
        return DropOperation::Cancel;
    }

    if offered_operation == DropOperation::Cancel {
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

    if let Some(accepted_operations) = &config.accepted_operations
        && !accepted_operations.contains(&operation)
    {
        return DropOperation::Cancel;
    }

    operation
}

fn source_allows_operation(config: &DragConfig, operation: DropOperation) -> bool {
    operation != DropOperation::Cancel
        && config
            .allowed_operations
            .as_ref()
            .is_none_or(|allowed_operations| allowed_operations.contains(&operation))
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
    if let Some(accepted_types) = &config.accepted_types {
        items
            .iter()
            .any(|item| preview_matches_accepted_types(item, accepted_types))
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use alloc::{borrow::ToOwned, format, string::String, sync::Arc, vec, vec::Vec};
    use core::{fmt::Write as _, mem::discriminant};
    use std::sync::Mutex;

    use ars_a11y::LiveAnnouncer;
    use ars_core::{AttrValue, Callback, HtmlAttr, MessageFn};
    use ars_i18n::{Locale, locales};

    use super::{
        DirectoryHandle, DragAnnouncements, DragConfig, DragEndEvent, DragItem, DragItemKind,
        DragItemPreview, DragStartEvent, DragState, DropConfig, DropEvent, DropIndicatorPosition,
        DropOperation, DropTargetEvent, FileHandle, KeyboardDragRegistry, KeyboardDropTarget,
        build_drop_attrs, use_drag, use_drop,
    };
    use crate::PointerType;

    fn preview(kind: DragItemKind, mime_types: &[&str]) -> DragItemPreview {
        DragItemPreview {
            kind,
            mime_types: mime_types.iter().map(|mime| (*mime).to_owned()).collect(),
        }
    }

    fn drag_state_is_idle(state: &DragState) -> bool {
        discriminant(state) == discriminant(&DragState::Idle)
    }

    fn drag_state_is_dragging(state: &DragState) -> bool {
        discriminant(state)
            == discriminant(&DragState::Dragging {
                items: Vec::new(),
                pointer_type: PointerType::Mouse,
            })
    }

    fn resolve_link_operation(_: DropTargetEvent) -> DropOperation {
        DropOperation::Link
    }

    fn ignore_drop_event(_: DropEvent) {}

    fn keyboard_target(element_id: &str, label: &str, config: DropConfig) -> KeyboardDropTarget {
        KeyboardDropTarget {
            element_id: element_id.to_owned(),
            label: label.to_owned(),
            config,
        }
    }

    fn preview_from_item(item: &DragItem) -> DragItemPreview {
        match item {
            DragItem::Text(_) => preview(DragItemKind::Text, &["text/plain"]),

            DragItem::Uri(_) => preview(DragItemKind::Uri, &["text/uri-list"]),

            DragItem::Html(_) => preview(DragItemKind::Html, &["text/html"]),

            DragItem::File { mime_type, .. } => DragItemPreview {
                kind: DragItemKind::File,
                mime_types: vec![mime_type.clone()],
            },

            DragItem::Directory { .. } => preview(DragItemKind::Directory, &["inode/directory"]),

            DragItem::Custom { mime_type, .. } => DragItemPreview {
                kind: DragItemKind::Custom,
                mime_types: vec![mime_type.clone()],
            },
        }
    }

    fn previews_from_items(items: &[DragItem]) -> Vec<DragItemPreview> {
        items.iter().map(preview_from_item).collect()
    }

    fn announcer_debug(announcer: &LiveAnnouncer) -> String {
        format!("{announcer:?}")
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
    fn keyboard_drag_registry_new_starts_empty() {
        let registry = KeyboardDragRegistry::new();

        assert!(registry.current().is_none());
    }

    #[test]
    fn keyboard_drag_registry_register_and_clear_reset_selection_without_dropping_targets() {
        let mut registry = KeyboardDragRegistry::new();

        registry.register(keyboard_target("drop-a", "Alpha", DropConfig::default()));

        assert!(registry.current().is_none());
        assert_eq!(
            registry
                .next()
                .expect("first next should select target")
                .element_id,
            "drop-a"
        );

        registry.clear();

        assert!(registry.current().is_none());
        assert_eq!(
            registry
                .next()
                .expect("targets should remain registered after clear")
                .element_id,
            "drop-a"
        );
    }

    #[test]
    fn keyboard_drag_registry_unregister_removes_target() {
        let mut registry = KeyboardDragRegistry::new();

        registry.register(keyboard_target("drop-a", "Alpha", DropConfig::default()));
        registry.register(keyboard_target("drop-b", "Beta", DropConfig::default()));

        registry.unregister("drop-a");

        assert!(registry.current().is_none());
        assert_eq!(
            registry
                .next()
                .expect("remaining target should still be reachable")
                .element_id,
            "drop-b"
        );
    }

    #[test]
    fn keyboard_drag_registry_unregister_selected_target_clears_selection() {
        let mut registry = KeyboardDragRegistry::new();

        registry.register(keyboard_target("drop-a", "Alpha", DropConfig::default()));
        registry.register(keyboard_target("drop-b", "Beta", DropConfig::default()));

        assert_eq!(
            registry
                .next()
                .expect("first target should be selectable")
                .element_id,
            "drop-a"
        );

        registry.unregister("drop-a");

        assert!(registry.current().is_none());
        assert_eq!(
            registry
                .next()
                .expect("remaining target should be reselected explicitly")
                .element_id,
            "drop-b"
        );
    }

    #[test]
    fn keyboard_drag_registry_unregister_before_selected_adjusts_index() {
        let mut registry = KeyboardDragRegistry::new();

        registry.register(keyboard_target("drop-a", "Alpha", DropConfig::default()));
        registry.register(keyboard_target("drop-b", "Beta", DropConfig::default()));
        registry.register(keyboard_target("drop-c", "Charlie", DropConfig::default()));

        // Navigate to drop-c (index 2): next→0, next→1, next→2
        let _ = registry.next();
        let _ = registry.next();

        let current = registry.next().expect("should reach drop-c");

        assert_eq!(current.element_id, "drop-c");

        // Unregister drop-a (index 0) which is before the selected target (index 2).
        // The selected index should shift down by 1 to keep pointing at drop-c.
        registry.unregister("drop-a");

        let current = registry.current().expect("selection should be preserved");

        assert_eq!(current.element_id, "drop-c");
    }

    #[test]
    fn keyboard_drag_registry_unregister_after_selected_preserves_index() {
        let mut registry = KeyboardDragRegistry::new();

        registry.register(keyboard_target("drop-a", "Alpha", DropConfig::default()));
        registry.register(keyboard_target("drop-b", "Beta", DropConfig::default()));
        registry.register(keyboard_target("drop-c", "Charlie", DropConfig::default()));

        // Navigate to drop-a (index 0)
        let current = registry.next().expect("should reach drop-a");

        assert_eq!(current.element_id, "drop-a");

        // Unregister drop-c (index 2) which is after the selected target (index 0).
        // The selected index should remain unchanged.
        registry.unregister("drop-c");

        let current = registry.current().expect("selection should be preserved");

        assert_eq!(current.element_id, "drop-a");
    }

    #[test]
    fn keyboard_drag_registry_unregister_unknown_target_is_noop() {
        let mut registry = KeyboardDragRegistry::new();

        registry.register(keyboard_target("drop-a", "Alpha", DropConfig::default()));
        registry.unregister("missing");

        assert!(registry.current().is_none());
        assert_eq!(
            registry
                .next()
                .expect("registered target should remain reachable")
                .element_id,
            "drop-a"
        );
    }

    #[test]
    fn keyboard_drag_registry_empty_navigation_returns_none() {
        let mut registry = KeyboardDragRegistry::new();

        assert!(registry.next().is_none());
        assert!(registry.prev().is_none());
        assert!(registry.current().is_none());
    }

    #[test]
    fn keyboard_drag_registry_next_wraps_across_targets() {
        let mut registry = KeyboardDragRegistry::new();

        registry.register(keyboard_target("drop-a", "Alpha", DropConfig::default()));
        registry.register(keyboard_target("drop-b", "Beta", DropConfig::default()));
        registry.register(keyboard_target("drop-c", "Gamma", DropConfig::default()));

        assert_eq!(
            registry.next().expect("first next should exist").element_id,
            "drop-a"
        );
        assert_eq!(
            registry
                .next()
                .expect("second next should exist")
                .element_id,
            "drop-b"
        );
        assert_eq!(
            registry.next().expect("third next should exist").element_id,
            "drop-c"
        );
        assert_eq!(
            registry
                .next()
                .expect("wrapped next should exist")
                .element_id,
            "drop-a"
        );
    }

    #[test]
    fn keyboard_drag_registry_prev_wraps_across_targets() {
        let mut registry = KeyboardDragRegistry::new();

        registry.register(keyboard_target("drop-a", "Alpha", DropConfig::default()));
        registry.register(keyboard_target("drop-b", "Beta", DropConfig::default()));
        registry.register(keyboard_target("drop-c", "Gamma", DropConfig::default()));

        assert_eq!(
            registry
                .prev()
                .expect("wrapped prev should exist")
                .element_id,
            "drop-c"
        );
        assert_eq!(
            registry
                .prev()
                .expect("second prev should exist")
                .element_id,
            "drop-b"
        );
        assert_eq!(
            registry.prev().expect("third prev should exist").element_id,
            "drop-a"
        );
    }

    #[test]
    fn keyboard_drag_registry_single_target_stays_stable() {
        let mut registry = KeyboardDragRegistry::new();

        registry.register(keyboard_target("drop-a", "Alpha", DropConfig::default()));

        assert_eq!(
            registry
                .next()
                .expect("single next should exist")
                .element_id,
            "drop-a"
        );
        assert_eq!(
            registry
                .prev()
                .expect("single prev should exist")
                .element_id,
            "drop-a"
        );
    }

    #[test]
    fn drag_announcements_default_messages_are_non_empty() {
        let announcements = DragAnnouncements::default();

        let locale = locales::en_us();

        assert!(!(announcements.drag_start)(1, &locale).is_empty());
        assert!(!(announcements.drag_enter)("Inbox", &locale).is_empty());
        assert!(!(announcements.drag_leave)("Inbox", &locale).is_empty());
        assert!(!(announcements.drop)(1, "Inbox", &locale).is_empty());
        assert!(!(announcements.drag_cancel)(&locale).is_empty());
    }

    #[test]
    fn drag_announcements_default_drag_start_handles_singular_and_plural_counts() {
        let announcements = DragAnnouncements::default();

        let locale = locales::en_us();

        assert!((announcements.drag_start)(1, &locale).contains("1 item"));
        assert!((announcements.drag_start)(3, &locale).contains("3 items"));
    }

    #[test]
    fn drag_announcements_default_enter_leave_and_drop_include_target_name() {
        let announcements = DragAnnouncements::default();

        let locale = locales::en_us();

        assert!((announcements.drag_enter)("Inbox", &locale).contains("Inbox"));
        assert!((announcements.drag_leave)("Inbox", &locale).contains("Inbox"));

        let drop_message = (announcements.drop)(2, "Inbox", &locale);

        assert!(drop_message.contains("2 items"));
        assert!(drop_message.contains("Inbox"));
    }

    #[test]
    fn drag_announcements_default_cancel_matches_spec_text() {
        let announcements = DragAnnouncements::default();

        assert_eq!(
            (announcements.drag_cancel)(&locales::en_us()),
            "Drag cancelled."
        );
    }

    #[test]
    fn drag_announcements_debug_is_non_exhaustive() {
        let debug = format!("{:?}", DragAnnouncements::default());

        assert_eq!(debug, "DragAnnouncements { .. }");
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
    fn previews_from_items_preserve_kind_and_mime_types() {
        let items = vec![
            DragItem::Text("text".into()),
            DragItem::Uri("https://example.com".into()),
            DragItem::Html("<p>hi</p>".into()),
            DragItem::File {
                name: "photo.jpg".into(),
                mime_type: "image/jpeg".into(),
                size: 42,
                handle: FileHandle(()),
            },
            DragItem::Directory {
                name: "assets".into(),
                handle: DirectoryHandle(()),
            },
            DragItem::Custom {
                mime_type: "application/x-ars-demo".into(),
                data: "payload".into(),
            },
        ];

        let previews = previews_from_items(&items);

        assert_eq!(previews.len(), 6);
        assert_eq!(previews[0].kind, DragItemKind::Text);
        assert_eq!(previews[0].mime_types, vec![String::from("text/plain")]);
        assert_eq!(previews[1].kind, DragItemKind::Uri);
        assert_eq!(previews[1].mime_types, vec![String::from("text/uri-list")]);
        assert_eq!(previews[2].kind, DragItemKind::Html);
        assert_eq!(previews[2].mime_types, vec![String::from("text/html")]);
        assert_eq!(previews[3].kind, DragItemKind::File);
        assert_eq!(previews[3].mime_types, vec![String::from("image/jpeg")]);
        assert_eq!(previews[4].kind, DragItemKind::Directory);
        assert_eq!(
            previews[4].mime_types,
            vec![String::from("inode/directory")]
        );
        assert_eq!(previews[5].kind, DragItemKind::Custom);
        assert_eq!(
            previews[5].mime_types,
            vec![String::from("application/x-ars-demo")]
        );
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
    fn keyboard_drag_start_transitions_to_dragging_and_announces_assertively() {
        let announcements = DragAnnouncements::default();

        let mut announcer = LiveAnnouncer::new();

        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        let event = result
            .start_keyboard_drag(&locales::en_us(), &mut announcer, &announcements)
            .expect("keyboard drag should start");

        assert_eq!(event.pointer_type, PointerType::Keyboard);

        match result.current_state() {
            DragState::Dragging { pointer_type, .. } => {
                assert_eq!(pointer_type, PointerType::Keyboard);
            }

            state => panic!("unexpected state after keyboard drag start: {state:?}"),
        }

        let debug = announcer_debug(&announcer);
        assert!(debug.contains("active_priority: Some(Assertive)"));
        assert!(debug.contains("Started dragging 1 item"));
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
    fn keyboard_drag_start_prefers_per_element_override_message() {
        type AnnouncementFn = dyn Fn(&[DragItem], &Locale) -> String + Send + Sync;

        let announcements = DragAnnouncements::default();

        let mut announcer = LiveAnnouncer::new();

        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            drag_start_announcement: Some(MessageFn::new(Arc::new(
                |items: &[DragItem], _locale: &Locale| format!("override start {}", items.len()),
            ) as Arc<AnnouncementFn>)),
            ..DragConfig::default()
        });

        drop(
            result
                .start_keyboard_drag(&locales::en_us(), &mut announcer, &announcements)
                .expect("keyboard drag should start"),
        );

        assert!(announcer_debug(&announcer).contains("override start 1"));
    }

    #[test]
    fn drag_result_start_drag_is_noop_when_already_active() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Mouse));

        assert!(result.start_drag(PointerType::Touch).is_none());
        assert!(drag_state_is_dragging(&result.current_state()));
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
    fn drag_result_enter_target_rejects_operations_outside_allowed_list() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            allowed_operations: Some(vec![DropOperation::Copy]),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Mouse));

        result.enter_target("target-1", DropOperation::Move);

        assert!(drag_state_is_dragging(&result.current_state()));
        assert!(result.complete_drop().is_none());
    }

    #[test]
    fn drag_result_enter_target_retargets_when_already_drag_over() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Mouse));

        result.enter_target("target-1", DropOperation::Copy);
        result.enter_target("target-2", DropOperation::Move);

        match result.current_state() {
            DragState::DragOver {
                target_id,
                current_operation,
                ..
            } => {
                assert_eq!(target_id, "target-2");
                assert_eq!(current_operation, DropOperation::Move);
            }

            state => panic!("unexpected state after retargeting drag over: {state:?}"),
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
        assert!(drag_state_is_idle(&result.current_state()));
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
    fn drag_result_cancel_drag_from_drag_over_transitions_to_idle() {
        let mut result = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        drop(result.start_drag(PointerType::Touch));

        result.enter_target("target-1", DropOperation::Move);

        let event = result
            .cancel_drag()
            .expect("drag-over state should cancel cleanly");

        assert_eq!(event.operation, DropOperation::Cancel);
        assert_eq!(event.pointer_type, PointerType::Touch);
        assert!(matches!(result.current_state(), DragState::Idle));
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

        assert!(drag_state_is_idle(&result.current_state()));
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
    fn drag_result_start_drag_uses_selected_items_when_primary_items_are_absent() {
        let mut result = use_drag(DragConfig {
            get_items: Some(Arc::new(|| vec![DragItem::Text("selected".into())])),
            ..DragConfig::default()
        });

        let event = result
            .start_drag(PointerType::Mouse)
            .expect("drag should start");

        assert_eq!(event.items.len(), 1);

        match &event.items[0] {
            DragItem::Text(text) => assert_eq!(text, "selected"),
            other => panic!("unexpected drag item: {other:?}"),
        }
    }

    #[test]
    fn drag_result_disabled_noops_all_transition_helpers() {
        let mut result = use_drag(DragConfig {
            disabled: true,
            ..DragConfig::default()
        });

        assert!(result.start_drag(PointerType::Mouse).is_none());

        result.enter_target("target-1", DropOperation::Move);
        result.leave_target();

        assert!(result.complete_drop().is_none());
        assert!(result.cancel_drag().is_none());

        assert!(drag_state_is_idle(&result.current_state()));
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
    fn keyboard_drag_navigation_triggers_enter_callback_and_polite_announcement() {
        let enter_events = Arc::new(Mutex::new(Vec::<DropTargetEvent>::new()));

        let observed_events = Arc::clone(&enter_events);

        let announcements = DragAnnouncements::default();

        let mut announcer = LiveAnnouncer::new();

        let target_config = DropConfig {
            on_drag_enter: Some(Callback::new(move |event: DropTargetEvent| {
                observed_events
                    .lock()
                    .expect("enter events lock should succeed")
                    .push(event);
            })),
            ..DropConfig::default()
        };

        let mut registry = KeyboardDragRegistry::new();

        let mut drag = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        let mut drop_result = use_drop(target_config.clone());

        registry.register(keyboard_target("drop-a", "Inbox", target_config));

        let start_event = drag
            .start_keyboard_drag(&locales::en_us(), &mut announcer, &announcements)
            .expect("keyboard drag should start");

        announcer.notify_announced();

        let target = registry.next().expect("next target should exist").clone();

        let previews = previews_from_items(&start_event.items);

        let operation = drop_result.keyboard_drag_enter(
            previews,
            DropOperation::Move,
            &target.label,
            &locales::en_us(),
            &mut announcer,
            &announcements,
        );

        drag.enter_target(target.element_id.clone(), operation);

        assert_eq!(operation, DropOperation::Move);
        assert_eq!(
            enter_events
                .lock()
                .expect("enter events lock should succeed")
                .len(),
            1
        );

        match drag.current_state() {
            DragState::DragOver {
                target_id,
                pointer_type,
                ..
            } => {
                assert_eq!(target_id, "drop-a");
                assert_eq!(pointer_type, PointerType::Keyboard);
            }

            state => panic!("unexpected state after keyboard enter: {state:?}"),
        }

        let debug = announcer_debug(&announcer);

        assert!(debug.contains("active_priority: Some(Polite)"));
        assert!(debug.contains("Drop target: Inbox"));
    }

    #[test]
    fn keyboard_drag_enter_rejected_target_does_not_announce() {
        let announcements = DragAnnouncements::default();

        let mut announcer = LiveAnnouncer::new();

        let mut result = use_drop(DropConfig {
            accepted_types: Some(vec!["image/*".into()]),
            ..DropConfig::default()
        });

        let operation = result.keyboard_drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            "Inbox",
            &locales::en_us(),
            &mut announcer,
            &announcements,
        );

        assert_eq!(operation, DropOperation::Cancel);
        assert!(!announcer_debug(&announcer).contains("Inbox"));
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
    fn drop_result_drag_enter_does_not_invoke_drag_over_callback() {
        let mut result = use_drop(DropConfig {
            on_drag_over: Some(Callback::new(resolve_link_operation)),
            ..DropConfig::default()
        });

        let operation = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Copy,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Copy);
    }

    #[test]
    fn drop_result_drag_over_rejects_operations_outside_accepted_list() {
        let mut result = use_drop(DropConfig {
            accepted_operations: Some(vec![DropOperation::Copy]),
            on_drag_over: Some(Callback::new(resolve_link_operation)),
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

        assert_eq!(operation, DropOperation::Cancel);
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
        assert!(result.indicator_position.is_none());
    }

    #[test]
    fn drop_result_drag_enter_cancel_clears_stale_active_snapshot() {
        let mut result = use_drop(DropConfig {
            accepted_operations: Some(vec![DropOperation::Copy]),
            drop_indicator_position: DropIndicatorPosition::After,
            ..DropConfig::default()
        });

        let initial = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Copy,
            PointerType::Mouse,
        );

        let rejected = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        assert_eq!(initial, DropOperation::Copy);
        assert_eq!(rejected, DropOperation::Cancel);
        assert_eq!(result.enter_count, 0);
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
        assert!(result.indicator_position.is_none());
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drop-operation")));
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drop-position")));
    }

    #[test]
    fn drop_result_drag_enter_cancel_operation_is_rejected() {
        let mut result = use_drop(DropConfig::default());

        let operation = result.drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Cancel,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Cancel);
        assert_eq!(result.enter_count, 0);
        assert!(!result.drag_over);
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
    fn drop_result_drag_over_recovery_fires_enter_callback_once() {
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

        let first = result.drag_over(
            &[preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        let second = result.drag_over(
            &[preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        let enter_events = enter_events
            .lock()
            .expect("enter events lock should succeed");

        assert_eq!(first, DropOperation::Move);
        assert_eq!(second, DropOperation::Move);
        assert_eq!(result.enter_count, 1);
        assert_eq!(enter_events.len(), 1);
        assert_eq!(enter_events[0].operation, DropOperation::Move);
        assert_eq!(enter_events[0].pointer_type, PointerType::Mouse);
    }

    #[test]
    fn drop_result_drag_over_updates_operation_and_position_snapshot() {
        let mut result = use_drop(DropConfig {
            on_drag_over: Some(Callback::new(resolve_link_operation)),
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
    fn drop_result_drag_over_preserves_cancel_offered_operation() {
        let drag_over_calls = Arc::new(Mutex::new(Vec::<DropTargetEvent>::new()));

        let observed_calls = Arc::clone(&drag_over_calls);

        let mut result = use_drop(DropConfig {
            on_drag_over: Some(Callback::new(move |event: DropTargetEvent| {
                observed_calls
                    .lock()
                    .expect("drag over calls lock should succeed")
                    .push(event);

                DropOperation::Link
            })),
            ..DropConfig::default()
        });

        let operation = result.drag_over(
            &[preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Cancel,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Cancel);
        assert_eq!(result.enter_count, 0);
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
        assert!(result.indicator_position.is_none());
        assert!(!result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
        assert!(
            drag_over_calls
                .lock()
                .expect("drag over calls lock should succeed")
                .is_empty()
        );
    }

    #[test]
    fn build_drop_attrs_omits_operation_when_snapshot_has_no_operation() {
        let attrs = build_drop_attrs(true, None, Some(DropIndicatorPosition::Before));

        assert!(attrs.contains(&HtmlAttr::Data("ars-drag-over")));
        assert!(!attrs.contains(&HtmlAttr::Data("ars-drop-operation")));
        assert!(matches!(
            attrs.get_value(&HtmlAttr::Data("ars-drop-position")),
            Some(AttrValue::String(value)) if value == "before"
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
    fn keyboard_drag_leave_announces_polite_message_on_final_leave() {
        let announcements = DragAnnouncements::default();
        let mut announcer = LiveAnnouncer::new();
        let mut result = use_drop(DropConfig::default());

        let _ = result.keyboard_drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            "Inbox",
            &locales::en_us(),
            &mut announcer,
            &announcements,
        );

        announcer.notify_announced();

        result.keyboard_drag_leave(
            &[preview(DragItemKind::Text, &["text/plain"])],
            "Inbox",
            &locales::en_us(),
            &mut announcer,
            &announcements,
        );

        let debug = announcer_debug(&announcer);

        assert!(debug.contains("active_priority: Some(Polite)"));
        assert!(debug.contains("Left drop target: Inbox."));
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
    fn keyboard_drop_triggers_on_drop_announces_and_moves_source_to_dropped() {
        let drop_events = Arc::new(Mutex::new(Vec::<DropEvent>::new()));

        let observed_events = Arc::clone(&drop_events);

        let announcements = DragAnnouncements::default();

        let mut announcer = LiveAnnouncer::new();

        let target_config = DropConfig {
            on_drop: Some(Callback::new(move |event: DropEvent| {
                observed_events
                    .lock()
                    .expect("drop events lock should succeed")
                    .push(event);
            })),
            ..DropConfig::default()
        };

        let mut registry = KeyboardDragRegistry::new();

        let mut drag = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        let mut drop_result = use_drop(target_config.clone());

        registry.register(keyboard_target("drop-a", "Inbox", target_config));

        let start_event = drag
            .start_keyboard_drag(&locales::en_us(), &mut announcer, &announcements)
            .expect("keyboard drag should start");

        let target = registry.next().expect("first target should exist").clone();

        let previews = previews_from_items(&start_event.items);

        let operation = drop_result.keyboard_drag_enter(
            previews,
            DropOperation::Move,
            &target.label,
            &locales::en_us(),
            &mut announcer,
            &announcements,
        );

        drag.enter_target(target.element_id.clone(), operation);

        announcer.notify_announced();

        let drop_event = drop_result
            .keyboard_drop(
                start_event.items.clone(),
                &target.label,
                &locales::en_us(),
                &mut announcer,
                &announcements,
            )
            .expect("keyboard drop should succeed");

        let drag_end = drag
            .complete_keyboard_drop(&mut registry)
            .expect("source should complete drop");

        assert_eq!(drop_event.pointer_type, PointerType::Keyboard);
        assert_eq!(drag_end.pointer_type, PointerType::Keyboard);
        assert_eq!(
            drop_events
                .lock()
                .expect("drop events lock should succeed")
                .len(),
            1
        );
        assert!(registry.current().is_none());
        assert_eq!(
            registry
                .next()
                .expect("registered target should remain after drop")
                .element_id,
            "drop-a"
        );
        assert!(matches!(
            drag.current_state(),
            DragState::Dropped {
                operation: DropOperation::Move
            }
        ));

        let debug = announcer_debug(&announcer);

        assert!(debug.contains("active_priority: Some(Assertive)"));
        assert!(debug.contains("Dropped 1 item into Inbox."));
    }

    #[test]
    fn keyboard_drop_prefers_per_element_override_message() {
        type AnnouncementFn = dyn Fn(&DropEvent, &Locale) -> String + Send + Sync;

        let announcements = DragAnnouncements::default();

        let mut announcer = LiveAnnouncer::new();

        let target_config = DropConfig {
            drop_announcement: Some(MessageFn::new(Arc::new(
                |event: &DropEvent, _locale: &Locale| {
                    format!("override drop {:?}", event.operation)
                },
            ) as Arc<AnnouncementFn>)),
            ..DropConfig::default()
        };

        let mut drag = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        let mut drop_result = use_drop(target_config.clone());

        let start_event = drag
            .start_keyboard_drag(&locales::en_us(), &mut announcer, &announcements)
            .expect("keyboard drag should start");

        let previews = previews_from_items(&start_event.items);

        let operation = drop_result.keyboard_drag_enter(
            previews,
            DropOperation::Move,
            "Inbox",
            &locales::en_us(),
            &mut announcer,
            &announcements,
        );

        drag.enter_target("drop-a", operation);

        announcer.notify_announced();

        drop(
            drop_result
                .keyboard_drop(
                    start_event.items.clone(),
                    "Inbox",
                    &locales::en_us(),
                    &mut announcer,
                    &announcements,
                )
                .expect("keyboard drop should succeed"),
        );

        assert!(announcer_debug(&announcer).contains("override drop Move"));
    }

    #[test]
    fn keyboard_drag_cancel_fires_end_callback_clears_registry_and_announces() {
        let end_events = Arc::new(Mutex::new(Vec::<DragEndEvent>::new()));

        let observed_events = Arc::clone(&end_events);

        let announcements = DragAnnouncements::default();

        let mut announcer = LiveAnnouncer::new();

        let mut registry = KeyboardDragRegistry::new();

        let mut drag = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            on_drag_end: Some(Callback::new(move |event: DragEndEvent| {
                observed_events
                    .lock()
                    .expect("end events lock should succeed")
                    .push(event);
            })),
            ..DragConfig::default()
        });

        let mut drop_result = use_drop(DropConfig::default());

        registry.register(keyboard_target("drop-a", "Inbox", DropConfig::default()));

        let start_event = drag
            .start_keyboard_drag(&locales::en_us(), &mut announcer, &announcements)
            .expect("keyboard drag should start");

        let previews = previews_from_items(&start_event.items);

        let operation = drop_result.keyboard_drag_enter(
            previews,
            DropOperation::Move,
            "Inbox",
            &locales::en_us(),
            &mut announcer,
            &announcements,
        );

        drag.enter_target("drop-a", operation);

        announcer.notify_announced();

        let event = drag
            .cancel_keyboard_drag(
                &mut drop_result,
                &mut registry,
                &locales::en_us(),
                &mut announcer,
                &announcements,
            )
            .expect("keyboard drag should cancel");

        assert_eq!(event.pointer_type, PointerType::Keyboard);
        assert!(!event.was_dropped);
        assert!(registry.current().is_none());
        assert!(!drop_result.drag_over);
        assert!(!drop_result.attrs.contains(&HtmlAttr::Data("ars-drag-over")));
        assert!(
            !drop_result
                .attrs
                .contains(&HtmlAttr::Data("ars-drop-operation"))
        );
        assert!(
            !drop_result
                .attrs
                .contains(&HtmlAttr::Data("ars-drop-position"))
        );
        assert_eq!(
            registry
                .next()
                .expect("registered target should remain after cancel")
                .element_id,
            "drop-a"
        );
        assert!(matches!(drag.current_state(), DragState::Idle));
        assert_eq!(
            end_events
                .lock()
                .expect("end events lock should succeed")
                .len(),
            1
        );

        let debug = announcer_debug(&announcer);

        assert!(debug.contains("active_priority: Some(Assertive)"));
        assert!(debug.contains("Drag cancelled."));
    }

    #[test]
    fn keyboard_drag_enter_prefers_per_element_override_message() {
        type AnnouncementFn = dyn Fn(&DropTargetEvent, &Locale) -> String + Send + Sync;

        let announcements = DragAnnouncements::default();

        let mut announcer = LiveAnnouncer::new();

        let mut result = use_drop(DropConfig {
            drag_enter_announcement: Some(MessageFn::new(Arc::new(
                |event: &DropTargetEvent, _locale: &Locale| {
                    format!("override enter {:?}", event.operation)
                },
            ) as Arc<AnnouncementFn>)),
            ..DropConfig::default()
        });

        let operation = result.keyboard_drag_enter(
            vec![preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Copy,
            "Inbox",
            &locales::en_us(),
            &mut announcer,
            &announcements,
        );

        assert_eq!(operation, DropOperation::Copy);
        assert!(announcer_debug(&announcer).contains("override enter Copy"));
    }

    #[test]
    fn drop_result_drag_over_rejects_callback_operation_outside_accepted_list() {
        let mut result = use_drop(DropConfig {
            accepted_operations: Some(vec![DropOperation::Copy]),
            on_drag_over: Some(Callback::new(resolve_link_operation)),
            ..DropConfig::default()
        });

        let operation = result.drag_over(
            &[preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Copy,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Cancel);
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
    }

    #[test]
    fn drop_result_drag_over_rejects_unaccepted_mime_types() {
        let mut result = use_drop(DropConfig {
            accepted_types: Some(vec!["image/png".into()]),
            ..DropConfig::default()
        });

        // First activate via drag_enter with accepted types so the target is active.
        let _ = result.drag_enter(
            vec![preview(DragItemKind::Text, &["image/png"])],
            DropOperation::Move,
            PointerType::Mouse,
        );
        assert!(result.drag_over);

        // Now send drag_over with a MIME type that does NOT match accepted_types.
        // This hits resolve_drag_over_operation → !accepts_preview_items → Cancel.
        let operation = result.drag_over(
            &[preview(DragItemKind::Text, &["text/plain"])],
            DropOperation::Move,
            PointerType::Mouse,
        );

        assert_eq!(operation, DropOperation::Cancel);
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
    }

    #[test]
    fn drop_result_drop_skips_callback_when_operation_is_cancel() {
        let mut result = use_drop(DropConfig {
            on_drop: Some(Callback::new(ignore_drop_event)),
            ..DropConfig::default()
        });

        result.drag_over = true;
        result.drop_operation = Some(DropOperation::Cancel);
        result.indicator_position = Some(DropIndicatorPosition::OnTarget);

        assert!(
            result
                .drop(vec![DragItem::Text("payload".into())], PointerType::Mouse)
                .is_none()
        );
        assert!(!result.drag_over);
        assert!(result.drop_operation.is_none());
        assert!(result.indicator_position.is_none());
    }

    #[test]
    fn keyboard_drop_without_active_target_returns_none_and_does_not_announce() {
        let announcements = DragAnnouncements::default();

        let mut announcer = LiveAnnouncer::new();

        let mut result = use_drop(DropConfig::default());

        assert!(
            result
                .keyboard_drop(
                    vec![DragItem::Text("payload".into())],
                    "Inbox",
                    &locales::en_us(),
                    &mut announcer,
                    &announcements,
                )
                .is_none()
        );
        assert!(!announcer_debug(&announcer).contains("Dropped"));
    }

    #[test]
    fn complete_keyboard_drop_without_drag_over_returns_none_and_keeps_registry() {
        let mut registry = KeyboardDragRegistry::new();

        let mut drag = use_drag(DragConfig {
            items: Some(Arc::new(|| vec![DragItem::Text("payload".into())])),
            ..DragConfig::default()
        });

        registry.register(keyboard_target("drop-a", "Inbox", DropConfig::default()));

        drop(drag.start_drag(PointerType::Keyboard));

        assert!(drag.complete_keyboard_drop(&mut registry).is_none());
        assert_eq!(
            registry
                .next()
                .expect("registry should remain intact")
                .element_id,
            "drop-a"
        );
    }

    #[test]
    fn cancel_keyboard_drag_without_active_drag_returns_none_and_keeps_registry() {
        let announcements = DragAnnouncements::default();

        let mut announcer = LiveAnnouncer::new();

        let mut registry = KeyboardDragRegistry::new();

        let mut drag = use_drag(DragConfig::default());

        let mut drop_result = use_drop(DropConfig::default());

        registry.register(keyboard_target("drop-a", "Inbox", DropConfig::default()));

        assert!(
            drag.cancel_keyboard_drag(
                &mut drop_result,
                &mut registry,
                &locales::en_us(),
                &mut announcer,
                &announcements,
            )
            .is_none()
        );
        assert_eq!(
            registry
                .next()
                .expect("registry should remain intact")
                .element_id,
            "drop-a"
        );
        assert!(!announcer_debug(&announcer).contains("Drag cancelled."));
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
