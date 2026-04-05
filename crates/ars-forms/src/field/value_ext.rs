//! Extension traits for field value emptiness checks.
//!
//! [`ValueExt`] provides an `is_empty()` method for [`Value`] and
//! related types. [`SelectionExt`] and [`CheckboxExt`] are trait
//! definitions for component-specific emptiness semantics — their
//! implementations live in the respective component crates.

use super::value::Value;

/// Utility predicates for form field values.
pub trait ValueExt {
    /// Returns `true` if the field has no meaningful value.
    fn is_empty(&self) -> bool;
}

/// Trait for types that expose a set of selected keys (e.g., `selection::State`).
///
/// Implemented by the selection module in its own crate to avoid a dependency
/// cycle where ars-forms would import concrete types from component crates.
pub trait SelectionExt {
    /// Returns `true` if at least one key is selected.
    fn is_any_selected(&self) -> bool;
    /// Returns `true` if all items are selected.
    fn is_all_selected(&self, total_items: usize) -> bool;
}

/// Trait for tri-state toggle types (e.g., `CheckboxState`).
///
/// Implemented by the checkbox module in its own crate to avoid a dependency
/// cycle where ars-forms would import concrete types from component crates.
pub trait CheckboxExt {
    /// Returns `true` if the state is indeterminate.
    fn is_indeterminate(&self) -> bool;
    /// Returns `true` if the state is checked or indeterminate.
    fn is_checked_or_indeterminate(&self) -> bool;
}

// Primary Value type — delegates to variant-specific emptiness.
impl ValueExt for Value {
    fn is_empty(&self) -> bool {
        match self {
            // Note: uses raw is_empty() (no trim), unlike RequiredValidator which trims.
            // This is intentional: is_empty() is a raw structural check (e.g., for "clear"
            // button visibility), while RequiredValidator applies semantic trimming.
            Value::Text(s) => s.is_empty(),
            Value::Number(n) => n.is_none(),
            Value::Bool(b) => !b,
            Value::Date(d) => d.is_none(),
            Value::Time(t) => t.is_none(),
            Value::DateRange(r) => r.is_none(),
            Value::File(f) => f.is_empty(),
            Value::MultipleText(l) => l.is_empty(),
        }
    }
}

// DateField-specific utilities (on Option<CalendarDate> from ars-i18n).
impl ValueExt for Option<ars_i18n::CalendarDate> {
    fn is_empty(&self) -> bool {
        self.is_none()
    }
}

// NumberInput-specific utilities.
impl ValueExt for Option<f64> {
    fn is_empty(&self) -> bool {
        self.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_empty() {
        assert!(Value::Text(String::new()).is_empty());
    }

    #[test]
    fn text_non_empty() {
        assert!(!Value::Text("x".to_string()).is_empty());
    }

    #[test]
    fn number_none_empty() {
        assert!(Value::Number(None).is_empty());
    }

    #[test]
    fn number_some_not_empty() {
        assert!(!Value::Number(Some(1.0)).is_empty());
    }

    #[test]
    fn bool_false_empty() {
        assert!(Value::Bool(false).is_empty());
    }

    #[test]
    fn bool_true_not_empty() {
        assert!(!Value::Bool(true).is_empty());
    }

    #[test]
    fn multiple_text_empty() {
        assert!(Value::MultipleText(vec![]).is_empty());
    }

    #[test]
    fn file_empty() {
        assert!(Value::File(vec![]).is_empty());
    }

    #[test]
    fn calendar_date_none_empty() {
        let date: Option<ars_i18n::CalendarDate> = None;
        assert!(date.is_empty());
    }

    #[test]
    fn option_f64_none_empty() {
        let n: Option<f64> = None;
        assert!(n.is_empty());
    }

    #[test]
    fn option_f64_some_not_empty() {
        let n: Option<f64> = Some(42.0);
        assert!(!n.is_empty());
    }
}
