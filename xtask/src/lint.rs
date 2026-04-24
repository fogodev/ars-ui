//! CI lint checks for repository-level testing policy.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Display, Write as _},
    fs, io,
    path::{Path, PathBuf},
};

use regex::Regex;

use crate::manifest;

/// Options for adapter parity linting.
#[derive(Debug, Clone)]
pub struct AdapterParityOptions {
    /// Directory containing Leptos adapter tests.
    pub leptos_test_dir: PathBuf,

    /// Directory containing Dioxus adapter tests.
    pub dioxus_test_dir: PathBuf,

    /// Maximum allowed per-component test-count delta.
    pub tolerance: usize,
}

/// Options for snapshot-count linting.
#[derive(Debug, Clone)]
pub struct SnapshotCountOptions {
    /// Directory tree containing `.snap` files.
    pub snapshots_dir: PathBuf,

    /// Minimum snapshot count per detected state variant.
    pub min_per_variant: usize,

    /// Maximum snapshot count per component before failure.
    pub max_per_component: usize,
}

/// Options for error-variant coverage linting.
#[derive(Debug, Clone)]
pub struct ErrorVariantCoverageOptions {
    /// Glob selecting Rust source files containing the enum.
    pub source_glob: String,

    /// Glob selecting Rust test files to inspect.
    pub test_glob: String,

    /// Enum name whose variants must be exercised.
    pub enum_name: String,
}

/// Errors from lint checks.
#[derive(Debug)]
pub enum Error {
    /// IO error reading repository files.
    Io(io::Error),

    /// Regex compilation error.
    Regex(regex::Error),

    /// Spec manifest operation failed.
    Manifest(manifest::Error),

    /// The requested enum was not found in any matched source file.
    EnumNotFound {
        /// Enum name requested by the caller.
        enum_name: String,
    },

