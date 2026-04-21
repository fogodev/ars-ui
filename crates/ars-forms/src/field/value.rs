//! Form field value types.
//!
//! [`Value`] is a sum type covering every kind of value a form field can
//! hold — text, numbers, booleans, dates, times, files, and multi-select lists.
//! [`FileRef`] is an opaque reference to an uploaded file.

/// A reference to an uploaded file.
#[derive(Clone, Debug, PartialEq)]
pub struct FileRef {
    /// The original file name.
    pub name: String,

    /// The file size in bytes.
    pub size: u64,

    /// The MIME type of the file.
    pub mime_type: String,
}

/// The value of a form field.
///
/// `Value` intentionally has no `Default` impl — the correct variant
/// depends on the field type (Text vs Number vs Date, etc.).
// Note: `f64` NaN violates PartialEq reflexivity (NaN != NaN). Number fields
// SHOULD reject NaN at the validator level. If NaN reaches Value, dirty
// tracking via PartialEq will incorrectly report the field as always dirty.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A text string value.
    Text(String),

    /// An optional numeric value.
    Number(Option<f64>),

    /// A boolean value (e.g., checkbox).
    Bool(bool),

    /// Multiple text values (e.g., multi-select as strings).
    MultipleText(Vec<String>),

    /// File references from a file-upload field.
    File(Vec<FileRef>),

    /// An optional calendar date.
    Date(Option<ars_i18n::CalendarDate>),

    /// An optional time of day.
    Time(Option<ars_i18n::Time>),

    /// An optional date range (start and end).
    DateRange(Option<ars_i18n::DateRange>),
}

impl Value {
    /// Extracts the text value, if this is a [`Text`](Value::Text) variant.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Value::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Extracts the numeric value, if this is a [`Number`](Value::Number)
    /// variant containing `Some`.
    #[must_use]
    pub const fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => *n,
            _ => None,
        }
    }

    /// Extracts the boolean value, if this is a [`Bool`](Value::Bool) variant.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Converts the value to a string suitable for validation.
    ///
    /// Each variant is serialized to its canonical string form:
    /// - `Text` → the string itself
    /// - `Number(Some(n))` → `n.to_string()`
    /// - `Bool(b)` → `"true"` or `"false"`
    /// - `MultipleText` → comma-joined
    /// - `File` → count as string
    /// - `Date/Time/DateRange` → ISO 8601
    #[must_use]
    pub fn to_string_for_validation(&self) -> String {
        match self {
            Value::Text(s) => s.clone(),
            Value::Number(Some(n)) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::MultipleText(v) => v.join(","),
            Value::File(v) => v.len().to_string(),
            Value::Date(Some(d)) => d.to_iso8601(),
            Value::Time(Some(t)) => t.to_iso8601(),
            Value::DateRange(Some(r)) => r.to_iso8601(),
            Value::Number(None)
            | Value::Date(None)
            | Value::Time(None)
            | Value::DateRange(None) => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_text_extracts_text() {
        let v = Value::Text("hello".to_string());

        assert_eq!(v.as_text(), Some("hello"));
    }

    #[test]
    fn as_text_none_for_other_variants() {
        assert_eq!(Value::Bool(true).as_text(), None);
        assert_eq!(Value::Number(Some(1.0)).as_text(), None);
    }

    #[test]
    fn as_number_extracts_some() {
        let v = Value::Number(Some(42.0));

        assert_eq!(v.as_number(), Some(42.0));
    }

    #[test]
    fn as_number_none_for_empty() {
        assert_eq!(Value::Number(None).as_number(), None);
    }

    #[test]
    fn as_number_none_for_other_variants() {
        assert_eq!(Value::Text("x".to_string()).as_number(), None);
    }

    #[test]
    fn as_bool_extracts_bool() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Bool(false).as_bool(), Some(false));
    }

    #[test]
    fn as_bool_none_for_other_variants() {
        assert_eq!(Value::Number(Some(1.0)).as_bool(), None);
    }

    #[test]
    fn to_string_for_validation_text() {
        let v = Value::Text("hello".to_string());

        assert_eq!(v.to_string_for_validation(), "hello");
    }

    #[test]
    fn to_string_for_validation_number_some() {
        let v = Value::Number(Some(42.0));

        assert_eq!(v.to_string_for_validation(), "42");
    }

    #[test]
    fn to_string_for_validation_number_none() {
        let v = Value::Number(None);

        assert_eq!(v.to_string_for_validation(), "");
    }

    #[test]
    fn to_string_for_validation_bool() {
        assert_eq!(Value::Bool(true).to_string_for_validation(), "true");
        assert_eq!(Value::Bool(false).to_string_for_validation(), "false");
    }

    #[test]
    fn to_string_for_validation_multiple_text() {
        let v = Value::MultipleText(vec!["a".to_string(), "b".to_string()]);

        assert_eq!(v.to_string_for_validation(), "a,b");
    }

    #[test]
    fn to_string_for_validation_file_count() {
        let v = Value::File(vec![
            FileRef {
                name: "a.txt".to_string(),
                size: 100,
                mime_type: "text/plain".to_string(),
            },
            FileRef {
                name: "b.txt".to_string(),
                size: 200,
                mime_type: "text/plain".to_string(),
            },
        ]);

        assert_eq!(v.to_string_for_validation(), "2");
    }

    #[test]
    fn to_string_for_validation_date_none() {
        assert_eq!(Value::Date(None).to_string_for_validation(), "");
        assert_eq!(Value::Time(None).to_string_for_validation(), "");
        assert_eq!(Value::DateRange(None).to_string_for_validation(), "");
    }

    #[test]
    fn to_string_for_validation_temporal_values() {
        let date = ars_i18n::CalendarDate::new_gregorian(2024, 3, 15)
            .expect("Gregorian fixture should validate");

        let time = ars_i18n::Time::new(9, 30, 45, 125).expect("time fixture should validate");

        let end = ars_i18n::CalendarDate::new_gregorian(2024, 3, 20)
            .expect("Gregorian fixture should validate");

        let range = ars_i18n::DateRange::new(date.clone(), end).expect("ordered range");

        assert_eq!(
            Value::Date(Some(date)).to_string_for_validation(),
            "2024-03-15"
        );
        assert_eq!(
            Value::Time(Some(time)).to_string_for_validation(),
            "09:30:45.125"
        );
        assert_eq!(
            Value::DateRange(Some(range)).to_string_for_validation(),
            "2024-03-15/2024-03-20"
        );
    }
}
