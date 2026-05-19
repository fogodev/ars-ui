//! `Highlight` component — text substring matching with Unicode-aware case folding.
//!
//! `Highlight` is a stateless utility that splits a text into alternating
//! highlighted / non-highlighted chunks based on one or more search queries.
//! It supports three match strategies (`Contains`, `StartsWith`, `Fuzzy`),
//! locale-aware Unicode case mapping via ICU4X `CaseMapper` (handles Turkic
//! dotted/dotless I, German eszett, Greek final sigma, Lithuanian dotted I,
//! and other CLDR-supplied language-tailored mappings), and multi-query
//! merging that consolidates both overlapping **and** adjacent ranges so
//! adapters never see back-to-back highlighted chunks.
//!
//! The component has no state machine. Adapters render each returned
//! `HighlightChunk` as a `<mark>` (`highlighted == true`) or `<span>`
//! (`highlighted == false`). The agnostic-core consists of `Props`,
//! `MatchStrategy`, `HighlightChunk`, the `Part` enum, the `Api` attribute
//! mapper, and the `highlight_chunks` standalone computation.
//!
//! This module is gated on `feature = "i18n"`: the locale-aware contract on
//! `Props::ignore_case = true` is part of the spec, so the module is only
//! compiled when `ars-i18n/icu4x` is enabled.
//!
//! See `spec/components/utility/highlight.md` for the authoritative contract.

use alloc::{borrow::ToOwned as _, string::String, vec, vec::Vec};

use ars_core::{AttrMap, ComponentPart, ConnectApi, HtmlAttr};
use ars_i18n::Locale;

/// Props for the [`Highlight`](crate::utility::highlight) component.
///
/// `Highlight` does not carry a DOM `id` — chunk output is a sequence of
/// adapter-rendered children, not a single identifiable element — so [`Props`]
/// does not derive [`HasId`](ars_core::HasId) like other stateless utilities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Props {
    /// The search queries to highlight. Supports a single query string or
    /// multiple queries to highlight simultaneously (e.g., multiple search
    /// terms). When multiple queries are provided, all matches from all
    /// queries are highlighted; overlapping ranges are merged into a single
    /// highlighted chunk. An empty vec, or a vec where every entry is empty,
    /// produces a single non-highlighted chunk containing the full text.
    pub query: Vec<String>,

    /// The full text to search within.
    pub text: String,

    /// When `true` (default), matching is case-insensitive via Unicode case
    /// folding — `ars_i18n::case_fold` with the [`Locale`] passed to
    /// [`highlight_chunks`] / [`Api::chunks`]. When `false`, byte-exact
    /// matching is used (no normalisation pass; the locale parameter is
    /// ignored).
    pub ignore_case: bool,

    /// The strategy used to match the query against the text. Defaults to
    /// [`MatchStrategy::Contains`].
    pub match_strategy: MatchStrategy,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            query: Vec::new(),
            text: String::new(),
            ignore_case: true,
            match_strategy: MatchStrategy::Contains,
        }
    }
}

impl Props {
    /// Returns fresh [`Props`] with the documented defaults — equivalent
    /// to [`Default::default`], offered as the entry point for the
    /// builder chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the query list. Accepts any iterable of `Into<String>`, so
    /// callers can pass `["foo"]`, `vec!["foo".to_string(), "bar".into()]`,
    /// `&["foo", "bar"][..]`, etc.
    #[must_use]
    pub fn query<I, S>(mut self, query: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.query = query.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the text to search within.
    #[must_use]
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Sets the case-sensitivity flag.
    #[must_use]
    pub const fn ignore_case(mut self, value: bool) -> Self {
        self.ignore_case = value;
        self
    }

    /// Sets the match strategy.
    #[must_use]
    pub const fn match_strategy(mut self, value: MatchStrategy) -> Self {
        self.match_strategy = value;
        self
    }
}

/// The strategy used to match the query against the text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MatchStrategy {
    /// Highlight every occurrence of the query within the text. The default
    /// — the most permissive strategy, matching Ark UI's default.
    #[default]
    Contains,

    /// Highlight only the leading run of the text when it starts with the
    /// query (after case folding, if `ignore_case` is `true`).
    StartsWith,

    /// Subsequence match: highlight each individual query character within
    /// the text in order, allowing arbitrary gaps.
    ///
    /// All query characters must be present (in order, not necessarily
    /// contiguous). When the full subsequence is found, each matched
    /// character contributes its own one-char highlighted range. When any
    /// query character cannot be located, **no fuzzy ranges are emitted
    /// for that query** (all-or-nothing) — partial matches would highlight
    /// characters that don't represent the query and confuse the user.
    Fuzzy,
}

/// One contiguous run of the original text, tagged as highlighted or not.
///
/// `text` borrows from [`Props::text`] (the lifetime is tied to the props
/// passed to [`highlight_chunks`] / [`Api::chunks`]); the chunk vector is
/// owned by the caller. Adapters render highlighted chunks as `<mark>` and
/// non-highlighted chunks as `<span>` per the component anatomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HighlightChunk<'a> {
    /// The slice of the source text covered by this chunk.
    pub text: &'a str,

    /// Whether the adapter should wrap this chunk in `<mark>`.
    pub highlighted: bool,
}

