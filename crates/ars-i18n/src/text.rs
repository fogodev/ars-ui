//! Unicode text-boundary helpers.

use alloc::string::String;

use unicode_segmentation::UnicodeSegmentation as _;

/// Counts extended grapheme clusters in a string.
///
/// This matches user-perceived character boundaries for composed characters,
/// combining marks, emoji modifier sequences, and zero-width-joiner emoji.
#[must_use]
pub fn grapheme_count(value: &str) -> usize {
    value.graphemes(true).count()
}

/// Returns the first `count` extended grapheme clusters from a string.
///
/// This preserves user-perceived character boundaries for composed characters,
/// combining marks, emoji modifier sequences, and zero-width-joiner emoji.
#[must_use]
pub fn take_graphemes(value: &str, count: usize) -> String {
    value.graphemes(true).take(count).collect()
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

    #[test]
    fn take_graphemes_preserves_user_visible_boundaries() {
        assert_eq!(take_graphemes("e\u{301}cole", 1), "e\u{301}");
        assert_eq!(
            take_graphemes("👨\u{200d}👩\u{200d}👧 family", 1),
            "👨\u{200d}👩\u{200d}👧"
        );
        assert_eq!(take_graphemes("張偉明", 2), "張偉");
    }
}
