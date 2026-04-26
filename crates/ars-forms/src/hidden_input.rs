//! Hidden input helpers for native form submission.
//!
//! Complex components (Select, `DatePicker`, etc.) must render hidden `<input>`
//! elements to participate in native HTML form submission. This module provides
//! [`Config`], [`Value`], and helper functions that produce [`AttrMap`]s
//! matching the component attribute system.

use alloc::{string::String, vec::Vec};

use ars_core::{AttrMap, HtmlAttr};

/// Configuration for a hidden input that submits with native forms.
#[derive(Clone, Debug)]
pub struct Config {
    /// The `name` attribute for the hidden input.
    pub name: String,

    /// The value to submit.
    pub value: Value,

    /// Optional `form` attribute for cross-form association.
    pub form_id: Option<String>,

    /// Whether the hidden input is disabled (excluded from submission).
    pub disabled: bool,
}

/// The value of a hidden input.
#[derive(Clone, Debug)]
pub enum Value {
    /// Single value.
    Single(String),

    /// Multiple values (rendered as multiple hidden inputs with the same name).
    Multiple(Vec<String>),

    /// No value (omitted from submission).
    None,
}

/// Builds the common attributes shared by single and multi hidden inputs.
fn base_attrs(config: &Config, value: &str) -> AttrMap {
    let mut map = AttrMap::new();

    map.set(HtmlAttr::Type, "hidden")
        .set(HtmlAttr::Name, &config.name)
        .set(HtmlAttr::Value, value);

    if config.disabled {
        map.set(HtmlAttr::Disabled, true);
    }

    if let Some(ref form_id) = config.form_id {
        map.set(HtmlAttr::Form, form_id);
    }

    map
}

/// Generate an [`AttrMap`] for a single hidden input.
///
/// Components render this as:
/// `<input type="hidden" name="{name}" value="{value}" />`
///
/// Returns `None` for [`Value::None`] (the element should not be rendered).
/// Panics in debug mode if called with [`Value::Multiple`] — use
/// [`multi_attrs()`] instead.
#[must_use]
pub fn attrs(config: &Config) -> Option<AttrMap> {
    match &config.value {
        Value::Single(v) => Some(base_attrs(config, v)),

        Value::Multiple(_) => {
            debug_assert!(false, "Use multi_attrs for Value::Multiple");
            None
        }

        Value::None => None,
    }
}

/// For multi-select: returns one [`AttrMap`] per value.
///
/// Propagates `form_id` and `disabled` from config, matching [`attrs()`].
#[must_use]
pub fn multi_attrs(config: &Config, values: &[String]) -> Vec<AttrMap> {
    values.iter().map(|v| base_attrs(config, v)).collect()
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};

    use ars_core::AttrValue;

    use super::*;

    /// Helper to extract a string attribute value from an `AttrMap`.
    fn get_str(map: &AttrMap, attr: HtmlAttr) -> Option<&str> {
        map.attrs()
            .iter()
            .find(|&(k, _)| *k == attr)
            .and_then(|(_, v)| v.as_str())
    }

    /// Helper to check whether a boolean attribute is present.
    fn has_bool(map: &AttrMap, attr: HtmlAttr) -> bool {
        map.attrs()
            .iter()
            .any(|(k, v)| *k == attr && matches!(v, AttrValue::Bool(true)))
    }

    #[test]
    fn single_value_attrs() {
        let config = Config {
            name: "country".to_string(),
            value: Value::Single("us".to_string()),
            form_id: None,
            disabled: false,
        };

        let map = attrs(&config).expect("should produce attrs");

        assert_eq!(get_str(&map, HtmlAttr::Type), Some("hidden"));
        assert_eq!(get_str(&map, HtmlAttr::Name), Some("country"));
        assert_eq!(get_str(&map, HtmlAttr::Value), Some("us"));
    }

    #[test]
    fn none_value_returns_none() {
        let config = Config {
            name: "field".to_string(),
            value: Value::None,
            form_id: None,
            disabled: false,
        };

        assert!(attrs(&config).is_none());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "Use multi_attrs for Value::Multiple")]
    fn multiple_value_panics_in_debug() {
        let config = Config {
            name: "field".to_string(),
            value: Value::Multiple(vec!["a".to_string(), "b".to_string()]),
            form_id: None,
            disabled: false,
        };

        drop(attrs(&config));
    }

    #[test]
    fn disabled_attr_included() {
        let config = Config {
            name: "field".to_string(),
            value: Value::Single("val".to_string()),
            form_id: None,
            disabled: true,
        };

        let map = attrs(&config).expect("should produce attrs");

        assert!(has_bool(&map, HtmlAttr::Disabled));
    }

    #[test]
    fn form_id_attr_included() {
        let config = Config {
            name: "field".to_string(),
            value: Value::Single("val".to_string()),
            form_id: Some("form-1".to_string()),
            disabled: false,
        };

        let map = attrs(&config).expect("should produce attrs");

        assert_eq!(get_str(&map, HtmlAttr::Form), Some("form-1"));
    }

    #[test]
    fn multi_produces_per_value_attrs() {
        let config = Config {
            name: "tags".to_string(),
            value: Value::Multiple(vec![]),
            form_id: None,
            disabled: false,
        };

        let values = vec!["a".to_string(), "b".to_string(), "c".to_string()];

        let result = multi_attrs(&config, &values);

        assert_eq!(result.len(), 3);

        for (i, map) in result.iter().enumerate() {
            assert_eq!(get_str(map, HtmlAttr::Type), Some("hidden"));
            assert_eq!(get_str(map, HtmlAttr::Name), Some("tags"));
            assert_eq!(get_str(map, HtmlAttr::Value), Some(values[i].as_str()));
        }
    }

    #[test]
    fn multi_propagates_form_and_disabled() {
        let config = Config {
            name: "tags".to_string(),
            value: Value::Multiple(vec![]),
            form_id: Some("myform".to_string()),
            disabled: true,
        };

        let values = vec!["x".to_string()];

        let result = multi_attrs(&config, &values);

        assert_eq!(result.len(), 1);

        let map = &result[0];

        assert!(has_bool(map, HtmlAttr::Disabled));
        assert_eq!(get_str(map, HtmlAttr::Form), Some("myform"));
    }
}
