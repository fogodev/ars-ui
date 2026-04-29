//! `spec_lint_code` — enforce code-block hygiene across the component specs.
//!
//! The workspace lint policy (`missing_docs = "warn"`,
//! `missing_debug_implementations = "warn"`, plus the clippy `derivable_impls`
//! / `missing_const_for_fn` rules) is enforced on Rust source under `crates/`.
//! Component spec files contain Rust code blocks that implementers port
//! verbatim, so they need to obey the same rules — otherwise every implementer
//! has to add doc comments, `#[derive(Debug)]`, and `#[must_use]` over and
//! over again, and any divergence between two implementers' adaptations
//! creates downstream drift.
//!
//! This linter parses the Rust code blocks under `### 1.1 Props` and
//! `### 1.2 Connect / API` for every component spec under
//! `spec/components/**/*.md` (or any file/directory passed on the CLI) and
//! enforces:
//!
//! - Every `pub struct`, `pub enum`, `pub fn`, `pub const fn`, public field,
//!   and public variant has a `///` doc comment.
//! - `pub struct Props` and `pub struct Api` include `Debug` in their derive
//!   list (or have `#[derive(Debug)]` on a separate `derive` attribute, or an
//!   explicit `impl Debug` is shown alongside).
//! - `pub struct Api` includes `Clone` in its derive list.
//!
//! These rules are deliberately a subset of the full workspace lint set —
//! the linter is fast, deterministic, and does not run a Rust compiler.
//! Misses are acceptable; false positives are not. Each rule has a
//! corresponding entry in the spec template's "Code-Block Hygiene"
//! conformance checklist.

use std::{fs, path::Path};

use crate::manifest::{Error, SpecRoot};

/// CLI entry point: lint code blocks under `spec/components/`.
///
/// Always returns `Ok(String)`. The text either begins with `ok:` (no
/// findings) or with `N finding(s):` followed by per-finding lines. The
/// main.rs handler inspects the output prefix and exits non-zero when
/// findings are present, mirroring the existing `spec validate` flow.
///
/// # Errors
///
/// Returns [`Error::Io`] when a target file cannot be read.
pub fn execute(root: &SpecRoot) -> Result<String, Error> {
    let components_dir = root.path.join("components");

    let mut files = Vec::new();

    collect_md_files(&components_dir, &mut files).map_err(Error::Io)?;

    files.sort();

    // Canonicalize the spec root once so paths reported in findings can be
    // shown relative to it (the walk produces absolute canonical paths).
    let display_root = fs::canonicalize(&root.path).unwrap_or_else(|_| root.path.clone());

    let mut findings = Vec::new();

    for path in &files {
        let source = fs::read_to_string(path).map_err(Error::Io)?;

        let display = path
            .strip_prefix(&display_root)
            .unwrap_or(path)
            .display()
            .to_string();

        for finding in lint_file(&display, &source) {
            findings.push(finding);
        }
    }

    let mut out = String::new();

    if findings.is_empty() {
        out.push_str(&format!(
            "ok: {} spec file(s) checked, no code-block hygiene findings\n",
            files.len()
        ));

        return Ok(out);
    }

    out.push_str(&format!(
        "{} finding(s) across {} spec file(s):\n\n",
        findings.len(),
        files.len()
    ));

    for f in &findings {
        out.push_str(&format!("{}:{}: {}\n", f.file, f.line, f.message));
    }

    Ok(out)
}

#[derive(Debug, Clone)]
struct Finding {
    file: String,
    line: usize,
    message: String,
}

