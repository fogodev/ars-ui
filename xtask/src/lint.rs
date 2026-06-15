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

    /// Directory containing Leptos adapter source modules.
    pub leptos_src_dir: PathBuf,

    /// Directory containing Dioxus adapter source modules.
    pub dioxus_src_dir: PathBuf,

    /// Directory containing Leptos widget examples.
    pub leptos_widgets_dir: PathBuf,

    /// Directory containing Dioxus widget examples.
    pub dioxus_widgets_dir: PathBuf,

    /// Directory containing Leptos CSS widget examples.
    pub leptos_css_widgets_dir: PathBuf,

    /// Directory containing Dioxus CSS widget examples.
    pub dioxus_css_widgets_dir: PathBuf,

    /// Directory containing Leptos Tailwind widget examples.
    pub leptos_tailwind_widgets_dir: PathBuf,

    /// Directory containing Dioxus Tailwind widget examples.
    pub dioxus_tailwind_widgets_dir: PathBuf,

    /// Directory containing Leptos E2E fixtures.
    pub leptos_e2e_fixture_dir: PathBuf,

    /// Directory containing Dioxus E2E fixtures.
    pub dioxus_e2e_fixture_dir: PathBuf,

    /// Directory containing E2E harness modules.
    pub e2e_src_dir: PathBuf,

    /// Maximum allowed per-component test-count delta.
    pub tolerance: usize,
}

impl AdapterParityOptions {
    /// Build adapter parity options for the current workspace layout.
    #[must_use]
    pub fn workspace_defaults() -> Self {
        Self {
            leptos_test_dir: PathBuf::from("crates/ars-leptos/tests"),
            dioxus_test_dir: PathBuf::from("crates/ars-dioxus/tests"),
            leptos_src_dir: PathBuf::from("crates/ars-leptos/src"),
            dioxus_src_dir: PathBuf::from("crates/ars-dioxus/src"),
            leptos_widgets_dir: PathBuf::from("examples/widgets-leptos"),
            dioxus_widgets_dir: PathBuf::from("examples/widgets-dioxus"),
            leptos_css_widgets_dir: PathBuf::from("examples/widgets-leptos-css"),
            dioxus_css_widgets_dir: PathBuf::from("examples/widgets-dioxus-css"),
            leptos_tailwind_widgets_dir: PathBuf::from("examples/widgets-leptos-tailwind"),
            dioxus_tailwind_widgets_dir: PathBuf::from("examples/widgets-dioxus-tailwind"),
            leptos_e2e_fixture_dir: PathBuf::from("crates/ars-e2e/fixtures/leptos"),
            dioxus_e2e_fixture_dir: PathBuf::from("crates/ars-e2e/fixtures/dioxus"),
            e2e_src_dir: PathBuf::from("crates/ars-e2e/src"),
            tolerance: 2,
        }
    }
}

/// Options for snapshot-count linting.
#[derive(Debug, Clone)]
pub struct SnapshotCountOptions {
    /// Directory tree containing `.snap` files.
    pub snapshots_dir: PathBuf,

    /// Minimum snapshot count per detected state variant.
    pub min_per_variant: usize,

    /// Hard ceiling — no component may exceed this regardless of anatomy
    /// part count. Acts as the workspace's review-fatigue safety cap.
    pub max_per_component: usize,

    /// Multiplier applied to `state_variants × anatomy_parts` when
    /// computing the per-component soft budget. The soft budget is
    /// `min(per_part_per_variant × variants × parts, max_per_component)`.
    pub per_part_per_variant: usize,
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

    let manifest_components = root
        .manifest
        .components
        .keys()
        .map(|name| name.replace('-', "_"))
        .collect::<BTreeSet<_>>();

    let implemented =
        implemented_adapter_components(&options.leptos_src_dir, &options.dioxus_src_dir)?;

    let components = manifest_components
        .intersection(&implemented)
        .cloned()
        .collect::<BTreeSet<_>>();
    let documented = documented_adapter_components()?;
    let delivery_components = components
        .intersection(&documented)
        .cloned()
        .collect::<BTreeSet<_>>();

    let leptos_counts = adapter_test_counts(&options.leptos_test_dir, &components)?;

    let dioxus_counts = adapter_test_counts(&options.dioxus_test_dir, &components)?;

    let presence = adapter_component_presence(options, &delivery_components)?;

    let (mut output, parity_failures) = adapter_parity_report(
        &components,
        &leptos_counts,
        &dioxus_counts,
        options.tolerance,
        &presence,
    );
    let mut failures = parity_failures;

