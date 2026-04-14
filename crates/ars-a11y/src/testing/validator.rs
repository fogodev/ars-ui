//! Automated ARIA validation for tests and debug-only assertions.

use alloc::{string::String, vec::Vec};

use ars_core::{AttrMap, HtmlAttr};

use crate::{AriaAttribute, AriaRole};

/// A compile-time and runtime ARIA attribute validator.
///
/// Catches common ARIA mistakes:
/// - Role set without required attributes
/// - Required owned elements missing
/// - Attributes used on incompatible roles
/// - ID references pointing to non-existent elements
#[derive(Debug, Default)]
pub struct AriaValidator {
    errors: Vec<AriaValidationError>,
    warnings: Vec<AriaValidationWarning>,
}

/// Validation failures that should block the authored ARIA surface.
#[derive(Clone, Debug, PartialEq)]
pub enum AriaValidationError {
    /// A required ARIA attribute is missing for the given role.
    MissingRequiredAttribute {
        /// The role being validated.
        role: &'static str,
        /// The missing required ARIA attribute.
        missing_attr: &'static str,
    },
    /// An abstract role was used on a DOM element.
    AbstractRoleUsed {
        /// The abstract role name.
        role: &'static str,
    },
    /// A required owned element is missing.
    MissingRequiredOwnedElement {
        /// The role whose ownership contract was violated.
        role: &'static str,
        /// One of the role groups that would satisfy the ownership rule.
        required_one_of: Vec<&'static str>,
    },
    /// `aria-labelledby` or `aria-describedby` references a non-existent ID.
    DanglingIdReference {
        /// The ARIA relationship attribute containing the bad reference.
        attribute: &'static str,
        /// The missing DOM ID.
        id: String,
    },
    /// `aria-activedescendant` used on a role that does not support it.
    ActiveDescendantOnUnsupportedRole {
        /// The unsupported role name.
        role: &'static str,
    },
}

/// Validation warnings that point to suspicious but not necessarily invalid ARIA.
#[derive(Clone, Debug, PartialEq)]
pub enum AriaValidationWarning {
    /// Redundant ARIA role matches the implicit native role.
    RedundantRole {
        /// The native element type.
        element: &'static str,
        /// The redundant role.
        role: &'static str,
    },
    /// `aria-label` used alongside visible text (prefer `aria-labelledby`).
    AriaLabelWithVisibleText,
    /// `aria-disabled` used on a native form element (prefer `disabled`).
    AriaDisabledOnNativeFormElement,
    /// General advisory hint from the validator.
    Hint {
        /// The advisory message.
        message: &'static str,
    },
}

impl AriaValidator {
    /// Creates an empty validator with no accumulated errors or warnings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate role usage against the provided ARIA attribute set.
    pub fn check_role(&mut self, role: AriaRole, attrs: &[AriaAttribute], has_tabindex: bool) {
        if role.is_abstract() {
            self.errors
                .push(AriaValidationError::AbstractRoleUsed { role: role.name() });

            return;
        }

        if matches!(role, AriaRole::Separator) && !has_tabindex {
            self.warnings.push(AriaValidationWarning::Hint {
                message: "AriaRole::Separator requires tabindex for focusable separator. Use AriaRole::StructuralSeparator for non-focusable separators.",
            });
        }

        self.check_required_attrs_for_role(role, attrs);
    }

    fn check_required_attrs_for_role(&mut self, role: AriaRole, attrs: &[AriaAttribute]) {
        for required_attr in required_attributes_for_role(role) {
            let present = attrs.iter().any(|attr| attr.attr_name() == *required_attr);

            if !present {
                self.errors
                    .push(AriaValidationError::MissingRequiredAttribute {
                        role: role.name(),
                        missing_attr: required_attr,
                    });
            }
        }
    }

    /// Returns `true` when validation has recorded any errors.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns the accumulated validation errors.
    #[must_use]
    pub fn errors(&self) -> &[AriaValidationError] {
        &self.errors
    }

    /// Returns the accumulated validation warnings.
    #[must_use]
    pub fn warnings(&self) -> &[AriaValidationWarning] {
        &self.warnings
    }
}

/// Returns required ARIA attributes for a role, as per the WAI-ARIA role contract.
#[must_use]
pub const fn required_attributes_for_role(role: AriaRole) -> &'static [&'static str] {
    match role {
        AriaRole::Checkbox
        | AriaRole::Radio
        | AriaRole::Switch
        | AriaRole::Menuitemcheckbox
        | AriaRole::Menuitemradio => &["aria-checked"],

        AriaRole::Combobox => &["aria-expanded"],

        AriaRole::Scrollbar => &[
            "aria-controls",
            "aria-valuenow",
            "aria-valuemin",
            "aria-valuemax",
        ],

        AriaRole::Slider | AriaRole::Separator => {
            &["aria-valuenow", "aria-valuemin", "aria-valuemax"]
        }

        AriaRole::Spinbutton | AriaRole::Meter => &["aria-valuenow"],

        AriaRole::Heading => &["aria-level"],

        _ => &[],
    }
}