    /// One or more lint checks failed.
    Failed {
        /// Human-readable failure report.
        summary: String,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "IO error: {error}"),
            Self::Regex(error) => write!(f, "regex error: {error}"),
            Self::Manifest(error) => write!(f, "{error}"),
            Self::EnumNotFound { enum_name } => {
                write!(
                    f,
                    "enum `{enum_name}` was not found in matched source files"
                )
            }
            Self::Failed { summary } => write!(f, "{summary}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<regex::Error> for Error {
    fn from(error: regex::Error) -> Self {
        Self::Regex(error)
    }
}

impl From<manifest::Error> for Error {
    fn from(error: manifest::Error) -> Self {
        Self::Manifest(error)
    }
}

/// Run the adapter parity lint.
///
/// # Errors
///
/// Returns [`Error::Failed`] when implemented components diverge beyond the
/// configured tolerance.
pub fn check_adapter_parity(options: &AdapterParityOptions) -> Result<String, Error> {
    let root = manifest::SpecRoot::discover(&std::env::current_dir()?)?;

    let components = root
        .manifest
        .components
        .keys()
        .map(|name| name.replace('-', "_"))
        .collect::<BTreeSet<_>>();

    let leptos_counts = adapter_test_counts(&options.leptos_test_dir, &components)?;
    let dioxus_counts = adapter_test_counts(&options.dioxus_test_dir, &components)?;

    let mut all_components = BTreeSet::new();

    all_components.extend(leptos_counts.keys().cloned());
    all_components.extend(dioxus_counts.keys().cloned());

    let (mut output, failures) = adapter_parity_report(
        &all_components,
        &leptos_counts,
        &dioxus_counts,
        options.tolerance,
    );

    if failures.is_empty() {
        Ok(output)
    } else {
        output.push_str("\nAdapter parity violations:\n");

        for failure in &failures {
            writeln!(output, "- {failure}").expect("write to string");
        }

        Err(Error::Failed { summary: output })
    }
}

fn adapter_parity_report(
    components: &BTreeSet<String>,
    leptos_counts: &BTreeMap<String, usize>,
    dioxus_counts: &BTreeMap<String, usize>,
    tolerance: usize,
) -> (String, Vec<String>) {
    let mut output = String::from("Component | Leptos | Dioxus | Delta | Status\n");

    output.push_str("----------|--------|--------|-------|-------\n");

    let mut failures = Vec::new();

    for component in components {
        let leptos = leptos_counts.get(component).copied().unwrap_or(0);

        let dioxus = dioxus_counts.get(component).copied().unwrap_or(0);

        let delta = leptos.abs_diff(dioxus);

        let status = if leptos == 0 || dioxus == 0 {
            failures.push(format!(
                "{component}: both adapters must have tests, leptos={leptos}, dioxus={dioxus}"
            ));
            "FAIL"
        } else if delta > tolerance {
            failures.push(format!(
                "{component}: leptos={leptos}, dioxus={dioxus}, delta={delta}, tolerance={tolerance}"
            ));
            "FAIL"
        } else {
            "OK"
        };

        writeln!(
            output,
            "{component} | {leptos} | {dioxus} | {delta} | {status}"
        )
        .expect("write to string");
    }

    (output, failures)
}

/// Run the snapshot count lint.
///
/// # Errors
///
/// Returns [`Error::Failed`] when detected stateful components have fewer than
/// the required snapshot count.
pub fn check_snapshot_count(options: &SnapshotCountOptions) -> Result<String, Error> {
    let snapshots = collect_files(&options.snapshots_dir, |path| {
        path.extension()
            .is_some_and(|extension| extension == "snap")
    })?;

    let total_snapshots = snapshots.len();

    let state_variants = detect_state_variants(&workspace_root_for(&options.snapshots_dir))?;
    let mut counts = BTreeMap::<String, usize>::new();

    for snapshot in snapshots {
        if let Some(component) = infer_snapshot_component(&snapshot)
            && state_variants.contains_key(&component)
        {
            *counts.entry(component).or_insert(0) += 1;
        }
    }

    let mut output = String::from("Component | Snapshots | State variants | Required | Status\n");

    output.push_str("----------|-----------|----------------|----------|-------\n");

    let mut components = BTreeSet::new();

    components.extend(counts.keys().cloned());
    components.extend(
        state_variants
            .iter()
            .filter_map(|(component, variants)| (*variants > 2).then_some(component.clone())),
    );

    let mut failures = Vec::new();
    let mut warnings = Vec::new();

    if total_snapshots > 500 {
        warnings.push(format!(
            "total snapshots={total_snapshots}, warning threshold=500"
        ));
    }

    for component in components {
        let count = counts.get(&component).copied().unwrap_or(0);

        let variants = state_variants.get(&component).copied().unwrap_or(0);

        let required = if variants > 2 {
            variants * options.min_per_variant
        } else {
            0
        };

        let mut status = "OK";

        if required > 0 && count < required {
            failures.push(format!(
                "{component}: snapshots={count}, state_variants={variants}, required={required}"
            ));

            status = "FAIL";
        }

        if count > options.max_per_component {
            failures.push(format!(
                "{component}: snapshots={count}, max={}",
                options.max_per_component
            ));

            status = "FAIL";
        }

        writeln!(
            output,
            "{component} | {count} | {variants} | {required} | {status}"
        )
        .expect("write to string");
    }

    if !warnings.is_empty() {
        output.push_str("\nSnapshot count warnings:\n");

        for warning in &warnings {
            writeln!(output, "- {warning}").expect("write to string");
        }
    }

    if failures.is_empty() {
        Ok(output)
    } else {
        output.push_str("\nSnapshot count violations:\n");

        for failure in &failures {
            writeln!(output, "- {failure}").expect("write to string");
        }

        Err(Error::Failed { summary: output })
    }
}

/// Run the error-variant coverage lint.
///
/// # Errors
///
/// Returns [`Error::Failed`] when one or more enum variants do not appear in a
/// test function body.
pub fn check_error_variant_coverage(
    options: &ErrorVariantCoverageOptions,
) -> Result<String, Error> {
    let source_files = files_matching_glob(&options.source_glob)?;

    let test_files = files_matching_glob(&options.test_glob)?;

    let variants = parse_enum_variants(&source_files, &options.enum_name)?;

    let test_bodies = parse_test_bodies(&test_files)?;

    let mut uncovered = Vec::new();

    for variant in &variants {
        if !test_bodies.iter().any(|body| body.contains(variant)) {
            uncovered.push(variant.clone());
        }
    }

    let mut output = format!(
        "Error variant coverage for `{}`: {} variants, {} test functions\n",
        options.enum_name,
        variants.len(),
        test_bodies.len()
    );

    if uncovered.is_empty() {
        output.push_str("All variants are covered.\n");

        Ok(output)
    } else {
        output.push_str("Uncovered variants:\n");

        for variant in &uncovered {
            writeln!(output, "- {variant}").expect("write to string");
        }

        Err(Error::Failed { summary: output })
    }
}

fn adapter_test_counts(
    dir: &Path,
    known_components: &BTreeSet<String>,
) -> Result<BTreeMap<String, usize>, Error> {
    if !dir.exists() {
        return Ok(BTreeMap::new());
    }

    let test_attr = Regex::new(r"#\[\s*(?:wasm_bindgen_)?test\s*\]")?;

    let files = collect_files(dir, |path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("test_") && name.ends_with(".rs"))
    })?;

    let mut counts = BTreeMap::new();

    for file in files {
        let Some(component) = extract_component_from_test_file(&file, known_components) else {
            continue;
        };

        let content = fs::read_to_string(&file)?;

        let count = test_attr.find_iter(&content).count();

        *counts.entry(component).or_insert(0) += count;
    }

    Ok(counts)
}