    failures.extend(adapter_semantic_boundary_failures(
        options,
        &delivery_components,
    )?);

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
    presence: &BTreeMap<String, AdapterComponentPresence>,
) -> (String, Vec<String>) {
    let mut output = String::from("Component | Leptos | Dioxus | Delta | Status\n");

    output.push_str("----------|--------|--------|-------|-------\n");

    let mut failures = Vec::new();

    for component in components {
        let leptos = leptos_counts.get(component).copied().unwrap_or(0);

        let dioxus = dioxus_counts.get(component).copied().unwrap_or(0);

        let delta = leptos.abs_diff(dioxus);

        let status = if leptos == 0 && dioxus == 0 {
            "SKIP"
        } else if leptos == 0 || dioxus == 0 {
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

        if let Some(presence) = presence.get(component) {
            failures.extend(presence.failures(component));
        }

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

    let workspace_root = workspace_root_for(&options.snapshots_dir);

    let state_variants = detect_state_variants(&workspace_root)?;

    let part_counts = detect_part_counts(&workspace_root)?;

    let mut counts = BTreeMap::new();

    for snapshot in snapshots {
        if let Some(component) = infer_snapshot_component(&snapshot)
            && state_variants.contains_key(&component)
        {
            *counts.entry(component).or_insert(0) += 1;
        }
    }

    let mut output =
        String::from("Component | Snapshots | Variants | Parts | Min | Max | Status\n");

    output.push_str("----------|-----------|----------|-------|-----|-----|-------\n");

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

        let parts = part_counts.get(&component).copied().unwrap_or(0);

        let min_per_variant =
            snapshot_min_per_variant(&workspace_root, &component, options.min_per_variant);

        let required = if variants > 2 {
            variants * min_per_variant
        } else {
            0
        };

        // Soft per-component budget — derived from anatomy size when
        // detectable, falling back to the hard ceiling. The formula
        // matches `spec/testing/13-policies.md` §3.1:
        //
        //     budget = min(per_part_per_variant × variants × parts,
        //                  max_per_component)
        //
        // When a component's anatomy parts cannot be detected (no
        // `#[derive(ComponentPart)]` enum found, or the component
        // is not yet implemented), the formula collapses to
        // `max_per_component` so the hard ceiling still applies.
        let formula_budget = if variants > 0 && parts > 0 {
            options
                .per_part_per_variant
                .saturating_mul(variants)
                .saturating_mul(parts)
        } else {
            options.max_per_component
        };

        let budget = formula_budget.min(options.max_per_component);

        let mut status = "OK";

        if required > 0 && count < required {
            failures.push(format!(
                "{component}: snapshots={count}, state_variants={variants}, required={required}"
            ));

            status = "FAIL";
        }

        if count > budget {
            failures.push(format!(
                "{component}: snapshots={count}, budget={budget} \
                 (formula = min({per_part} × {variants} × {parts}, {ceiling}))",
                per_part = options.per_part_per_variant,
                ceiling = options.max_per_component
            ));

            status = "FAIL";
        }

        writeln!(
            output,
            "{component} | {count} | {variants} | {parts} | {required} | {budget} | {status}"
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

    let test_attr = Regex::new(r"#\[\s*(?:wasm_bindgen_)?test\s*(?:\([^)]*\))?\s*\]")?;

    let files = collect_files(dir, is_adapter_test_file)?;

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

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AdapterComponentPresence {
    leptos_ssr_test: bool,
    dioxus_ssr_test: bool,
    leptos_wasm_test: bool,
    dioxus_wasm_test: bool,
    leptos_composition_test: bool,
    dioxus_composition_test: bool,
    all_widgets: bool,
    leptos_e2e_fixture: bool,
    dioxus_e2e_fixture: bool,
    e2e_harness: bool,
    requires_composition: bool,
    requires_wasm: bool,
}

impl AdapterComponentPresence {
    fn failures(&self, component: &str) -> Vec<String> {
        let mut failures = Vec::new();

        for (label, present) in [
            ("Leptos SSR/unit test", self.leptos_ssr_test),
            ("Dioxus SSR/unit test", self.dioxus_ssr_test),
            ("all six widget examples", self.all_widgets),
            ("Leptos E2E fixture", self.leptos_e2e_fixture),
            ("Dioxus E2E fixture", self.dioxus_e2e_fixture),
            ("E2E harness", self.e2e_harness),
        ] {
            if !present {
                failures.push(format!("{component}: missing {label}"));
            }
        }

        if self.requires_wasm {
            for (label, present) in [
                ("Leptos wasm test", self.leptos_wasm_test),
                ("Dioxus wasm test", self.dioxus_wasm_test),
            ] {
                if !present {
                    failures.push(format!("{component}: interactive adapter missing {label}"));
                }
            }
        }

        if self.requires_composition {
            for (label, present) in [
                (
                    "Leptos Form/Fieldset composition test",
                    self.leptos_composition_test,
                ),
                (
                    "Dioxus Form/Fieldset composition test",
                    self.dioxus_composition_test,
                ),
            ] {
                if !present {
                    failures.push(format!(
                        "{component}: context-integrated adapter missing {label}"
                    ));
                }
            }
        }

        failures
    }
}

fn adapter_component_presence(
    options: &AdapterParityOptions,
    components: &BTreeSet<String>,
) -> Result<BTreeMap<String, AdapterComponentPresence>, Error> {
    let mut result = BTreeMap::new();

    for component in components {
        let requires_wasm = component_requires_wasm(&options.leptos_src_dir, component)?
            || component_requires_wasm(&options.dioxus_src_dir, component)?;
        let requires_composition =
            component_requires_composition(&options.leptos_src_dir, component)?
                || component_requires_composition(&options.dioxus_src_dir, component)?;

        let leptos_ssr = adapter_test_path(&options.leptos_test_dir, component, false);
        let dioxus_ssr = adapter_test_path(&options.dioxus_test_dir, component, false);
        let leptos_wasm = adapter_test_path(&options.leptos_test_dir, component, true);
        let dioxus_wasm = adapter_test_path(&options.dioxus_test_dir, component, true);

        let leptos_ssr_content = read_optional(&leptos_ssr)?;
        let dioxus_ssr_content = read_optional(&dioxus_ssr)?;

        result.insert(
            component.clone(),
            AdapterComponentPresence {
                leptos_ssr_test: leptos_ssr_content.is_some(),
                dioxus_ssr_test: dioxus_ssr_content.is_some(),
                leptos_wasm_test: leptos_wasm.exists(),
                dioxus_wasm_test: dioxus_wasm.exists(),
                leptos_composition_test: leptos_ssr_content
                    .as_deref()
                    .is_some_and(has_composition_test),
                dioxus_composition_test: dioxus_ssr_content
                    .as_deref()
                    .is_some_and(has_composition_test),
                all_widgets: all_widget_category_files_exist(options, component),
                leptos_e2e_fixture: e2e_fixture_exists(&options.leptos_e2e_fixture_dir, component),
                dioxus_e2e_fixture: e2e_fixture_exists(&options.dioxus_e2e_fixture_dir, component),
                e2e_harness: e2e_harness_exists(&options.e2e_src_dir, component),
                requires_composition,
                requires_wasm,
            },
        );
    }

    Ok(result)
}

fn adapter_semantic_boundary_failures(
    options: &AdapterParityOptions,
    components: &BTreeSet<String>,
) -> Result<Vec<String>, Error> {
    let mut failures = Vec::new();

    for component in components {
        let leptos_path = component_module_path(&options.leptos_src_dir, component);
        let dioxus_path = component_module_path(&options.dioxus_src_dir, component);

        let Some(leptos_content) = read_optional(&leptos_path)? else {
            continue;
        };

        let Some(dioxus_content) = read_optional(&dioxus_path)? else {
            continue;
        };

        failures.extend(adapter_api_extension_trait_failures(
            component,
            &leptos_path,
            &leptos_content,
        )?);

        failures.extend(adapter_api_extension_trait_failures(
            component,
            &dioxus_path,
            &dioxus_content,
        )?);

        let leptos_helpers = adapter_private_helpers(&leptos_content)?;
        let dioxus_helpers = adapter_private_helpers(&dioxus_content)?;

        for name in leptos_helpers
            .keys()
            .filter(|name| dioxus_helpers.contains_key(*name))
        {
            let leptos_helper = &leptos_helpers[name];
            let dioxus_helper = &dioxus_helpers[name];

            if leptos_helper.has_boundary_marker || dioxus_helper.has_boundary_marker {
                continue;
            }

            failures.push(format!(
                "{component}: duplicated adapter helper `{name}` appears in Leptos and Dioxus; \
                 move renderer-independent logic to the agnostic/shared layer or mark it as \
                 adapter rendering/framework glue with a reason"
            ));
        }
    }

    Ok(failures)
}

#[derive(Debug, Clone)]
struct AdapterPrivateHelper {
    has_boundary_marker: bool,
}

fn adapter_private_helpers(content: &str) -> Result<BTreeMap<String, AdapterPrivateHelper>, Error> {
    let helper = Regex::new(r"(?m)^(?P<indent>\s*)fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*\(")?;
    let mut helpers = BTreeMap::new();

    for captures in helper.captures_iter(content) {
        if captures
            .name("indent")
            .is_some_and(|indent| !indent.as_str().is_empty())
        {
            continue;
        }

        let name = captures["name"].to_owned();

        if is_adapter_boundary_ignored_helper(&name) {
            continue;
        }

        let start = captures
            .get(0)
            .expect("full match exists for helper")
            .start();

        let has_boundary_marker = previous_lines(content, start, 3)
            .iter()
            .any(|line| is_adapter_boundary_marker(line));

        helpers.insert(
            name,
            AdapterPrivateHelper {
                has_boundary_marker,
            },
        );
    }

    Ok(helpers)
}

fn adapter_api_extension_trait_failures(
    component: &str,
    path: &Path,
    content: &str,
) -> Result<Vec<String>, Error> {
    let extension_trait = Regex::new(
        r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?trait\s+[A-Za-z0-9_]*(?:Api|Props|State|Event)Ext\b",
    )?;

    let extension_impl = Regex::new(
        r"(?m)^\s*impl(?:<[^>]+>)?\s+[A-Za-z0-9_]*(?:Api|Props|State|Event)Ext\s+for\s+(?:[A-Za-z0-9_:]+::)?(?:Api|Props|State|Event)\b",
    )?;

    let mut failures = Vec::new();

    if extension_trait.is_match(content) || extension_impl.is_match(content) {
        failures.push(format!(
            "{component}: adapter-local extension trait over agnostic API in {}; \
             add shared methods to the agnostic component API instead",
            path.display()
        ));
    }

    Ok(failures)
}

fn previous_lines(content: &str, offset: usize, count: usize) -> Vec<&str> {
    content[..offset].lines().rev().take(count).collect()
}

fn is_adapter_boundary_marker(line: &str) -> bool {
    [
        "adapter-rendering-glue",
        "adapter-framework-glue",
        "adapter-context-glue",
        "adapter-prop-glue",
    ]
    .iter()
    .any(|marker| line.contains(marker))
}

fn is_adapter_boundary_ignored_helper(name: &str) -> bool {
    [
        "attr_",
        "add_dynamic_",
        "apply_",
        "use_",
        "build_",
        "render_",
        "view_",
        "merge_",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
}

fn is_adapter_test_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            (name.starts_with("test_") || !name.starts_with('_')) && name.ends_with(".rs")
        })
}

fn adapter_test_path(dir: &Path, component: &str, wasm: bool) -> PathBuf {
    let suffix = if wasm { "_wasm.rs" } else { ".rs" };
    let preferred = dir.join(format!("{component}{suffix}"));

    if preferred.exists() {
        return preferred;
    }

    dir.join(format!("test_{component}{suffix}"))
}

fn read_optional(path: &Path) -> Result<Option<String>, Error> {
    if path.exists() {
        Ok(Some(fs::read_to_string(path)?))
    } else {
        Ok(None)
    }
}

fn has_composition_test(content: &str) -> bool {
    content.contains("Fieldset") && content.contains("Form") && content.contains("validation")
}

fn component_requires_wasm(src_dir: &Path, component: &str) -> Result<bool, Error> {
    Ok(
        read_component_source(src_dir, component)?.is_some_and(|content| {
            [
                "onclick",
                "onkeydown",
                "on_submit",
                "on_reset",
                "on_checked_change",
                "use_machine",
            ]
            .iter()
            .any(|needle| content.contains(needle))
        }),
    )
}

fn component_requires_composition(src_dir: &Path, component: &str) -> Result<bool, Error> {
    Ok(
        read_component_source(src_dir, component)?.is_some_and(|content| {
            [
                "field_support",
                "use_fieldset",
                "use_form",
                "validation_errors",
            ]
            .iter()
            .any(|needle| content.contains(needle))
        }),
    )
}

fn read_component_source(src_dir: &Path, component: &str) -> Result<Option<String>, Error> {
    let path = component_module_path(src_dir, component);

    if !path.exists() {
        return Ok(None);
    }

    if path.file_name().and_then(|name| name.to_str()) != Some("mod.rs") {
        return read_optional(&path);
    }

    let Some(component_dir) = path.parent() else {
        return read_optional(&path);
    };

    let mut content = String::new();

    for path in rust_files_under(component_dir)? {
        content.push_str(&fs::read_to_string(path)?);
        content.push('\n');
    }

    Ok(Some(content))
}

fn rust_files_under(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::new();
    collect_rust_files(dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_rust_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), Error> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();

        if path.is_dir() {
            collect_rust_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }

    Ok(())
}

fn component_module_path(src_dir: &Path, component: &str) -> PathBuf {
    for category in component_category_dirs() {
        let nested = src_dir.join(category).join(component).join("mod.rs");

        if nested.exists() {
            return nested;
        }

        let flat = src_dir.join(category).join(format!("{component}.rs"));

        if flat.exists() {
            return flat;
        }
    }

    src_dir.join(format!("{component}.rs"))
}

fn implemented_adapter_components(
    leptos_src: &Path,
    dioxus_src: &Path,
) -> Result<BTreeSet<String>, Error> {
    let mut components = BTreeSet::new();

    for src in [leptos_src, dioxus_src] {
        if !src.exists() {
            continue;
        }

        for category in component_category_dirs() {
            let dir = src.join(category);

            if !dir.exists() {
                continue;
            }

            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    if let Some(name) = path.file_stem().and_then(|stem| stem.to_str())
                        && name != "mod"
                    {
                        components.insert(name.to_string());
                    }
                } else if path.join("mod.rs").exists()
                    && let Some(name) = path.file_name().and_then(|name| name.to_str())
                {
                    components.insert(name.to_string());
                }
            }
        }
    }

    Ok(components)
}