/// DOM parts of the `Highlight` component.
///
/// Only [`Part::Root`] is a static anatomy slot. The per-chunk
/// `<mark>` / `<span>` elements are emitted dynamically per
/// [`HighlightChunk`], parametric on the runtime `highlighted` boolean — a
/// value that [`ConnectApi::part_attrs`] (which takes a `Part` by value, no
/// payload) cannot express. Adapters call [`Api::chunk_attrs`] per chunk
/// instead. See `spec/components/utility/highlight.md` §2 for the rendered
/// shape.
#[derive(ComponentPart)]
#[scope = "highlight"]
pub enum Part {
    /// The root `<span>` that wraps the chunk children.
    Root,
}

/// Attribute-mapper API for the `Highlight` component.
///
/// Constructed via [`Api::new`] from [`Props`] and queried via
/// [`Api::root_attrs`] / [`Api::chunk_attrs`] for adapter-rendered DOM
/// attributes, or [`Api::chunks`] / the standalone [`highlight_chunks`]
/// function for the chunk sequence itself.
#[derive(Clone, Debug)]
pub struct Api {
    props: Props,
}

impl Api {
    /// Wraps the given props in an `Api`. The props are stored verbatim and
    /// can be read back via [`Api::props`] or the typed accessors.
    #[must_use]
    pub const fn new(props: Props) -> Self {
        Self { props }
    }

    /// Returns a reference to the underlying [`Props`].
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Returns the configured query list.
    #[must_use]
    pub fn query(&self) -> &[String] {
        &self.props.query
    }

    /// Returns the configured source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.props.text
    }

    /// Returns whether matching is case-insensitive.
    #[must_use]
    pub const fn ignore_case(&self) -> bool {
        self.props.ignore_case
    }

    /// Returns the configured match strategy.
    #[must_use]
    pub const fn match_strategy(&self) -> MatchStrategy {
        self.props.match_strategy
    }

    /// Attributes for the root `<span>` wrapper.
    ///
    /// Emits `data-ars-scope="highlight"` + `data-ars-part="root"` (via
    /// `Part::Root.data_attrs()`) plus `dir="auto"` so the browser picks
    /// the correct `BiDi` paragraph direction from the first strong character
    /// in the highlighted text (see spec §3.2). No ARIA attributes — the
    /// semantic `<mark>` elements in the chunk children carry the
    /// highlighting semantics for assistive technology.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Dir, "auto");

        attrs
    }

    /// Attributes for one chunk element.
    ///
    /// Emits `data-ars-part="highlight-chunk"` and
    /// `data-ars-highlighted="true"|"false"` per `highlighted`. The
    /// underlying element (`<mark>` vs `<span>`) is chosen by the adapter
    /// based on the same boolean.
    #[must_use]
    pub fn chunk_attrs(&self, highlighted: bool) -> AttrMap {
        let mut attrs = AttrMap::new();

        attrs
            .set(HtmlAttr::Data("ars-part"), "highlight-chunk")
            .set(
                HtmlAttr::Data("ars-highlighted"),
                if highlighted { "true" } else { "false" },
            );

        attrs
    }

    /// Computes the chunk sequence for the configured props at the given
    /// locale. Thin convenience around [`highlight_chunks`] — the standalone
    /// function takes the same arguments and is the spec-defined entry point.
    #[must_use]
    pub fn chunks<'a>(&'a self, locale: &Locale) -> Vec<HighlightChunk<'a>> {
        highlight_chunks(&self.props, locale)
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}

