//! Automated ARIA validation for tests and debug-only assertions.

use alloc::{string::String, vec::Vec};

use ars_core::{AriaAttr, AttrMap, HtmlAttr};

use crate::{AriaAttribute, AriaRole};

/// Additional subtree context used for ARIA validation beyond a single element's `AttrMap`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AriaValidationContext<'a> {
    /// The DOM IDs currently present in the validated subtree.
    pub known_ids: &'a [&'a str],
    /// The direct owned child roles currently present under the validated role owner.
    pub owned_roles: &'a [AriaRole],
}

impl<'a> AriaValidationContext<'a> {
    /// Creates an empty validation context with no known IDs or owned roles.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            known_ids: &[],
            owned_roles: &[],
        }
    }
}

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

    /// Validate role usage against the provided ARIA attribute set and subtree role context.
    pub fn check_role(
        &mut self,
        role: AriaRole,
        attrs: &[AriaAttribute],
        has_tabindex: bool,
        owned_roles: &[AriaRole],
    ) {
        if role.is_abstract() {
            self.errors
                .push(AriaValidationError::AbstractRoleUsed { role: role.name() });

            return;
        }

        if matches!(role, AriaRole::Separator | AriaRole::StructuralSeparator) {
            let message = match role {
                AriaRole::Separator if !has_tabindex => Some(
                    "AriaRole::Separator requires tabindex for focusable separator. Use AriaRole::StructuralSeparator for non-focusable separators.",
                ),
                AriaRole::StructuralSeparator if has_tabindex => Some(
                    "AriaRole::StructuralSeparator is the non-focusable separator role. Use AriaRole::Separator for focusable separators with tabindex.",
                ),
                _ => None,
            };

            if let Some(message) = message {
                self.warnings.push(AriaValidationWarning::Hint { message });
            }
        }

        self.check_required_attrs_for_role(role, attrs);
        self.check_required_owned_elements_for_role(role, owned_roles);
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

    fn check_required_owned_elements_for_role(&mut self, role: AriaRole, owned_roles: &[AriaRole]) {
        let required_groups = role.required_owned_elements();
        if required_groups.is_empty() {
            return;
        }

        let satisfies_required_owned_elements = required_groups.iter().any(|group| {
            group
                .iter()
                .all(|required_role| owned_roles.contains(required_role))
        });

        if satisfies_required_owned_elements {
            return;
        }

        let mut required_one_of = Vec::new();
        for group in required_groups {
            for required_role in *group {
                let role_name = required_role.name();
                if !required_one_of.contains(&role_name) {
                    required_one_of.push(role_name);
                }
            }
        }

        self.errors
            .push(AriaValidationError::MissingRequiredOwnedElement {
                role: role.name(),
                required_one_of,
            });
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

fn is_known_id(id: &str, attr_map: &AttrMap, known_ids: &[&str]) -> bool {
    attr_map.get(&HtmlAttr::Id) == Some(id) || known_ids.contains(&id)
}

fn supported_aria_attribute(attr: AriaAttr) -> Option<AriaAttribute> {
    match attr {
        #[cfg(not(feature = "aria-drag-drop-compat"))]
        AriaAttr::DropEffect | AriaAttr::Grabbed => None,
        _ => Some(AriaAttribute::from(attr)),
    }
}

const fn idref_attr_name(attr: HtmlAttr) -> Option<&'static str> {
    match attr {
        HtmlAttr::Aria(AriaAttr::ActiveDescendant) => Some("aria-activedescendant"),
        HtmlAttr::Aria(AriaAttr::Controls) => Some("aria-controls"),
        HtmlAttr::Aria(AriaAttr::DescribedBy) => Some("aria-describedby"),
        HtmlAttr::Aria(AriaAttr::Details) => Some("aria-details"),
        HtmlAttr::Aria(AriaAttr::ErrorMessage) => Some("aria-errormessage"),
        HtmlAttr::Aria(AriaAttr::FlowTo) => Some("aria-flowto"),
        HtmlAttr::Aria(AriaAttr::LabelledBy) => Some("aria-labelledby"),
        HtmlAttr::Aria(AriaAttr::Owns) => Some("aria-owns"),
        _ => None,
    }
}

/// Validate that an `AttrMap` produced by a connect surface is ARIA-conformant
/// within the provided subtree context.
#[must_use]
pub fn validate_attr_map(
    role: Option<AriaRole>,
    attr_map: &AttrMap,
    context: AriaValidationContext<'_>,
) -> AriaValidator {
    let mut validator = AriaValidator::new();

    let aria_attrs: Vec<AriaAttribute> = attr_map
        .iter_attrs()
        .filter_map(|(key, _)| match key {
            HtmlAttr::Aria(attr) => supported_aria_attribute(*attr),
            _ => None,
        })
        .collect();

    let has_tabindex = attr_map.contains(&HtmlAttr::TabIndex);

    if let Some(role) = role {
        validator.check_role(role, &aria_attrs, has_tabindex, context.owned_roles);
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

    for (attr, value) in attr_map.iter() {
        let Some(attribute) = idref_attr_name(*attr) else {
            continue;
        };

        for id in value.as_str().into_iter().flat_map(str::split_whitespace) {
            if !is_known_id(id, attr_map, context.known_ids) {
                validator
                    .errors
                    .push(AriaValidationError::DanglingIdReference {
                        attribute,
                        id: String::from(id),
                    });
            }
        }
    }

    validator
}

#[cfg(test)]
mod tests {
    use alloc::vec;

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

            validator.check_role(role, &[], false, &[]);

            assert_eq!(
                validator.errors(),
                &[AriaValidationError::AbstractRoleUsed { role: role.name() }]
            );
        }
    }

    #[test]
    fn checkbox_requires_aria_checked() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::Checkbox, &[], false, &[]);

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

            validator.check_role(role, &[], false, &[]);

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

        validator.check_role(AriaRole::Slider, &[], false, &[]);

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

            validator.check_role(role, &[], false, &[]);

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

        validator.check_role(AriaRole::Scrollbar, &[], false, &[]);

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

        validator.check_role(AriaRole::Heading, &[], false, &[]);

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

        validator.check_role(AriaRole::Button, &[], false, &[]);

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

        validator.check_role(AriaRole::Separator, &attrs, false, &[]);

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

        validator.check_role(AriaRole::Separator, &attrs, true, &[]);

        assert!(validator.errors().is_empty());
        assert!(validator.warnings().is_empty());
    }

    #[test]
    fn structural_separator_with_tabindex_emits_hint_warning() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::StructuralSeparator, &[], true, &[]);

        assert_eq!(
            validator.warnings(),
            &[AriaValidationWarning::Hint {
                message: "AriaRole::StructuralSeparator is the non-focusable separator role. Use AriaRole::Separator for focusable separators with tabindex.",
            }]
        );
        assert!(validator.errors().is_empty());
    }

    #[test]
    fn option_has_no_globally_required_attributes() {
        assert!(required_attributes_for_role(AriaRole::Option).is_empty());
    }

    #[test]
    fn validate_attr_map_catches_missing_combobox_expanded() {
        let attr_map = AttrMap::new();

        let validator = validate_attr_map(
            Some(AriaRole::Combobox),
            &attr_map,
            AriaValidationContext::new(),
        );

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

        let validator = validate_attr_map(
            Some(AriaRole::Button),
            &attr_map,
            AriaValidationContext {
                known_ids: &["item-1"],
                owned_roles: &[],
            },
        );

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

        let validator = validate_attr_map(
            Some(AriaRole::Combobox),
            &attr_map,
            AriaValidationContext {
                known_ids: &["item-1"],
                owned_roles: &[],
            },
        );

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

        let validator = validate_attr_map(
            Some(AriaRole::Separator),
            &attr_map,
            AriaValidationContext::new(),
        );

        assert!(validator.errors().is_empty());
        assert!(validator.warnings().is_empty());
    }

    #[test]
    fn validate_attr_map_structural_separator_with_tabindex_emits_hint_warning() {
        let mut attr_map = AttrMap::new();

        attr_map.set(HtmlAttr::TabIndex, AttrValue::from("0"));

        let validator = validate_attr_map(
            Some(AriaRole::StructuralSeparator),
            &attr_map,
            AriaValidationContext::new(),
        );

        assert_eq!(
            validator.warnings(),
            &[AriaValidationWarning::Hint {
                message: "AriaRole::StructuralSeparator is the non-focusable separator role. Use AriaRole::Separator for focusable separators with tabindex.",
            }]
        );
        assert!(validator.errors().is_empty());
    }

    #[test]
    fn listbox_requires_owned_option_or_group_role() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::Listbox, &[], false, &[]);

        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::MissingRequiredOwnedElement {
                    role: "listbox",
                    required_one_of: vec!["option", "group"],
                })
        );
    }

    #[test]
    fn listbox_owned_roles_satisfy_required_owned_element_contract() {
        let mut validator = AriaValidator::new();

        validator.check_role(AriaRole::Listbox, &[], false, &[AriaRole::Option]);

        assert!(
            !validator
                .errors()
                .contains(&AriaValidationError::MissingRequiredOwnedElement {
                    role: "listbox",
                    required_one_of: vec!["option", "group"],
                })
        );
    }

    #[test]
    fn validate_attr_map_catches_dangling_aria_labelledby_reference() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::LabelledBy),
            AttrValue::from("missing-id"),
        );

        let validator = validate_attr_map(
            None,
            &attr_map,
            AriaValidationContext {
                known_ids: &[],
                owned_roles: &[],
            },
        );

        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::DanglingIdReference {
                    attribute: "aria-labelledby",
                    id: String::from("missing-id"),
                })
        );
    }

    #[test]
    fn validate_attr_map_accepts_known_aria_describedby_references() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::DescribedBy),
            AttrValue::from("description-id"),
        );

        let validator = validate_attr_map(
            None,
            &attr_map,
            AriaValidationContext {
                known_ids: &["description-id"],
                owned_roles: &[],
            },
        );

        assert!(
            !validator
                .errors()
                .contains(&AriaValidationError::DanglingIdReference {
                    attribute: "aria-describedby",
                    id: String::from("description-id"),
                })
        );
    }

    #[test]
    fn validate_attr_map_catches_dangling_aria_errormessage_reference() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::ErrorMessage),
            AttrValue::from("error-id"),
        );

        let validator = validate_attr_map(None, &attr_map, AriaValidationContext::new());

        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::DanglingIdReference {
                    attribute: "aria-errormessage",
                    id: String::from("error-id"),
                })
        );
    }

    #[test]
    fn validate_attr_map_checks_all_tokens_for_single_idref_attributes() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::ErrorMessage),
            AttrValue::from("known-id missing-id"),
        );

        let validator = validate_attr_map(
            None,
            &attr_map,
            AriaValidationContext {
                known_ids: &["known-id"],
                owned_roles: &[],
            },
        );

        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::DanglingIdReference {
                    attribute: "aria-errormessage",
                    id: String::from("missing-id"),
                })
        );
    }

    #[test]
    fn validate_attr_map_catches_dangling_ids_for_all_remaining_idref_attrs() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::Controls),
            AttrValue::from("known-controls missing-controls"),
        );
        attr_map.set(
            HtmlAttr::Aria(AriaAttr::Details),
            AttrValue::from("missing-details"),
        );
        attr_map.set(
            HtmlAttr::Aria(AriaAttr::FlowTo),
            AttrValue::from("known-flow missing-flow"),
        );
        attr_map.set(
            HtmlAttr::Aria(AriaAttr::Owns),
            AttrValue::from("known-owns missing-owns"),
        );

        let validator = validate_attr_map(
            None,
            &attr_map,
            AriaValidationContext {
                known_ids: &["known-controls", "known-flow", "known-owns"],
                owned_roles: &[],
            },
        );

        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::DanglingIdReference {
                    attribute: "aria-controls",
                    id: String::from("missing-controls"),
                })
        );
        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::DanglingIdReference {
                    attribute: "aria-details",
                    id: String::from("missing-details"),
                })
        );
        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::DanglingIdReference {
                    attribute: "aria-flowto",
                    id: String::from("missing-flow"),
                })
        );
        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::DanglingIdReference {
                    attribute: "aria-owns",
                    id: String::from("missing-owns"),
                })
        );
    }

    #[test]
    fn validate_attr_map_ignores_non_string_idref_values() {
        let mut attr_map = AttrMap::new();

        attr_map.set(HtmlAttr::Aria(AriaAttr::Controls), AttrValue::None);

        let validator = validate_attr_map(None, &attr_map, AriaValidationContext::new());

        assert!(validator.errors().is_empty());
        assert!(validator.warnings().is_empty());
    }

    #[test]
    fn validate_attr_map_ignores_unsupported_drag_drop_compat_attrs() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::DropEffect),
            AttrValue::from("copy"),
        );
        attr_map.set(HtmlAttr::Aria(AriaAttr::Grabbed), AttrValue::from("true"));

        let validator = validate_attr_map(None, &attr_map, AriaValidationContext::new());

        assert!(validator.errors().is_empty());
        assert!(validator.warnings().is_empty());
    }

    #[test]
    fn validate_attr_map_still_checks_role_requirements_with_unsupported_aria_keys() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::DropEffect),
            AttrValue::from("move"),
        );

        let validator = validate_attr_map(
            Some(AriaRole::Combobox),
            &attr_map,
            AriaValidationContext::new(),
        );

        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::MissingRequiredAttribute {
                    role: "combobox",
                    missing_attr: "aria-expanded",
                })
        );
    }

    #[test]
    fn validate_attr_map_catches_missing_required_owned_roles_from_context() {
        let attr_map = AttrMap::new();

        let validator = validate_attr_map(
            Some(AriaRole::Listbox),
            &attr_map,
            AriaValidationContext::new(),
        );

        assert!(
            validator
                .errors()
                .contains(&AriaValidationError::MissingRequiredOwnedElement {
                    role: "listbox",
                    required_one_of: vec!["option", "group"],
                })
        );
    }

    #[test]
    fn validate_attr_map_without_role_skips_role_checks() {
        let mut attr_map = AttrMap::new();

        attr_map.set(
            HtmlAttr::Aria(AriaAttr::ActiveDescendant),
            AttrValue::from("item-1"),
        );

        let validator = validate_attr_map(
            None,
            &attr_map,
            AriaValidationContext {
                known_ids: &["item-1"],
                owned_roles: &[],
            },
        );

        assert!(validator.errors().is_empty());
        assert!(validator.warnings().is_empty());
    }
}