fn all_widget_category_files_exist(options: &AdapterParityOptions, component: &str) -> bool {
    widget_category_for(component).is_some_and(|category| {
        [
            &options.leptos_widgets_dir,
            &options.dioxus_widgets_dir,
            &options.leptos_css_widgets_dir,
            &options.dioxus_css_widgets_dir,
            &options.leptos_tailwind_widgets_dir,
            &options.dioxus_tailwind_widgets_dir,
        ]
        .iter()
        .all(|dir| widget_category_file_contains_component(dir, category, component))
    })
}

fn widget_category_file_contains_component(dir: &Path, category: &str, component: &str) -> bool {
    let path = dir.join("src/categories").join(format!("{category}.rs"));
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };

    file_content_mentions_component(&content, component)
}

fn file_content_mentions_component(content: &str, component: &str) -> bool {
    let kebab_component = component.replace('_', "-");
    let pascal_component = component
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<String>();

    content.contains(component)
        || content.contains(&kebab_component)
        || content.contains(&pascal_component)
}

fn e2e_fixture_exists(dir: &Path, component: &str) -> bool {
    widget_category_for(component).is_some_and(|category| {
        e2e_flat_fixture_contains_component(dir, category, component)
            || dir
                .join("src/categories")
                .join(category)
                .join(format!("{component}.rs"))
                .exists()
    })
}

