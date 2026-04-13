//! Label, description, and field accessibility helpers for form controls.

use alloc::{string::String, vec::Vec};

use ars_core::{AriaAttr, AttrMap, AttrValue, ComponentIds, HtmlAttr};

use crate::{
    aria::attribute::{AriaAttribute, AriaIdList, AriaIdRef, AriaInvalid, AriaLive},
    set_disabled, set_invalid,
};

/// Resolves the accessible name for a form element from multiple possible sources.
///
/// Priority (per accname-1.2 spec):
/// 1. `aria-labelledby` referencing visible text
/// 2. `aria-label` (string)
/// 3. `<label for="...">` association
/// 4. `title` attribute
/// 5. `placeholder` attribute (last resort; discouraged)
#[derive(Clone, Debug, Default)]
pub struct LabelConfig {
    /// IDs of elements that label this element via `aria-labelledby`.
    pub labelledby_ids: Vec<String>,
    /// Inline string label applied through `aria-label`.
    pub label: Option<String>,
    /// ID of a `<label>` element associated with this input.
    pub html_for_id: Option<String>,
}

impl LabelConfig {
    /// Applies the highest-priority available accessible-name attributes.
    pub fn apply_to(&self, attrs: &mut AttrMap) {
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), AttrValue::None);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), AttrValue::None);

        if !self.labelledby_ids.is_empty() {
            AriaAttribute::LabelledBy(AriaIdList(self.labelledby_ids.clone())).apply_to(attrs);
        } else if let Some(ref label) = self.label {
            AriaAttribute::Label(label.clone()).apply_to(attrs);
        }
        // html_for_id is handled by the <label> element itself via `for` attribute.
    }
}

/// Associates a description with an element.
///
/// Multiple description sources can be combined because `aria-describedby`
/// accepts a space-separated list of IDs.
#[derive(Clone, Debug, Default)]
pub struct DescriptionConfig {
    /// IDs of elements describing this element.
    pub describedby_ids: Vec<String>,
    /// Additional details element ID exposed through `aria-details`.
    pub details_id: Option<String>,
}

impl DescriptionConfig {
    /// Applies description-related attributes to the given attribute map.
    pub fn apply_to(&self, attrs: &mut AttrMap) {
        attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), AttrValue::None);
        attrs.set(HtmlAttr::Aria(AriaAttr::Details), AttrValue::None);

        if !self.describedby_ids.is_empty() {
            AriaAttribute::DescribedBy(AriaIdList(self.describedby_ids.clone())).apply_to(attrs);
        }
        if let Some(ref id) = self.details_id {
            AriaAttribute::Details(AriaIdRef(id.clone())).apply_to(attrs);
        }
    }
}

/// Shared form-field accessibility state for labels, descriptions, and errors.
///
/// Components such as text fields, selects, comboboxes, and sliders use this
/// context to derive the input element's accessible name, descriptions, and
/// validation wiring from a single set of IDs and state flags.
#[derive(Clone, Debug)]
pub struct FieldContext {
    /// Stable component IDs used to derive element relationships.
    pub ids: ComponentIds,
    /// Accessible-name configuration for the input element.
    pub label: LabelConfig,
    /// Description and details configuration for the input element.
    pub description: DescriptionConfig,
    /// Whether the field is required.
    pub is_required: bool,
    /// Whether the field is readonly.
    pub is_readonly: bool,
    /// Whether the field is disabled.
    pub is_disabled: bool,
    /// Validation state exposed through ARIA.
    pub invalid: AriaInvalid,
}

impl FieldContext {
    /// Creates a field context with default accessibility state for the given IDs.
    #[must_use]
    pub fn new(ids: ComponentIds) -> Self {
        Self {
            ids,
            label: LabelConfig::default(),
            description: DescriptionConfig::default(),
            is_required: false,
            is_readonly: false,
            is_disabled: false,
            invalid: AriaInvalid::False,
        }
    }
}