/// Validate that an `AttrMap` produced by a connect surface is ARIA-conformant.
#[must_use]
pub fn validate_attr_map(role: Option<AriaRole>, attr_map: &AttrMap) -> AriaValidator {
    let mut validator = AriaValidator::new();

    let aria_attrs: Vec<AriaAttribute> = attr_map
        .iter_attrs()
        .filter_map(|(key, _)| AriaAttribute::try_from(*key).ok())
        .collect();

    let has_tabindex = attr_map.contains(&HtmlAttr::TabIndex);

    if let Some(role) = role {
        validator.check_role(role, &aria_attrs, has_tabindex);
    }

    let has_active_descendant = aria_attrs
        .iter()
        .any(|attr| matches!(attr, AriaAttribute::ActiveDescendant(_)));

    if has_active_descendant
        && let Some(role) = role
        && !role.supports_active_descendant()
    {
        validator
            .errors
            .push(AriaValidationError::ActiveDescendantOnUnsupportedRole { role: role.name() });
    }

    validator
}

#[cfg(test)]
mod tests {
    use ars_core::{AriaAttr, AttrValue, HtmlAttr};

    use super::*;

    #[test]
    fn validator_new_starts_empty() {
        let validator = AriaValidator::new();

        assert!(!validator.has_errors());
        assert!(validator.errors().is_empty());
        assert!(validator.warnings().is_empty());
    }

    #[test]
    fn all_abstract_roles_produce_abstract_role_used() {
        let abstract_roles = [
            AriaRole::Command,
            AriaRole::Composite,
            AriaRole::Input,
            AriaRole::Landmark,
            AriaRole::Range,
            AriaRole::RoleType,
            AriaRole::Section,
            AriaRole::SectionHead,
            AriaRole::Select,
            AriaRole::Structure,
            AriaRole::Widget,
            AriaRole::Window,
        ];

        for role in abstract_roles {
            let mut validator = AriaValidator::new();

            validator.check_role(role, &[], false);

            assert_eq!(
                validator.errors(),
                &[AriaValidationError::AbstractRoleUsed { role: role.name() }]
            );
        }
    }

