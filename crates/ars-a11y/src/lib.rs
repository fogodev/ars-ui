//! Typed ARIA attributes, roles, and screen-reader support helpers.
//!
//! This crate provides the accessibility building blocks used by all ars-ui components:
//! typed WAI-ARIA roles and attributes, focus helpers, and live-announcement support.

#![no_std]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;

pub mod announcer;
pub mod aria;
/// Shared focus management contracts consumed by DOM and adapter layers.
pub mod focus;
pub mod visually_hidden;

pub use announcer::{Announcement, AnnouncementPriority, LiveAnnouncer};
#[cfg(feature = "aria-drag-drop-compat")]
pub use aria::attribute::AriaDropeffect;
pub use aria::{
    apply::{apply_aria, apply_role},
    attribute::{
        AriaAttribute, AriaAutocomplete, AriaChecked, AriaCurrent, AriaHasPopup, AriaIdList,
        AriaIdRef, AriaInvalid, AriaLive, AriaOrientation, AriaPressed, AriaRelevant, AriaSort,
    },
    role::AriaRole,
    state::{set_busy, set_checked, set_disabled, set_expanded, set_invalid, set_selected},
};
pub use focus::{FocusRing, FocusScopeBehavior, FocusScopeOptions, FocusStrategy, FocusTarget};
pub use visually_hidden::{
    VisuallyHiddenCssDoc, VisuallyHiddenFocusableCssDoc, visually_hidden_attrs,
    visually_hidden_focusable_attrs,
};

/// Custom data attribute used to expose machine state on the root DOM element.
///
/// Components set `data-ars-state` to the current state name, enabling CSS selectors
/// like `[data-ars-state="open"]` for styling and test assertions.
pub const DATA_ARS_STATE: &str = "data-ars-state";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aria_role_clone_and_equality() {
        let role = AriaRole::Button;
        #[expect(clippy::clone_on_copy, reason = "deliberately testing Clone impl")]
        let cloned = role.clone();
        assert_eq!(role, cloned);
        assert_ne!(AriaRole::Button, AriaRole::Dialog);
    }

    #[test]
    fn aria_attribute_clone_and_equality() {
        let attr = AriaAttribute::Disabled(true);
        let cloned = attr.clone();
        assert_eq!(attr, cloned);
        assert_ne!(
            AriaAttribute::Disabled(true),
            AriaAttribute::Disabled(false)
        );
    }

    #[test]
    fn data_ars_state_constant_value() {
        assert_eq!(DATA_ARS_STATE, "data-ars-state");
    }
}
