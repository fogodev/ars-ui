#![doc = r#"
Companion stylesheet for `ars-core`.

Applications should ship [`ars-base.css`](../ars-base.css) alongside their built assets and add it
to the document with a standard stylesheet link:

```html
<link rel="stylesheet" href="ars-base.css">
```

This module is intentionally empty. It exists to document the companion stylesheet in generated
Rust docs while keeping the CSS file itself as the source of truth.

If the `embedded-css` feature is enabled, this module also exposes [`ARS_BASE_CSS`], which embeds
the stylesheet into the compiled artifact for consumers that prefer programmatic asset packaging.
That feature is opt-in to avoid making binary embedding the default delivery path.

```css
"#]
#![doc = include_str!("../ars-base.css")]
#![doc = "```"]

/// Embedded contents of `ars-base.css` for opt-in programmatic asset packaging.
#[cfg(feature = "embedded-css")]
pub const ARS_BASE_CSS: &str = include_str!("../ars-base.css");