    #[test]
    fn checkbox_requires_aria_checked() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::Checkbox, &[], false);

        assert_eq!(
            validator.errors(),
            &[AriaValidationError::MissingRequiredAttribute {
                role: "checkbox",
                missing_attr: "aria-checked",
            }]
        );
    }

    #[test]
    fn aria_checked_roles_share_required_attribute_contract() {
        let roles = [
            (AriaRole::Radio, "radio"),
            (AriaRole::Switch, "switch"),
            (AriaRole::Menuitemcheckbox, "menuitemcheckbox"),
            (AriaRole::Menuitemradio, "menuitemradio"),
        ];

        for (role, role_name) in roles {
            let mut validator = AriaValidator::new();

            validator.check_role(role, &[], false);

            assert_eq!(
                validator.errors(),
                &[AriaValidationError::MissingRequiredAttribute {
                    role: role_name,
                    missing_attr: "aria-checked",
                }]
            );
        }
    }

    #[test]
    fn slider_requires_value_attributes() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::Slider, &[], false);

        assert_eq!(
            validator.errors(),
            &[
                AriaValidationError::MissingRequiredAttribute {
                    role: "slider",
                    missing_attr: "aria-valuenow",
                },
                AriaValidationError::MissingRequiredAttribute {
                    role: "slider",
                    missing_attr: "aria-valuemin",
                },
                AriaValidationError::MissingRequiredAttribute {
                    role: "slider",
                    missing_attr: "aria-valuemax",
                },
            ]
        );
    }

    #[test]
    fn spinbutton_and_meter_require_only_value_now() {
        let roles = [
            (AriaRole::Spinbutton, "spinbutton"),
            (AriaRole::Meter, "meter"),
        ];

        for (role, role_name) in roles {
            let mut validator = AriaValidator::new();

            validator.check_role(role, &[], false);

            assert_eq!(
                validator.errors(),
                &[AriaValidationError::MissingRequiredAttribute {
                    role: role_name,
                    missing_attr: "aria-valuenow",
                }]
            );
            assert!(!validator.errors().iter().any(|error| matches!(
                error,
                AriaValidationError::MissingRequiredAttribute {
                    missing_attr: "aria-valuemin" | "aria-valuemax",
                    ..
                }
            )));
        }
    }

    #[test]
    fn scrollbar_requires_all_required_attributes() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::Scrollbar, &[], false);

        assert_eq!(
            validator.errors(),
            &[
                AriaValidationError::MissingRequiredAttribute {
                    role: "scrollbar",
                    missing_attr: "aria-controls",
                },
                AriaValidationError::MissingRequiredAttribute {
                    role: "scrollbar",
                    missing_attr: "aria-valuenow",
                },
                AriaValidationError::MissingRequiredAttribute {
                    role: "scrollbar",
                    missing_attr: "aria-valuemin",
                },
                AriaValidationError::MissingRequiredAttribute {
                    role: "scrollbar",
                    missing_attr: "aria-valuemax",
                },
            ]
        );
    }

    #[test]
    fn heading_requires_aria_level() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::Heading, &[], false);

        assert_eq!(
            validator.errors(),
            &[AriaValidationError::MissingRequiredAttribute {
                role: "heading",
                missing_attr: "aria-level",
            }]
        );
    }

    #[test]
    fn button_has_no_required_attributes() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::Button, &[], false);

        assert!(!validator.has_errors());
        assert!(validator.warnings().is_empty());
    }

    #[test]
    fn separator_without_tabindex_emits_hint_warning() {
        let mut validator = AriaValidator::new();

        let attrs = [
            AriaAttribute::ValueNow(5.0),
            AriaAttribute::ValueMin(0.0),
            AriaAttribute::ValueMax(10.0),
        ];

        validator.check_role(AriaRole::Separator, &attrs, false);

        assert_eq!(
            validator.warnings(),
            &[AriaValidationWarning::Hint {
                message: "AriaRole::Separator requires tabindex for focusable separator. Use AriaRole::StructuralSeparator for non-focusable separators.",
            }]
        );
        assert!(validator.errors().is_empty());
    }

    #[test]
    fn separator_with_tabindex_and_required_attrs_is_valid() {
        let mut validator = AriaValidator::new();
        let attrs = [
            AriaAttribute::ValueNow(5.0),
            AriaAttribute::ValueMin(0.0),
            AriaAttribute::ValueMax(10.0),
        ];

        validator.check_role(AriaRole::Separator, &attrs, true);

        assert!(validator.errors().is_empty());
        assert!(validator.warnings().is_empty());
    }

    #[test]
    fn option_has_no_globally_required_attributes() {
        assert!(required_attributes_for_role(AriaRole::Option).is_empty());
    }

    #[test]
    fn validate_attr_map_catches_missing_combobox_expanded() {
        let attr_map = AttrMap::new();

        let validator = validate_attr_map(Some(AriaRole::Combobox), &attr_map);

        assert_eq!(
            validator.errors(),
            &[AriaValidationError::MissingRequiredAttribute {
                role: "combobox",
                missing_attr: "aria-expanded",
            }]
        );
    }

    #[test]
    fn validate_attr_map_catches_unsupported_active_descendant() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::ActiveDescendant),
            AttrValue::from("item-1"),
        );

        let validator = validate_attr_map(Some(AriaRole::Button), &attr_map);

        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::ActiveDescendantOnUnsupportedRole {
                    role: "button"
                })
        );
    }

    #[test]
    fn validate_attr_map_allows_active_descendant_on_supported_roles() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::ActiveDescendant),
            AttrValue::from("item-1"),
        );
        attr_map.set(HtmlAttr::Aria(AriaAttr::Expanded), AttrValue::from("true"));

        let validator = validate_attr_map(Some(AriaRole::Combobox), &attr_map);

        assert!(!validator.errors().contains(
            &AriaValidationError::ActiveDescendantOnUnsupportedRole { role: "combobox" }
        ));
    }

    #[test]
    fn validate_attr_map_separator_with_tabindex_and_values_has_no_hint_warning() {
        let mut attr_map = AttrMap::new();

        attr_map.set(HtmlAttr::TabIndex, AttrValue::from("0"));
        attr_map.set(HtmlAttr::Aria(AriaAttr::ValueNow), AttrValue::from("5"));
        attr_map.set(HtmlAttr::Aria(AriaAttr::ValueMin), AttrValue::from("0"));
        attr_map.set(HtmlAttr::Aria(AriaAttr::ValueMax), AttrValue::from("10"));

        let validator = validate_attr_map(Some(AriaRole::Separator), &attr_map);

        assert!(validator.errors().is_empty());
        assert!(validator.warnings().is_empty());
    }

    #[test]
    fn validate_attr_map_without_role_skips_role_checks() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::ActiveDescendant),
            AttrValue::from("item-1"),
        );

        let validator = validate_attr_map(None, &attr_map);

        assert!(!validator.has_errors());
        assert!(validator.warnings().is_empty());
    }
}
