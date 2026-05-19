---
component: Highlight
category: utility
tier: stateless
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    ark-ui: Highlight
---

# Highlight

`Highlight` is a stateless utility that splits a text into alternating highlighted / non-highlighted chunks based on one or more search queries. It has no state machine. Adapters render the returned chunks as `<mark>` (highlighted) or `<span>` (non-highlighted) inside a single `<span>` root.

## 1. API

`Highlight` follows the standard stateless utility shape: `Props` + `Part` + `Api`, plus a public free function (`highlight_chunks`) that does the chunk computation. Adapters spread `Api::root_attrs()` / `Api::chunk_attrs(highlighted)` onto the rendered elements and iterate `Api::chunks(locale)` (or call `highlight_chunks` directly) for the chunk sequence.

### 1.1 Props

```rust
/// Props for the `Highlight` component.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Props {
    /// The search queries to highlight. Supports a single query string or
    /// multiple queries to highlight simultaneously (e.g., multiple search
    /// terms). When multiple queries are provided, all matches from all
    /// queries are highlighted; overlapping or adjacent ranges are merged
    /// into a single highlighted chunk. An empty vec, or a vec containing
    /// only empty strings, produces a single non-highlighted chunk
    /// wrapping the full text. Default: empty.
    pub query: Vec<String>,
    /// The full text to search within. Default: empty.
    pub text: String,
    /// Case-insensitive matching. Default: `true`.
    pub ignore_case: bool,
    /// Matching strategy. Default: `MatchStrategy::Contains`.
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

/// The strategy used to match the query against the text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MatchStrategy {
    /// Highlight every occurrence of the query within the text. Default.
    #[default]
    Contains,
    /// Highlight only the leading run of the text when it starts with the
    /// query (after case folding, if `ignore_case` is `true`).
    StartsWith,
    /// Subsequence match: highlight each individual query character within
    /// the text in order, allowing arbitrary gaps. All query characters
    /// must be present; if any character is missing, no fuzzy ranges are
    /// emitted for that query (all-or-nothing).
    Fuzzy,
}
```

`Highlight` does **not** derive `HasId` and has no `id` field — the rendered output is a sequence of chunks rather than a single addressable DOM element.

A fluent builder is provided per the workspace stateless-utility convention (see `foundation/10-component-spec-template.md` §4.1.2):

```rust,no_check
let props = Props::new()
    .query(["foo", "bar"])              // accepts any IntoIterator<Item = Into<String>>
    .text("foobar")
    .ignore_case(false)
    .match_strategy(MatchStrategy::Fuzzy);
```

### 1.2 Connect / API

