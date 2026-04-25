//! ID extraction helpers for SSR / hydration tests.
//!
//! During SSR hydration, the server and client must emit identical IDs on
//! every element that participates in ARIA relationships — otherwise
//! `aria-labelledby`, `aria-controls`, `aria-describedby`, and `<label for="…">`
//! references break and the hydration pass warns. The [`extract_all_ids`]
//! helper collects every ID-bearing attribute value from a set of
//! [`AttrMap`]s so tests can diff server output against client output.
//!
//! The canonical usage pattern is documented in
//! `spec/testing/07-ssr-hydration.md` §4.
//!
//! # Example
//!
//! ```no_run
//! # use ars_core::{AriaAttr, AttrMap, AttrValue, HtmlAttr};
//! # use ars_a11y::testing::extract_all_ids;
//! let mut root = AttrMap::new();
//! root.set(HtmlAttr::Id, AttrValue::String("dialog-1".into()));
//! root.set(
//!     HtmlAttr::Aria(AriaAttr::LabelledBy),
//!     AttrValue::String("dialog-1-title".into()),
//! );
//!
//! let ids = extract_all_ids(&[root]);
//! assert!(ids.contains("dialog-1"));
//! assert!(ids.contains("dialog-1-title"));
//! ```

use alloc::{
    collections::BTreeSet,
    string::{String, ToString},
};

use ars_core::{AriaAttr, AttrMap, HtmlAttr};

/// Attributes whose values contribute IDs to the hydration cross-check.
///
/// Each token inside the attribute value is treated as an ID. For
/// single-valued attributes (`id`, `for`) there is only one token; for
/// space-separated attributes (`aria-labelledby`, `aria-controls`,
/// `aria-describedby`) each whitespace-separated token counts as its own ID.
const ID_ATTRS: &[HtmlAttr] = &[
    HtmlAttr::Id,
    HtmlAttr::For,
    HtmlAttr::Aria(AriaAttr::LabelledBy),
    HtmlAttr::Aria(AriaAttr::Controls),
    HtmlAttr::Aria(AriaAttr::DescribedBy),
];

/// Extract every ID referenced by the supplied [`AttrMap`]s.
///
/// Walks each map looking at a fixed set of ID-bearing attributes
/// (`id`, `for`, `aria-labelledby`, `aria-controls`, `aria-describedby`),
/// splits space-separated values into individual tokens, and returns the
/// sorted, deduplicated set of IDs.
///
/// SSR hydration tests compare this set between server and client renders;
/// any difference in content or cardinality indicates a hydration-breaking
/// ID mismatch.
#[must_use]
pub fn extract_all_ids(attr_maps: &[AttrMap]) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();

    for attrs in attr_maps {
        for attr in ID_ATTRS {
            if let Some(value) = attrs.get(attr) {
                for token in value.split_whitespace() {
                    ids.insert(token.to_string());
                }
            }
        }
    }

    ids
}

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::string::ToString;

    use ars_core::{AriaAttr, AttrMap, AttrValue, HtmlAttr};

    use super::extract_all_ids;

    fn attr_map_with(attrs: &[(HtmlAttr, &str)]) -> AttrMap {
        let mut map = AttrMap::new();

        for (attr, value) in attrs {
            map.set(*attr, AttrValue::String((*value).to_string()));
        }

        map
    }

    #[test]
    fn extracts_id_and_for_and_aria_references() {
        let root = attr_map_with(&[
            (HtmlAttr::Id, "dialog-1"),
            (
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                "dialog-1-title dialog-1-subtitle",
            ),
            (
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                "dialog-1-description",
            ),
        ]);

        let label = attr_map_with(&[(HtmlAttr::For, "dialog-1-input")]);

        let ids = extract_all_ids(&[root, label]);

        assert!(ids.contains("dialog-1"));
        assert!(ids.contains("dialog-1-title"));
        assert!(ids.contains("dialog-1-subtitle"));
        assert!(ids.contains("dialog-1-description"));
        assert!(ids.contains("dialog-1-input"));
        assert_eq!(ids.len(), 5);
    }

    #[test]
    fn empty_input_returns_empty_set() {
        let ids = extract_all_ids(&[]);

        assert!(ids.is_empty());
    }

    #[test]
    fn non_id_attributes_are_ignored() {
        let root = attr_map_with(&[
            (HtmlAttr::Role, "dialog"),
            (HtmlAttr::Aria(AriaAttr::Label), "Confirm"),
        ]);

        let ids = extract_all_ids(&[root]);

        assert!(ids.is_empty());
    }

    #[test]
    fn whitespace_is_canonicalized_when_splitting_tokens() {
        // Space-separated attributes may be separated by multiple whitespace
        // characters; split_whitespace handles this correctly.
        let root = attr_map_with(&[(
            HtmlAttr::Aria(AriaAttr::Controls),
            "panel-a   panel-b\tpanel-c",
        )]);

        let ids = extract_all_ids(&[root]);

        assert_eq!(ids.len(), 3);
        assert!(ids.contains("panel-a"));
        assert!(ids.contains("panel-b"));
        assert!(ids.contains("panel-c"));
    }

    #[test]
    fn duplicate_ids_across_maps_are_deduplicated() {
        let first = attr_map_with(&[(HtmlAttr::Id, "shared")]);
        let second = attr_map_with(&[(HtmlAttr::Aria(AriaAttr::LabelledBy), "shared")]);

        let ids = extract_all_ids(&[first, second]);

        assert_eq!(ids.len(), 1);
        assert!(ids.contains("shared"));
    }
}