fn extract_component_from_test_file(
    path: &Path,
    known_components: &BTreeSet<String>,
) -> Option<String> {
    let stem = path.file_stem()?.to_str()?.strip_prefix("test_")?;
    known_components
        .iter()
        .filter(|component| {
            stem == component.as_str() || stem.starts_with(&format!("{component}_"))
        })
        .max_by_key(|component| component.len())
        .cloned()
        .or_else(|| {
            stem.rsplit_once('_')
                .map(|(component, _)| component.to_owned())
        })
}

fn infer_snapshot_component(path: &Path) -> Option<String> {
    for ancestor in path.ancestors() {
        if ancestor.file_name().and_then(|name| name.to_str()) == Some("snapshots")
            && let Some(parent) = ancestor.parent()
            && let Some(component) = parent.file_name().and_then(|name| name.to_str())
            && component != "tests"
            && component != "src"
        {
            return Some(component.replace('-', "_"));
        }
    }

    let stem = path.file_stem()?.to_str()?;

    let compact = stem.split("__").collect::<Vec<_>>();

    if compact
        .first()
        .is_some_and(|crate_name| crate_name.starts_with("ars_"))
        && let Some(component) = compact.get(1)
    {
        return Some((*component).replace('-', "_"));
    }

    compact
        .windows(2)
        .find_map(|parts| (parts[1] == "component").then(|| parts[0].replace('-', "_")))
        .or_else(|| stem.split("__").next().map(|name| name.replace('-', "_")))
}

fn detect_state_variants(root: &Path) -> Result<BTreeMap<String, usize>, Error> {
    let enum_start = Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+State\s*\{")?;

    let variant = Regex::new(r"^\s*([A-Z][A-Za-z0-9_]*)\s*(?:[{(,]|$)")?;

    let files = collect_files(root, |path| {
        path.extension().is_some_and(|extension| extension == "rs")
            && path.file_name().is_some_and(|name| name == "component.rs")
            && path
                .components()
                .any(|component| component.as_os_str() == "src")
            && !path
                .components()
                .any(|component| component.as_os_str() == "target")
    })?;

    let mut variants = BTreeMap::new();

    for file in files {
        let content = fs::read_to_string(&file)?;

        if enum_start.find(&content).is_none() {
            continue;
        }

        let Some(component) = infer_source_component(&file) else {
            continue;
        };

        let count = parse_state_variant_count(&content, &variant);

        if count > 0 {
            variants
                .entry(component)
                .and_modify(|existing: &mut usize| *existing = (*existing).max(count))
                .or_insert(count);
        }
    }

    Ok(variants)
}

fn parse_state_variant_count(content: &str, variant: &Regex) -> usize {
    let Some(start) = content.find("enum State") else {
        return 0;
    };

    let Some(open_offset) = content[start..].find('{') else {
        return 0;
    };

    let body_start = start + open_offset + 1;

    let mut depth = 1usize;

    let mut end = body_start;

    for (offset, ch) in content[body_start..].char_indices() {
        match ch {
            '{' => depth += 1,

            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = body_start + offset;
                    break;
                }
            }

            _ => {}
        }
    }

    content[body_start..end]
        .lines()
        .filter(|line| variant.is_match(line.trim()))
        .count()
}