/// Collects `*.md` spec files under `root` via an iterative depth-first
/// walk. The traversal is bounded to the spec root: each candidate path is
/// canonicalized and checked to lie under the canonical root before being
/// pushed onto the stack, and symlinks are skipped so the walk cannot
/// escape via a malicious link.
fn collect_md_files(root: &Path, into: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    if !root.exists() {
        return Ok(());
    }

    let canonical_root = fs::canonicalize(root)?;

    let mut stack: Vec<std::path::PathBuf> = Vec::from([canonical_root.clone()]);

    while let Some(dir) = stack.pop() {
        let listing = fs::read_dir(&dir)?;

        for entry in listing {
            let entry = entry?;

            let file_type = entry.file_type()?;

            if file_type.is_symlink() {
                // Skip symlinks defensively.
                continue;
            }

            let candidate = entry.path();

            let Ok(canonical) = fs::canonicalize(&candidate) else {
                continue;
            };

            // Refuse to descend outside the spec root, even if the
            // filesystem layout somehow produced such a path.
            if !canonical.starts_with(&canonical_root) {
                continue;
            }

            if file_type.is_dir() {
                stack.push(canonical);
            } else if file_type.is_file()
                && canonical.extension().is_some_and(|ext| ext == "md")
                && canonical
                    .file_name()
                    .is_some_and(|name| name != "_category.md" && name != "CLAUDE.md")
            {
                into.push(canonical);
            }
        }
    }

    Ok(())
}

/// Lint a single spec file. Public for unit testing.
fn lint_file(display_path: &str, source: &str) -> Vec<Finding> {
    let blocks = extract_api_rust_blocks(source);

    let mut findings = Vec::new();

    for block in blocks {
        for (offset, message) in lint_block(&block.content) {
            findings.push(Finding {
                file: display_path.to_string(),
                line: block.start_line + offset,
                message,
            });
        }
    }

    findings
}

#[derive(Debug)]
struct CodeBlock {
    /// Spec-file line on which this block's first content line lives.
    start_line: usize,

    /// Block body, without the fence lines.
    content: String,
}

/// Extracts all fenced Rust code blocks under the explicit `### 1.1 Props`
/// and `### 1.2 Connect / API` headings.
fn extract_api_rust_blocks(source: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();

    let mut in_target_section = false;

    let mut in_block = false;

    let mut current = String::new();

    let mut current_start = 0usize;

    for (idx, line) in source.lines().enumerate() {
        let line_no = idx + 1;

        let trimmed = line.trim_start();

        if let Some((level, body)) = parse_atx_heading(trimmed) {
            // Section heading — re-evaluate target-section state.
            if level == 3 {
                in_target_section = is_target_api_heading(body);
            } else if level < 3 {
                in_target_section = false;
            }
        } else if trimmed.starts_with("##") {
            // Higher-or-equal heading (level 1 or 2) ends any target section.
            in_target_section = false;
        }

        if in_target_section {
            if !in_block && (trimmed == "```rust" || trimmed.starts_with("```rust")) {
                in_block = true;

                current.clear();

                current_start = line_no + 1;

                continue;
            }

            if in_block && trimmed == "```" {
                blocks.push(CodeBlock {
                    start_line: current_start,
                    content: core::mem::take(&mut current),
                });

                in_block = false;

                continue;
            }

            if in_block {
                current.push_str(line);

                current.push('\n');
            }
        }
    }

    blocks
}

/// Parses an ATX heading into `(level, body)`.
fn parse_atx_heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|&c| c == '#').count();

    if level == 0 || level > 6 {
        return None;
    }

    if !line[level..].starts_with(char::is_whitespace) {
        return None;
    }

    let body = line[level..].trim();

    if body.is_empty() {
        return None;
    }

    Some((level, body.trim_end_matches('#').trim_end()))
}

/// Returns whether a level-3 heading is one of the spec sections linted for
/// component public API hygiene.
fn is_target_api_heading(body: &str) -> bool {
    body == "1.1 Props" || body == "1.2 Connect / API"
}

