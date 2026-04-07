//! Typed ARIA attributes, roles, and component ID generation for accessibility.
//!
//! This crate provides the accessibility building blocks used by all ars-ui components:
//! typed WAI-ARIA roles and attributes, and a namespaced ID generator for associating
//! labels, descriptions, and error messages with their form fields.

#![no_std]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;

use alloc::{format, string::String};

pub mod aria;
/// Shared focus management contracts consumed by DOM and adapter layers.
pub mod focus;

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

/// Custom data attribute used to expose machine state on the root DOM element.
///
/// Components set `data-ars-state` to the current state name, enabling CSS selectors
/// like `[data-ars-state="open"]` for styling and test assertions.
pub const DATA_ARS_STATE: &str = "data-ars-state";

/// Derives component part IDs from an adapter-provided base ID.
///
/// The base ID comes from the adapter's hydration-safe ID utility
/// (e.g., `use_id()` in ars-leptos, scope ID in ars-dioxus).
/// All relationship attributes (`aria-labelledby`, `aria-describedby`,
/// `aria-controls`, `aria-activedescendant`) use IDs derived from
/// this single base to guarantee uniqueness and consistency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentIds {
    base: String,
}

impl ComponentIds {
    /// Creates from an adapter-provided base ID.
    /// The base ID must be unique and hydration-safe.
    #[must_use]
    pub fn from_id(base_id: &str) -> Self {
        debug_assert!(!base_id.is_empty(), "Component base ID must not be empty");
        Self {
            base: String::from(base_id),
        }
    }

    /// Returns the base ID (for the root element).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.base
    }

    /// Derives a part ID: `"{base}-{part}"`.
    ///
    /// Use for fixed structural parts of a component (trigger, content, label, etc.).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ars_a11y::ComponentIds;
    /// let ids = ComponentIds::from_id("dialog-3");
    /// assert_eq!(ids.part("title"), "dialog-3-title");
    /// assert_eq!(ids.part("content"), "dialog-3-content");
    /// ```
    #[must_use]
    pub fn part(&self, part: &str) -> String {
        format!("{}-{}", self.base, part)
    }

    /// Derives a keyed item ID: `"{base}-{part}-{key}"`.
    ///
    /// Use for per-item IDs in collection components (lists, grids, trees, menus).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ars_a11y::ComponentIds;
    /// let ids = ComponentIds::from_id("listbox-2");
    /// assert_eq!(ids.item("item", &"option-a"), "listbox-2-item-option-a");
    /// ```
    #[must_use]
    pub fn item(&self, part: &str, key: &impl core::fmt::Display) -> String {
        format!("{}-{}-{}", self.base, part, key)
    }

    /// Derives a keyed item sub-part ID: `"{base}-{part}-{key}-{sub}"`.
    ///
    /// Use for sub-elements within a keyed item (text label, indicator, etc.).
    ///
    /// # Examples
    ///
    /// ```
    /// # use ars_a11y::ComponentIds;
    /// let ids = ComponentIds::from_id("listbox-2");
    /// assert_eq!(ids.item_part("item", &"opt-a", "text"), "listbox-2-item-opt-a-text");
    /// ```
    #[must_use]
    pub fn item_part(&self, part: &str, key: &impl core::fmt::Display, sub: &str) -> String {
        format!("{}-{}-{}-{}", self.base, part, key, sub)
    }
}

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
    fn component_ids_from_id_stores_base() {
        let ids = ComponentIds::from_id("dialog-3");
        assert_eq!(ids.id(), "dialog-3");
    }

    #[test]
    fn component_ids_part_derives_structural_id() {
        let ids = ComponentIds::from_id("dialog-3");
        assert_eq!(ids.part("title"), "dialog-3-title");
        assert_eq!(ids.part("content"), "dialog-3-content");
        assert_eq!(ids.part("description"), "dialog-3-description");
    }

    #[test]
    fn component_ids_item_derives_keyed_id() {
        let ids = ComponentIds::from_id("listbox-2");
        assert_eq!(ids.item("item", &"option-a"), "listbox-2-item-option-a");
        assert_eq!(ids.item("item", &42), "listbox-2-item-42");
    }

    #[test]
    fn component_ids_item_part_derives_sub_element_id() {
        let ids = ComponentIds::from_id("listbox-2");
        assert_eq!(
            ids.item_part("item", &"opt-a", "text"),
            "listbox-2-item-opt-a-text"
        );
    }

    #[test]
    fn data_ars_state_constant_value() {
        assert_eq!(DATA_ARS_STATE, "data-ars-state");
    }
}
