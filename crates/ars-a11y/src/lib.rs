//! Typed ARIA attributes, roles, and component ID generation for accessibility.
//!
//! This crate provides the accessibility building blocks used by all ars-ui components:
//! typed WAI-ARIA roles and attributes, and a namespaced ID generator for associating
//! labels, descriptions, and error messages with their form fields.

#![no_std]
#![warn(clippy::std_instead_of_core)]

extern crate alloc;

use alloc::string::String;

/// Custom data attribute used to expose machine state on the root DOM element.
///
/// Components set `data-ars-state` to the current state name, enabling CSS selectors
/// like `[data-ars-state="open"]` for styling and test assertions.
pub const DATA_ARS_STATE: &str = "data-ars-state";

/// A WAI-ARIA role that conveys the semantic purpose of a DOM element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AriaRole {
    /// An interactive element that triggers an action when activated.
    Button,
    /// A checkable input with `true`, `false`, or `mixed` states.
    Checkbox,
    /// A modal or non-modal dialog window.
    Dialog,
    /// A generic grouping container for related elements.
    Group,
    /// An element removed from the accessibility tree (purely decorative).
    Presentation,
}

/// A WAI-ARIA state or property attribute applied to a DOM element.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AriaAttribute {
    /// Defines a human-readable label for the element (`aria-label`).
    Label,
    /// References the ID of the element that labels this one (`aria-labelledby`).
    LabelledBy,
    /// References the ID of the element that describes this one (`aria-describedby`).
    DescribedBy,
    /// Indicates the element's value is invalid (`aria-invalid`).
    Invalid,
}

/// A set of related DOM IDs for a component and its associated elements.
///
/// Used to wire up `aria-labelledby`, `aria-describedby`, and `aria-errormessage`
/// attributes that link a component to its label, description, and error elements.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComponentIds {
    /// The root element's DOM ID.
    pub root: String,
    /// The label element's DOM ID, if present.
    pub label: Option<String>,
    /// The description element's DOM ID, if present.
    pub description: Option<String>,
    /// The error message element's DOM ID, if present.
    pub error: Option<String>,
}

impl ComponentIds {
    /// Creates a new [`ComponentIds`] with the given root ID and no associated elements.
    #[must_use]
    pub fn named(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            ..Self::default()
        }
    }
}