/// Returns `(line_offset_within_block, message)` pairs for any rule violations.
fn lint_block(content: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = content.lines().collect();

    let mut findings = Vec::new();

    for (idx, raw_line) in lines.iter().enumerate() {
        let line = raw_line.trim();

        // Rule R1: `pub struct X {` or `pub struct X<...>` must have a `///`
        // doc comment on the previous non-attribute, non-blank line.
        if (line.starts_with("pub struct ") || line.starts_with("pub enum "))
            && !preceded_by_doc(&lines, idx)
        {
            let kind = if line.starts_with("pub struct ") {
                "struct"
            } else {
                "enum"
            };

            findings.push((
                idx,
                format!("public {kind} declaration is missing a `///` doc comment"),
            ));
        }

        // Rule R2: `pub struct Props` and `pub struct Api` must implement
        // `Debug` — either via `#[derive(Debug)]` or a manual
        // `impl Debug for <Name>` block within the same code block (used
        // when one of the fields is not `Debug`-derivable, e.g. a `dyn Fn`
        // closure or a `MessageFn` callback).
        if line.starts_with("pub struct Props") || line.starts_with("pub struct Api") {
            let derives = collect_derives(&lines, idx);

            let type_name = if line.starts_with("pub struct Props") {
                "Props"
            } else {
                "Api"
            };

            let has_debug =
                derives.contains(&"Debug") || has_manual_impl(&lines, "Debug", type_name);

            let has_clone =
                derives.contains(&"Clone") || has_manual_impl(&lines, "Clone", type_name);

            if !has_debug {
                findings.push((
                    idx,
                    format!(
                        "`{type_name}` must implement `Debug` — add it to \
                         `#[derive(...)]` or provide an `impl Debug for {type_name}` \
                         block (workspace `missing_debug_implementations` lint)"
                    ),
                ));
            }

            if line.starts_with("pub struct Api") && !has_clone {
                findings.push((
                    idx,
                    "`Api` must implement `Clone` — add it to `#[derive(...)]` \
                     or provide an `impl Clone for Api` block, so adapters can \
                     compose it freely"
                        .to_string(),
                ));
            }
        }

        // Rule R3: `pub fn` and `pub const fn` need a `///` above. Skip
        // function bodies — only declaration lines are checked.
        if (line.starts_with("pub fn ") || line.starts_with("pub const fn "))
            && !preceded_by_doc(&lines, idx)
        {
            findings.push((
                idx,
                "public fn declaration is missing a `///` doc comment".to_string(),
            ));
        }

        // Rule R4: public fields (lines like `pub name: Type,`) inside a
        // struct body must have a `///` above. Skip lines inside fn bodies
        // by requiring the line ends with `,` or is `pub field: Type` form
        // (not `pub fn ...`).
        if line.starts_with("pub ")
            && !line.starts_with("pub fn ")
            && !line.starts_with("pub const fn ")
            && !line.starts_with("pub struct ")
            && !line.starts_with("pub enum ")
            && !line.starts_with("pub mod ")
            && !line.starts_with("pub use ")
            && line.contains(": ")
            && line.trim_end().ends_with(',')
            && !preceded_by_doc(&lines, idx)
        {
            findings.push((
                idx,
                "public field is missing a `///` doc comment".to_string(),
            ));
        }
    }

    // Enum variants: detect `pub enum Name {` then walk the body until the
    // matching `}` and require each variant identifier to be doc-commented.
    findings.extend(check_enum_variants(&lines));

    findings
}

/// Returns `true` if `lines[idx]` is preceded (after skipping blank lines and
/// `#[...]` attribute lines) by a `///` doc comment.
fn preceded_by_doc(lines: &[&str], idx: usize) -> bool {
    let mut cursor = idx;

    while cursor > 0 {
        cursor -= 1;

        let prev = lines[cursor].trim();

        if prev.is_empty() {
            return false;
        }

        if prev.starts_with("#[") || prev.starts_with("#![") {
            continue;
        }

        return prev.starts_with("///") || prev.starts_with("//!");
    }

    false
}