fn e2e_flat_fixture_contains_component(dir: &Path, category: &str, component: &str) -> bool {
    let path = dir.join("src/categories").join(format!("{category}.rs"));
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };

    file_content_mentions_component(&content, component)
}

fn e2e_harness_exists(dir: &Path, component: &str) -> bool {
    widget_category_for(component)
        .is_some_and(|category| dir.join(category).join(format!("{component}.rs")).exists())
}

fn widget_category_for(component: &str) -> Option<&'static str> {
    match component {
        "checkbox" => Some("input"),
        "button" | "field" | "fieldset" | "form" => Some("utility"),
        "tabs" => Some("navigation"),
        _ => None,
    }
}

const fn component_category_dirs() -> &'static [&'static str] {
    &[
        "input",
        "selection",
        "overlay",
        "navigation",
        "date_time",
        "data_display",
        "layout",
        "specialized",
        "utility",
    ]
}

fn documented_adapter_components() -> Result<BTreeSet<String>, Error> {
    let dir = Path::new("docs/implementation/adapter-components");

    if !dir.exists() {
        return Ok(BTreeSet::new());
    }

    let mut components = BTreeSet::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if let Some(name) = path.file_name().and_then(|name| name.to_str())
            && let Some(component) = name.strip_suffix("-usage.md")
        {
            components.insert(component.replace('-', "_"));
        }
    }

    Ok(components)
}

fn extract_component_from_test_file(
    path: &Path,
    known_components: &BTreeSet<String>,
) -> Option<String> {
    let stem = path
        .file_stem()?
        .to_str()?
        .strip_prefix("test_")
        .unwrap_or(path.file_stem()?.to_str()?);

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
    let stem = path.file_stem()?.to_str()?;

    let compact = stem.split("__").collect::<Vec<_>>();

    if compact
        .first()
        .is_some_and(|crate_name| crate_name.starts_with("ars_"))
    {
        if compact
            .get(1)
            .is_some_and(|segment| is_component_category(segment))
            && let Some(component) = compact.get(2)
        {
            return Some((*component).replace('-', "_"));
        }

        if let Some(component) = compact.get(1) {
            return Some((*component).replace('-', "_"));
        }
    }

    for ancestor in path.ancestors() {
        if ancestor.file_name().and_then(|name| name.to_str()) == Some("snapshots")
            && let Some(parent) = ancestor.parent()
            && let Some(component) = parent.file_name().and_then(|name| name.to_str())
            && component != "tests"
            && component != "src"
            && !is_component_category(component)
        {
            return Some(component.replace('-', "_"));
        }
    }

    compact
        .windows(2)
        .find_map(|parts| (parts[1] == "component").then(|| parts[0].replace('-', "_")))
        .or_else(|| stem.split("__").next().map(|name| name.replace('-', "_")))
}