```rust
/// One contiguous run of the original text, tagged as highlighted or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HighlightChunk<'a> {
    /// The slice of the source text covered by this chunk.
    pub text: &'a str,
    /// Whether the adapter should wrap this chunk in `<mark>`.
    pub highlighted: bool,
}

/// DOM parts of the `Highlight` component.
///
/// Only `Root` is a static anatomy slot. The per-chunk `<mark>` / `<span>`
/// elements are emitted dynamically per `HighlightChunk`, parametric on the
/// runtime `highlighted` boolean — a value `ConnectApi::part_attrs` (which
/// takes a `Part` by value, no payload) cannot express. Adapters call
/// `Api::chunk_attrs(highlighted)` per chunk instead. See §2 Anatomy.
#[derive(ComponentPart)]
#[scope = "highlight"]
pub enum Part {
    Root,
}

pub struct Api { props: Props }

impl Api {
    pub const fn new(props: Props) -> Self;
    pub const fn props(&self) -> &Props;
    pub fn query(&self) -> &[String];
    pub fn text(&self) -> &str;
    pub const fn ignore_case(&self) -> bool;
    pub const fn match_strategy(&self) -> MatchStrategy;

    /// Attributes for the root `<span>`. Emits `data-ars-scope="highlight"`,
    /// `data-ars-part="root"`, and `dir="auto"` (so the browser picks the
    /// correct BiDi paragraph direction from the first strong character —
    /// see §3.2).
    pub fn root_attrs(&self) -> AttrMap;

    /// Attributes for one chunk element. Emits
    /// `data-ars-part="highlight-chunk"` and
    /// `data-ars-highlighted="true"|"false"`. The element type (`<mark>`
    /// vs `<span>`) is chosen by the adapter from the same boolean.
    pub fn chunk_attrs(&self, highlighted: bool) -> AttrMap;

    /// Convenience around `highlight_chunks`.
    pub fn chunks<'a>(&'a self, locale: &Locale) -> Vec<HighlightChunk<'a>>;
}

impl ConnectApi for Api {
    type Part = Part;
    fn part_attrs(&self, part: Part) -> AttrMap;  // Part::Root → root_attrs
}

/// Splits the text into alternating highlighted / non-highlighted segments.
///
/// When `ignore_case` is `true`, matching uses Unicode **case folding**
/// (per Unicode Technical Report 21 — the canonical primitive for
/// case-insensitive matching, as opposed to case **transformation** which
/// is what `to_lowercase` / `to_uppercase` do) via `ars_i18n::case_fold`.
/// This wraps ICU4X `CaseMapper::fold_string` (or `fold_turkic_string`
/// under Turkic locales) and delivers true case equivalence:
///
/// - German eszett: `"ß"`, `"ss"`, and `"SS"` all match each other
///   (the fold expands `ß → ss`).
/// - Greek: final-sigma `ς`, medial `σ`, and capital `Σ` all collapse to
///   the same fold form.
/// - Turkic (`tr` / `az`): `İ ↔ i` and `I ↔ ı` per the Turkic fold; other
///   locales use the standard Unicode fold (`İ → i\u{307}`).
/// - Lithuanian, Armenian, and other CLDR-tailored mappings supplied by
///   `CaseMapper`.
///
/// When `ignore_case` is `false`, matching is byte-exact and the `locale`
/// parameter is ignored.
///
/// When multiple queries (or fuzzy hits) produce overlapping or adjacent
/// ranges, the agnostic core merges them into a single highlighted chunk
/// before returning — adapters never see back-to-back `<mark>` elements
/// covering contiguous text.
///
/// Edge cases:
///
/// - Empty `query`, or `query` containing only empty strings → one
///   non-highlighted chunk wrapping the full text.
/// - Empty `text` → an empty `Vec` (nothing to render).
/// - Query longer than text → one non-highlighted chunk.
/// - `MatchStrategy::Fuzzy` with any missing query character → that query
///   contributes zero ranges (all-or-nothing).
pub fn highlight_chunks<'a>(props: &'a Props, locale: &Locale) -> Vec<HighlightChunk<'a>>;
```

### 1.3 Build features

The agnostic-core lives in `ars-components` gated on `feature = "i18n"`, which enables `ars-i18n/icu4x`. The locale-aware case-mapping guarantee on `ignore_case = true` is non-negotiable, so the module is unavailable in builds that don't have ICU4X. Adapters using the browser-native `Intl` backend (`ars-i18n/web-intl`) cannot also use Highlight in the same build — the two ars-i18n backends are mutually exclusive.

## 2. Anatomy

```text
Highlight
└── Root                <span>                  data-ars-scope="highlight" data-ars-part="root" dir="auto"
    └── Chunk           <mark> | <span>   (×N) — parametric per `HighlightChunk`
```

| Part  | Element           | Key Attributes                                                                                                 |
| ----- | ----------------- | -------------------------------------------------------------------------------------------------------------- |
| Root  | `<span>`          | `data-ars-scope="highlight"`, `data-ars-part="root"`, `dir="auto"`                                             |
| Chunk | `<mark>`/`<span>` | `data-ars-part="highlight-chunk"`, `data-ars-highlighted="true\|false"` — emitted via `Api::chunk_attrs(bool)` |

**`Chunk` is a parametric anatomy slot.** It is intentionally absent from the `Part` enum because its attribute output depends on a runtime boolean (`highlighted`) that `ConnectApi::part_attrs` cannot carry. Adapters call `Api::chunk_attrs(chunk.highlighted)` per chunk. The same shape applies to other parametric anatomies in the catalog (per-row, per-item, per-tag slots).