fn infer_source_component(path: &Path) -> Option<String> {
    let mut parts = path.components().peekable();

    while let Some(part) = parts.next() {
        if part.as_os_str() == "src" {
            return parts
                .next()
                .and_then(|component| {
                    let path = Path::new(component.as_os_str());

                    if path.extension().is_some_and(|extension| extension == "rs") {
                        path.file_stem()
                    } else {
                        path.file_name()
                    }
                    .and_then(|name| name.to_str())
                    .map(str::to_owned)
                })
                .or_else(|| {
                    path.parent()
                        .and_then(Path::file_name)
                        .and_then(|name| name.to_str())
                        .map(str::to_owned)
                });
        }
    }

    None
}

fn workspace_root_for(path: &Path) -> PathBuf {
    let mut root = PathBuf::new();

    for component in path.components() {
        if component.as_os_str() == "crates" {
            return if root.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                root
            };
        }

        root.push(component.as_os_str());
    }

    PathBuf::from(".")
}

fn parse_enum_variants(files: &[PathBuf], enum_name: &str) -> Result<Vec<String>, Error> {
    let enum_start = Regex::new(&format!(
        r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+{}\s*\{{",
        regex::escape(enum_name)
    ))?;

    let variant = Regex::new(r"^\s+(\w+)\s*[{(,]")?;

    let mut parsed = BTreeSet::new();

    let mut found = false;

    for file in files {
        let content = fs::read_to_string(file)?;

        let Some(start_match) = enum_start.find(&content) else {
            continue;
        };

        found = true;

        let body_start = start_match.end();

        let mut depth = 1usize;

        let mut body_end = body_start;

        for (offset, ch) in content[body_start..].char_indices() {
            match ch {
                '{' => depth += 1,

                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        body_end = body_start + offset;
                        break;
                    }
                }

                _ => {}
            }
        }

        for line in content[body_start..body_end].lines() {
            if let Some(captures) = variant.captures(line) {
                parsed.insert(captures[1].to_owned());
            }
        }
    }

    if !found {
        return Err(Error::EnumNotFound {
            enum_name: enum_name.to_owned(),
        });
    }

    Ok(parsed.into_iter().collect())
}

fn parse_test_bodies(files: &[PathBuf]) -> Result<Vec<String>, Error> {
    let test_attr = Regex::new(r"#\[\s*test\s*\]")?;

    let fn_start = Regex::new(r"fn\s+\w+\s*\([^)]*\)\s*\{")?;

    let mut bodies = Vec::new();

    for file in files {
        let content = fs::read_to_string(file)?;

        for attr_match in test_attr.find_iter(&content) {
            let Some(fn_match) = fn_start.find(&content[attr_match.end()..]) else {
                continue;
            };

            let body_start = attr_match.end() + fn_match.end();

            if let Some(body) = balanced_body(&content[body_start..]) {
                bodies.push(body.to_owned());
            }
        }
    }

    Ok(bodies)
}

fn balanced_body(content: &str) -> Option<&str> {
    let mut depth = 1usize;

    for (offset, ch) in content.char_indices() {
        match ch {
            '{' => depth += 1,

            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&content[..offset]);
                }
            }

            _ => {}
        }
    }

    None
}

fn files_matching_glob(pattern: &str) -> Result<Vec<PathBuf>, Error> {
    let regex = Regex::new(&format!("^{}$", glob_to_regex(pattern)))?;

    let root = glob_root(pattern);

    collect_files(&root, |path| {
        let normalized = normalize_path(path);

        regex.is_match(&normalized)
    })
}

fn glob_root(pattern: &str) -> PathBuf {
    let wildcard = pattern
        .char_indices()
        .find(|(_, ch)| matches!(ch, '*' | '?' | '['))
        .map_or(pattern.len(), |(idx, _)| idx);

    let prefix = &pattern[..wildcard];

    let root = prefix.rsplit_once('/').map_or(".", |(root, _)| root);

    if root.is_empty() {
        PathBuf::from(".")
    } else {
        PathBuf::from(root)
    }
}

fn glob_to_regex(pattern: &str) -> String {
    let mut out = String::new();

    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();

                if chars.peek() == Some(&'/') {
                    chars.next();

                    out.push_str("(?:.*/)?");
                } else {
                    out.push_str(".*");
                }
            }

            '*' => out.push_str("[^/]*"),

            '?' => out.push_str("[^/]"),

            '/' => out.push('/'),

            other => out.push_str(&regex::escape(&other.to_string())),
        }
    }

    out
}