fn detect_state_variants(root: &Path) -> Result<BTreeMap<String, usize>, Error> {
    let enum_start = Regex::new(r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?enum\s+State\s*\{")?;

    let state_alias =
        Regex::new(r"(?m)^\s*pub\s+type\s+State\s*=\s*(?P<target>[A-Za-z0-9_:]+)::State\s*;")?;

    let variant = Regex::new(r"^\s*([A-Z][A-Za-z0-9_]*)\s*(?:[{(,]|$)")?;

    let files = collect_files(root, |path| {
        path.extension().is_some_and(|extension| extension == "rs")
            && path
                .components()
                .any(|component| component.as_os_str() == "src")
            && (path.file_name().is_some_and(|name| name == "component.rs")
                || is_ars_components_machine_module(path))
            && !path
                .components()
                .any(|component| component.as_os_str() == "target")
    })?;

    let mut variants = BTreeMap::new();
    let mut aliases = Vec::new();

    for file in files {
        let content = fs::read_to_string(&file)?;

        let Some(component) = infer_source_component(&file) else {
            continue;
        };

        if enum_start.find(&content).is_some() {
            let count = parse_state_variant_count(&content, &variant);

            if count > 0 {
                variants
                    .entry(component.clone())
                    .and_modify(|existing: &mut usize| *existing = (*existing).max(count))
                    .or_insert(count);
            }
        }

        if let Some(captures) = state_alias.captures(&content)
            && let Some(target) = captures.name("target").and_then(alias_target_component)
        {
            aliases.push((component, target));
        }
    }

    for (component, target) in aliases {
        if let Some(count) = variants.get(&target).copied() {
            variants
                .entry(component)
                .and_modify(|existing: &mut usize| *existing = (*existing).max(count))
                .or_insert(count);
        }
    }

    Ok(variants)
}

fn parse_state_variant_count(content: &str, variant: &Regex) -> usize {
    parse_enum_variant_count(content, "enum State", variant)
}

fn alias_target_component(target: regex::Match<'_>) -> Option<String> {
    target
        .as_str()
        .rsplit("::")
        .next()
        .map(|component| component.replace('-', "_"))
}

/// Counts variants of any `pub enum Part` decorated with a
/// [`ComponentPart`] derive — the canonical way component machines
/// declare their anatomy. Returns the maximum count across all matching
/// enums in `content`, so a module that re-exports multiple Part enums
/// still surfaces the largest anatomy.
///
/// `derive_marker` is the regex compiled by [`detect_part_counts`] — it
/// matches `#[derive(...)]` attributes that include `ComponentPart` as a
/// token regardless of path qualification (`ComponentPart`,
/// `ars_core::ComponentPart`, etc.) and regardless of whether the derive
/// list contains other traits.
///
/// [`ComponentPart`]: ars_core::ComponentPart
fn parse_part_variant_count(content: &str, derive_marker: &Regex, variant: &Regex) -> usize {
    let mut max = 0;
    let mut search_start = 0;

    while let Some(m) = derive_marker.find(&content[search_start..]) {
        // Skip past the derive attribute and look for the next `enum`.
        let derive_end_abs = search_start + m.end();

        if let Some(enum_offset) = content[derive_end_abs..].find("enum ") {
            let enum_abs = derive_end_abs + enum_offset;

            let count = parse_enum_variant_count(&content[enum_abs..], "enum ", variant);

            max = max.max(count);

            search_start = enum_abs + "enum ".len();
        } else {
            break;
        }
    }

    max
}

fn parse_enum_variant_count(content: &str, marker: &str, variant: &Regex) -> usize {
    let Some(start) = content.find(marker) else {
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

/// Detects the anatomy-part count for each component by walking source
/// files for `#[derive(ComponentPart)] enum Part { … }` and counting
/// variants. Components without an anatomy enum (or with their Part enum
/// in a non-standard location) report 0 — the caller falls back to the
/// hard ceiling for the budget.
fn detect_part_counts(root: &Path) -> Result<BTreeMap<String, usize>, Error> {
    let variant = Regex::new(r"^\s*([A-Z][A-Za-z0-9_]*)\s*(?:[{(,]|$)")?;

    // Recognize both the bare `#[derive(ComponentPart)]` form and any
    // path-qualified spelling such as `#[derive(ars_core::ComponentPart)]`,
    // and also tolerate ComponentPart appearing alongside other derives
    // (`#[derive(Clone, ComponentPart, Debug)]`). Word boundaries keep us
    // from matching look-alikes like `ComponentPartial`.
    let derive_marker = Regex::new(r"#\[derive\([^)]*\bComponentPart\b[^)]*\)\]")?;

    let files = collect_files(root, |path| {
        path.extension().is_some_and(|extension| extension == "rs")
            && path
                .components()
                .any(|component| component.as_os_str() == "src")
            && (path.file_name().is_some_and(|name| name == "component.rs")
                || is_ars_components_machine_module(path))
            && !path
                .components()
                .any(|component| component.as_os_str() == "target")
    })?;

    let mut parts = BTreeMap::new();

    for file in files {
        let content = fs::read_to_string(&file)?;

        if !derive_marker.is_match(&content) {
            continue;
        }

        let Some(component) = infer_source_component(&file) else {
            continue;
        };

        let count = parse_part_variant_count(&content, &derive_marker, &variant);

        if count > 0 {
            parts
                .entry(component)
                .and_modify(|existing: &mut usize| *existing = (*existing).max(count))
                .or_insert(count);
        }
    }

    Ok(parts)
}

fn infer_source_component(path: &Path) -> Option<String> {
    let mut parts = path.components().peekable();

    while let Some(part) = parts.next() {
        if part.as_os_str() == "src" {
            let remaining = parts
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>();

            if remaining
                .first()
                .is_some_and(|segment| is_component_category(segment))
                && let Some(component) = remaining.get(1)
            {
                let component_path = Path::new(component);

                return if component_path
                    .extension()
                    .is_some_and(|extension| extension == "rs")
                {
                    component_path
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .map(str::to_owned)
                } else {
                    Some(component.replace('-', "_"))
                };
            }

            return remaining
                .first()
                .and_then(|component| {
                    let path = Path::new(component);

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

fn is_component_category(segment: &str) -> bool {
    // Both the hyphenated spec form (`date-time`) and the underscored source /
    // snapshot-file form (`date_time`) must be recognized: source directories
    // and insta snapshot filenames use underscores, while spec paths use
    // hyphens. Matching only the hyphenated form silently dropped the
    // `date_time` and `data_display` categories from the snapshot-count gate.
    matches!(
        segment,
        "data-display"
            | "data_display"
            | "date-time"
            | "date_time"
            | "input"
            | "layout"
            | "navigation"
            | "overlay"
            | "selection"
            | "specialized"
            | "utility"
    )
}

fn is_ars_components_machine_module(path: &Path) -> bool {
    let segments = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    let Some(crate_index) = segments
        .iter()
        .position(|segment| segment == "ars-components")
    else {
        return false;
    };

    let Some(src_index) = segments.iter().position(|segment| segment == "src") else {
        return false;
    };

    crate_index < src_index
        && segments
            .get(src_index + 1)
            .is_some_and(|segment| is_component_category(segment))
        && !segments.iter().any(|segment| segment == "snapshots")
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

fn snapshot_min_per_variant(root: &Path, component: &str, default_min: usize) -> usize {
    if is_ars_components_component(root, component) {
        1
    } else {
        default_min
    }
}

fn is_ars_components_component(root: &Path, component: &str) -> bool {
    // These are joined as filesystem paths under `crates/ars-components/src`, so
    // they must use the underscored source-directory names (`date_time`,
    // `data_display`) — not the hyphenated spec forms. Using hyphens here meant
    // the function never matched date-time / data-display machines, so
    // `snapshot_min_per_variant` fell back to the rendered-component floor (3
    // per variant) instead of the spec's 1-per-variant floor for those crates.
    for category in [
        "input",
        "selection",
        "overlay",
        "navigation",
        "date_time",
        "data_display",
        "layout",
        "specialized",
        "utility",
    ] {
        let direct = root
            .join("crates/ars-components/src")
            .join(category)
            .join(format!("{component}.rs"));
        let nested = root
            .join("crates/ars-components/src")
            .join(category)
            .join(component)
            .join("mod.rs");

        if direct.exists() || nested.exists() {
            return true;
        }
    }

    false
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
    fn component_category_recognizes_underscore_and_hyphen_forms() {
        // Source directories and insta snapshot filenames use underscores
        // (`date_time`), while spec paths use hyphens (`date-time`). Both forms
        // must be recognized so the snapshot-count gate covers the `date_time`
        // and `data_display` categories — matching only the hyphen form
        // silently dropped them.
        for category in ["date_time", "data_display", "date-time", "data-display"] {
            assert!(
                is_component_category(category),
                "{category} should be recognized as a component category",
            );
        }

        assert!(!is_component_category("not_a_category"));
    }

    #[test]
    fn ars_components_machine_in_underscored_category_gets_one_per_variant_floor() {
        // `is_ars_components_component` joins filesystem paths, so it must match
        // the underscored source dirs (`date_time`, `data_display`). When it
        // does, a machine module gets the spec's 1-per-variant snapshot floor,
        // not the rendered-component default.
        let root = temp_dir("floor-underscore");
        write(
            &root.join("crates/ars-components/src/date_time/picker/mod.rs"),
            "pub struct Machine;\n",
        );
        write(
            &root.join("crates/ars-components/src/data_display/gauge/mod.rs"),
            "pub struct Machine;\n",
        );

        assert!(is_ars_components_component(&root, "picker"));
        assert!(is_ars_components_component(&root, "gauge"));
        assert_eq!(snapshot_min_per_variant(&root, "picker", 3), 1);
        assert_eq!(snapshot_min_per_variant(&root, "gauge", 3), 1);
        // A component that does not exist still falls back to the default floor.
        assert_eq!(snapshot_min_per_variant(&root, "missing", 3), 3);
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
    fn adapter_parity_counts_component_named_test_files() {
        let root = temp_dir("adapter-component-names");

        let leptos = root.join("leptos");
        let dioxus = root.join("dioxus");

        write(
            &leptos.join("checkbox.rs"),
            "#[test]\nfn ssr() {}\n#[test]\nfn composition() {}\n",
        );
        write(
            &leptos.join("checkbox_wasm.rs"),
            "#[wasm_bindgen_test(async)]\nasync fn browser() {}\n",
        );
        write(
            &dioxus.join("checkbox.rs"),
            "#[test]\nfn ssr() {}\n#[test]\nfn composition() {}\n",
        );
        write(
            &dioxus.join("checkbox_wasm.rs"),
            "#[wasm_bindgen_test]\nfn browser() {}\n",
        );

        let components = BTreeSet::from(["checkbox".to_owned()]);

        let leptos_counts = adapter_test_counts(&leptos, &components).expect("leptos counts");
        let dioxus_counts = adapter_test_counts(&dioxus, &components).expect("dioxus counts");

        assert_eq!(leptos_counts["checkbox"], 3);
        assert_eq!(dioxus_counts["checkbox"], 3);

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn adapter_presence_requires_composition_wasm_widgets_and_e2e_for_checkbox() {
        let root = temp_dir("adapter-presence");

        let options = AdapterParityOptions {
            leptos_test_dir: root.join("crates/ars-leptos/tests"),
            dioxus_test_dir: root.join("crates/ars-dioxus/tests"),
            leptos_src_dir: root.join("crates/ars-leptos/src"),
            dioxus_src_dir: root.join("crates/ars-dioxus/src"),
            leptos_widgets_dir: root.join("examples/widgets-leptos"),
            dioxus_widgets_dir: root.join("examples/widgets-dioxus"),
            leptos_css_widgets_dir: root.join("examples/widgets-leptos-css"),
            dioxus_css_widgets_dir: root.join("examples/widgets-dioxus-css"),
            leptos_tailwind_widgets_dir: root.join("examples/widgets-leptos-tailwind"),
            dioxus_tailwind_widgets_dir: root.join("examples/widgets-dioxus-tailwind"),
            leptos_e2e_fixture_dir: root.join("crates/ars-e2e/fixtures/leptos"),
            dioxus_e2e_fixture_dir: root.join("crates/ars-e2e/fixtures/dioxus"),
            e2e_src_dir: root.join("crates/ars-e2e/src"),
            tolerance: 2,
        };

        write(
            &options.leptos_src_dir.join("input/checkbox.rs"),
            "field_support on_checked_change",
        );
        write(
            &options.dioxus_src_dir.join("input/checkbox.rs"),
            "field_support on_checked_change",
        );
        write(
            &options.leptos_test_dir.join("checkbox.rs"),
            "#[test]\nfn one() {}\n",
        );
        write(
            &options.dioxus_test_dir.join("checkbox.rs"),
            "#[test]\nfn one() {}\n",
        );

        let components = BTreeSet::from(["checkbox".to_owned()]);
        let presence = adapter_component_presence(&options, &components).expect("presence");
        let failures = presence["checkbox"].failures("checkbox");

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("Leptos wasm test")),
            "{failures:?}"
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("composition test")),
            "{failures:?}"
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("all six widget examples")),
            "{failures:?}"
        );
        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("E2E harness")),
            "{failures:?}"
        );

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn widget_category_file_must_mention_component() {
        let root = temp_dir("widget-category-component");
        let widgets = root.join("examples/widgets-leptos");

        write(
            &widgets.join("src/categories/input.rs"),
            "pub fn input_panel() { Checkbox {} }\n",
        );

        assert!(widget_category_file_contains_component(
            &widgets, "input", "checkbox"
        ));
        assert!(!widget_category_file_contains_component(
            &widgets,
            "input",
            "text_field"
        ));

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn component_source_scan_includes_directory_child_modules() {
        let root = temp_dir("component-source-dir");
        let src = root.join("crates/ars-dioxus/src");

        write(&src.join("input/checkbox/mod.rs"), "mod control;\n");
        write(
            &src.join("input/checkbox/control.rs"),
            "on_checked_change\n",
        );

        assert!(
            component_requires_wasm(&src, "checkbox").expect("scan component source"),
            "child module event handlers must require wasm coverage"
        );

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn e2e_flat_fixture_must_mention_component() {
        let root = temp_dir("e2e-flat-fixture-component");
        let fixtures = root.join("crates/ars-e2e/fixtures/leptos");

        write(
            &fixtures.join("src/categories/input.rs"),
            "pub fn input_panel() { TextField {} }\n",
        );

        assert!(!e2e_fixture_exists(&fixtures, "checkbox"));

        write(
            &fixtures.join("src/categories/input.rs"),
            "pub fn input_panel() { Checkbox {} }\n",
        );

        assert!(e2e_fixture_exists(&fixtures, "checkbox"));

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn adapter_semantic_boundary_flags_duplicated_private_helpers() {
        let root = temp_dir("adapter-semantic-duplicated");

        let options = AdapterParityOptions {
            leptos_test_dir: root.join("crates/ars-leptos/tests"),
            dioxus_test_dir: root.join("crates/ars-dioxus/tests"),
            leptos_src_dir: root.join("crates/ars-leptos/src"),
            dioxus_src_dir: root.join("crates/ars-dioxus/src"),
            leptos_widgets_dir: root.join("examples/widgets-leptos"),
            dioxus_widgets_dir: root.join("examples/widgets-dioxus"),
            leptos_css_widgets_dir: root.join("examples/widgets-leptos-css"),
            dioxus_css_widgets_dir: root.join("examples/widgets-dioxus-css"),
            leptos_tailwind_widgets_dir: root.join("examples/widgets-leptos-tailwind"),
            dioxus_tailwind_widgets_dir: root.join("examples/widgets-dioxus-tailwind"),
            leptos_e2e_fixture_dir: root.join("crates/ars-e2e/fixtures/leptos"),
            dioxus_e2e_fixture_dir: root.join("crates/ars-e2e/fixtures/dioxus"),
            e2e_src_dir: root.join("crates/ars-e2e/src"),
            tolerance: 2,
        };

        write(
            &options.leptos_src_dir.join("input/checkbox.rs"),
            "fn requested_toggle_state() -> State { State::Checked }\n",
        );
        write(
            &options.dioxus_src_dir.join("input/checkbox.rs"),
            "fn requested_toggle_state() -> State { State::Checked }\n",
        );

        let components = BTreeSet::from(["checkbox".to_owned()]);
        let failures =
            adapter_semantic_boundary_failures(&options, &components).expect("semantic failures");

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("duplicated adapter helper")),
            "{failures:?}"
        );

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn adapter_semantic_boundary_allows_marked_renderer_glue() {
        let root = temp_dir("adapter-semantic-marked");

        let options = AdapterParityOptions {
            leptos_test_dir: root.join("crates/ars-leptos/tests"),
            dioxus_test_dir: root.join("crates/ars-dioxus/tests"),
            leptos_src_dir: root.join("crates/ars-leptos/src"),
            dioxus_src_dir: root.join("crates/ars-dioxus/src"),
            leptos_widgets_dir: root.join("examples/widgets-leptos"),
            dioxus_widgets_dir: root.join("examples/widgets-dioxus"),
            leptos_css_widgets_dir: root.join("examples/widgets-leptos-css"),
            dioxus_css_widgets_dir: root.join("examples/widgets-dioxus-css"),
            leptos_tailwind_widgets_dir: root.join("examples/widgets-leptos-tailwind"),
            dioxus_tailwind_widgets_dir: root.join("examples/widgets-dioxus-tailwind"),
            leptos_e2e_fixture_dir: root.join("crates/ars-e2e/fixtures/leptos"),
            dioxus_e2e_fixture_dir: root.join("crates/ars-e2e/fixtures/dioxus"),
            e2e_src_dir: root.join("crates/ars-e2e/src"),
            tolerance: 2,
        };

        write(
            &options.leptos_src_dir.join("input/checkbox.rs"),
            "// adapter-rendering-glue: needs framework event conversion\nfn event_value() -> bool { true }\n",
        );
        write(
            &options.dioxus_src_dir.join("input/checkbox.rs"),
            "fn event_value() -> bool { true }\n",
        );

        let components = BTreeSet::from(["checkbox".to_owned()]);
        let failures =
            adapter_semantic_boundary_failures(&options, &components).expect("semantic failures");

        assert!(failures.is_empty(), "{failures:?}");

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn adapter_semantic_boundary_flags_api_extension_traits() {
        let root = temp_dir("adapter-semantic-ext");

        let options = AdapterParityOptions {
            leptos_test_dir: root.join("crates/ars-leptos/tests"),
            dioxus_test_dir: root.join("crates/ars-dioxus/tests"),
            leptos_src_dir: root.join("crates/ars-leptos/src"),
            dioxus_src_dir: root.join("crates/ars-dioxus/src"),
            leptos_widgets_dir: root.join("examples/widgets-leptos"),
            dioxus_widgets_dir: root.join("examples/widgets-dioxus"),
            leptos_css_widgets_dir: root.join("examples/widgets-leptos-css"),
            dioxus_css_widgets_dir: root.join("examples/widgets-dioxus-css"),
            leptos_tailwind_widgets_dir: root.join("examples/widgets-leptos-tailwind"),
            dioxus_tailwind_widgets_dir: root.join("examples/widgets-dioxus-tailwind"),
            leptos_e2e_fixture_dir: root.join("crates/ars-e2e/fixtures/leptos"),
            dioxus_e2e_fixture_dir: root.join("crates/ars-e2e/fixtures/dioxus"),
            e2e_src_dir: root.join("crates/ars-e2e/src"),
            tolerance: 2,
        };

        write(
            &options.leptos_src_dir.join("input/checkbox.rs"),
            "trait CheckboxApiExt {}\nimpl CheckboxApiExt for Api<'_> {}\n",
        );
        write(&options.dioxus_src_dir.join("input/checkbox.rs"), "");

        let components = BTreeSet::from(["checkbox".to_owned()]);
        let failures =
            adapter_semantic_boundary_failures(&options, &components).expect("semantic failures");

        assert!(
            failures
                .iter()
                .any(|failure| failure.contains("extension trait")),
            "{failures:?}"
        );

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn adapter_parity_counts_async_wasm_bindgen_tests() {
        // Regression: `#[wasm_bindgen_test(async)]` (Leptos wasm test
        // harness style) must be counted just like the bare
        // `#[wasm_bindgen_test]` (Dioxus harness style) — otherwise wasm
        // suites with the parameterized form silently report zero tests
        // and the parity check breaks.
        let root = temp_dir("adapter-async-wasm");

        let leptos = root.join("leptos");
        let dioxus = root.join("dioxus");

        write(
            &leptos.join("test_widget_wasm.rs"),
            "#[wasm_bindgen_test(async)]\nasync fn one() {}\n#[wasm_bindgen_test(async)]\nasync fn two() {}\n",
        );

        write(
            &dioxus.join("test_widget_wasm.rs"),
            "#[wasm_bindgen_test]\nfn one() {}\n#[wasm_bindgen_test]\nfn two() {}\n",
        );

        let components = BTreeSet::from(["widget".to_owned()]);

        let leptos_counts = adapter_test_counts(&leptos, &components).expect("leptos counts");
        let dioxus_counts = adapter_test_counts(&dioxus, &components).expect("dioxus counts");

        assert_eq!(leptos_counts["widget"], 2);
        assert_eq!(dioxus_counts["widget"], 2);

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

        let (_output, failures) = adapter_parity_report(
            &components,
            &leptos_counts,
            &dioxus_counts,
            2,
            &BTreeMap::new(),
        );

        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("both adapters must have tests"));
    }

    #[test]
    fn adapter_parity_reports_manifest_components_with_no_tests_as_skipped() {
        let components = BTreeSet::from(["button".to_owned()]);

        let (output, failures) = adapter_parity_report(
            &components,
            &BTreeMap::new(),
            &BTreeMap::new(),
            2,
            &BTreeMap::new(),
        );

        assert!(failures.is_empty());
        assert!(output.contains("button | 0 | 0 | 0 | SKIP"));
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
            per_part_per_variant: 3,
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
            per_part_per_variant: 3,
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
            per_part_per_variant: 3,
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
            per_part_per_variant: 3,
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
            per_part_per_variant: 3,
        };

        let output = check_snapshot_count(&options).expect("warning should not fail");

        assert!(output.contains("Snapshot count warnings"));
        assert!(output.contains("total snapshots=501"));

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn detect_part_counts_recognizes_namespaced_component_part_derive() {
        // Components written as `#[derive(ars_core::ComponentPart)]`
        // (path-qualified) must produce the same `parts` count as
        // components using the bare `#[derive(ComponentPart)]` form;
        // otherwise `compute_max_per_component` falls back to the
        // hard ceiling and the snapshot-budget formula is silently
        // bypassed for those components.
        let root = temp_dir("part-counts-namespaced");

        // Bare derive — control case.
        write(
            &root.join("crates/ars-components/src/utility/bare/mod.rs"),
            "#[derive(ComponentPart)]\npub enum BarePart {\n    Root,\n    Trigger,\n    Content,\n}\n",
        );

        // Namespaced derive — the exact form used by `form_submit.rs`.
        write(
            &root.join("crates/ars-components/src/utility/namespaced/mod.rs"),
            "#[derive(ars_core::ComponentPart)]\npub enum NamespacedPart {\n    Root,\n    Trigger,\n    Content,\n    Status,\n}\n",
        );

        // Combined-with-other-derives — also a legal Rust spelling we
        // shouldn't miss.
        write(
            &root.join("crates/ars-components/src/utility/combined/mod.rs"),
            "#[derive(Clone, ComponentPart, Debug)]\npub enum CombinedPart {\n    Root,\n    Trigger,\n}\n",
        );

        // Look-alike that must NOT match — defends the `\bComponentPart\b`
        // word boundaries against false positives.
        write(
            &root.join("crates/ars-components/src/utility/lookalike/mod.rs"),
            "#[derive(MyComponentPartial)]\npub enum LookalikePart {\n    Root,\n}\n",
        );

        let counts = detect_part_counts(&root).expect("detect_part_counts");

        assert_eq!(counts.get("bare"), Some(&3), "bare derive must be detected");
        assert_eq!(
            counts.get("namespaced"),
            Some(&4),
            "namespaced derive must be detected"
        );
        assert_eq!(
            counts.get("combined"),
            Some(&2),
            "ComponentPart inside a multi-derive list must be detected"
        );
        assert_eq!(
            counts.get("lookalike"),
            None,
            "ComponentPartial must not be misread as ComponentPart"
        );

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

    #[test]
    fn infer_snapshot_component_handles_ars_components_snapshot_names() {
        let path = Path::new(
            "crates/ars-components/src/overlay/snapshots/ars_components__overlay__presence__tests__presence_root_mounted.snap",
        );

        assert_eq!(infer_snapshot_component(path).as_deref(), Some("presence"));
    }

    #[test]
    fn infer_source_component_handles_ars_components_nested_modules() {
        let file = Path::new("crates/ars-components/src/utility/field/mod.rs");
        let file_direct = Path::new("crates/ars-components/src/overlay/presence/mod.rs");

        assert_eq!(infer_source_component(file).as_deref(), Some("field"));
        assert_eq!(
            infer_source_component(file_direct).as_deref(),
            Some("presence")
        );
    }

    #[test]
    fn snapshot_count_detects_ars_components_machine_modules() {
        let root = temp_dir("snapshot-ars-components");

        write(
            &root.join("crates/ars-components/src/overlay/presence/mod.rs"),
            "pub enum State {\n    Unmounted,\n    Mounting,\n    Mounted,\n    UnmountPending,\n}\n",
        );

        for idx in 0..4 {
            write(
                &root.join(format!(
                    "crates/ars-components/src/overlay/snapshots/ars_components__overlay__presence__tests__presence_root_{idx}.snap"
                )),
                "snapshot",
            );
        }

        let options = SnapshotCountOptions {
            snapshots_dir: root.join("crates"),
            min_per_variant: 3,
            max_per_component: 20,
            per_part_per_variant: 3,
        };

        let output = check_snapshot_count(&options).expect("ars-components snapshots should count");

        assert!(output.contains("presence | 4 | 4 | 0 | 4 | 20 | OK"));

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn snapshot_count_detects_state_alias_machine_modules() {
        let root = temp_dir("snapshot-state-alias");

        write(
            &root.join("crates/ars-components/src/overlay/dialog/mod.rs"),
            "pub enum State {\n    Closed,\n    Open,\n}\n",
        );

        write(
            &root.join("crates/ars-components/src/overlay/alert_dialog/mod.rs"),
            "pub type State = dialog::State;\n",
        );

        for idx in 0..2 {
            write(
                &root.join(format!(
                    "crates/ars-components/src/overlay/alert_dialog/snapshots/ars_components__overlay__alert_dialog__tests__alert_dialog_root_{idx}.snap"
                )),
                "snapshot",
            );
        }

        let options = SnapshotCountOptions {
            snapshots_dir: root.join("crates"),
            min_per_variant: 3,
            max_per_component: 20,
            per_part_per_variant: 3,
        };

        let output = check_snapshot_count(&options)
            .expect("state alias components should participate in snapshot-count");

        assert!(output.contains("alert_dialog | 2 | 2 | 0 | 0 | 20 | OK"));

        drop(fs::remove_dir_all(root));
    }
}