/// Returns `true` if the code block contains a manual `impl <trait> for
/// <type_name>` line. Used as fallback evidence that a trait is implemented
/// when the type cannot rely on `#[derive(...)]` (e.g. an `Api` struct that
/// holds a `dyn Fn` closure or a `MessageFn` callback that lacks `Debug`).
///
/// Matches both lifetime-parameterized (`impl Debug for Api<'_>`,
/// `impl<'a> Debug for Api<'a>`) and plain (`impl Debug for Api`) forms.
fn has_manual_impl(lines: &[&str], trait_name: &str, type_name: &str) -> bool {
    let needles = [
        format!("impl {trait_name} for {type_name}"),
        format!("impl {trait_name} for {type_name}<"),
        format!("> {trait_name} for {type_name}"),
    ];

    lines
        .iter()
        .any(|line| needles.iter().any(|n| line.contains(n.as_str())))
}

/// Collects derive identifiers from any `#[derive(...)]` attribute lines in
/// the contiguous attribute block immediately above `idx` (interleaved with
/// other `#[...]` attributes is allowed).
fn collect_derives<'a>(lines: &'a [&str], idx: usize) -> Vec<&'a str> {
    let mut out = Vec::new();

    let mut cursor = idx;

    while cursor > 0 {
        cursor -= 1;

        let prev = lines[cursor].trim();

        if prev.is_empty() || prev.starts_with("///") || prev.starts_with("//!") {
            break;
        }

        if let Some(rest) = prev.strip_prefix("#[derive(")
            && let Some(inner) = rest.strip_suffix(")]")
        {
            for tok in inner.split(',') {
                let tok = tok.trim();

                if !tok.is_empty() {
                    out.push(tok);
                }
            }
        } else if !prev.starts_with("#[") && !prev.starts_with("#![") {
            break;
        }
    }

    out
}

/// Walks `pub enum X { ... }` blocks and emits findings for variants
/// without a `///` doc comment.
fn check_enum_variants(lines: &[&str]) -> Vec<(usize, String)> {
    let mut findings = Vec::new();

    let mut iter = lines.iter().enumerate().peekable();

    while let Some((idx, line)) = iter.next() {
        let trimmed = line.trim();

        if !trimmed.starts_with("pub enum ") {
            continue;
        }

        // Find the opening brace (may be on same or next non-blank line).
        let mut depth = if trimmed.contains('{') { 1usize } else { 0 };

        if depth == 0 {
            // Look ahead for `{` line.
            for (_, peek) in iter.by_ref() {
                if peek.contains('{') {
                    depth = 1;

                    break;
                }
            }
        }

        let mut cursor = idx + 1;

        while depth > 0 && cursor < lines.len() {
            let body = lines[cursor];

            let body_trimmed = body.trim();

            for ch in body.chars() {
                match ch {
                    '{' => depth += 1,

                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }

                    _ => {}
                }
            }

            // Variant detection: an identifier line that does NOT start with
            // `#[`, `///`, `//`, `}`, or `{`. Allow trailing `,`, `(...)`,
            // or `{ ... }` (struct-style variant).
            let is_variant_line = !body_trimmed.is_empty()
                && !body_trimmed.starts_with("///")
                && !body_trimmed.starts_with("//")
                && !body_trimmed.starts_with("#[")
                && !body_trimmed.starts_with('{')
                && !body_trimmed.starts_with('}')
                && body_trimmed
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_uppercase());

            if is_variant_line && !preceded_by_doc(lines, cursor) {
                findings.push((
                    cursor,
                    "enum variant is missing a `///` doc comment".to_string(),
                ));
            }

            cursor += 1;
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(content: &str) -> Vec<String> {
        lint_file("test.md", content)
            .into_iter()
            .map(|f| f.message)
            .collect()
    }

    #[test]
    fn extracts_explicit_props_and_api_sections() {
        let md = "\
### 1.1 Props

```rust
pub struct Props {}
```

### 1.2 Connect / API

```rust
pub struct Api {}
```
";

        assert_eq!(extract_api_rust_blocks(md).len(), 2);
    }

    #[test]
    fn flags_missing_struct_doc() {
        let md = "\
### 1.1 Props

```rust
#[derive(Debug)]
pub struct Props {
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("public struct declaration is missing"))
        );
    }

    #[test]
    fn passes_when_struct_has_doc() {
        let md = "\
### 1.1 Props

```rust
/// Props for the component.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Props {
    /// The id.
    pub id: String,
}
```
";

        assert!(run(md).is_empty(), "expected clean run, got {:?}", run(md));
    }

    #[test]
    fn flags_missing_debug_on_api() {
        let md = "\
### 1.2 Connect / API

```rust
/// API for the component.
#[derive(Clone)]
pub struct Api {
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("`Api` must implement `Debug`"))
        );
    }

    #[test]
    fn flags_missing_clone_on_api() {
        let md = "\
### 1.2 Connect / API

```rust
/// API for the component.
#[derive(Debug)]
pub struct Api {
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("`Api` must implement `Clone`"))
        );
    }

    #[test]
    fn accepts_manual_impl_debug_for_api() {
        // Real-world case: `Api` holds a non-Debug field (e.g. `MessageFn`
        // with a closure), so `Debug` is provided via a manual impl that
        // uses `finish_non_exhaustive`.
        let md = "\
### 1.2 Connect / API

```rust
/// API for the component.
#[derive(Clone)]
pub struct Api {
    messages: Messages,
}

impl Debug for Api {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(\"Api\").finish_non_exhaustive()
    }
}
```
";

        let messages = run(md);

        assert!(
            !messages
                .iter()
                .any(|m| m.contains("`Api` must implement `Debug`")),
            "expected no Debug finding (manual impl present), got {messages:?}"
        );
    }

    #[test]
    fn accepts_manual_impl_clone_with_lifetime() {
        let md = "\
### 1.2 Connect / API

```rust
/// API for the component.
#[derive(Debug)]
pub struct Api<'a> {
    state: &'a State,
}

