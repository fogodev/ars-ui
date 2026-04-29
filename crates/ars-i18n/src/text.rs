//! Unicode text-boundary helpers.

use unicode_segmentation::UnicodeSegmentation as _;

/// Counts extended grapheme clusters in a string.
///
/// This matches user-perceived character boundaries for composed characters,
/// combining marks, emoji modifier sequences, and zero-width-joiner emoji.
#[must_use]
pub fn grapheme_count(value: &str) -> usize {
    value.graphemes(true).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grapheme_count_treats_composed_text_as_user_visible_characters() {
        assert_eq!(grapheme_count("cafe"), 4);
        assert_eq!(grapheme_count("e\u{301}"), 1);
        assert_eq!(grapheme_count("👨\u{200d}👩\u{200d}👧"), 1);
    }
}
