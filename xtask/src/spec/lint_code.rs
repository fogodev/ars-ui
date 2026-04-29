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

        if in_block {
            if trimmed.trim_end() == "```" {
                blocks.push(CodeBlock {
                    start_line: current_start,
                    content: core::mem::take(&mut current),
                });

                in_block = false;

                continue;
            }

            current.push_str(line);

            current.push('\n');

            continue;
        }

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

        if in_target_section && (trimmed == "```rust" || trimmed.starts_with("```rust")) {
            in_block = true;

            current.clear();

            current_start = line_no + 1;
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

            let has_debug = derives.iter().any(|derive| derive == "Debug")
                || has_manual_impl(&lines, "Debug", type_name);

            let has_clone = derives.iter().any(|derive| derive == "Clone")
                || has_manual_impl(&lines, "Clone", type_name);

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

        // Rule R3: any public fn declaration needs a `///` above. Covers
        // every combination of `const`, `async`, `unsafe`, and
        // `extern "ABI"` modifiers between `pub` and `fn`. Only declaration
        // lines are checked, not function bodies.
        if is_pub_fn_decl(line) && !preceded_by_doc(&lines, idx) {
            findings.push((
                idx,
                "public fn declaration is missing a `///` doc comment".to_string(),
            ));
        }

        // Rule R4: public fields (lines like `pub name: Type`) inside a
        // struct body must have a `///` above. Skip every public fn shape
        // (covered by R3) and other top-level public items.
        if line.starts_with("pub ")
            && !is_pub_fn_decl(line)
            && !line.starts_with("pub struct ")
            && !line.starts_with("pub enum ")
            && !line.starts_with("pub mod ")
            && !line.starts_with("pub use ")
            && line.contains(": ")
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

/// Returns `true` if `line` is a public fn declaration in any of its
/// permitted shapes:
///
/// - `pub fn name(...)`
/// - `pub const fn name(...)`
/// - `pub async fn name(...)`
/// - `pub unsafe fn name(...)`
/// - `pub extern "ABI" fn name(...)` (or `pub extern fn ...`)
/// - any combination of `const`, `async`, `unsafe`, `extern "ABI"` modifiers
///   between `pub` and `fn` (in source-legal orderings)
///
/// Used by Rule R3 (mandatory `///` doc comment on public fn) and by
/// Rule R4 (exclude every public fn shape from the field-doc check).
///
/// The check tokenises the line on whitespace and walks the modifier
/// sequence between `pub` and `fn`. An unrecognised modifier
/// short-circuits to `false`, which keeps R4 from over-broadly excluding
/// declarations that aren't actually fn lines.
fn is_pub_fn_decl(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("pub ") else {
        return false;
    };

    let mut tokens = rest.split_whitespace();

    while let Some(tok) = tokens.next() {
        match tok {
            "fn" => return true,
            "const" | "async" | "unsafe" => {}
            "extern" => match tokens.next() {
                Some("fn") => return true,
                Some(next) if next.starts_with('"') => {}
                _ => return false,
            },
            _ => return false,
        }
    }

    false
}

/// Returns `true` if `lines[idx]` is preceded (after skipping blank lines and
/// `#[...]` attribute lines) by a `///` doc comment.
fn preceded_by_doc(lines: &[&str], idx: usize) -> bool {
    let mut cursor = idx;

    while cursor > 0 {
        if let Some((start, ..)) = attribute_span_before(lines, cursor) {
            cursor = start;
            continue;
        }

        let prev = lines[cursor - 1].trim();

        if prev.is_empty() {
            return false;
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
    let mut current_impl_header = None::<String>;

    for line in code_lines_without_comments_and_strings(lines) {
        let trimmed = line.trim();

        if trimmed.starts_with("impl") {
            current_impl_header = Some(trimmed.to_string());
        } else if let Some(header) = &mut current_impl_header {
            header.push(' ');
            header.push_str(trimmed);
        }

        let Some(header) = &current_impl_header else {
            continue;
        };

        if manual_impl_line_matches(header, trait_name, type_name) {
            return true;
        }

        if trimmed.contains('{') || trimmed.ends_with(';') {
            current_impl_header = None;
        }
    }

    false
}

/// Returns whether a comment-free line contains `impl <trait_name> for
/// <type_name>` with identifier boundaries around both names.
fn manual_impl_line_matches(line: &str, trait_name: &str, type_name: &str) -> bool {
    let Some(for_idx) = line.find(" for ") else {
        return false;
    };

    let impl_prefix = &line[..for_idx];
    let implemented_type = line[for_idx + " for ".len()..].trim_start();

    impl_trait_matches(impl_prefix, trait_name)
        && implemented_type.starts_with(type_name)
        && implemented_type[type_name.len()..]
            .chars()
            .next()
            .is_none_or(|ch| !is_rust_identifier_continue(ch))
}

/// Returns whether the actual trait path in an `impl ... for` prefix matches
/// the linted trait.
fn impl_trait_matches(impl_prefix: &str, trait_name: &str) -> bool {
    let Some(rest) = impl_prefix.trim().strip_prefix("impl") else {
        return false;
    };

    let rest = rest.trim_start();
    let rest = if rest.starts_with('<') {
        let Some(generics_end) = matching_angle_end(rest) else {
            return false;
        };

        rest[generics_end + 1..].trim_start()
    } else {
        rest
    };

    let Some(trait_path) = rest.split_whitespace().next_back() else {
        return false;
    };

    trait_path_segment_matches(trait_path, trait_name)
}

/// Returns the byte index of the matching `>` for a leading `<...>` group.
fn matching_angle_end(source: &str) -> Option<usize> {
    let mut depth = 0usize;

    for (idx, ch) in source.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth = depth.checked_sub(1)?;

                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }

    None
}

/// Returns whether the last path segment of `trait_path` is `trait_name`.
fn trait_path_segment_matches(trait_path: &str, trait_name: &str) -> bool {
    let trait_path = trait_path.split('<').next().unwrap_or(trait_path);
    let trait_path = trait_path.trim_end_matches('{').trim_end();

    if trait_path == trait_name {
        return true;
    }

    let Some((_, segment)) = trait_path.rsplit_once("::") else {
        return false;
    };

    segment == trait_name
}

/// Returns whether a character can continue a Rust identifier.
fn is_rust_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

/// Collects derive identifiers from any `#[derive(...)]` attribute lines in
/// the contiguous attribute block immediately above `idx` (interleaved with
/// other `#[...]` attributes is allowed).
fn collect_derives(lines: &[&str], idx: usize) -> Vec<String> {
    let mut out = Vec::new();

    let mut cursor = idx;

    while cursor > 0 {
        let prev = lines[cursor - 1].trim();

        if prev.is_empty() || prev.starts_with("///") || prev.starts_with("//!") {
            break;
        }

        let Some((start, _, attr)) = attribute_span_before(lines, cursor) else {
            break;
        };

        if let Some(rest) = attr.trim().strip_prefix("#[derive(")
            && let Some(inner) = rest.strip_suffix(")]")
        {
            out.extend(
                inner
                    .split(',')
                    .map(str::trim)
                    .filter(|tok| !tok.is_empty())
                    .map(str::to_string),
            );
        }

        cursor = start;
    }

    out
}

/// Returns the contiguous attribute span immediately before `idx`, including
/// multiline attributes formatted by rustfmt.
fn attribute_span_before<'a>(lines: &'a [&'a str], idx: usize) -> Option<(usize, usize, String)> {
    if idx == 0 {
        return None;
    }

    let end = idx - 1;
    let end_line = lines[end].trim();

    if end_line.is_empty() || end_line.starts_with("///") || end_line.starts_with("//!") {
        return None;
    }

    if !end_line.contains(']') && !end_line.starts_with("#[") && !end_line.starts_with("#![") {
        return None;
    }

    for start in (0..=end).rev() {
        let line = lines[start].trim();

        if line.is_empty() || line.starts_with("///") || line.starts_with("//!") {
            return None;
        }

        if line.starts_with("#[") || line.starts_with("#![") {
            let span_lines = &lines[start..=end];

            if span_lines[1..]
                .iter()
                .any(|line| is_rust_item_line(line.trim()))
            {
                return None;
            }

            let attr = span_lines.join("\n");

            return attr_brackets_are_balanced(&attr).then_some((start, end, attr));
        }
    }

    None
}

/// Returns whether a line is clearly Rust item code rather than an attribute
/// continuation.
fn is_rust_item_line(line: &str) -> bool {
    line.starts_with("pub ")
        || line.starts_with("impl ")
        || line.starts_with("struct ")
        || line.starts_with("enum ")
        || line.starts_with("fn ")
        || line.starts_with("const ")
        || line.starts_with("mod ")
        || line.starts_with("use ")
        || line.ends_with(';')
}

/// Returns whether `#[...]` delimiters are balanced in an attribute span.
fn attr_brackets_are_balanced(attr: &str) -> bool {
    let mut depth = 0usize;

    for ch in attr.chars() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth = depth.checked_sub(1).unwrap_or(usize::MAX);

                if depth == usize::MAX {
                    return false;
                }
            }
            _ => {}
        }
    }

    depth == 0
}

