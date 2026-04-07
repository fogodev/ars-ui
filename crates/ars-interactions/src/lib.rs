//! Input interaction state types and attribute merging utilities.
//!
//! This crate defines the shared interaction states (press, focus) used across
//! components and provides [`compose::merge_attrs`] for merging attribute maps
//! from multiple interaction sources into a single [`ars_core::AttrMap`].

pub mod compose;
pub mod direction;
pub mod focus;
pub mod hover;
pub mod press;

pub use ars_core::{
    Callback, DefaultModalityContext, KeyModifiers, KeyboardKey, ModalityContext, ModalitySnapshot,
    NullModalityContext, PointerType, SharedFlag, SharedState,
};
pub use compose::merge_attrs;
pub use direction::{LogicalDirection, resolve_arrow_key};
pub use focus::{
    FocusConfig, FocusEvent, FocusEventType, FocusResult, FocusState, FocusWithinConfig,
    FocusWithinResult, use_focus, use_focus_within,
};
pub use hover::{HoverConfig, HoverEvent, HoverEventType, HoverResult, HoverState, use_hover};
pub use press::{PressConfig, PressEvent, PressEventType, PressResult, PressState, use_press};
