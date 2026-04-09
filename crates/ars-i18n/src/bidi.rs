use alloc::string::String;

use unicode_segmentation::UnicodeSegmentation;

/// `BiDi` isolation direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IsolateDirection {
    /// Left-to-right isolate.
    Ltr,
    /// Right-to-left isolate.
    Rtl,
    /// First-strong isolate.
    FirstStrong,
}

impl IsolateDirection {
    const fn opening_mark(self) -> char {
        // Unicode UAX #9 §2.4 defines these isolate initiators:
        // LRI U+2066, RLI U+2067, and FSI U+2068.
        // https://unicode.org/reports/tr9/
        match self {
            Self::Ltr => '\u{2066}',
            Self::Rtl => '\u{2067}',
            Self::FirstStrong => '\u{2068}',
        }
    }
}

// Unicode UAX #9 §2.5 defines PDI U+2069 as the terminator for the
// last LRI, RLI, or FSI isolate.
// https://unicode.org/reports/tr9/
const PDI: char = '\u{2069}';

/// Wraps text in Unicode bidirectional isolation marks without splitting grapheme clusters.
#[must_use]
pub fn isolate_text_safe(text: &str, direction: IsolateDirection) -> String {
    if text.is_empty() {
        return String::new();
    }

    // Reserve space for the original UTF-8 text plus the opening isolate and
    // closing PDI markers, which are 3 bytes each in UTF-8.
    let mut out = String::with_capacity(text.len() + 6);
    out.push(direction.opening_mark());

    for cluster in text.graphemes(true) {
        out.push_str(cluster);
    }

    out.push(PDI);
    out
}