impl<'a> Clone for Api<'a> {
    fn clone(&self) -> Self { Self { state: self.state } }
}
```
";

        let messages = run(md);

        assert!(
            !messages
                .iter()
                .any(|m| m.contains("`Api` must implement `Clone`")),
            "expected no Clone finding (manual impl with lifetime present), got {messages:?}"
        );
    }

    #[test]
    fn flags_missing_field_doc() {
        let md = "\
### 1.1 Props

```rust
/// Props.
#[derive(Debug)]
pub struct Props {
    pub id: String,
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("public field is missing"))
        );
    }

    #[test]
    fn flags_missing_variant_doc() {
        let md = "\
### 1.2 Connect / API

```rust
/// Parts.
pub enum Part {
    Root,
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("enum variant is missing"))
        );
    }

    #[test]
    fn ignores_blocks_outside_target_sections() {
        let md = "\
### 6.1 Examples

```rust
pub struct Demo {}
```
";

        assert!(run(md).is_empty());
    }

    #[test]
    fn ignores_state_machine_sections_that_share_numbering_prefix() {
        let md = "\
### 1.1 States

```rust
pub struct InternalState {
    pub selected: bool,
}
```

### 1.2 Events

```rust
pub enum InternalEvent {
    Open,
}
```
";

        assert!(run(md).is_empty());
    }

    #[test]
    fn does_not_inherit_derives_from_previous_item() {
        let md = "\
### 1.1 Props

```rust
/// Prior helper.
#[derive(Clone, Debug)]
pub struct Helper;

/// Props for the component.
#[cfg(feature = \"std\")]
pub struct Props {
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("`Props` must implement `Debug`"))
        );
    }

    #[test]
    fn flags_missing_fn_doc() {
        let md = "\
### 1.2 Connect / API

```rust
/// Api.
#[derive(Clone, Debug)]
pub struct Api { }

impl Api {
    pub const fn new() -> Self { Self {} }
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("public fn declaration is missing"))
        );
    }
}