Adapters render each `HighlightChunk` as a `<mark>` (when `highlighted == true`) or `<span>` (when `highlighted == false`).

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- `<mark>` is the semantic HTML element for highlighted text. Screen-reader announcement of `<mark>` content varies by browser/AT combination — VoiceOver and JAWS announce highlighted runs in many configurations; NVDA in its default profile does not. The agnostic core does not add ARIA attributes; adapters that need a guaranteed announcement (e.g., live-region tickers around the highlighted region) layer that on top.
- For custom styling, adapters carry the `data-ars-part="highlight-chunk"` and `data-ars-highlighted="true|false"` attributes via `Api::chunk_attrs(highlighted)`.
- **Adjacent / overlapping chunk consolidation is done in the agnostic core.** `highlight_chunks` merges overlapping and adjacent ranges before returning, so adapters never see back-to-back `<mark>` elements covering contiguous text and screen readers never get a stutter of "highlighted, highlighted, highlighted" for adjacent fuzzy hits. Adapters MAY still provide a summary announcement (e.g., "N characters matched") for high-cardinality fuzzy results, as a layered enhancement.

### 3.2 Bidirectional Text

`Api::root_attrs()` emits `dir="auto"` on the root `<span>` so the browser picks the correct BiDi paragraph direction from the first strong character of the highlighted text — adapters do not need to add it themselves. Adapters MAY additionally apply `unicode-bidi: isolate` CSS to chunk elements when rendering English query terms inside Arabic or Hebrew text, to prevent chunk boundaries from re-ordering visually. The styling is a CSS concern outside the agnostic core's reach.

## 4. Adapter Rendering

Adapters spread `Api::root_attrs()` and `Api::chunk_attrs(highlighted)` rather than hardcoding the `data-ars-*` strings — keeping the attribute names typed and DRY against renames.

```rust,no_check
// Leptos example
let api = Api::new(props);
let chunks = api.chunks(&locale);

view! {
    <span ..api.root_attrs()>
        {chunks.into_iter().map(|chunk| {
            let attrs = api.chunk_attrs(chunk.highlighted);
            if chunk.highlighted {
                view! { <mark ..attrs>{chunk.text}</mark> }
            } else {
                view! { <span ..attrs>{chunk.text}</span> }
            }
        }).collect_view()}
    </span>
}
```

## 5. Library Parity

> Compared against: Ark UI (`Highlight`).

### 5.1 Props

| Feature        | ars-ui                          | Ark UI                      | Notes                                         |
| -------------- | ------------------------------- | --------------------------- | --------------------------------------------- |
| Query          | `query: Vec<String>`            | `query: string \| string[]` | Both support multiple queries                 |
| Text           | `text`                          | `text`                      | Both libraries                                |
| Ignore case    | `ignore_case`                   | `ignoreCase`                | Both libraries                                |
| Match strategy | `match_strategy: MatchStrategy` | `exactMatch`, `matchAll`    | ars-ui uses enum; Ark uses boolean flags      |
| Locale         | `locale`                        | --                          | ars-ui addition for locale-aware case folding |

**Gaps:** None.

### 5.2 Anatomy

| Part  | ars-ui              | Ark UI              | Notes                                        |
| ----- | ------------------- | ------------------- | -------------------------------------------- |
| Root  | `Root`              | (inline rendering)  | Both produce chunks for rendering            |
| Chunk | `<mark>` / `<span>` | `<mark>` / `<span>` | Both libraries use mark for highlighted text |

**Gaps:** None.

### 5.3 Features

| Feature                        | ars-ui                            | Ark UI |
| ------------------------------ | --------------------------------- | ------ |
| Multi-query                    | Yes                               | Yes    |
| Case-insensitive               | Yes                               | Yes    |
| Fuzzy matching                 | Yes (`MatchStrategy::Fuzzy`)      | --     |
| StartsWith matching            | Yes (`MatchStrategy::StartsWith`) | --     |
| Locale-aware folding           | Yes                               | --     |
| Adjacent/overlap merge in core | Yes                               | --     |
| `dir="auto"` on root           | Yes                               | --     |

**Gaps:** None.

### 5.4 Summary

- **Overall:** Full parity — ars-ui is a superset.
- **Divergences:** ars-ui uses `MatchStrategy` enum (Contains/StartsWith/Fuzzy) while Ark uses `exactMatch`/`matchAll` booleans. ars-ui adds locale-aware Unicode case folding, fuzzy matching, core-side adjacency merging, and a BiDi-safe root via `dir="auto"`.
- **Recommended additions:** None.
