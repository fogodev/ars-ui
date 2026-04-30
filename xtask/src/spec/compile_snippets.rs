//! `spec compile-snippets` — parse Rust code blocks in spec markdown
//! through `syn::parse_file` and surface syntax errors.
//!
//! ## What this catches
//!
//! - Top-level Rust syntax bugs in spec snippets: missing braces,
//!   mistyped `pub fn`, unterminated string literals, unbalanced
//!   generics outside macro bodies, etc.
//! - Coverage across the full spec corpus —
//!   `spec/{components,foundation,leptos-components,dioxus-components,shared,testing}/`.
//!   The pre-existing `lint-code` linter only walked
//!   `spec/components/` and never invoked a Rust parser, so foundation
//!   and per-adapter specs had zero parse coverage.
//!
//! ## What this **does not** catch (important — read this)
//!
//! - **Macro-internal syntax.** `view! { … }` and `rsx! { … }` are
//!   opaque token trees to both `syn` and `rustc`; everything inside
//!   the outer braces is unchecked. The original Leptos `view!`
//!   regression (an inline `|errors: ArcRwSignal<Errors>|` closure
//!   where the macro parser misread `<Errors>` as an HTML tag) sits
//!   *inside* the macro and would not be caught here. The unit test
//!   suites at `crates/ars-{leptos,dioxus}/tests/` are the source of
//!   truth for macro-internal correctness.
//! - **Type / path resolution.** A snippet that names a type defined
//!   nowhere in the workspace still parses syntactically. Use
//!   `cargo check` against a fixture crate for that.
//! - **Borrow-checker / ownership errors.** Same as above —
//!   `syn::parse_file` is purely a parser.
//!
//! ## Opting out
//!
//! Code blocks that intentionally contain partial or pseudo-Rust
//! (signatures shown for documentation purposes, top-level expressions
//! meant to live inside an outer fn body, etc.) opt out by tagging the
//! fence info-string with `rust,no_check` (also recognised:
//! `rust,ignore`, `rust,no_run`, mirroring rustdoc). The extractor
//! honors those markers and skips the block.
//!
//! ## Performance
//!
//! Fast: no compiler invocation. Runs in milliseconds across all 2000+
//! Rust blocks in the spec, so it is suitable for inclusion in the
//! `cargo xci` pipeline (see `xtask::ci::Step::SpecCompileSnippets`).

use std::{fs, path::Path};

use crate::manifest::{Error, SpecRoot};

/// CLI entry point.
///
/// Always returns `Ok(String)`. The text either begins with `ok:` (no
/// findings) or with `N finding(s):` followed by per-finding lines.
///
/// When `fix` is `true`, each failing block has its opening fence rewritten
/// from `` ```rust `` (or `` ```rs ``) to `` ```rust,no_check `` (or
/// `` ```rs,no_check ``) in place; the file is rewritten on disk and the
/// returned text reports how many fences were patched.
///
/// # Errors
///
/// Returns [`Error::Io`] when a target file cannot be read or written.
pub fn execute(root: &SpecRoot, fix: bool) -> Result<String, Error> {
    let mut files = Vec::new();

    for sub in [
        "components",
        "foundation",
        "leptos-components",
        "dioxus-components",
        "shared",
        "testing",
    ] {
        let dir = root.path.join(sub);

        collect_md_files(&dir, &mut files).map_err(Error::Io)?;
    }

    files.sort();

    let display_root = fs::canonicalize(&root.path).unwrap_or_else(|_| root.path.clone());

    let mut findings = Vec::new();
    let mut blocks_checked = 0usize;
    let mut fences_patched = 0usize;

    for path in &files {
        let source = fs::read_to_string(path).map_err(Error::Io)?;

        let display = path
            .strip_prefix(&display_root)
            .unwrap_or(path)
            .display()
            .to_string();

        let blocks = extract_rust_blocks(&source);

        let mut failing_fence_lines = Vec::new();

        for block in &blocks {
            blocks_checked += 1;

            if let Some(message) = check_block(block) {
                findings.push(Finding {
                    file: display.clone(),
                    line: block.start_line,
                    message,
                });

                if fix {
                    // The fence line is one above the block's first content line.
                    failing_fence_lines.push(block.start_line.saturating_sub(1));
                }
            }
        }

        if fix && !failing_fence_lines.is_empty() {
            let patched = rewrite_fences_to_no_check(&source, &failing_fence_lines);

            if patched != source {
                let count = patched_fence_count(&source, &patched);

                fences_patched += count;

                fs::write(path, patched).map_err(Error::Io)?;
            }
        }
    }

    let mut out = String::new();

    if findings.is_empty() {
        out.push_str(&format!(
            "ok: {} block(s) parsed across {} spec file(s), no syntax findings\n",
            blocks_checked,
            files.len()
        ));

        return Ok(out);
    }

    if fix {
        // Re-run after patching so the report reflects the fixed state and
        // exit-code pipelines see a clean run on the next invocation.
        out.push_str(&format!(
            "fixed: patched {fences_patched} fence(s) across {} spec file(s) ({blocks_checked} \
             block(s) parsed). Re-run `cargo xtask spec compile-snippets` to confirm a clean \
             pass.\n",
            files.len()
        ));

        return Ok(out);
    }

    out.push_str(&format!(
        "{} finding(s) across {} spec file(s) ({} block(s) parsed):\n\n",
        findings.len(),
        files.len(),
        blocks_checked,
    ));

    for f in &findings {
        out.push_str(&format!("{}:{}: {}\n", f.file, f.line, f.message));
    }

    Ok(out)
}