/// Splits the configured text into alternating highlighted / non-highlighted
/// chunks under the given locale.
///
/// # Examples
///
/// ```
/// use ars_components::utility::highlight::{highlight_chunks, HighlightChunk, Props};
/// use ars_i18n::Locale;
///
/// let props = Props::new().query(["world"]).text("Hello world!");
/// let locale = Locale::parse("en-US").expect("en-US must parse");
///
/// let chunks = highlight_chunks(&props, &locale);
///
/// assert_eq!(
///     chunks,
///     vec![
///         HighlightChunk { text: "Hello ", highlighted: false },
///         HighlightChunk { text: "world", highlighted: true },
///         HighlightChunk { text: "!", highlighted: false },
///     ]
/// );
/// ```
///
/// When `props.ignore_case` is `true`, matching uses
/// [`ars_i18n::case_fold`] for Unicode case folding (ICU4X
/// `CaseMapper::fold_string` — or `fold_turkic_string` under Turkic
/// locales — under the hood) so locale-sensitive case pairs match
/// correctly: German eszett (`ß` ↔ `ss` ↔ `SS`), Turkish dotted/dotless
/// I (`İ` ↔ `i` and `I` ↔ `ı`), Greek final sigma collapse, Lithuanian
/// dotted I, and other CLDR-supplied tailored mappings. When
/// `props.ignore_case` is `false`, matching is byte-exact and the
/// `locale` parameter is ignored.
///
/// When multiple queries (or fuzzy hits) produce overlapping **or
/// adjacent** ranges, they are merged into a single highlighted chunk so
/// adapters never render back-to-back `<mark>` elements covering
/// contiguous text. This satisfies spec §3.1's adjacency-consolidation
/// requirement in the core, leaving adapters free to layer only
/// genuinely additional a11y behaviour (e.g., summary announcements for
/// high-cardinality fuzzy results) on top.
///
/// `MatchStrategy::Contains` scans for **every** occurrence of the query
/// including overlapping ones (e.g., `"ana"` against `"banana"` matches
/// at byte offsets 1 and 3, merging into a single highlight covering
/// `"anana"`). When a match's end falls inside a case-fold expansion
/// (e.g., `"stras"` against German `"Straße"` where `ß` folds to `ss`),
/// the highlighted span extends to cover the **full** source character
/// that contributed the matched fold bytes, so the rendered output is
/// `"Straß"`, never a mid-codepoint slice.
///
/// `MatchStrategy::Fuzzy` consumes as many consecutive query characters
/// as appear, in order, inside each source character's fold expansion.
/// This keeps fuzzy matching symmetric with the other strategies under
/// many-to-one folds (`"ss"` matches `"ß"` under German, just like the
/// other direction does for `Contains` and `StartsWith`).
///
/// # Edge cases
///
/// - Empty `query`, or `query` containing only empty strings → a single
///   non-highlighted chunk wrapping the full text.
/// - Empty `text` → an empty [`Vec`] (nothing to render).
/// - Query longer than text → a single non-highlighted chunk.
/// - `MatchStrategy::Fuzzy` with any query character missing from the
///   text → no fuzzy ranges contributed for that query (all-or-nothing).
#[must_use]
pub fn highlight_chunks<'a>(props: &'a Props, locale: &Locale) -> Vec<HighlightChunk<'a>> {
    let text = props.text.as_str();

    if text.is_empty() {
        return Vec::new();
    }

    let non_empty_queries: Vec<&str> = props
        .query
        .iter()
        .map(String::as_str)
        .filter(|q| !q.is_empty())
        .collect();

    if non_empty_queries.is_empty() {
        return vec![HighlightChunk {
            text,
            highlighted: false,
        }];
    }

    let normalised = Normalised::build(text, props.ignore_case, locale);

    let mut ranges: Vec<(usize, usize)> = Vec::new();

    for query in &non_empty_queries {
        // Every assigned Unicode character folds to a non-empty
        // replacement (TR21), so for non-empty input `case_fold` never
        // produces an empty string. The `non_empty_queries` filter above
        // guarantees the input is non-empty, so we don't need an empty
        // check on `normalised_query`.
        let normalised_query = if props.ignore_case {
            case_fold(query, locale)
        } else {
            (*query).to_owned()
        };

        match props.match_strategy {
            MatchStrategy::Contains => {
                push_contains_matches(&normalised, &normalised_query, &mut ranges);
            }

            MatchStrategy::StartsWith => {
                push_starts_with_match(&normalised, &normalised_query, &mut ranges);
            }

            MatchStrategy::Fuzzy => {
                push_fuzzy_matches(
                    text,
                    &normalised_query,
                    props.ignore_case,
                    locale,
                    &mut ranges,
                );
            }
        }
    }

    if ranges.is_empty() {
        return vec![HighlightChunk {
            text,
            highlighted: false,
        }];
    }

    let merged = merge_ranges(ranges);

    build_chunks(text, &merged)
}

/// Normalised text plus a byte-level mapping back to the original source
/// text. Each normalised byte `i` has two map entries:
///
/// - `source_start[i]` — original byte index where the source character
///   that produced normalised byte `i` *starts*.
/// - `source_end[i]` — original byte index where that source character
///   *ends* (i.e., `source_start + char.len_utf8()`).
///
/// Both arrays are aligned: `source_start.len() == source_end.len() ==
/// normalised.len()`. For a normalised match `[a, b)` we always map to
/// the source range `[source_start[a], source_end[b - 1]]`, even when
/// `a` or `b` falls *inside* a case-fold expansion (e.g. `ß → ss`).
/// Using `source_end[b - 1]` rather than `source_start[b]` is what lets
/// a match that ends mid-expansion still highlight the full source
/// character that contributed the matched bytes — without this, queries
/// like `"stras"` against text `"Straße"` would highlight only the
/// `"Stra"` prefix and drop the `ß` that contributed the trailing `s`.
struct Normalised {
    text: String,
    source_start: Vec<usize>,
    source_end: Vec<usize>,
}

impl Normalised {
    /// Build the normalised view of `text`. When `!ignore_case`, the
    /// normalised text is the original verbatim and the maps are the
    /// per-byte identity.
    fn build(text: &str, ignore_case: bool, locale: &Locale) -> Self {
        if !ignore_case {
            let source_start: Vec<usize> = (0..text.len()).collect();
            let source_end: Vec<usize> = (1..=text.len()).collect();
            return Self {
                text: text.to_owned(),
                source_start,
                source_end,
            };
        }

        let mut normalised = String::with_capacity(text.len());
        let mut source_start = Vec::with_capacity(text.len());
        let mut source_end = Vec::with_capacity(text.len());

        for (orig_byte, ch) in text.char_indices() {
            let mut buf = [0u8; 4];
            let ch_str = ch.encode_utf8(&mut buf);
            let folded = case_fold(ch_str, locale);
            let end_of_source_char = orig_byte + ch.len_utf8();

            for _ in 0..folded.len() {
                source_start.push(orig_byte);
                source_end.push(end_of_source_char);
            }

            normalised.push_str(&folded);
        }

        Self {
            text: normalised,
            source_start,
            source_end,
        }
    }

    /// Map a normalised half-open range `[start_norm, end_norm)` to the
    /// corresponding original-text half-open range. `end_norm` must be
    /// strictly greater than `start_norm` (the caller filters empty
    /// matches before calling).
    fn map_range(&self, start_norm: usize, end_norm: usize) -> (usize, usize) {
        debug_assert!(
            end_norm > start_norm,
            "Normalised::map_range requires a non-empty range"
        );
        (self.source_start[start_norm], self.source_end[end_norm - 1])
    }
}

