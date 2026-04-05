//! Role assignment helpers for `connect()` implementations.
//!
//! These functions apply ARIA roles and attributes to typed [`ars_core::AttrMap`]s,
//! providing the shared accessibility layer used by all component connect
//! functions.

use ars_core::{AttrMap, HtmlAttr};

use super::{attribute::AriaAttribute, role::AriaRole};

/// Applies an ARIA role to an attribute map.
///
/// Abstract roles are silently ignored — callers may derive roles dynamically,
/// so validation is deferred to `AriaValidator` rather than panicking here.
///
/// # Examples
///
/// ```
/// # use ars_a11y::aria::apply::apply_role;
/// # use ars_a11y::AriaRole;
/// # use ars_core::{AttrMap, HtmlAttr};
/// let mut attrs = AttrMap::new();
/// apply_role(&mut attrs, AriaRole::Button);
/// assert_eq!(attrs.get(&HtmlAttr::Role), Some("button"));
/// ```
#[inline]
pub fn apply_role(attrs: &mut AttrMap, role: AriaRole) {
    if let Some(value) = role.to_attr_value() {
        attrs.set(HtmlAttr::Role, value);
    }
    // Abstract roles are silently ignored — no debug_assert here because
    // callers may derive roles dynamically; instead, validate via AriaValidator.
}

/// Batch-applies ARIA attributes to an attribute map.
///
/// Each attribute delegates to [`AriaAttribute::apply_to`], which handles
/// serialization and nullable attribute removal.
///
/// # Examples
///
/// ```
/// # use ars_a11y::aria::apply::apply_aria;
/// # use ars_a11y::AriaAttribute;
/// # use ars_core::AttrMap;
/// let mut attrs = AttrMap::new();
/// apply_aria(&mut attrs, [
///     AriaAttribute::Disabled(true),
///     AriaAttribute::Expanded(Some(false)),
/// ]);
/// ```
#[inline]
pub fn apply_aria(attr_map: &mut AttrMap, aria_attrs: impl IntoIterator<Item = AriaAttribute>) {
    for attr in aria_attrs {
        attr.apply_to(attr_map);
    }
}

/// Compile-time checked role assignment.
///
/// Use this macro to get a compile error for abstract roles.
///
/// **Note:** The compile-time check is enforced only when the argument is a `const`
/// expression (e.g., a literal `AriaRole::Button`). For runtime role values, call
/// [`AriaRole::is_abstract()`] manually before calling `set_role!`.
///
/// # Usage
///
/// ```
/// # use ars_a11y::set_role;
/// # use ars_a11y::AriaRole;
/// # use ars_core::{AttrMap, HtmlAttr};
/// let mut attrs = AttrMap::new();
/// set_role!(attrs, AriaRole::Button);
/// assert_eq!(attrs.get(&HtmlAttr::Role), Some("button"));
/// ```
#[macro_export]
macro_rules! set_role {
    ($attrs:expr, $role:expr) => {{
        const _: () = {
            // Evaluated at compile time — will fail if role is abstract.
            // The trick: abstract roles have `to_attr_value` returning None,
            // but we use a const fn check.
            assert!(
                !$role.is_abstract(),
                "Cannot set an abstract ARIA role on a DOM element"
            );
        };
        $crate::aria::apply::apply_role(&mut $attrs, $role);
    }};
}

#[cfg(test)]
mod tests {
    use ars_core::{AriaAttr, AttrMap, HtmlAttr};

    use super::*;

    #[test]
    fn apply_role_sets_role_attribute() {
        let mut attrs = AttrMap::new();
        apply_role(&mut attrs, AriaRole::Button);
        assert_eq!(attrs.get(&HtmlAttr::Role), Some("button"));
    }

    #[test]
    fn apply_role_ignores_abstract_role() {
        let mut attrs = AttrMap::new();
        apply_role(&mut attrs, AriaRole::Widget);
        assert!(!attrs.contains(&HtmlAttr::Role));
    }

    #[test]
    fn apply_aria_applies_multiple_attributes() {
        let mut attrs = AttrMap::new();
        apply_aria(
            &mut attrs,
            [
                AriaAttribute::Disabled(true),
                AriaAttribute::Expanded(Some(false)),
            ],
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("false")
        );
    }

    #[test]
    fn apply_aria_empty_iterator_is_noop() {
        let mut attrs = AttrMap::new();
        apply_aria(&mut attrs, core::iter::empty());
        assert_eq!(attrs.attrs().len(), 0);
    }

    #[test]
    fn set_role_macro_with_concrete_role() {
        let mut attrs = AttrMap::new();
        set_role!(attrs, AriaRole::Checkbox);
        assert_eq!(attrs.get(&HtmlAttr::Role), Some("checkbox"));
    }
}
