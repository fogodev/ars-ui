#[cfg(feature = "i18n")]
use super::*;

proptest! {
    #![proptest_config(crate::common::proptest_config())]

    /// For any combination of props and locale, `highlight_chunks` must:
    ///
    /// 1. **Never panic** (covered by the test reaching its end).
    /// 2. **Roundtrip the text** — concatenating all chunk slices in order
    ///    yields the original `props.text` byte-for-byte. Catches any
    ///    range / index / byte-map regression silently dropping or
    ///    duplicating text.
    /// 3. **Never emit two adjacent highlighted chunks** — the agnostic
    ///    core's adjacency-merge contract (spec §3.1) must hold for every
    ///    strategy and every query combination.
    /// 4. **Never emit empty chunks** — zero-length segments are skipped
    ///    by `build_chunks`; this guards against silent regressions there.
    /// 5. **Empty query (or all-empty queries)** must produce exactly one
    ///    non-highlighted chunk wrapping the full text, when the text is
    ///    non-empty. Empty text yields an empty `Vec`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_highlight_chunks_invariants(
        props in arb_highlight_props(),
        locale in arb_highlight_locale(),
    ) {
        let chunks = utility_core::highlight::highlight_chunks(&props, &locale);

        // (2) roundtrip
        let concatenated: String = chunks.iter().map(|c| c.text).collect();

        prop_assert_eq!(
            concatenated.as_str(),
            props.text.as_str(),
            "chunks must reconstruct the original text"
        );

        // (3) no two adjacent highlighted chunks
        for window in chunks.windows(2) {
            prop_assert!(
                !(window[0].highlighted && window[1].highlighted),
                "adjacency-merge regression: {:?}", window
            );
        }

        // (4) no empty chunks
        for chunk in &chunks {
            prop_assert!(
                !chunk.text.is_empty(),
                "empty chunk emitted: {:?}", chunk
            );
        }

        // (5) empty-query / empty-text special cases
        let all_queries_empty = props.query.iter().all(String::is_empty);

        if props.text.is_empty() {
            prop_assert!(chunks.is_empty(), "empty text should yield empty Vec");
        } else if props.query.is_empty() || all_queries_empty {
            prop_assert_eq!(chunks.len(), 1, "empty query → exactly one chunk");
            prop_assert!(!chunks[0].highlighted, "empty-query chunk must not be highlighted");
            prop_assert_eq!(chunks[0].text, props.text.as_str());
        }
    }
}