/// Unicode case fold for case-insensitive comparison. Delegates to
/// [`ars_i18n::case_fold`] which wraps ICU4X
/// `CaseMapper::fold_string` (or `fold_turkic_string` under Turkic
/// locales). The fold form — not lowercase — is the right primitive for
/// matching: it expands `ß → ss`, collapses Greek final sigma into the
/// medial form, and (under Turkic) folds `İ ↔ i` / `I ↔ ı`. This is what
/// the spec's `ignore_case = true` contract requires.
fn case_fold(text: &str, locale: &Locale) -> String {
    ars_i18n::case_fold(text, locale)
}

/// Push every occurrence of `normalised_query` inside `normalised.text`,
/// **including overlapping ones** (`str::match_indices` only yields the
/// non-overlapping ones, which would miss `"ana"` at position 3 of
/// `"banana"`). Advances by one character past each match's start to
/// hit the next overlapping occurrence.
fn push_contains_matches(
    normalised: &Normalised,
    normalised_query: &str,
    out: &mut Vec<(usize, usize)>,
) {
    if normalised_query.is_empty() {
        return;
    }

    let haystack = normalised.text.as_str();
    let mut search_start = 0;

    while let Some(rel) = haystack[search_start..].find(normalised_query) {
        let start = search_start + rel;
        let end = start + normalised_query.len();
        let (orig_start, orig_end) = normalised.map_range(start, end);

        out.push((orig_start, orig_end));

        // Advance past one character so the next iteration can find
        // overlapping matches. UTF-8 guarantees that `find` returned a
        // position on a char boundary, so `chars().next()` is sound.
        let advance = haystack[start..].chars().next().map_or(1, char::len_utf8);
        search_start = start + advance;
    }
}

fn push_starts_with_match(
    normalised: &Normalised,
    normalised_query: &str,
    out: &mut Vec<(usize, usize)>,
) {
    if normalised_query.is_empty() {
        return;
    }

    if normalised.text.starts_with(normalised_query) {
        let end = normalised_query.len();
        let (_, orig_end) = normalised.map_range(0, end);
        out.push((0, orig_end));
    }
}

/// Walk the original text char-by-char and consume the query in order.
/// Each source character that contributes at least one matched query
/// character produces a one-char highlighted range. Fuzzy matching is
/// all-or-nothing — if any query character cannot be located in order,
/// the function emits zero ranges for this query.
///
/// **Multi-char fold expansions**: a single source char can fold to
/// multiple chars (e.g. `ß → ss` under German), so we consume *as many*
/// query characters as match the source char's folded expansion *in
/// order*. Without this, `MatchStrategy::Fuzzy` with `query=["ss"]` and
/// `text="ß"` would consume only the first `s` from the query, fail on
/// the second, and emit no match — violating the spec's eszett
/// equivalence contract.
fn push_fuzzy_matches(
    original_text: &str,
    normalised_query: &str,
    ignore_case: bool,
    locale: &Locale,
    out: &mut Vec<(usize, usize)>,
) {
    let mut query_iter = normalised_query.chars().peekable();
    let mut local = Vec::new();

    for (orig_byte, ch) in original_text.char_indices() {
        if query_iter.peek().is_none() {
            break;
        }

        let normalised_ch_string = if ignore_case {
            let mut buf = [0u8; 4];
            let ch_str = ch.encode_utf8(&mut buf);
            case_fold(ch_str, locale)
        } else {
            let mut s = String::new();
            s.push(ch);
            s
        };

        // Consume every leading query character that appears, in order,
        // in this source char's folded expansion. For single-char folds
        // (the common case) this consumes at most one query char; for
        // expansions like `ß → ss` it may consume two or more.
        let mut consumed_any = false;
        for folded_char in normalised_ch_string.chars() {
            if let Some(&want) = query_iter.peek() && want == folded_char {
                query_iter.next();
                consumed_any = true;
            }
        }

        if consumed_any {
            local.push((orig_byte, orig_byte + ch.len_utf8()));
        }
    }

    if query_iter.peek().is_none() {
        out.extend(local);
    }
}

/// Sort ranges by start and merge overlapping or adjacent ones (`c <= b`).
fn merge_ranges(mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    ranges.sort_by_key(|&(start, _)| start);

    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(ranges.len());

    for (start, end) in ranges {
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            if end > last.1 {
                last.1 = end;
            }
        } else {
            merged.push((start, end));
        }
    }

    merged
}