impl FieldContext {
    /// Applies the input element's label, description, and validation attributes.
    ///
    /// `aria-describedby` follows the spec priority order: error message first,
    /// then the configured description IDs.
    pub fn apply_input_attrs(&self, attrs: &mut AttrMap) {
        self.label.apply_to(attrs);

        let error_id = if self.invalid == AriaInvalid::False {
            None
        } else {
            Some(self.ids.part("error-message"))
        };

        let description_ids = error_id
            .into_iter()
            .chain(self.description.describedby_ids.iter().cloned())
            .collect::<Vec<_>>();

        if description_ids.is_empty() {
            attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), AttrValue::None);
        } else {
            AriaAttribute::DescribedBy(AriaIdList(description_ids)).apply_to(attrs);
        }

        if let Some(ref id) = self.description.details_id {
            AriaAttribute::Details(AriaIdRef(id.clone())).apply_to(attrs);
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Details), AttrValue::None);
        }

        if self.is_required {
            AriaAttribute::Required(true).apply_to(attrs);
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::Required), AttrValue::None);
        }

        if self.is_readonly {
            AriaAttribute::ReadOnly(true).apply_to(attrs);
        } else {
            attrs.set(HtmlAttr::Aria(AriaAttr::ReadOnly), AttrValue::None);
        }

        set_disabled(attrs, self.is_disabled);

        if self.invalid == AriaInvalid::False {
            set_invalid(attrs, AriaInvalid::False, None);
        } else {
            set_invalid(attrs, self.invalid, Some(&self.ids.part("error-message")));
        }
    }

    /// Returns attributes for the visible `<label>` element associated with the input.
    #[must_use]
    pub fn label_element_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ids.part("label"));
        attrs.set(
            HtmlAttr::For,
            self.label
                .html_for_id
                .clone()
                .unwrap_or_else(|| self.ids.part("input")),
        );
        attrs
    }

    /// Returns attributes for the field description element.
    #[must_use]
    pub fn description_element_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ids.part("description"));
        attrs
    }

    /// Returns attributes for the field error-message element.
    ///
    /// The error region always uses polite live-region semantics and is hidden
    /// from assistive technology when no error is currently visible.
    #[must_use]
    pub fn error_message_attrs(&self, is_visible: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Id, self.ids.part("error-message"));
        AriaAttribute::Live(AriaLive::Polite).apply_to(&mut attrs);
        AriaAttribute::Atomic(true).apply_to(&mut attrs);
        if !is_visible {
            AriaAttribute::Hidden(Some(true)).apply_to(&mut attrs);
        }
        attrs
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use ars_core::{AriaAttr, HtmlAttr};

    use super::*;

    fn test_ids() -> ComponentIds {
        ComponentIds::from_id("field-1")
    }

    #[test]
    fn label_config_applies_aria_labelledby() {
        let config = LabelConfig {
            labelledby_ids: vec![String::from("label-a"), String::from("label-b")],
            label: Some(String::from("Fallback")),
            html_for_id: Some(String::from("field-1-input")),
        };

        let mut attrs = AttrMap::new();

        config.apply_to(&mut attrs);

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("label-a label-b")
        );
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Label)));
    }

    #[test]
    fn label_config_applies_aria_label_when_no_labelledby_ids() {
        let config = LabelConfig {
            labelledby_ids: Vec::new(),
            label: Some(String::from("Name")),
            html_for_id: None,
        };

        let mut attrs = AttrMap::new();

        config.apply_to(&mut attrs);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Name"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::LabelledBy)));
    }

    #[test]
    fn label_config_prefers_labelledby_over_label() {
        let config = LabelConfig {
            labelledby_ids: vec![String::from("visible-label")],
            label: Some(String::from("Fallback")),
            html_for_id: None,
        };

        let mut attrs = AttrMap::new();

        config.apply_to(&mut attrs);

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("visible-label")
        );
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Label)));
    }

    #[test]
    fn label_config_empty_labelledby_ids_falls_through_to_label() {
        let config = LabelConfig {
            labelledby_ids: Vec::new(),
            label: Some(String::from("Email")),
            html_for_id: Some(String::from("field-1-input")),
        };

        let mut attrs = AttrMap::new();

        config.apply_to(&mut attrs);

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Label)), Some("Email"));
    }

    #[test]
    fn label_config_default_applies_no_label_attrs() {
        let config = LabelConfig::default();

        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Aria(AriaAttr::Label), "stale-label");
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), "stale-id");

        config.apply_to(&mut attrs);

        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Label)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::LabelledBy)));
    }

    #[test]
    fn description_config_applies_describedby_and_details() {
        let config = DescriptionConfig {
            describedby_ids: vec![String::from("help"), String::from("hint")],
            details_id: Some(String::from("details")),
        };

        let mut attrs = AttrMap::new();

        config.apply_to(&mut attrs);

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("help hint")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Details)),
            Some("details")
        );
    }

    #[test]
    fn description_config_default_applies_no_description_attrs() {
        let config = DescriptionConfig::default();

        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Aria(AriaAttr::DescribedBy), "stale-help");
        attrs.set(HtmlAttr::Aria(AriaAttr::Details), "stale-details");

        config.apply_to(&mut attrs);

        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::DescribedBy)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Details)));
    }

    #[test]
    fn field_context_new_uses_spec_defaults() {
        let context = FieldContext::new(test_ids());

        assert_eq!(context.ids.id(), "field-1");
        assert!(context.label.labelledby_ids.is_empty());
        assert_eq!(context.label.label, None);
        assert_eq!(context.label.html_for_id, None);
        assert!(context.description.describedby_ids.is_empty());
        assert_eq!(context.description.details_id, None);
        assert!(!context.is_required);
        assert!(!context.is_readonly);
        assert!(!context.is_disabled);
        assert_eq!(context.invalid, AriaInvalid::False);
    }

    #[test]
    fn apply_input_attrs_applies_label_description_and_state_attrs() {
        let context = FieldContext {
            ids: test_ids(),
            label: LabelConfig {
                labelledby_ids: vec![String::from("field-1-label")],
                label: None,
                html_for_id: None,
            },
            description: DescriptionConfig {
                describedby_ids: vec![String::from("field-1-description")],
                details_id: Some(String::from("field-1-details")),
            },
            is_required: true,
            is_readonly: true,
            is_disabled: true,
            invalid: AriaInvalid::True,
        };

        let mut attrs = AttrMap::new();

        context.apply_input_attrs(&mut attrs);

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::LabelledBy)),
            Some("field-1-label")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("field-1-error-message field-1-description")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Details)),
            Some("field-1-details")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Required)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ReadOnly)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ErrorMessage)),
            Some("field-1-error-message")
        );
    }

    #[test]
    fn apply_input_attrs_prepends_error_message_id_when_invalid() {
        let context = FieldContext {
            ids: test_ids(),
            label: LabelConfig::default(),
            description: DescriptionConfig {
                describedby_ids: vec![String::from("help"), String::from("hint")],
                details_id: None,
            },
            is_required: false,
            is_readonly: false,
            is_disabled: false,
            invalid: AriaInvalid::Grammar,
        };

        let mut attrs = AttrMap::new();

        context.apply_input_attrs(&mut attrs);

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("field-1-error-message help hint")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)),
            Some("grammar")
        );
    }

    #[test]
    fn apply_input_attrs_does_not_add_error_message_id_when_valid() {
        let context = FieldContext {
            ids: test_ids(),
            label: LabelConfig::default(),
            description: DescriptionConfig {
                describedby_ids: vec![String::from("help")],
                details_id: None,
            },
            is_required: false,
            is_readonly: false,
            is_disabled: false,
            invalid: AriaInvalid::False,
        };

        let mut attrs = AttrMap::new();

        context.apply_input_attrs(&mut attrs);

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("help")
        );
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("false"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::ErrorMessage)));
    }

    #[test]
    fn apply_input_attrs_with_no_description_or_details_leaves_attrs_absent() {
        let context = FieldContext {
            ids: test_ids(),
            label: LabelConfig::default(),
            description: DescriptionConfig::default(),
            is_required: false,
            is_readonly: false,
            is_disabled: false,
            invalid: AriaInvalid::False,
        };

        let mut attrs = AttrMap::new();

        context.apply_input_attrs(&mut attrs);

        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::DescribedBy)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Details)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Required)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::ReadOnly)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Disabled)));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("false"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::ErrorMessage)));
    }

    #[test]
    fn apply_input_attrs_clears_stale_managed_attrs_when_reused() {
        let invalid_context = FieldContext {
            ids: test_ids(),
            label: LabelConfig {
                labelledby_ids: vec![String::from("field-1-label")],
                label: None,
                html_for_id: None,
            },
            description: DescriptionConfig {
                describedby_ids: vec![String::from("field-1-description")],
                details_id: Some(String::from("field-1-details")),
            },
            is_required: true,
            is_readonly: true,
            is_disabled: false,
            invalid: AriaInvalid::True,
        };

        let valid_context = FieldContext::new(test_ids());

        let mut attrs = AttrMap::new();

        invalid_context.apply_input_attrs(&mut attrs);
        valid_context.apply_input_attrs(&mut attrs);

        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Label)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::LabelledBy)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::DescribedBy)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Details)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Required)));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::ReadOnly)));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("false"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::ErrorMessage)));
    }

    #[test]
    fn label_element_attrs_match_spec_ids() {
        let context = FieldContext::new(test_ids());

        let attrs = context.label_element_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("field-1-label"));
        assert_eq!(attrs.get(&HtmlAttr::For), Some("field-1-input"));
    }

    #[test]
    fn label_element_attrs_prefers_custom_html_for_id() {
        let mut context = FieldContext::new(test_ids());

        context.label.html_for_id = Some(String::from("custom-input"));

        let attrs = context.label_element_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("field-1-label"));
        assert_eq!(attrs.get(&HtmlAttr::For), Some("custom-input"));
    }

    #[test]
    fn description_element_attrs_match_spec_ids() {
        let context = FieldContext::new(test_ids());

        let attrs = context.description_element_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("field-1-description"));
    }

    #[test]
    fn error_message_attrs_visible_has_live_region_without_hidden() {
        let context = FieldContext::new(test_ids());

        let attrs = context.error_message_attrs(true);

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("field-1-error-message"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Hidden)));
    }

    #[test]
    fn error_message_attrs_hidden_adds_aria_hidden() {
        let context = FieldContext::new(test_ids());

        let attrs = context.error_message_attrs(false);

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("field-1-error-message"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Live)), Some("polite"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Atomic)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), Some("true"));
    }
}