/// Removes Rust comments and string literal contents before doing string-based
/// lint probes.
fn code_lines_without_comments_and_strings(lines: &[&str]) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut in_block_comment = false;
    let mut in_raw_string: Option<usize> = None;

    for line in lines {
        let mut code = String::new();
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            if let Some(hash_count) = in_raw_string {
                if ch == '"' {
                    let mut matched = true;
                    for _ in 0..hash_count {
                        if chars.peek() != Some(&'#') {
                            matched = false;
                            break;
                        }

                        chars.next();
                    }

                    if matched {
                        in_raw_string = None;
                    }
                }

                continue;
            }

            if in_block_comment {
                if ch == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    in_block_comment = false;
                }

                continue;
            }

            if ch == '/' {
                match chars.peek() {
                    Some('/') => break,
                    Some('*') => {
                        chars.next();
                        in_block_comment = true;
                        continue;
                    }
                    _ => {}
                }
            }

            if ch == '"' {
                let mut escaped = false;

                for inner in chars.by_ref() {
                    if escaped {
                        escaped = false;
                    } else if inner == '\\' {
                        escaped = true;
                    } else if inner == '"' {
                        break;
                    }
                }

                continue;
            }

            if ch == 'r' {
                let mut clone = chars.clone();
                let mut hash_count = 0usize;

                while clone.peek() == Some(&'#') {
                    hash_count += 1;
                    clone.next();
                }

                if clone.peek() == Some(&'"') {
                    for _ in 0..hash_count {
                        chars.next();
                    }

                    chars.next();
                    in_raw_string = Some(hash_count);

                    continue;
                }
            }

            code.push(ch);
        }

        out.push(code);
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
        let mut cursor = idx + 1;

        if depth == 0 {
            // Look ahead for `{` line.
            for (peek_idx, peek) in iter.by_ref() {
                if peek.contains('{') {
                    depth = 1;
                    cursor = peek_idx + 1;

                    break;
                }
            }
        }

        let mut tuple_payload_depth = 0usize;

        while depth > 0 && cursor < lines.len() {
            let body = lines[cursor];

            let body_trimmed = body.trim();
            let inside_tuple_payload = tuple_payload_depth > 0;

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

            for ch in body.chars() {
                match ch {
                    '(' => tuple_payload_depth += 1,
                    ')' => {
                        tuple_payload_depth = tuple_payload_depth.saturating_sub(1);
                    }
                    _ => {}
                }
            }

            // Variant detection: an identifier line that does NOT start with
            // `#[`, `///`, `//`, `}`, or `{`. Allow trailing `,`, `(...)`,
            // or `{ ... }` (struct-style variant).
            let is_variant_line = !body_trimmed.is_empty()
                && !inside_tuple_payload
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
    fn accepts_multiline_manual_impl_header() {
        let md = "\
### 1.2 Connect / API

```rust
/// API for the component.
pub struct Api<T> {
    state: T,
}

impl<T>
    Debug
    for Api<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(\"Api\").finish_non_exhaustive()
    }
}

impl<T>
    Clone
    for Api<T>
where
    T: Clone,
{
    fn clone(&self) -> Self { Self { state: self.state.clone() } }
}
```
";

        let messages = run(md);

        assert!(
            !messages
                .iter()
                .any(|m| m.contains("`Api` must implement `Debug`")),
            "expected no Debug finding for multiline manual impl, got {messages:?}"
        );
        assert!(
            !messages
                .iter()
                .any(|m| m.contains("`Api` must implement `Clone`")),
            "expected no Clone finding for multiline manual impl, got {messages:?}"
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
    fn flags_missing_field_doc_without_trailing_comma() {
        let md = "\
### 1.1 Props

```rust
/// Props.
#[derive(Debug)]
pub struct Props {
    pub id: String
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
    fn enum_body_scan_starts_after_late_opening_brace() {
        let md = "\
### 1.2 Connect / API

```rust
/// Parts.
pub enum Part
where
    Self: Sized,
{
    /// Root part.
    Root,
}
```
";

        let messages = run(md);

        assert!(
            !messages
                .iter()
                .any(|m| m.contains("enum variant is missing")),
            "expected no enum variant findings, got {messages:?}"
        );
    }

    #[test]
    fn ignores_tuple_variant_payload_lines() {
        let md = "\
### 1.2 Connect / API

```rust
/// Parts.
pub enum Part {
    /// Root part.
    Root(
        String,
    ),
}
```
";

        let messages = run(md);

        assert!(
            !messages
                .iter()
                .any(|m| m.contains("enum variant is missing")),
            "expected no enum variant findings, got {messages:?}"
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
    fn ignores_atx_heading_like_lines_inside_rust_blocks() {
        let md = "\
### 1.2 Connect / API

```rust
/// Api for the component.
#[derive(Clone, Debug)]
pub struct Api {
}

const SAMPLE: &str = r#\"
### Not a markdown heading
\"#;

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

    #[test]
    fn accepts_closing_rust_fence_with_trailing_whitespace() {
        let md = "### 1.2 Connect / API\n\n```rust\npub fn missing_doc() {}\n```   \n";

        let messages = run(md);

        assert!(
            messages
                .iter()
                .any(|m| m.contains("public fn declaration is missing")),
            "expected lint findings from block closed with trailing whitespace, got {messages:?}"
        );
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
    fn accepts_multiline_derive_attributes() {
        let md = "\
### 1.2 Connect / API

```rust
/// Api for the component.
#[derive(
    Clone,
    Debug,
)]
pub struct Api {
}
```
";

        assert!(run(md).is_empty());
    }

    #[test]
    fn ignores_commented_manual_impls() {
        let md = "\
### 1.2 Connect / API

```rust
/// Api for the component.
pub struct Api {
}

// impl Debug for Api {}
/*
impl Clone for Api {}
*/
```
";

        let findings = run(md);

        assert!(
            findings
                .iter()
                .any(|m| m.contains("`Api` must implement `Debug`"))
        );
        assert!(
            findings
                .iter()
                .any(|m| m.contains("`Api` must implement `Clone`"))
        );
    }

    #[test]
    fn ignores_manual_impls_inside_string_literals() {
        let md = "\
### 1.2 Connect / API

```rust
/// Api for the component.
pub struct Api {
}

const NORMAL: &str = \"impl Clone for Api {}\";
const RAW: &str = r#\"
impl Debug for Api {}
\"#;
```
";

        let findings = run(md);

        assert!(
            findings
                .iter()
                .any(|m| m.contains("`Api` must implement `Debug`"))
        );
        assert!(
            findings
                .iter()
                .any(|m| m.contains("`Api` must implement `Clone`"))
        );
    }

    #[test]
    fn ignores_manual_impls_for_prefixed_type_names() {
        let md = "\
### 1.2 Connect / API

```rust
/// Api for the component.
pub struct Api {
}

impl Debug for ApiWrapper {}
impl Clone for Api2 {}
```
";

        let findings = run(md);

        assert!(
            findings
                .iter()
                .any(|m| m.contains("`Api` must implement `Debug`"))
        );
        assert!(
            findings
                .iter()
                .any(|m| m.contains("`Api` must implement `Clone`"))
        );
    }

    #[test]
    fn matches_manual_impl_trait_name_not_generic_bound() {
        let md = "\
### 1.2 Connect / API

```rust
/// Api for the component.
pub struct Api<T> {
    _marker: core::marker::PhantomData<T>,
}

impl<T: Debug> Clone for Api<T> {}
```
";

        let findings = run(md);

        assert!(
            findings
                .iter()
                .any(|m| m.contains("`Api` must implement `Debug`"))
        );
        assert!(
            !findings
                .iter()
                .any(|m| m.contains("`Api` must implement `Clone`"))
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

    #[test]
    fn flags_missing_async_fn_doc() {
        // Regression for the P2 review on PR #621: Rule R3 originally only
        // checked `pub fn` and `pub const fn`, missing every other modifier
        // shape. An undocumented `pub async fn` must now be flagged just
        // like a plain `pub fn`.
        let md = "\
### 1.2 Connect / API

```rust
/// Api.
#[derive(Clone, Debug)]
pub struct Api { }

impl Api {
    pub async fn fetch(&self) -> u32 { 0 }
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("public fn declaration is missing")),
            "expected `pub async fn fetch` to be flagged"
        );
    }

    #[test]
    fn accepts_documented_async_fn() {
        // Companion to `flags_missing_async_fn_doc`: a documented
        // `pub async fn` must NOT trigger the missing-doc rule, and must
        // not be misinterpreted as a public field by Rule R4.
        let md = "\
### 1.2 Connect / API

```rust
/// Api.
#[derive(Clone, Debug)]
pub struct Api { }

impl Api {
    /// Fetches the value asynchronously.
    pub async fn fetch(&self) -> u32 { 0 }
}
```
";

        let messages = run(md);
        assert!(
            !messages
                .iter()
                .any(|m| m.contains("public fn declaration is missing")),
            "documented async fn must not be flagged, got {messages:?}"
        );
        assert!(
            !messages
                .iter()
                .any(|m| m.contains("public field is missing")),
            "documented async fn must not be misclassified as a field, got {messages:?}"
        );
    }

    #[test]
    fn flags_missing_unsafe_fn_doc() {
        let md = "\
### 1.2 Connect / API

```rust
/// Api.
#[derive(Clone, Debug)]
pub struct Api { }

impl Api {
    pub unsafe fn raw(&self) {}
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("public fn declaration is missing")),
            "expected `pub unsafe fn raw` to be flagged"
        );
    }

    #[test]
    fn flags_missing_extern_fn_doc() {
        let md = "\
### 1.2 Connect / API

```rust
/// Api.
#[derive(Clone, Debug)]
pub struct Api { }

impl Api {
    pub extern \"C\" fn callback() {}
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("public fn declaration is missing")),
            "expected `pub extern \"C\" fn callback` to be flagged"
        );
    }

    #[test]
    fn flags_missing_extern_fn_without_abi_doc() {
        // `pub extern fn` (without an ABI string literal) defaults to
        // `extern "C"`. Covers the branch in `is_pub_fn_decl` where the
        // token after `extern` is `fn`, not a string literal.
        let md = "\
### 1.2 Connect / API

```rust
/// Api.
#[derive(Clone, Debug)]
pub struct Api { }

impl Api {
    pub extern fn callback() {}
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("public fn declaration is missing")),
            "expected `pub extern fn callback` (no ABI) to be flagged"
        );
    }

    #[test]
    fn flags_missing_combined_modifier_fn_doc() {
        // Rule R3 must accept any source-legal combination of fn modifiers
        // (here `unsafe extern "C"`), not just plain `pub fn` /
        // `pub const fn`.
        let md = "\
### 1.2 Connect / API

```rust
/// Api.
#[derive(Clone, Debug)]
pub struct Api { }

impl Api {
    pub unsafe extern \"C\" fn trampoline() {}
}
```
";

        assert!(
            run(md)
                .iter()
                .any(|m| m.contains("public fn declaration is missing")),
            "expected `pub unsafe extern \"C\" fn trampoline` to be flagged"
        );
    }
}