/// Walk the merged ranges and emit alternating non-highlighted / highlighted
/// chunks borrowing from `text`. Zero-length segments are skipped.
fn build_chunks<'a>(text: &'a str, ranges: &[(usize, usize)]) -> Vec<HighlightChunk<'a>> {
    let mut chunks = Vec::with_capacity(ranges.len() * 2 + 1);
    let mut cursor = 0usize;

    for &(start, end) in ranges {
        if start > cursor {
            chunks.push(HighlightChunk {
                text: &text[cursor..start],
                highlighted: false,
            });
        }

        if end > start {
            chunks.push(HighlightChunk {
                text: &text[start..end],
                highlighted: true,
            });
        }

        cursor = end;
    }

    if cursor < text.len() {
        chunks.push(HighlightChunk {
            text: &text[cursor..],
            highlighted: false,
        });
    }

    chunks
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};

    use ars_core::HtmlAttr;
    use insta::assert_snapshot;

    use super::*;

    fn en_us() -> Locale {
        Locale::parse("en-US").expect("en-US must parse")
    }

    fn turkish() -> Locale {
        Locale::parse("tr").expect("tr must parse")
    }

    fn german() -> Locale {
        Locale::parse("de").expect("de must parse")
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        alloc::format!("{attrs:#?}")
    }

    fn snapshot_chunks(chunks: &[HighlightChunk<'_>]) -> String {
        alloc::format!("{chunks:#?}")
    }

    // ── Strategy & matching behaviour ──────────────────────────────

    #[test]
    fn contains_strategy_matches_substring() {
        let props = Props::new()
            .query(["world"])
            .text("Hello world!")
            .match_strategy(MatchStrategy::Contains);

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![
                HighlightChunk {
                    text: "Hello ",
                    highlighted: false,
                },
                HighlightChunk {
                    text: "world",
                    highlighted: true,
                },
                HighlightChunk {
                    text: "!",
                    highlighted: false,
                },
            ]
        );
    }

    #[test]
    fn contains_strategy_finds_all_occurrences() {
        let props = Props::new().query(["la"]).text("la la la");

        let chunks = highlight_chunks(&props, &en_us());

        // 3 highlighted "la"s separated by 2 single-space gaps. No leading
        // or trailing gap because "la" starts and ends the text.
        assert_eq!(chunks.len(), 5);
        assert_eq!(chunks[0].text, "la");
        assert!(chunks[0].highlighted);
        assert_eq!(chunks[1].text, " ");
        assert!(!chunks[1].highlighted);
        assert_eq!(chunks[2].text, "la");
        assert!(chunks[2].highlighted);
        assert_eq!(chunks[3].text, " ");
        assert!(!chunks[3].highlighted);
        assert_eq!(chunks[4].text, "la");
        assert!(chunks[4].highlighted);
    }

    #[test]
    fn starts_with_strategy_matches_at_start() {
        let props = Props::new()
            .query(["hello"])
            .text("Hello world")
            .match_strategy(MatchStrategy::StartsWith);

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![
                HighlightChunk {
                    text: "Hello",
                    highlighted: true,
                },
                HighlightChunk {
                    text: " world",
                    highlighted: false,
                },
            ]
        );
    }

    #[test]
    fn starts_with_strategy_no_match_in_middle() {
        // "world" is in the middle of the text — StartsWith returns one
        // non-highlighted chunk.
        let props = Props::new()
            .query(["world"])
            .text("Hello world")
            .match_strategy(MatchStrategy::StartsWith);

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![HighlightChunk {
                text: "Hello world",
                highlighted: false,
            }]
        );
    }

    #[test]
    fn fuzzy_strategy_matches_in_order_with_gaps() {
        let props = Props::new()
            .query(["abc"])
            .text("a_b_c")
            .match_strategy(MatchStrategy::Fuzzy);

        let chunks = highlight_chunks(&props, &en_us());

        // Three highlighted single-char chunks separated by two
        // non-highlighted "_" chunks.
        assert_eq!(
            chunks,
            vec![
                HighlightChunk {
                    text: "a",
                    highlighted: true,
                },
                HighlightChunk {
                    text: "_",
                    highlighted: false,
                },
                HighlightChunk {
                    text: "b",
                    highlighted: true,
                },
                HighlightChunk {
                    text: "_",
                    highlighted: false,
                },
                HighlightChunk {
                    text: "c",
                    highlighted: true,
                },
            ]
        );
    }

    #[test]
    fn fuzzy_strategy_no_match_when_chars_missing() {
        // Query "xyz" is missing from text "abc" → no match → one
        // non-highlighted chunk.
        let props = Props::new()
            .query(["xyz"])
            .text("abc")
            .match_strategy(MatchStrategy::Fuzzy);

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![HighlightChunk {
                text: "abc",
                highlighted: false,
            }]
        );
    }

    #[test]
    fn unicode_case_folding_turkic_dotted_i() {
        // In Turkish, "İ" (dotted capital I) lowercases to "i". The query
        // "İ" should highlight "i" in the text under tr locale.
        let props = Props::new().query(["İ"]).text("istanbul");

        let chunks = highlight_chunks(&props, &turkish());

        assert_eq!(chunks.len(), 2);
        assert!(chunks[0].highlighted);
        assert_eq!(chunks[0].text, "i");
        assert!(!chunks[1].highlighted);
        assert_eq!(chunks[1].text, "stanbul");
    }

    #[test]
    fn unicode_case_folding_german_eszett() {
        // Per Unicode TR21, `case_fold("ß") == "ss"` and `case_fold("SS")
        // == "ss"`. So `"ß"`, `"ss"`, and `"SS"` must all be equivalent
        // for case-insensitive matching — the user typing `"ss"` should
        // find every form, including the eszett `ß`.

        // Direction A: query "ss" against text containing both ß and SS.
        let props_a = Props::new().query(["ss"]).text("Straße ist STRASSE");

        let chunks_a = highlight_chunks(&props_a, &german());

        let highlighted_a: Vec<&str> = chunks_a
            .iter()
            .filter(|c| c.highlighted)
            .map(|c| c.text)
            .collect();

        assert_eq!(
            highlighted_a,
            vec!["ß", "SS"],
            "query \"ss\" must match both \"ß\" and \"SS\""
        );

        // Direction B: query "SS" against text containing ß — must match.
        let props_b = Props::new().query(["SS"]).text("Straße");

        let chunks_b = highlight_chunks(&props_b, &german());

        let highlighted_b: Vec<&str> = chunks_b
            .iter()
            .filter(|c| c.highlighted)
            .map(|c| c.text)
            .collect();

        assert_eq!(
            highlighted_b,
            vec!["ß"],
            "query \"SS\" must match \"ß\" under case fold"
        );

        // Direction C: query "ß" against text with only "SS" — reverse
        // direction must also match.
        let props_c = Props::new().query(["ß"]).text("STRASSE");

        let chunks_c = highlight_chunks(&props_c, &german());

        let highlighted_c: Vec<&str> = chunks_c
            .iter()
            .filter(|c| c.highlighted)
            .map(|c| c.text)
            .collect();

        assert_eq!(
            highlighted_c,
            vec!["SS"],
            "query \"ß\" must match \"SS\" under case fold"
        );
    }

    #[test]
    fn multi_query_fully_contained_range_does_not_extend() {
        // Branch coverage for `merge_ranges`: when an incoming range is
        // fully contained inside an existing merged range
        // (`start <= last.1` AND `end <= last.1`), the body is a no-op.
        // The two queries below produce ranges (0, 6) and (1, 5); the
        // second is entirely inside the first, so the merged output
        // remains a single (0, 6) range.
        let props = Props::new().query(["foobar", "ooba"]).text("foobar");

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![HighlightChunk {
                text: "foobar",
                highlighted: true,
            }]
        );
    }

    #[test]
    fn multi_query_idempotent_under_duplicate_queries() {
        // Defensive invariant: passing the same query twice must produce
        // the same output as passing it once (the per-query loop has no
        // hidden side-effects). Catches regressions that would, e.g.,
        // double-count ranges before the merge pass.
        let single = Props::new().query(["foo"]).text("foobar foo");

        let duplicated = Props::new().query(["foo", "foo", "foo"]).text("foobar foo");

        assert_eq!(
            highlight_chunks(&single, &en_us()),
            highlight_chunks(&duplicated, &en_us())
        );
    }

    #[test]
    fn fold_expansion_partial_match_highlights_full_source_char() {
        // Under en-US (non-Turkic), `İ` case-folds to `i\u{307}` (i +
        // combining dot above, 3 bytes). A query of `"i"` matches the
        // leading byte of that expansion. The implementation must
        // highlight the **entire** source character `İ` (not nothing,
        // and not just the leading byte), because rendering only part
        // of a code point would corrupt the displayed text. Verified
        // for both Contains and StartsWith — the byte-map's
        // `source_end` mapping is what lets the match extend to the
        // source char's full UTF-8 span.
        for strategy in [MatchStrategy::Contains, MatchStrategy::StartsWith] {
            let props = Props::new()
                .query(["i"])
                .text("İa")
                .match_strategy(strategy);

            let chunks = highlight_chunks(&props, &en_us());

            let highlighted: Vec<&str> = chunks
                .iter()
                .filter(|c| c.highlighted)
                .map(|c| c.text)
                .collect();

            assert_eq!(
                highlighted,
                vec!["İ"],
                "{strategy:?}: must highlight the full `İ`, got {highlighted:?}"
            );
        }
    }

    #[test]
    fn contains_strategy_finds_overlapping_occurrences() {
        // `str::match_indices` only yields non-overlapping matches, so
        // a naive implementation would highlight only the first `ana`
        // in `banana` and miss the second. The spec says Contains
        // highlights **every** occurrence then merges overlaps, so the
        // expected result is one merged highlight covering `anana`.
        let props = Props::new().query(["ana"]).text("banana");

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![
                HighlightChunk {
                    text: "b",
                    highlighted: false,
                },
                HighlightChunk {
                    text: "anana",
                    highlighted: true,
                },
            ]
        );
    }

    #[test]
    fn starts_with_extends_match_to_full_fold_expansion_source_char() {
        // German `Straße` folds to `strasse`. Query `stras` matches
        // the folded prefix, with its final byte coming from the
        // `ß`'s fold expansion. The highlighted span must extend to
        // cover the full `ß` so the rendered output is `Straß`, not
        // `Stra` (which would slice off the ß mid-codepoint).
        let props = Props::new()
            .query(["stras"])
            .text("Straße")
            .match_strategy(MatchStrategy::StartsWith);

        let chunks = highlight_chunks(&props, &german());

        assert_eq!(
            chunks,
            vec![
                HighlightChunk {
                    text: "Straß",
                    highlighted: true,
                },
                HighlightChunk {
                    text: "e",
                    highlighted: false,
                },
            ]
        );
    }

    #[test]
    fn fuzzy_consumes_full_folded_expansion_for_eszett() {
        // Under German, `ß` folds to `ss`. A fuzzy query of `ss`
        // against text `ß` must consume **both** query characters
        // against the single source char's folded expansion, then
        // emit a one-char highlight for the `ß` itself. Without
        // multi-char consumption the second `s` is left in the query,
        // fuzzy's all-or-nothing rule kicks in, and `ß` ↔ `ss`
        // equivalence breaks asymmetrically with the other strategies.
        let props = Props::new()
            .query(["ss"])
            .text("ß")
            .match_strategy(MatchStrategy::Fuzzy);

        let chunks = highlight_chunks(&props, &german());

        assert_eq!(
            chunks,
            vec![HighlightChunk {
                text: "ß",
                highlighted: true,
            }]
        );
    }

    #[test]
    fn multi_query_overlapping_ranges_merged() {
        // Queries "foo" and "oob" overlap on "oo" within "foobar". The
        // merged range covers "foob", producing one highlighted chunk.
        let props = Props::new().query(["foo", "oob"]).text("foobar");

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![
                HighlightChunk {
                    text: "foob",
                    highlighted: true,
                },
                HighlightChunk {
                    text: "ar",
                    highlighted: false,
                },
            ]
        );
    }

    #[test]
    fn empty_query_returns_single_non_highlighted_chunk() {
        let props = Props::new().text("hello");

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![HighlightChunk {
                text: "hello",
                highlighted: false,
            }]
        );
    }

    #[test]
    fn all_empty_queries_returns_single_non_highlighted_chunk() {
        let props = Props::new().query(["", "", ""]).text("hello");

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![HighlightChunk {
                text: "hello",
                highlighted: false,
            }]
        );
    }

    // ── Invariant tests ────────────────────────────────────────────

    fn assert_chunks_concatenate_to_text(chunks: &[HighlightChunk<'_>], expected: &str) {
        let concatenated: String = chunks.iter().map(|c| c.text).collect();

        assert_eq!(
            concatenated, expected,
            "chunks must reconstruct the original text"
        );
    }

    fn assert_no_two_adjacent_highlighted(chunks: &[HighlightChunk<'_>]) {
        for window in chunks.windows(2) {
            assert!(
                !(window[0].highlighted && window[1].highlighted),
                "two adjacent highlighted chunks should have been merged: {window:?}"
            );
        }
    }

    fn assert_no_empty_chunks(chunks: &[HighlightChunk<'_>]) {
        for chunk in chunks {
            assert!(
                !chunk.text.is_empty(),
                "highlight_chunks must never emit an empty chunk: {chunk:?}"
            );
        }
    }

    #[test]
    fn chunks_concatenate_to_original_text_across_strategies() {
        // Roundtrip invariant: for any input, concatenating the chunk
        // texts in order produces the original text verbatim. Exercised
        // here across every strategy + ignore_case combination to catch
        // off-by-one / boundary regressions silently producing wrong text.
        let text = "Foo bar BAZ ßaz İstanbul fooo";
        let queries: &[&[&str]] = &[
            &["foo"],
            &["BAR"],
            &["foo", "BAZ"],
            &["ßaz"],
            &["i"],
            &["z"],
            &[],
            &[""],
            &["", "foo"],
        ];

        let strategies = [
            MatchStrategy::Contains,
            MatchStrategy::StartsWith,
            MatchStrategy::Fuzzy,
        ];

        for ignore_case in [true, false] {
            for query in queries {
                for strategy in strategies {
                    let props = Props::new()
                        .query(query.iter().copied())
                        .text(text)
                        .ignore_case(ignore_case)
                        .match_strategy(strategy);

                    let chunks = highlight_chunks(&props, &en_us());

                    assert_chunks_concatenate_to_text(&chunks, text);
                    assert_no_two_adjacent_highlighted(&chunks);
                    assert_no_empty_chunks(&chunks);
                }
            }
        }
    }

    #[test]
    fn ignore_case_true_matches_case_insensitive() {
        let props = Props::new().query(["hello"]).text("HELLO world");

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![
                HighlightChunk {
                    text: "HELLO",
                    highlighted: true,
                },
                HighlightChunk {
                    text: " world",
                    highlighted: false,
                },
            ]
        );
    }

    #[test]
    fn ignore_case_false_matches_case_sensitive() {
        let props = Props::new()
            .query(["hello"])
            .text("HELLO world")
            .ignore_case(false);

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![HighlightChunk {
                text: "HELLO world",
                highlighted: false,
            }]
        );
    }

    #[test]
    fn empty_text_returns_empty_vec() {
        let props = Props::new().query(["anything"]).text("");

        let chunks = highlight_chunks(&props, &en_us());

        assert!(chunks.is_empty());
    }

    #[test]
    fn query_longer_than_text_returns_single_non_highlighted_chunk() {
        let props = Props::new().query(["needles in haystacks"]).text("hi");

        let chunks = highlight_chunks(&props, &en_us());

        assert_eq!(
            chunks,
            vec![HighlightChunk {
                text: "hi",
                highlighted: false,
            }]
        );
    }

    // ── Props / Api ergonomics ─────────────────────────────────────

    #[test]
    fn props_default_is_contains_ignore_case_true() {
        let p = Props::default();

        assert!(p.query.is_empty());
        assert!(p.text.is_empty());
        assert!(p.ignore_case);
        assert_eq!(p.match_strategy, MatchStrategy::Contains);
    }

    #[test]
    fn props_clone_and_partial_eq_round_trip() {
        let original = Props {
            query: vec!["a".to_string(), "b".to_string()],
            text: "ab".to_string(),
            ignore_case: false,
            match_strategy: MatchStrategy::Fuzzy,
        };

        let cloned = original.clone();

        assert_eq!(cloned, original);

        let mutated = Props {
            ignore_case: true,
            ..original.clone()
        };

        assert_ne!(mutated, original);
    }

    #[test]
    fn props_debug_impl_non_empty() {
        let api = Api::new(Props::new().text("dbg"));

        let props_dbg = alloc::format!("{:?}", api.props());
        let api_dbg = alloc::format!("{api:?}");

        assert!(props_dbg.contains("Props"), "Props Debug = {props_dbg}");
        assert!(api_dbg.contains("Api"), "Api Debug = {api_dbg}");
    }

    #[test]
    fn props_builder_round_trips() {
        let p = Props::new()
            .query(["foo", "bar"])
            .text("foobar")
            .ignore_case(false)
            .match_strategy(MatchStrategy::Fuzzy);

        assert_eq!(p.query, vec!["foo".to_string(), "bar".to_string()]);
        assert_eq!(p.text, "foobar");
        assert!(!p.ignore_case);
        assert_eq!(p.match_strategy, MatchStrategy::Fuzzy);
    }

    #[test]
    fn api_exposes_all_props_fields() {
        let original = Props {
            query: vec!["q1".to_string()],
            text: "txt".to_string(),
            ignore_case: false,
            match_strategy: MatchStrategy::StartsWith,
        };

        let api = Api::new(original.clone());

        assert_eq!(api.query(), &["q1".to_string()][..]);
        assert_eq!(api.text(), "txt");
        assert!(!api.ignore_case());
        assert_eq!(api.match_strategy(), MatchStrategy::StartsWith);
        assert_eq!(api.props(), &original);
    }

    #[test]
    fn api_ignore_case_returns_both_directions() {
        // Positive-direction test for the `ignore_case` accessor —
        // ensures it isn't accidentally hardcoded to a single value.
        // `Props::default()` is `ignore_case = true`; the explicit
        // `false` builder produces the other side.
        assert!(Api::new(Props::default()).ignore_case());
        assert!(!Api::new(Props::new().ignore_case(false)).ignore_case());
    }

    #[test]
    fn api_props_round_trips() {
        let original = Props::new()
            .query(["needle"])
            .text("Find the needle in the haystack");

        let api = Api::new(original.clone());

        assert_eq!(api.props(), &original);
    }

    #[test]
    fn part_attrs_root_equals_root_attrs() {
        let api = Api::new(Props::new().text("hi"));

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn root_attrs_emits_scope_and_part() {
        let api = Api::new(Props::default());

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("highlight"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    #[test]
    fn chunk_attrs_highlighted_emits_true() {
        let api = Api::new(Props::default());

        let attrs = api.chunk_attrs(true);

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-part")),
            Some("highlight-chunk")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-highlighted")), Some("true"));
    }

    #[test]
    fn chunk_attrs_not_highlighted_emits_false() {
        let api = Api::new(Props::default());

        let attrs = api.chunk_attrs(false);

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-part")),
            Some("highlight-chunk")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-highlighted")), Some("false"));
    }

    #[test]
    fn chunk_attrs_branches_produce_different_attrs() {
        // Defensive cross-branch inequality — a regression that emits the
        // same AttrMap for highlighted and non-highlighted chunks must not
        // pass silently.
        let api = Api::new(Props::default());

        assert_ne!(api.chunk_attrs(true), api.chunk_attrs(false));
    }

    #[test]
    fn api_chunks_matches_standalone_function() {
        let props = Props::new().query(["foo"]).text("foobar");

        let api = Api::new(props.clone());

        assert_eq!(api.chunks(&en_us()), highlight_chunks(&props, &en_us()));
    }

    // ── Snapshots ──────────────────────────────────────────────────

    #[test]
    fn highlight_root_snapshot() {
        assert_snapshot!(
            "highlight_root",
            snapshot_attrs(&Api::new(Props::default()).root_attrs())
        );
    }

    #[test]
    fn highlight_chunk_highlighted_snapshot() {
        assert_snapshot!(
            "highlight_chunk_highlighted",
            snapshot_attrs(&Api::new(Props::default()).chunk_attrs(true))
        );
    }

    #[test]
    fn highlight_chunk_not_highlighted_snapshot() {
        assert_snapshot!(
            "highlight_chunk_not_highlighted",
            snapshot_attrs(&Api::new(Props::default()).chunk_attrs(false))
        );
    }

    #[test]
    fn highlight_chunks_contains_en_snapshot() {
        let props = Props::new().query(["world"]).text("Hello world!");

        assert_snapshot!(
            "highlight_chunks_contains_en",
            snapshot_chunks(&highlight_chunks(&props, &en_us()))
        );
    }

    #[test]
    fn highlight_chunks_starts_with_snapshot() {
        let props = Props::new()
            .query(["hello"])
            .text("Hello world")
            .match_strategy(MatchStrategy::StartsWith);

        assert_snapshot!(
            "highlight_chunks_starts_with",
            snapshot_chunks(&highlight_chunks(&props, &en_us()))
        );
    }

    #[test]
    fn highlight_chunks_fuzzy_snapshot() {
        let props = Props::new()
            .query(["abc"])
            .text("a_b_c")
            .match_strategy(MatchStrategy::Fuzzy);

        assert_snapshot!(
            "highlight_chunks_fuzzy",
            snapshot_chunks(&highlight_chunks(&props, &en_us()))
        );
    }

    #[test]
    fn highlight_chunks_multi_query_merged_snapshot() {
        let props = Props::new().query(["foo", "oob"]).text("foobar");

        assert_snapshot!(
            "highlight_chunks_multi_query_merged",
            snapshot_chunks(&highlight_chunks(&props, &en_us()))
        );
    }

    #[test]
    fn highlight_chunks_turkic_snapshot() {
        let props = Props::new().query(["İ"]).text("istanbul");

        assert_snapshot!(
            "highlight_chunks_turkic",
            snapshot_chunks(&highlight_chunks(&props, &turkish()))
        );
    }

    #[test]
    fn highlight_chunks_german_eszett_snapshot() {
        let props = Props::new().query(["ss"]).text("Straße ist STRASSE");

        assert_snapshot!(
            "highlight_chunks_german_eszett",
            snapshot_chunks(&highlight_chunks(&props, &german()))
        );
    }
}