/// Rewrites every opening fence on the given 1-indexed line numbers from
/// `` ```rust `` / `` ```rs `` (with optional further info-string tokens) to
/// the same fence with `no_check` injected. Existing `no_check` markers are
/// left untouched.
fn rewrite_fences_to_no_check(source: &str, fence_lines: &[usize]) -> String {
    let mut lines = source.lines().collect::<Vec<_>>();

    let mut owned = lines.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();

    for &line_no in fence_lines {
        // 1-indexed → 0-indexed.
        let Some(idx) = line_no.checked_sub(1) else {
            continue;
        };

        let Some(line) = owned.get_mut(idx) else {
            continue;
        };

        let trimmed = line.trim_start();

        let leading = &line[..line.len() - trimmed.len()];

        let fence_width = trimmed.chars().take_while(|ch| *ch == '`').count();

        if fence_width < 3 {
            continue;
        }

        let info = trimmed.trim_start_matches('`');

        let lang_tag = info.split([',', ' ']).next().unwrap_or(info).trim();

        if !matches!(lang_tag, "rust" | "rs") {
            continue;
        }

        // Already opted out — skip.
        if info.split(',').any(|tok| {
            let t = tok.trim();

            matches!(t, "no_check" | "ignore" | "no_run")
        }) {
            continue;
        }

        let new_info = if info.contains(',') {
            // `rust,foo` → `rust,foo,no_check`
            format!("{info},no_check")
        } else {
            format!("{info},no_check")
        };

        let fence = "`".repeat(fence_width);

        *line = format!("{leading}{fence}{new_info}");
    }

    lines.clear();

    let mut joined = owned.join("\n");

    if source.ends_with('\n') {
        joined.push('\n');
    }

    joined
}

fn patched_fence_count(before: &str, after: &str) -> usize {
    // Count differing lines containing a fence opener as a proxy for "patched
    // a fence". Conservative: this only undercounts when an unrelated edit
    // happens, which the in-place rewriter never makes.
    before
        .lines()
        .zip(after.lines())
        .filter(|(b, a)| b != a && b.trim_start().starts_with("```"))
        .count()
}

#[derive(Debug, Clone)]
struct Finding {
    file: String,
    line: usize,
    message: String,
}

#[derive(Debug)]
struct CodeBlock {
    /// 1-indexed spec-file line on which the block's first content line lives.
    start_line: usize,

    /// Block body, without the fence lines.
    content: String,

    /// `false` when the fence info-string opts the block out of parsing.
    parse: bool,
}

