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

`Highlight` is a pure computation utility that splits text into highlighted and non-highlighted chunks based on a search query. It has no state machine — the adapter renders `<mark>` elements for highlighted chunks.

## 1. API

### 1.1 Props

```rust
/// Props for the `Highlight` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Props {
    /// The search queries to highlight. Supports a single query string or
    /// multiple queries to highlight simultaneously (e.g., multiple search
    /// terms). When multiple queries are provided, all matches from all
    /// queries are highlighted; overlapping matches are merged.
    pub query: Vec<String>,
    /// The full text to search within.
    pub text: String,
    /// Case-insensitive matching (default: true).
    pub ignore_case: bool,
    /// Matching strategy.
    pub match_strategy: MatchStrategy,
}

/// The strategy to use for matching the query within the text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MatchStrategy {
    /// Highlight all occurrences of the query within the text.
    Contains,
    /// Highlight only if the text starts with the query.
    StartsWith,
    /// Highlight individual characters for fuzzy matching.
    Fuzzy,
}
```

### 1.2 Connect / API

`Highlight` uses a function-based API that returns text chunks rather than the standard `ConnectApi` / `AttrMap` pattern. The adapter renders each chunk as a DOM element.

```rust
/// A chunk of text that has been highlighted or not.
#[derive(Clone, Debug, PartialEq)]
pub struct HighlightChunk<'a> {
    pub text: &'a str,
    pub highlighted: bool,
}

/// Returns the text split into alternating highlighted/non-highlighted segments.
///
/// When `ignore_case` is `true`, the implementation MUST use Unicode case folding
/// rather than ASCII `to_lowercase()`. If `locale` is provided, use locale-aware
/// case folding via ICU4X `CaseMapper::fold_turkic()` for Turkic locales. Otherwise,
/// use default Unicode case folding (`CaseMapper::fold()`). This ensures correct
/// matching for Turkish dotted-I (İ/i vs I/ı), German eszett (ß/SS), and other
/// locale-specific case mappings.
pub fn highlight_chunks<'a>(props: &'a Props, locale: &Locale) -> Vec<HighlightChunk<'a>> {
    // Returns the text split into alternating highlighted/non-highlighted segments.
    // When queries are empty or all empty strings, returns the full text as a single
    // non-highlighted chunk. When multiple queries match overlapping regions, the
    // overlapping ranges are merged into a single highlighted chunk.
}
```

## 2. Anatomy

```text
Highlight
└── Root        <span>   data-ars-scope="highlight" data-ars-part="root"
    └── Chunk   <mark> | <span>   (×N, per highlight_chunks output)
```

| Part  | Element           | Key Attributes                                                          |
| ----- | ----------------- | ----------------------------------------------------------------------- |
| Root  | `<span>`          | `data-ars-scope="highlight"`, `data-ars-part="root"`                    |
| Chunk | `<mark>`/`<span>` | `data-ars-part="highlight-chunk"`, `data-ars-highlighted="true\|false"` |

Adapters render each `HighlightChunk` as a `<mark>` (when `highlighted == true`) or `<span>` (when `highlighted == false`).

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- `<mark>` elements are used for highlighted text. Screen readers announce `<mark>` content as "highlighted" in most browser/AT combinations.
- No ARIA attributes needed — the semantic `<mark>` element conveys the highlighting.
- For custom styling, adapters set `data-ars-part="highlight-chunk"` and `data-ars-highlighted="true|false"` on each chunk.
- When using fuzzy matching (`MatchStrategy::Fuzzy`), many small `<mark>` elements may be generated. Each `<mark>` is announced as 'highlighted' by screen readers, which can create excessive verbosity. For fuzzy matches with many small highlights, adapters SHOULD consolidate adjacent highlighted chunks into a single `<mark>` element, or provide a summary announcement (e.g., 'N characters matched') instead of wrapping each character individually.

### 3.2 Bidirectional Text

When highlighting text in mixed-direction content (e.g., English query in Arabic text), chunk boundaries may disrupt BiDi ordering. The adapter SHOULD set `dir="auto"` on the root container element, and consider applying `unicode-bidi: isolate` on individual `<mark>` and `<span>` elements to prevent chunk boundaries from affecting text reordering.

## 4. Adapter Rendering

```rust
// Leptos example:
view! {
    <span data-ars-scope="highlight" data-ars-part="root">
        {highlight_chunks(&props).into_iter().map(|chunk| {
            if chunk.highlighted {
                view! { <mark>{chunk.text}</mark> }
            } else {
                view! { <span>{chunk.text}</span> }
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

| Feature              | ars-ui                            | Ark UI |
| -------------------- | --------------------------------- | ------ |
| Multi-query          | Yes                               | Yes    |
| Case-insensitive     | Yes                               | Yes    |
| Fuzzy matching       | Yes (`MatchStrategy::Fuzzy`)      | --     |
| StartsWith matching  | Yes (`MatchStrategy::StartsWith`) | --     |
| Locale-aware folding | Yes                               | --     |

**Gaps:** None.

### 5.4 Summary

- **Overall:** Full parity -- ars-ui is a superset.
- **Divergences:** ars-ui uses `MatchStrategy` enum (Contains/StartsWith/Fuzzy) while Ark uses `exactMatch`/`matchAll` booleans. ars-ui adds locale-aware Unicode case folding and fuzzy matching.
- **Recommended additions:** None.
