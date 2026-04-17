//! Input interaction state types and attribute merging utilities.
//!
//! This crate defines the shared interaction states (press, focus) used across
//! components and provides [`compose::merge_attrs`] for merging attribute maps
//! from multiple interaction sources into a single [`ars_core::AttrMap`].

pub mod companion_css;
pub mod compose;
pub mod direction;
pub mod dismissable;
pub mod drag_drop;
pub mod focus;
pub mod hover;
pub mod interact_outside;
pub mod keyboard;
pub mod long_press;
pub mod move_interaction;
pub mod press;

pub use ars_core::{
    Callback, ComponentIds, DefaultModalityContext, KeyModifiers, KeyboardKey, ModalityContext,
    ModalitySnapshot, NullModalityContext, PointerType, SharedFlag, SharedState, TimerHandle,
};
pub use compose::merge_attrs;
pub use direction::{LogicalDirection, resolve_arrow_key};
pub use dismissable::dismiss_button_attrs;
/// Test-only helpers for constructing drag payloads in downstream crates.
#[cfg(feature = "test-support")]
pub use drag_drop::test_support;
pub use drag_drop::{
    DirectoryHandle, DragAnnouncements, DragConfig, DragEndEvent, DragItem, DragItemKind,
    DragItemPreview, DragResult, DragStartEvent, DragState, DropConfig, DropEvent,
    DropIndicatorPosition, DropOperation, DropResult, DropTargetEvent, FileHandle,
    KeyboardDragRegistry, KeyboardDropTarget, use_drag, use_drop,
};
pub use focus::{
    FocusConfig, FocusEvent, FocusEventType, FocusResult, FocusState, FocusWithinConfig,
    FocusWithinResult, use_focus, use_focus_within,
};
pub use hover::{HoverConfig, HoverEvent, HoverEventType, HoverResult, HoverState, use_hover};
pub use interact_outside::{
    InteractOutsideConfig, InteractOutsideEvent, InteractOutsideStandalone,
};
pub use keyboard::{ArsKeyboardEvent, KeyboardConfig, KeyboardEventData};
pub use long_press::{
    LongPressConfig, LongPressEvent, LongPressEventType, LongPressResult, LongPressState,
    use_long_press,
};
pub use move_interaction::{MoveConfig, MoveEvent, MoveEventType, MoveResult, MoveState, use_move};
pub use press::{PressConfig, PressEvent, PressEventType, PressResult, PressState, use_press};