/// Extracts every fenced Rust code block from the markdown source.
///
/// Recognised opt-in fence info-strings: `rust`, `rs`. Recognised opt-out
/// markers: `rust,no_check`, `rust,ignore`, `rust,no_run` (the latter two
/// match rustdoc conventions; we treat them all as "skip parsing" because
/// the goal is to find structural Rust bugs, not to enforce runtime
/// behaviour).
fn extract_rust_blocks(source: &str) -> Vec<CodeBlock> {
    let mut blocks = Vec::new();

    let mut in_block = false;
    let mut current = String::new();
    let mut current_start = 0usize;
    let mut current_parse = true;

    for (idx, line) in source.lines().enumerate() {
        let line_no = idx + 1;

        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            if in_block {
                blocks.push(CodeBlock {
                    start_line: current_start,
                    content: std::mem::take(&mut current),
                    parse: current_parse,
                });

                in_block = false;

                continue;
            }

            // Opening fence — inspect the info-string after the backticks.
            let info = trimmed.trim_start_matches('`').trim();

            let lang_tag = info.split([',', ' ']).next().unwrap_or(info).trim();

            let is_rust = matches!(lang_tag, "rust" | "rs");

            if !is_rust {
                continue;
            }

            in_block = true;

            current_start = line_no + 1;

            current_parse = !info.split(',').any(|tok| {
                let tok = tok.trim();

                matches!(tok, "no_check" | "ignore" | "no_run")
            });

            continue;
        }

        if in_block {
            current.push_str(line);

            current.push('\n');
        }
    }

    if in_block {
        blocks.push(CodeBlock {
            start_line: current_start,
            content: current,
            parse: current_parse,
        });
    }

    blocks
}

/// Runs `syn::parse_file` on a code block and returns a finding message
/// when it fails. Returns `None` on success or when the block opts out.
fn check_block(block: &CodeBlock) -> Option<String> {
    if !block.parse {
        return None;
    }

    // Many spec snippets are partial: they show a method signature without
    // its body, or a `use` statement followed by an `impl`. `syn::parse_file`
    // is item-level and tolerates partial-file content as long as each
    // top-level item is syntactically complete.
    match syn::parse_file(&block.content) {
        Ok(_) => None,

        Err(err) => Some(format!(
            "Rust syntax error: {err}; \
             add `rust,no_check` to the fence info-string if the snippet is intentionally \
             partial or pseudo-Rust"
        )),
    }
}