fn collect_files(
    root: &Path,
    mut include: impl FnMut(&Path) -> bool,
) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::new();

    if !root.exists() {
        return Ok(files);
    }

    collect_files_inner(root, &mut include, &mut files)?;

    files.sort();

    Ok(files)
}

fn collect_files_inner(
    root: &Path,
    include: &mut impl FnMut(&Path) -> bool,
    files: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;

        let path = entry.path();

        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| matches!(name, "target" | ".git"))
        {
            continue;
        }

        if path.is_dir() {
            collect_files_inner(&path, include, files)?;
        } else if include(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();

        let path = std::env::temp_dir().join(format!("ars-ui-lint-{name}-{nanos}"));

        fs::create_dir_all(&path).expect("create temp dir");

        path
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }

        fs::write(path, content).expect("write file");
    }

    #[test]
    fn adapter_parity_counts_equal_tests() {
        let root = temp_dir("adapter-pass");

        let leptos = root.join("leptos");
        let dioxus = root.join("dioxus");

        write(
            &leptos.join("test_button_basic.rs"),
            "#[test]\nfn one() {}\n#[wasm_bindgen_test]\nfn two() {}\n",
        );

        write(
            &dioxus.join("test_button_basic.rs"),
            "#[test]\nfn one() {}\n#[wasm_bindgen_test]\nfn two() {}\n",
        );

        let components = BTreeSet::from(["button".to_owned()]);

        let leptos_counts = adapter_test_counts(&leptos, &components).expect("leptos counts");
        let dioxus_counts = adapter_test_counts(&dioxus, &components).expect("dioxus counts");

        assert_eq!(leptos_counts["button"], 2);
        assert_eq!(dioxus_counts["button"], 2);

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn adapter_parity_handles_missing_test_directories() {
        let root = temp_dir("adapter-missing");

        let counts =
            adapter_test_counts(&root.join("missing"), &BTreeSet::new()).expect("missing counts");

        assert!(counts.is_empty());

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn adapter_parity_reports_divergent_counts() {
        let root = temp_dir("adapter-fail");

        let leptos = root.join("leptos");
        let dioxus = root.join("dioxus");

        write(
            &leptos.join("test_dialog_basic.rs"),
            "#[test]\nfn one() {}\n#[test]\nfn two() {}\n#[test]\nfn three() {}\n",
        );
        write(&dioxus.join("test_dialog_basic.rs"), "");

        let components = BTreeSet::from(["dialog".to_owned()]);

        let leptos_counts = adapter_test_counts(&leptos, &components).expect("leptos counts");
        let dioxus_counts = adapter_test_counts(&dioxus, &components).expect("dioxus counts");

        assert!(leptos_counts["dialog"].abs_diff(dioxus_counts["dialog"]) > 2);

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn adapter_parity_reports_one_sided_zero_counts() {
        let components = BTreeSet::from(["dialog".to_owned()]);

        let leptos_counts = BTreeMap::from([("dialog".to_owned(), 1)]);
        let dioxus_counts = BTreeMap::new();

        let (_output, failures) =
            adapter_parity_report(&components, &leptos_counts, &dioxus_counts, 2);

        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("both adapters must have tests"));
    }

    #[test]
    fn snapshot_count_passes_when_budget_is_met() {
        let root = temp_dir("snapshot-pass");

        for idx in 0..9 {
            write(
                &root.join(format!(
                    "crates/demo/src/widget/snapshots/widget_{idx}.snap"
                )),
                "snapshot",
            );
        }

        write(
            &root.join("crates/demo/src/widget/component.rs"),
            "pub enum State {\n    Idle,\n    Open,\n    Closed,\n}\n",
        );

        let state_variants = detect_state_variants(&root).expect("state variants");

        assert_eq!(state_variants["widget"], 3);

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn snapshot_count_detects_undercovered_component() {
        let root = temp_dir("snapshot-fail");

        for idx in 0..2 {
            write(
                &root.join(format!("crates/demo/src/field/snapshots/field_{idx}.snap")),
                "snapshot",
            );
        }

        write(
            &root.join("crates/demo/src/field/component.rs"),
            "pub enum State {\n    Idle,\n    Valid,\n    Invalid,\n}\n",
        );

        let options = SnapshotCountOptions {
            snapshots_dir: root.join("crates/demo/src/field/snapshots"),
            min_per_variant: 3,
            max_per_component: 20,
        };

        let result = check_snapshot_count(&options);

        assert!(matches!(result, Err(Error::Failed { .. })));

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn snapshot_count_detects_stateful_component_with_zero_snapshots() {
        let root = temp_dir("snapshot-zero");

        write(
            &root.join("crates/demo/src/menu/component.rs"),
            "pub enum State {\n    Closed,\n    Open,\n    Highlighted,\n}\n",
        );

        let options = SnapshotCountOptions {
            snapshots_dir: root.join("crates"),
            min_per_variant: 3,
            max_per_component: 20,
        };

        let result = check_snapshot_count(&options);

        assert!(matches!(result, Err(Error::Failed { .. })));

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn snapshot_count_fails_for_maximum() {
        let root = temp_dir("snapshot-max");

        for idx in 0..3 {
            write(
                &root.join(format!(
                    "crates/demo/src/button/snapshots/button_{idx}.snap"
                )),
                "snapshot",
            );
        }

        write(
            &root.join("crates/demo/src/button/component.rs"),
            "pub enum State {\n    Idle,\n}\n",
        );

        let options = SnapshotCountOptions {
            snapshots_dir: root.join("crates/demo/src/button/snapshots"),
            min_per_variant: 3,
            max_per_component: 2,
        };

        let result = check_snapshot_count(&options);

        assert!(matches!(result, Err(Error::Failed { .. })));

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn snapshot_count_ignores_non_component_snapshot_suites() {
        let root = temp_dir("snapshot-non-component");

        for idx in 0..3 {
            write(
                &root.join(format!(
                    "crates/ars-core/tests/snapshots/snapshot_smoke__{idx}.snap"
                )),
                "snapshot",
            );
        }

        let options = SnapshotCountOptions {
            snapshots_dir: root.join("crates"),
            min_per_variant: 3,
            max_per_component: 2,
        };

        let output = check_snapshot_count(&options).expect("non-component snapshots are ignored");

        assert!(!output.contains("snapshot_smoke"));

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn snapshot_count_warns_when_total_exceeds_threshold() {
        let root = temp_dir("snapshot-total");

        for idx in 0..501 {
            write(
                &root.join(format!(
                    "crates/demo/src/button/snapshots/button_{idx}.snap"
                )),
                "snapshot",
            );
        }

        let options = SnapshotCountOptions {
            snapshots_dir: root.join("crates/demo/src/button/snapshots"),
            min_per_variant: 3,
            max_per_component: 600,
        };

        let output = check_snapshot_count(&options).expect("warning should not fail");

        assert!(output.contains("Snapshot count warnings"));
        assert!(output.contains("total snapshots=501"));

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn error_variant_coverage_passes_when_all_variants_are_covered() {
        let root = temp_dir("error-pass");

        write(
            &root.join("src/error.rs"),
            "pub enum ComponentError {\n    MissingId,\n    DisabledGate { event: String },\n    InvalidStateTransition(String),\n}\n",
        );
        write(
            &root.join("tests/error.rs"),
            "#[test]\nfn covers() {\n    let _ = ComponentError::MissingId;\n    let _ = ComponentError::DisabledGate { event: String::new() };\n    let _ = ComponentError::InvalidStateTransition(String::new());\n}\n",
        );

        let options = ErrorVariantCoverageOptions {
            source_glob: format!("{}/src/**/*.rs", normalize_path(&root)),
            test_glob: format!("{}/tests/**/*.rs", normalize_path(&root)),
            enum_name: "ComponentError".to_owned(),
        };

        check_error_variant_coverage(&options).expect("all variants covered");

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn error_variant_coverage_reports_uncovered_variants() {
        let root = temp_dir("error-fail");

        write(
            &root.join("src/error.rs"),
            "pub enum ComponentError {\n    MissingId,\n    DisabledGate { event: String },\n}\n",
        );
        write(
            &root.join("tests/error.rs"),
            "#[test]\nfn covers() {\n    let _ = ComponentError::MissingId;\n}\n",
        );

        let options = ErrorVariantCoverageOptions {
            source_glob: format!("{}/src/**/*.rs", normalize_path(&root)),
            test_glob: format!("{}/tests/**/*.rs", normalize_path(&root)),
            enum_name: "ComponentError".to_owned(),
        };

        let result = check_error_variant_coverage(&options);

        assert!(matches!(result, Err(Error::Failed { .. })));

        drop(fs::remove_dir_all(root));
    }
}
