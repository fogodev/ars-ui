#![doc = r#"
Companion stylesheet for `ars-interactions`.

Applications should ship [`ars-interactions.css`](../ars-interactions.css) alongside their built
assets and add it to the document with a standard stylesheet link:

```html
<link rel="stylesheet" href="ars-interactions.css">
```

This stylesheet provides forced-colors and high-contrast media query rules for interaction data
attributes (`data-ars-focus-visible`, `data-ars-pressed`, `data-ars-disabled`, `data-ars-dragging`,
`data-ars-state~="selected"`). See `spec/foundation/05-interactions.md` §10.

This module is intentionally empty. It exists to document the companion stylesheet in generated
Rust docs while keeping the CSS file itself as the source of truth.

If the `embedded-css` feature is enabled, this module also exposes [`ARS_INTERACTIONS_CSS`], which
embeds the stylesheet into the compiled artifact for consumers that prefer programmatic asset
packaging. That feature is opt-in to avoid making binary embedding the default delivery path.

```css
"#]
#![doc = include_str!("../ars-interactions.css")]
#![doc = "```"]

/// Embedded contents of `ars-interactions.css` for opt-in programmatic asset packaging.
#[cfg(feature = "embedded-css")]
pub const ARS_INTERACTIONS_CSS: &str = include_str!("../ars-interactions.css");