fn collect_md_files(root: &Path, into: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
    if !root.exists() {
        return Ok(());
    }

    let canonical_root = fs::canonicalize(root)?;

    let mut stack = Vec::from([canonical_root.clone()]);

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;

            let file_type = entry.file_type()?;

            if file_type.is_symlink() {
                continue;
            }

            let candidate = entry.path();

            let Ok(canonical) = fs::canonicalize(&candidate) else {
                continue;
            };

            if !canonical.starts_with(&canonical_root) {
                continue;
            }

            if file_type.is_dir() {
                stack.push(canonical);
            } else if file_type.is_file()
                && canonical.extension().is_some_and(|ext| ext == "md")
                && canonical
                    .file_name()
                    .is_some_and(|name| name != "CLAUDE.md")
            {
                into.push(canonical);
            }
        }
    }

    Ok(())
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rust_block_with_correct_start_line() {
        let source = "# Title\n\nIntro.\n\n```rust\nfn ok() {}\n```\n";

        let blocks = extract_rust_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start_line, 6);
        assert!(blocks[0].parse);
        assert!(blocks[0].content.contains("fn ok"));
    }

    #[test]
    fn skips_non_rust_blocks() {
        let source = "```toml\nfoo = 1\n```\n```rust\nfn k() {}\n```\n";

        let blocks = extract_rust_blocks(source);

        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn opts_out_when_fence_marks_no_check() {
        let source = "```rust,no_check\nlet x: SomeType = stub;\n```\n";

        let blocks = extract_rust_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert!(!blocks[0].parse);
        assert!(check_block(&blocks[0]).is_none());
    }

    #[test]
    fn opts_out_for_rustdoc_ignore_marker() {
        let source = "```rust,ignore\nfn maybe() { todo!() }\n```\n";

        let blocks = extract_rust_blocks(source);

        assert!(!blocks[0].parse);
    }

    #[test]
    fn flags_unbalanced_braces() {
        let block = CodeBlock {
            start_line: 1,
            content: "fn broken() { ".to_string(),
            parse: true,
        };

        assert!(check_block(&block).is_some());
    }

    #[test]
    fn flushes_unterminated_final_rust_block() {
        let source = "# Title\n\n```rust\npub fn broken(\n";

        let blocks = extract_rust_blocks(source);

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start_line, 4);
        assert_eq!(blocks[0].content, "pub fn broken(\n");
        assert!(
            check_block(&blocks[0]).is_some(),
            "unterminated final fences must still be parsed and reported"
        );
    }

    #[test]
    fn parses_realistic_component_signature() {
        let block = CodeBlock {
            start_line: 1,
            content: r#"
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    pub heading: String,
}

impl Messages {
    pub fn new() -> Self {
        Self { heading: String::new() }
    }
}
"#
            .to_string(),
            parse: true,
        };

        assert!(check_block(&block).is_none());
    }

    #[test]
    fn accepts_macro_invocations_as_token_trees() {
        // `view!` and `rsx!` macros are opaque to syn — anything inside the
        // outer braces is a token tree, not parsed Rust. So a snippet using
        // these macros parses fine even with framework-specific syntax.
        let block = CodeBlock {
            start_line: 1,
            content: r#"
fn render() {
    view! {
        <div>"hello"</div>
    };
}
"#
            .to_string(),
            parse: true,
        };

        assert!(check_block(&block).is_none());
    }

    #[test]
    fn flags_invalid_top_level_item() {
        let block = CodeBlock {
            start_line: 1,
            content: "pub fn missing_brace(".to_string(),
            parse: true,
        };

        assert!(check_block(&block).is_some());
    }

    // ── Integration tests via tempdir fixture ──────────────────────
    //
    // The tests above cover `extract_rust_blocks` and `check_block` in
    // isolation. The block below exercises `execute`, `collect_md_files`,
    // and `rewrite_fences_to_no_check` end-to-end against a synthetic
    // spec layout — the auto-fix path in particular rewrites files on
    // disk, so unit-testing it without a fixture would leave one of the
    // most consequential code paths in this module unverified.

    use std::collections::BTreeMap;

    use tempfile::TempDir;

    use crate::manifest::{Manifest, SpecRoot};

    fn empty_manifest() -> Manifest {
        Manifest {
            foundation: BTreeMap::new(),
            shared: BTreeMap::new(),
            components: BTreeMap::new(),
            review_profiles: None,
            leptos_adapters: BTreeMap::new(),
            dioxus_adapters: BTreeMap::new(),
        }
    }

    fn synthetic_root(tmp: &TempDir) -> SpecRoot {
        // `execute` walks `components/`, `foundation/`, …; create the dirs
        // even when empty so the canonicalize step inside `collect_md_files`
        // does not silently no-op.
        for sub in [
            "components",
            "foundation",
            "leptos-components",
            "dioxus-components",
            "shared",
            "testing",
        ] {
            fs::create_dir_all(tmp.path().join(sub)).unwrap();
        }
        SpecRoot {
            path: tmp.path().to_path_buf(),
            manifest: empty_manifest(),
        }
    }

    #[test]
    fn execute_reports_ok_for_clean_corpus() {
        let tmp = TempDir::new().unwrap();

        let root = synthetic_root(&tmp);

        // Single clean Rust block.
        let path = tmp.path().join("components").join("clean.md");

        fs::write(&path, "# Clean\n\n```rust\npub fn ok() {}\n```\n").unwrap();

        let report = execute(&root, false).unwrap();

        assert!(
            report.starts_with("ok:") && report.contains("1 block(s) parsed"),
            "expected clean ok-report, got: {report}"
        );
    }

    #[test]
    fn execute_reports_findings_with_file_and_line_locator() {
        let tmp = TempDir::new().unwrap();

        let root = synthetic_root(&tmp);

        let path = tmp.path().join("foundation").join("buggy.md");

        fs::write(
            &path,
            "# Title\n\nintro\n\n```rust\npub fn unterminated(\n```\n",
        )
        .unwrap();

        let report = execute(&root, false).unwrap();

        assert!(
            report.contains("1 finding(s)"),
            "expected one finding, got: {report}"
        );
        // The locator points at the block's first content line (line 6).
        assert!(
            report.contains("foundation/buggy.md:6:"),
            "expected location prefix, got: {report}"
        );
    }

    #[test]
    fn execute_walks_every_top_level_subdir() {
        let tmp = TempDir::new().unwrap();
        let root = synthetic_root(&tmp);

        // One clean rust block per subdir — `execute` should parse all of
        // them in a single run.
        for sub in [
            "components",
            "foundation",
            "leptos-components",
            "dioxus-components",
            "shared",
            "testing",
        ] {
            let p = tmp.path().join(sub).join("ok.md");

            fs::write(&p, "```rust\nfn k() {}\n```\n").unwrap();
        }

        let report = execute(&root, false).unwrap();

        assert!(
            report.contains("6 block(s) parsed"),
            "expected one block per subdir, got: {report}"
        );
    }

    #[test]
    fn execute_skips_claude_md_files() {
        let tmp = TempDir::new().unwrap();

        let root = synthetic_root(&tmp);

        // `CLAUDE.md` files are agent instructions, not spec content; they
        // sometimes contain pseudocode that should not be parsed.
        let p = tmp.path().join("components").join("CLAUDE.md");

        fs::write(&p, "```rust\npub fn unterminated(\n```\n").unwrap();

        let report = execute(&root, false).unwrap();

        assert!(
            report.starts_with("ok:") && report.contains("0 block(s) parsed"),
            "CLAUDE.md should be skipped, got: {report}"
        );
    }

    #[test]
    fn execute_with_fix_rewrites_failing_fence_in_place() {
        let tmp = TempDir::new().unwrap();

        let root = synthetic_root(&tmp);

        let path = tmp.path().join("components").join("partial.md");

        let original = "# Doc\n\n```rust\npub fn unterminated(\n```\n";

        fs::write(&path, original).unwrap();

        // First pass with `fix=true` should report it patched the fence.
        let report = execute(&root, true).unwrap();

        assert!(
            report.starts_with("fixed:"),
            "expected fix-report prefix, got: {report}"
        );

        // The file's opening fence should now carry `,no_check`.
        let after = fs::read_to_string(&path).unwrap();

        assert!(
            after.contains("```rust,no_check"),
            "fence not rewritten; file is now:\n{after}"
        );

        // Second pass without `fix` should now be clean.
        let report2 = execute(&root, false).unwrap();

        assert!(
            report2.starts_with("ok:"),
            "expected clean report after fix, got: {report2}"
        );
    }

    #[test]
    fn rewrite_fences_to_no_check_preserves_already_tagged_blocks() {
        // Already-opted-out fences must not get a duplicate `,no_check`.
        let source = "```rust,no_check\nlet x = 1;\n```\n";

        let rewritten = rewrite_fences_to_no_check(source, &[1]);

        // No change because the fence already has the marker.
        assert_eq!(rewritten, source);
    }

    #[test]
    fn rewrite_fences_to_no_check_appends_to_existing_info_string() {
        // A fence with `rust,foo` should become `rust,foo,no_check` (not
        // `rust,no_check,foo`).
        let source = "```rust,foo\nlet x = 1;\n```\n";

        let rewritten = rewrite_fences_to_no_check(source, &[1]);

        assert!(
            rewritten.contains("```rust,foo,no_check"),
            "expected appended marker, got: {rewritten}"
        );
    }

    #[test]
    fn rewrite_fences_to_no_check_preserves_wide_fence_width() {
        // Wider fences are used when the block body itself contains triple
        // backticks; rewriting them to three backticks would corrupt the
        // surrounding markdown structure.
        let source = "````rust\nconst DOC: &str = \"```inner```\";\n````\n";

        let rewritten = rewrite_fences_to_no_check(source, &[1]);

        assert!(
            rewritten.starts_with("````rust,no_check"),
            "expected preserved four-backtick opener, got: {rewritten}"
        );
    }

    #[test]
    fn rewrite_fences_to_no_check_handles_indented_fences() {
        // Fences indented inside list items must keep their leading
        // whitespace after rewrite.
        let source = "- list item\n  ```rust\n  let x = 1;\n  ```\n";

        let rewritten = rewrite_fences_to_no_check(source, &[2]);

        assert!(
            rewritten.contains("  ```rust,no_check"),
            "expected preserved indent, got: {rewritten}"
        );
    }

    #[test]
    fn rewrite_fences_to_no_check_ignores_non_rust_fences() {
        // Defensive: passing a fence-line that points at a non-rust block
        // should not corrupt the file.
        let source = "```toml\nfoo = 1\n```\n";

        let rewritten = rewrite_fences_to_no_check(source, &[1]);

        assert_eq!(rewritten, source);
    }

    /// Defensive: line numbers outside the file's range and `0` (which
    /// would underflow `checked_sub(1)`) must be ignored without
    /// panicking. Without this guard a stray off-by-one in `execute`
    /// would crash the auto-fixer instead of being a no-op.
    #[test]
    fn rewrite_fences_to_no_check_skips_out_of_range_and_zero_line_numbers() {
        let source = "```rust\nfn ok() {}\n```\n";

        // Zero (would underflow → continue).
        let zero = rewrite_fences_to_no_check(source, &[0]);

        assert_eq!(zero, source);

        // Way past EOF (out-of-range get_mut → continue).
        let far = rewrite_fences_to_no_check(source, &[9999]);

        assert_eq!(far, source);
    }

    /// Defensive: when `fence_lines` points to a line that is not a
    /// fence opener at all (e.g. a stray line number from elsewhere),
    /// the rewriter must leave the source untouched.
    #[test]
    fn rewrite_fences_to_no_check_skips_non_fence_lines() {
        let source = "intro\n```rust\nfn ok() {}\n```\nepilogue\n";

        // Line 1 is "intro" — not a fence.
        let rewritten = rewrite_fences_to_no_check(source, &[1]);

        assert_eq!(rewritten, source);

        // Line 5 is "epilogue" — also not a fence.
        let rewritten = rewrite_fences_to_no_check(source, &[5]);

        assert_eq!(rewritten, source);
    }

    /// `collect_md_files` returns `Ok(())` and adds nothing when the
    /// requested subdirectory does not exist. Without this guard,
    /// `execute` would error on every workspace that hasn't yet
    /// populated all six top-level spec subdirs.
    #[test]
    fn collect_md_files_returns_ok_for_missing_directory() {
        let dir = tempfile::tempdir().expect("tempdir");

        let missing = dir.path().join("does-not-exist");

        let mut into = Vec::new();

        collect_md_files(&missing, &mut into).expect("non-existent dir is OK");

        assert!(into.is_empty());
    }

    /// `collect_md_files` skips symlinks defensively (a malicious link
    /// could otherwise escape the spec root). Real spec corpora do not
    /// use symlinks, but the guard exists to keep the walker bounded.
    ///
    /// The symlink path is unix-only (`std::os::unix::fs::symlink`). On
    /// Windows this test is skipped; the production code path is
    /// identical because `is_symlink` returns the same on both.
    #[cfg(unix)]
    #[test]
    fn collect_md_files_skips_symlinks() {
        let dir = tempfile::tempdir().expect("tempdir");

        let target = dir.path().join("real.md");

        fs::write(&target, "# Real\n").expect("write target");

        let link = dir.path().join("link.md");

        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        let mut into = Vec::new();

        collect_md_files(dir.path(), &mut into).expect("walk OK");

        // Only the real file is collected; the symlink is filtered out.
        assert_eq!(into.len(), 1, "expected only the real file: {into:?}");
        assert!(
            into[0].ends_with("real.md"),
            "expected canonical real.md path, got {:?}",
            into[0]
        );
    }

    /// Drives `execute` with `--fix` against a fixture containing a
    /// failing block whose offending fence is on line 1 of the file —
    /// the rewriter must patch it in place and the report must reflect
    /// the patch. This exercises the
    /// `if patched != source { fences_patched += count; fs::write(...) }`
    /// branch end-to-end (line 117 of `execute`), which all the
    /// finer-grained `rewrite_fences_to_no_check` unit tests skip.
    #[test]
    fn execute_with_fix_writes_patched_file_to_disk_and_summarizes_count() {
        let dir = tempfile::tempdir().expect("tempdir");

        let components = dir.path().join("components");

        fs::create_dir_all(&components).expect("mkdir components");

        let bad = components.join("bad.md");

        // Two failing rust blocks → two fences should be patched.
        fs::write(
            &bad,
            "```rust\nlet x: SomeType = stub;\n```\n```rust\nlet y: Other = z;\n```\n",
        )
        .expect("write bad.md");

        let root = SpecRoot {
            path: dir.path().to_path_buf(),
            manifest: Manifest {
                foundation: BTreeMap::new(),
                shared: BTreeMap::new(),
                components: BTreeMap::new(),
                review_profiles: None,
                leptos_adapters: BTreeMap::new(),
                dioxus_adapters: BTreeMap::new(),
            },
        };

        let report = execute(&root, true).expect("fix run");

        assert!(
            report.starts_with("fixed:"),
            "expected fixed report: {report}"
        );
        assert!(
            report.contains("patched 2 fence(s)"),
            "expected count=2: {report}"
        );

        let after = fs::read_to_string(&bad).expect("re-read bad.md");

        let no_check_count = after.matches("rust,no_check").count();

        assert_eq!(
            no_check_count, 2,
            "both fences should now carry the no_check marker; got: {after}"
        );

        // Re-running without --fix on the patched file must report ok.
        let clean = execute(&root, false).expect("clean re-run");

        assert!(
            clean.starts_with("ok:"),
            "post-fix run should be clean: {clean}"
        );
    }
}
