//! Code coverage threshold enforcement.
//!
//! Parses lcov output from `cargo-llvm-cov` and verifies per-crate line and
//! branch coverage against configurable thresholds. Used by CI to gate merges
//! and by developers for local coverage checks.

use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fmt::{self, Display, Write},
    fs, io,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::{self, Output},
};

use serde::Deserialize;

/// Per-crate coverage threshold.
#[derive(Debug, Clone)]
pub struct CrateThreshold {
    /// Crate name as it appears in the workspace (e.g., `ars-core`).
    pub package: String,

    /// Minimum line coverage percentage (0.0–100.0).
    pub min_line: f64,

    /// Minimum branch coverage percentage (0.0–100.0).
    pub min_branch: f64,
}

/// Default thresholds for all enforced crates.
///
/// These are ratcheted from current baselines. As coverage improves, raise them
/// toward the spec targets in `spec/testing/13-policies.md` §4.
///
/// | Crate                    | Current (line/branch) | Enforced (line/branch) |
/// |--------------------------|-----------------------|------------------------|
/// | ars-core                 | 77.1 / 75.0           | 75 / 70                |
/// | ars-a11y                 | 81.8 / 70.6           | 80 / 70                |
/// | ars-collections          | 99.5 / 92.0           | 95 / 90                |
/// | ars-components           | 90.5 / 92.3           | 90 / 90                |
/// | ars-dom                  | 85.2 / 52.6           | 85 / 50                |
/// | ars-forms                | 95.8 / 76.5           | 90 / 75                |
/// | ars-i18n                 | 96.4 / 69.1           | 96 / 65                |
/// | ars-interactions         | 99.9 / 95.0           | 98 / 90                |
/// | ars-leptos               | 74.5 / 89.3           | 74 / 55                |
/// | ars-dioxus               | 77.6 / 91.0           | 77 / 70                |
/// | ars-test-harness         |  99.8 / n/a           |  99 / 0                |
/// | ars-test-harness-leptos  | 62.4 / 50.0           | 60 / 0                 |
/// | ars-test-harness-dioxus  | 64.1 / 75.0           | 60 / 0                 |
/// | ars-derive               | 97.9 / 83.3           | 95 / 80                |
/// | xtask                    | 30.4 / 37.7           | 30 / 35                |
pub fn default_thresholds() -> Vec<CrateThreshold> {
    vec![
        CrateThreshold {
            package: "ars-core".into(),
            min_line: 75.0,
            min_branch: 70.0,
        },
        CrateThreshold {
            package: "ars-a11y".into(),
            min_line: 80.0,
            min_branch: 70.0,
        },
        CrateThreshold {
            package: "ars-collections".into(),
            min_line: 95.0,
            min_branch: 90.0,
        },
        CrateThreshold {
            package: "ars-components".into(),
            min_line: 90.0,
            min_branch: 90.0,
        },
        CrateThreshold {
            package: "ars-forms".into(),
            min_line: 90.0,
            min_branch: 75.0,
        },
        CrateThreshold {
            package: "ars-dom".into(),
            min_line: 85.0,
            min_branch: 50.0,
        },
        CrateThreshold {
            package: "ars-interactions".into(),
            min_line: 98.0,
            min_branch: 90.0,
        },
        CrateThreshold {
            package: "ars-i18n".into(),
            min_line: 96.0,
            min_branch: 65.0,
        },
        CrateThreshold {
            package: "ars-leptos".into(),
            min_line: 74.0,
            min_branch: 55.0,
        },
        CrateThreshold {
            package: "ars-dioxus".into(),
            min_line: 77.0,
            min_branch: 70.0,
        },
        CrateThreshold {
            package: "ars-test-harness".into(),
            // 99% (not 100%) is the achievable line floor under
            // `cargo +nightly llvm-cov nextest --branch`. With branch
            // instrumentation enabled, the LCOV `LF`/`LH` line totals
            // diverge slightly from the source-region view shown by
            // `cargo llvm-cov report` — every annotated source line is
            // covered, but LCOV reports ~2 lines short on a ~1200-line
            // crate (≈99.83% measured). Holding the threshold at 100%
            // would fail CI even with all source lines covered.
            min_line: 99.0,
            min_branch: 0.0,
        },
        CrateThreshold {
            package: "ars-test-harness-leptos".into(),
            min_line: 60.0,
            min_branch: 0.0,
        },
        CrateThreshold {
            package: "ars-test-harness-dioxus".into(),
            min_line: 60.0,
            min_branch: 0.0,
        },
        CrateThreshold {
            package: "ars-derive".into(),
            min_line: 95.0,
            min_branch: 80.0,
        },
        CrateThreshold {
            package: "xtask".into(),
            min_line: 30.0,
            min_branch: 35.0,
        },
    ]
}

/// Errors from coverage operations.
#[derive(Debug)]
pub enum Error {
    /// IO error reading the lcov file.
    Io(io::Error),

    /// A required external tool is not available.
    MissingTool {
        /// Human-readable tool name.
        tool: String,

        /// Suggested install command or hint.
        install_hint: String,
    },

    /// A subprocess exited unsuccessfully.
    CommandFailed {
        /// Display form of the command that failed.
        command: String,

        /// Exit code, if available.
        code: Option<i32>,
    },

    /// No source files matched the requested package.
    NoSourceFiles {
        /// The package that was looked up.
        package: String,
    },

    /// The wasm coverage pipeline found no relevant compiler artifacts.
    NoArtifacts {
        /// Package name requested by the user.
        package: String,
    },

    /// The wasm coverage pipeline generated no `.profraw` files.
    NoProfiles {
        /// Directory searched for profiling outputs.
        directory: PathBuf,
    },

    /// One or more crates fell below their coverage thresholds.
    BelowThreshold {
        /// Human-readable summary including the full results table.
        summary: String,
    },
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error reading lcov file: {e}"),

            Self::MissingTool { tool, install_hint } => {
                write!(
                    f,
                    "missing required tool: {tool}\n  install: {install_hint}"
                )
            }

            Self::CommandFailed { command, code } => {
                write!(f, "command failed")?;

                if let Some(code) = code {
                    write!(f, " (exit code {code})")?;
                }

                write!(f, ": {command}")
            }

            Self::NoSourceFiles { package } => {
                write!(
                    f,
                    "no source files found for package '{package}' in lcov data \
                     (expected paths matching crates/{package}/src/ or \
                     {package}/src/)"
                )
            }

            Self::NoArtifacts { package } => {
                write!(
                    f,
                    "no wasm coverage artifacts found for package '{package}'"
                )
            }

            Self::NoProfiles { directory } => {
                write!(f, "no .profraw files generated in {}", directory.display())
            }

            Self::BelowThreshold { summary } => write!(f, "{summary}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct BranchKey {
    line: u32,
    block: u32,
    branch: u32,
}

#[derive(Debug, Default, Clone)]
struct FileCoverage {
    lines: BTreeMap<u32, bool>,
    branches: BTreeMap<BranchKey, bool>,
}

impl FileCoverage {
    fn record_line(&mut self, line: u32, hit: bool) {
        self.lines
            .entry(line)
            .and_modify(|existing| *existing |= hit)
            .or_insert(hit);
    }

    fn record_branch(&mut self, key: BranchKey, hit: bool) {
        self.branches
            .entry(key)
            .and_modify(|existing| *existing |= hit)
            .or_insert(hit);
    }
}

/// Accumulated coverage counters for a single crate.
#[derive(Debug, Default)]
struct CrateStats {
    lines_found: u64,
    lines_hit: u64,
    branches_found: u64,
    branches_hit: u64,
}

impl CrateStats {
    fn line_pct(&self) -> f64 {
        if self.lines_found == 0 {
            return 0.0;
        }

        (self.lines_hit as f64 / self.lines_found as f64) * 100.0
    }

    fn branch_pct(&self) -> f64 {
        if self.branches_found == 0 {
            return 0.0;
        }

        (self.branches_hit as f64 / self.branches_found as f64) * 100.0
    }
}

#[derive(Debug, Deserialize)]
struct CargoArtifactMessage {
    reason: String,
    target: Option<CargoTarget>,
    filenames: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
}

/// Options for generating experimental wasm coverage.
#[derive(Debug, Clone)]
pub struct WasmCoverageOptions {
    /// Crate package name (e.g. `ars-dom`).
    pub package: String,

    /// Output lcov path to write.
    pub output: PathBuf,

    /// Feature flags passed to cargo.
    pub features: Vec<String>,

    /// Whether to disable the package's default features.
    pub no_default_features: bool,

    /// Extra arguments appended after `cargo test --`.
    pub extra_test_args: Vec<String>,
}

/// Default wasm/browser coverage target used by CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WasmCoverageTarget {
    /// Crate package name (e.g. `ars-dom`).
    pub package: &'static str,

    /// Cargo features required for the browser test build.
    pub features: &'static [&'static str],

    /// Whether CI coverage must disable the crate's default features.
    pub no_default_features: bool,
}

/// Browser-test packages whose `web`/`wasm32` code paths are merged into CI coverage.
pub const fn default_wasm_coverage_targets() -> &'static [WasmCoverageTarget] {
    &[
        WasmCoverageTarget {
            package: "ars-dom",
            features: &["web"],
            no_default_features: false,
        },
        WasmCoverageTarget {
            package: "ars-i18n",
            features: &["web-intl"],
            no_default_features: true,
        },
        WasmCoverageTarget {
            package: "ars-leptos",
            features: &["csr"],
            no_default_features: false,
        },
        WasmCoverageTarget {
            package: "ars-dioxus",
            features: &["web"],
            no_default_features: false,
        },
        WasmCoverageTarget {
            package: "ars-test-harness-leptos",
            features: &[],
            no_default_features: false,
        },
        WasmCoverageTarget {
            package: "ars-test-harness-dioxus",
            features: &[],
            no_default_features: false,
        },
    ]
}

/// Parse lcov content into per-file line/branch hit maps.
fn parse_lcov_records(content: &str) -> BTreeMap<String, FileCoverage> {
    let mut files = BTreeMap::<String, FileCoverage>::new();

    let mut current_file: Option<String> = None;

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if let Some(path) = line.strip_prefix("SF:") {
            current_file = Some(path.to_owned());

            files.entry(path.to_owned()).or_default();

            continue;
        }

        if line == "end_of_record" {
            current_file = None;

            continue;
        }

        let Some(path) = current_file.as_ref() else {
            continue;
        };

        let Some(file) = files.get_mut(path) else {
            continue;
        };

        if let Some(rest) = line.strip_prefix("DA:") {
            let mut parts = rest.split(',');

            let Some(line_no) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
                continue;
            };

            let Some(hit_count) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
                continue;
            };

            file.record_line(line_no, hit_count > 0);

            continue;
        }

        if let Some(rest) = line.strip_prefix("BRDA:") {
            let mut parts = rest.split(',');

            let Some(line_no) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
                continue;
            };

            let Some(block) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
                continue;
            };

            let Some(branch) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
                continue;
            };

            let Some(taken) = parts.next() else {
                continue;
            };

            if taken == "-" {
                // Some wasm lcov exporters emit branch records without taken counts.
                // Treat those as "no branch data" instead of uncovered branches.
                continue;
            }

            let hit = taken.parse::<u64>().ok().is_some_and(|count| count > 0);

            file.record_branch(
                BranchKey {
                    line: line_no,
                    block,
                    branch,
                },
                hit,
            );
        }
    }

    files
}

fn parse_wasm_bindgen_version_from_lock(lock_content: &str) -> Result<String, Error> {
    let doc = lock_content.parse::<toml::Table>().map_err(|error| {
        Error::Io(io::Error::new(
            ErrorKind::InvalidData,
            format!("Cargo.lock: {error}"),
        ))
    })?;

    let packages = doc
        .get("package")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| {
            Error::Io(io::Error::new(
                ErrorKind::InvalidData,
                "Cargo.lock: missing [[package]] entries",
            ))
        })?;

    for package in packages {
        let Some(table) = package.as_table() else {
            continue;
        };

        let Some(name) = table.get("name").and_then(toml::Value::as_str) else {
            continue;
        };

        if name != "wasm-bindgen" {
            continue;
        }

        let Some(version) = table.get("version").and_then(toml::Value::as_str) else {
            continue;
        };

        return Ok(version.to_owned());
    }

    Err(Error::Io(io::Error::new(
        ErrorKind::InvalidData,
        "Cargo.lock: missing wasm-bindgen package entry",
    )))
}

/// Convert merged lcov records back into a tracefile.
fn write_lcov_records(records: &BTreeMap<String, FileCoverage>) -> String {
    let mut out = String::new();

    for (path, file) in records {
        writeln!(out, "SF:{path}").expect("write to String");

        let mut line_hits = 0_u64;

        for (line, hit) in &file.lines {
            let count = if *hit { 1 } else { 0 };

            line_hits += u64::from(*hit);

            writeln!(out, "DA:{line},{count}").expect("write to String");
        }

        writeln!(out, "LF:{}", file.lines.len()).expect("write to String");
        writeln!(out, "LH:{line_hits}").expect("write to String");

        if !file.branches.is_empty() {
            let mut branch_hits = 0_u64;

            for (branch, hit) in &file.branches {
                let taken = if *hit { "1" } else { "0" };

                branch_hits += u64::from(*hit);

                writeln!(
                    out,
                    "BRDA:{},{},{},{}",
                    branch.line, branch.block, branch.branch, taken
                )
                .expect("write to String");
            }

            writeln!(out, "BRF:{}", file.branches.len()).expect("write to String");
            writeln!(out, "BRH:{branch_hits}").expect("write to String");
        }

        writeln!(out, "end_of_record").expect("write to String");
    }

    out
}

fn package_source_needles(package: &str) -> [String; 2] {
    [format!("crates/{package}/src/"), format!("/{package}/src/")]
}

/// Parse lcov content and accumulate stats for a single package.
///
/// Source files are matched by path substring `crates/{package}/src/` for
/// member crates under `crates/`, or `/{package}/src/` for workspace-root
/// packages such as `xtask`.
fn parse_package_stats(lcov_content: &str, package: &str) -> CrateStats {
    let needles = package_source_needles(package);

    let records = parse_lcov_records(lcov_content);

    let mut stats = CrateStats::default();

    for (path, file) in records {
        if !needles.iter().any(|needle| path.contains(needle)) {
            continue;
        }

        stats.lines_found += file.lines.len() as u64;
        stats.lines_hit += file.lines.values().filter(|hit| **hit).count() as u64;
        stats.branches_found += file.branches.len() as u64;
        stats.branches_hit += file.branches.values().filter(|hit| **hit).count() as u64;
    }

    stats
}

/// Merge multiple lcov tracefiles by unioning line and branch hits.
///
/// If the same file/line or file/branch appears in multiple inputs, a hit in
/// any input marks it as covered in the output.
///
/// # Errors
///
/// Returns [`Error::Io`] if any input cannot be read or the output cannot be
/// written.
pub fn merge_files(files: &[PathBuf], output: &Path) -> Result<String, Error> {
    let mut merged = BTreeMap::<String, FileCoverage>::new();

    for file in files {
        let content = fs::read_to_string(file).map_err(Error::Io)?;

        for (path, incoming) in parse_lcov_records(&content) {
            let entry = merged.entry(path).or_default();

            for (line, hit) in incoming.lines {
                entry.record_line(line, hit);
            }

            for (branch, hit) in incoming.branches {
                entry.record_branch(branch, hit);
            }
        }
    }

    let rendered = write_lcov_records(&merged);

    fs::write(output, &rendered).map_err(Error::Io)?;

    Ok(format!(
        "Merged {} lcov files into {}",
        files.len(),
        output.display()
    ))
}

/// Generate experimental wasm coverage for a single package.
///
/// This follows the `wasm-bindgen-test` coverage recipe: compile with
/// instrumentation on nightly, run browser tests to emit `.profraw`, compile
/// the emitted LLVM IR back to coverage-mapped object files with a matching
/// `clang`, merge the raw profiles with `llvm-profdata`, and export lcov with
/// `llvm-cov`. A
/// wasm-capable `clang` is still required because `minicov` builds a small C
/// shim for `wasm32-unknown-unknown` during the test compile.
///
/// # Errors
///
/// Returns [`Error`] if required tools are missing, tests fail, or coverage
/// artifacts cannot be generated.
pub fn generate_wasm_lcov(options: &WasmCoverageOptions) -> Result<String, Error> {
    preflight_nightly()?;
    preflight_nightly_wasm32()?;
    preflight_wasm_runner()?;

    let tools = NightlyLlvmTools::discover()?;

    let wasm_clang = discover_wasm_clang()?;

    let chromedriver = discover_chromedriver()?;

    let work_dir = Path::new("target")
        .join("wasm-coverage")
        .join(&options.package);

    let cargo_target_dir = work_dir.join("cargo-target");

    let profraw_dir = work_dir.join("profraw");

    let objects_dir = work_dir.join("objects");

    let profdata_path = work_dir.join("coverage.profdata");

    if work_dir.exists() {
        fs::remove_dir_all(&work_dir).map_err(Error::Io)?;
    }

    fs::create_dir_all(&profraw_dir).map_err(Error::Io)?;
    fs::create_dir_all(&objects_dir).map_err(Error::Io)?;

    let profiler_runtime_shim = build_profiler_runtime_shim(&work_dir, &wasm_clang)?;

    let rustflags = coverage_rustflags(Some(&profiler_runtime_shim));

    let runner = OsString::from("wasm-bindgen-test-runner");

    let package_snake = options.package.replace('-', "_");

    let test_args = wasm_test_args(options);

    run_command(
        nightly_cargo_command(
            &rustflags,
            &cargo_target_dir,
            &profraw_dir,
            &chromedriver,
            &wasm_clang,
            &runner,
        )
        .args(&test_args),
    )?;

    let no_run_args = wasm_no_run_args(options);

    let artifact_output = capture_output(
        nightly_cargo_command(
            &rustflags,
            &cargo_target_dir,
            &profraw_dir,
            &chromedriver,
            &wasm_clang,
            &runner,
        )
        .args(&no_run_args),
    )?;

    let artifacts = parse_wasm_artifacts(&artifact_output.stdout, &package_snake)?;

    if artifacts.is_empty() {
        return Err(Error::NoArtifacts {
            package: options.package.clone(),
        });
    }

    let mut linked_wasm_paths = BTreeSet::new();

    for artifact in artifacts {
        let ir_path = artifact_to_ir_path(&artifact);

        let object_path = objects_dir.join(object_name_for_ir(&ir_path));

        run_command(
            process::Command::new(&wasm_clang)
                .arg("--target=wasm32-unknown-unknown")
                .arg(&ir_path)
                .arg("-Wno-override-module")
                .arg("-c")
                .arg("-o")
                .arg(&object_path),
        )?;

        let linked_wasm_path = objects_dir.join(linked_wasm_name_for_ir(&ir_path));

        run_command(
            process::Command::new(&tools.rust_lld)
                .arg("-flavor")
                .arg("wasm")
                .arg("--no-entry")
                .arg("--export-dynamic")
                .arg("--allow-undefined")
                .arg(&object_path)
                .arg("-o")
                .arg(&linked_wasm_path),
        )?;

        linked_wasm_paths.insert(linked_wasm_path);
    }

    let profraw_files = collect_profraw_files(&profraw_dir)?;

    if profraw_files.is_empty() {
        return Err(Error::NoProfiles {
            directory: profraw_dir,
        });
    }

    let mut profdata_command = process::Command::new(&tools.llvm_profdata);

    profdata_command.arg("merge").arg("-sparse");

    for file in &profraw_files {
        profdata_command.arg(file);
    }

    profdata_command.arg("-o").arg(&profdata_path);

    run_command(&mut profdata_command)?;

    let crate_sources = Path::new("crates").join(&options.package).join("src");

    let mut linked_wasm_paths = linked_wasm_paths.into_iter();

    let Some(primary_binary) = linked_wasm_paths.next() else {
        return Err(Error::NoArtifacts {
            package: options.package.clone(),
        });
    };

    let mut export_command = process::Command::new(&tools.llvm_cov);

    export_command
        .arg("export")
        .arg("--format=lcov")
        .arg("--instr-profile")
        .arg(&profdata_path)
        .arg(&primary_binary)
        .arg(&crate_sources);

    for binary in linked_wasm_paths {
        export_command.arg("--object").arg(binary);
    }

    let output = capture_output(&mut export_command)?;

    let parent = options
        .output
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);

    fs::create_dir_all(parent).map_err(Error::Io)?;

    fs::write(&options.output, output.stdout).map_err(Error::Io)?;

    Ok(format!(
        "Generated wasm lcov for {} at {}",
        options.package,
        options.output.display()
    ))
}

/// Check a single crate's coverage against thresholds.
///
/// Returns a human-readable summary on success. Fails with
/// [`CoverageError::BelowThreshold`] if either line or branch coverage is
/// below the minimum, or [`CoverageError::NoSourceFiles`] if no matching
/// source files were found in the lcov data.
///
/// # Errors
///
/// - [`CoverageError::Io`] if the lcov file cannot be read.
/// - [`CoverageError::NoSourceFiles`] if no source files match the package.
/// - [`CoverageError::BelowThreshold`] if coverage is below the minimum.
pub fn check(file: &Path, package: &str, min_line: f64, min_branch: f64) -> Result<String, Error> {
    let content = fs::read_to_string(file).map_err(Error::Io)?;

    check_from_content(&content, package, min_line, min_branch)
}

/// Inner implementation that operates on lcov content directly (testable
/// without file I/O).
fn check_from_content(
    content: &str,
    package: &str,
    min_line: f64,
    min_branch: f64,
) -> Result<String, Error> {
    let stats = parse_package_stats(content, package);

    if stats.lines_found == 0 {
        return Err(Error::NoSourceFiles {
            package: package.to_string(),
        });
    }

    let line_pct = stats.line_pct();

    let branch_pct = stats.branch_pct();

    let no_branch_data = stats.branches_found == 0;

    let line_ok = line_pct >= min_line;

    let branch_ok = no_branch_data || branch_pct >= min_branch;

    let mut out = String::new();

    writeln!(out, "{package}:").expect("write to String");

    writeln!(
        out,
        "  lines:    {:.1}% ({}/{}) — min {min_line:.0}% {}",
        line_pct,
        stats.lines_hit,
        stats.lines_found,
        if line_ok { "PASS" } else { "FAIL" },
    )
    .expect("write to String");

    if no_branch_data {
        writeln!(
            out,
            "  branches: no data (LLVM instrumentation did not emit branch records) — SKIP"
        )
        .expect("write to String");
    } else {
        writeln!(
            out,
            "  branches: {:.1}% ({}/{}) — min {min_branch:.0}% {}",
            branch_pct,
            stats.branches_hit,
            stats.branches_found,
            if branch_ok { "PASS" } else { "FAIL" },
        )
        .expect("write to String");
    }

    if line_ok && branch_ok {
        Ok(out)
    } else {
        Err(Error::BelowThreshold { summary: out })
    }
}

/// Check all crates against the provided thresholds in one pass.
///
/// Prints a summary table and returns it on success. Fails with
/// [`CoverageError::BelowThreshold`] if any crate is below its threshold.
///
/// # Errors
///
/// - [`CoverageError::Io`] if the lcov file cannot be read.
/// - [`CoverageError::BelowThreshold`] if any crate fails its threshold.
pub fn check_all(file: &Path, thresholds: &[CrateThreshold]) -> Result<String, Error> {
    let content = fs::read_to_string(file).map_err(Error::Io)?;

    check_all_from_content(&content, thresholds)
}

/// Inner implementation that operates on lcov content directly.
fn check_all_from_content(content: &str, thresholds: &[CrateThreshold]) -> Result<String, Error> {
    let mut out = String::new();

    let mut any_failed = false;

    let crate_width = thresholds
        .iter()
        .map(|threshold| threshold.package.len())
        .max()
        .unwrap_or(0)
        .max("Crate".len());

    writeln!(
        out,
        "{: <crate_width$} {: >8} {: >8} {: >8} {: >8}   Status",
        "Crate", "Lines", "Min", "Branch", "Min"
    )
    .expect("write to String");

    writeln!(
        out,
        "{:-<crate_width$} {:->8} {:->8} {:->8} {:->8}   {:-<6}",
        "", "", "", "", "", ""
    )
    .expect("write to String");

    for threshold in thresholds {
        let stats = parse_package_stats(content, &threshold.package);

        if stats.lines_found == 0 {
            writeln!(
                out,
                "{:<crate_width$} {:>8} {:>7}% {:>8} {:>7}%   SKIP",
                threshold.package, "—", threshold.min_line, "—", threshold.min_branch,
            )
            .expect("write to String");

            continue;
        }

        let line_pct = stats.line_pct();

        let branch_pct = stats.branch_pct();

        let no_branch_data = stats.branches_found == 0;

        let line_ok = line_pct >= threshold.min_line;

        let branch_ok = no_branch_data || branch_pct >= threshold.min_branch;

        let passed = line_ok && branch_ok;

        if !passed {
            any_failed = true;
        }

        let branch_display = if no_branch_data {
            "—".to_string()
        } else {
            format!("{branch_pct:.1}%")
        };

        let status = if passed { "PASS" } else { "FAIL" };

        writeln!(
            out,
            "{:<crate_width$} {:>7.1}% {:>7.0}% {:>8} {:>7.0}%   {}",
            threshold.package,
            line_pct,
            threshold.min_line,
            branch_display,
            threshold.min_branch,
            status,
        )
        .expect("write to String");
    }

    if any_failed {
        writeln!(out).expect("write to String");

        writeln!(
            out,
            "Coverage check FAILED — one or more crates below threshold."
        )
        .expect("write to String");

        Err(Error::BelowThreshold { summary: out })
    } else {
        writeln!(out).expect("write to String");

        writeln!(out, "All crates meet coverage thresholds.").expect("write to String");

        Ok(out)
    }
}

#[cfg(test)]
#[expect(
    clippy::items_after_test_module,
    reason = "coverage helpers are defined after tests to keep the public workflow grouped"
)]
mod tests {
    use super::*;

    const SAMPLE_LCOV: &str = "\
SF:crates/ars-core/src/lib.rs
DA:1,1
DA:2,1
DA:3,0
DA:4,1
DA:5,1
BRDA:2,0,0,1
BRDA:2,0,1,0
BRDA:3,0,0,1
BRDA:3,0,1,1
end_of_record
SF:crates/ars-core/src/connect.rs
DA:1,1
DA:2,0
BRDA:1,0,0,1
BRDA:1,0,1,0
end_of_record
SF:crates/ars-forms/src/lib.rs
DA:1,1
DA:2,1
DA:3,1
end_of_record
SF:crates/ars-other/src/lib.rs
DA:1,1
end_of_record
";

    const DUPLICATE_LCOV: &str = "\
SF:crates/ars-core/src/lib.rs
DA:1,0
DA:2,1
BRDA:2,0,0,0
BRDA:2,0,1,1
end_of_record
SF:crates/ars-core/src/lib.rs
DA:1,1
DA:2,0
BRDA:2,0,0,1
BRDA:2,0,1,0
end_of_record
";

    const ROOT_PACKAGE_LCOV: &str = "\
SF:/workspace/xtask/src/lib.rs
DA:1,1
DA:2,0
BRDA:2,0,0,1
BRDA:2,0,1,0
end_of_record
";

    const NO_BRANCH_DATA_LCOV: &str = "\
SF:crates/ars-i18n/src/plural.rs
DA:1,1
DA:2,1
BRDA:2,0,0,-
BRDA:2,0,1,-
end_of_record
";

    #[test]
    fn parse_stats_aggregates_across_files() {
        let stats = parse_package_stats(SAMPLE_LCOV, "ars-core");

        // lib.rs: 5 lines with 4 hits + connect.rs: 2 lines with 1 hit
        assert_eq!(stats.lines_found, 7);
        assert_eq!(stats.lines_hit, 5);

        // lib.rs: 4 branches with 3 hits + connect.rs: 2 branches with 1 hit
        assert_eq!(stats.branches_found, 6);
        assert_eq!(stats.branches_hit, 4);
    }

    #[test]
    fn parse_stats_filters_by_package() {
        let stats = parse_package_stats(SAMPLE_LCOV, "ars-forms");

        assert_eq!(stats.lines_found, 3);
        assert_eq!(stats.lines_hit, 3);
        assert_eq!(stats.branches_found, 0);
        assert_eq!(stats.branches_hit, 0);
    }

    #[test]
    fn parse_stats_nonexistent_package() {
        let stats = parse_package_stats(SAMPLE_LCOV, "ars-nonexistent");

        assert_eq!(stats.lines_found, 0);
    }

    #[test]
    fn parse_stats_unions_duplicate_file_records() {
        let stats = parse_package_stats(DUPLICATE_LCOV, "ars-core");

        assert_eq!(stats.lines_found, 2);
        assert_eq!(stats.lines_hit, 2);
        assert_eq!(stats.branches_found, 2);
        assert_eq!(stats.branches_hit, 2);
    }

    #[test]
    fn parse_stats_matches_workspace_root_package() {
        let stats = parse_package_stats(ROOT_PACKAGE_LCOV, "xtask");

        assert_eq!(stats.lines_found, 2);
        assert_eq!(stats.lines_hit, 1);
        assert_eq!(stats.branches_found, 2);
        assert_eq!(stats.branches_hit, 1);
    }

    #[test]
    fn parse_stats_ignores_branch_records_without_taken_counts() {
        let stats = parse_package_stats(NO_BRANCH_DATA_LCOV, "ars-i18n");

        assert_eq!(stats.lines_found, 2);
        assert_eq!(stats.lines_hit, 2);
        assert_eq!(stats.branches_found, 0);
        assert_eq!(stats.branches_hit, 0);
    }

    #[test]
    fn check_passes_when_above_threshold() {
        // ars-core: 5/7 = 71.4% lines, 4/6 = 66.7% branches
        let result = check_from_content(SAMPLE_LCOV, "ars-core", 70.0, 60.0);

        assert!(result.is_ok());

        let output = result.expect("should pass");

        assert!(output.contains("PASS"));
    }

    #[test]
    fn check_fails_when_below_line_threshold() {
        let result = check_from_content(SAMPLE_LCOV, "ars-core", 90.0, 60.0);

        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(matches!(err, Error::BelowThreshold { .. }));
        assert!(err.to_string().contains("FAIL"));
    }

    #[test]
    fn check_fails_when_below_branch_threshold() {
        let result = check_from_content(SAMPLE_LCOV, "ars-core", 70.0, 90.0);

        assert!(result.is_err());
    }

    #[test]
    fn check_skips_branch_when_no_data() {
        // ars-forms has BRF=0, BRH=0 — branch check should pass regardless
        let result = check_from_content(SAMPLE_LCOV, "ars-forms", 90.0, 90.0);

        assert!(result.is_ok());

        let output = result.expect("should pass with no branch data");

        assert!(output.contains("SKIP"));
    }

    #[test]
    fn check_errors_on_missing_package() {
        let result = check_from_content(SAMPLE_LCOV, "ars-nonexistent", 50.0, 50.0);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoSourceFiles { .. }));
    }

    #[test]
    fn check_all_reports_table() {
        let thresholds = vec![
            CrateThreshold {
                package: "ars-core".into(),
                min_line: 70.0,
                min_branch: 60.0,
            },
            CrateThreshold {
                package: "ars-forms".into(),
                min_line: 90.0,
                min_branch: 90.0,
            },
        ];

        let result = check_all_from_content(SAMPLE_LCOV, &thresholds);

        assert!(result.is_ok());

        let output = result.expect("all should pass");

        assert!(output.contains("ars-core"));
        assert!(output.contains("ars-forms"));
        assert!(output.contains("All crates meet coverage thresholds."));
    }

    #[test]
    fn check_all_expands_crate_column_for_long_names() {
        let thresholds = vec![CrateThreshold {
            package: "ars-test-harness-leptos".into(),
            min_line: 50.0,
            min_branch: 50.0,
        }];

        let result = check_all_from_content(SAMPLE_LCOV, &thresholds);

        assert!(result.is_ok());

        let output = result.expect("long-name table should render");

        assert!(output.contains("ars-test-harness-leptos"));
        assert!(!output.contains("ars-test-harness-leptos—"));
    }

    #[test]
    fn check_all_fails_on_any_below() {
        let thresholds = vec![
            CrateThreshold {
                package: "ars-core".into(),
                min_line: 99.0,
                min_branch: 99.0,
            },
            CrateThreshold {
                package: "ars-forms".into(),
                min_line: 90.0,
                min_branch: 90.0,
            },
        ];

        let result = check_all_from_content(SAMPLE_LCOV, &thresholds);

        assert!(result.is_err());

        let err = result.unwrap_err();

        assert!(err.to_string().contains("FAIL"));
        assert!(err.to_string().contains("Coverage check FAILED"));
    }

    #[test]
    fn check_all_skips_missing_packages() {
        let thresholds = vec![CrateThreshold {
            package: "ars-nonexistent".into(),
            min_line: 50.0,
            min_branch: 50.0,
        }];

        let result = check_all_from_content(SAMPLE_LCOV, &thresholds);

        assert!(result.is_ok());

        let output = result.expect("should skip missing");

        assert!(output.contains("SKIP"));
    }

    #[test]
    fn line_pct_zero_found() {
        let stats = CrateStats::default();

        assert_eq!(stats.line_pct(), 0.0);
        assert_eq!(stats.branch_pct(), 0.0);
    }

    #[test]
    fn merge_files_unions_hits_without_double_counting() {
        let tempdir = std::env::temp_dir().join(format!("ars-ui-coverage-merge-{}", process::id()));

        drop(fs::remove_dir_all(&tempdir));

        fs::create_dir_all(&tempdir).expect("temp dir");

        let first = tempdir.join("first.info");
        let second = tempdir.join("second.info");

        let output = tempdir.join("merged.info");

        fs::write(&first, DUPLICATE_LCOV).expect("write first");
        fs::write(&second, SAMPLE_LCOV).expect("write second");

        merge_files(&[first, second], &output).expect("merge succeeds");

        let merged = fs::read_to_string(output).expect("read merged");

        let stats = parse_package_stats(&merged, "ars-core");

        assert_eq!(stats.lines_found, 7);
        assert_eq!(stats.lines_hit, 5);

        drop(fs::remove_dir_all(&tempdir));
    }

    #[test]
    fn artifact_to_ir_path_translates_rlib_and_wasm_artifacts() {
        let rlib = Path::new("/tmp/deps/libars_dom-1234.rlib");
        let wasm = Path::new("/tmp/deps/ars_dom-1234.wasm");

        assert_eq!(
            artifact_to_ir_path(rlib),
            PathBuf::from("/tmp/deps/ars_dom-1234.ll")
        );
        assert_eq!(
            artifact_to_ir_path(wasm),
            PathBuf::from("/tmp/deps/ars_dom-1234.ll")
        );
    }

    #[test]
    fn parses_wasm_bindgen_version_from_lock_content() {
        let lock = r#"
version = 4

[[package]]
name = "other"
version = "1.0.0"

[[package]]
name = "wasm-bindgen"
version = "0.2.118"
"#;

        let version =
            parse_wasm_bindgen_version_from_lock(lock).expect("should parse wasm-bindgen version");

        assert_eq!(version, "0.2.118");
    }

    #[test]
    fn llvm_profile_file_uses_runner_placeholders_in_profraw_dir() {
        let profraw_dir = std::env::temp_dir().join("ars-ui-profraw");

        let profile = llvm_profile_file(profraw_dir);

        assert_eq!(
            profile,
            std::env::temp_dir().join("ars-ui-profraw/wasm-coverage-%m-%p.profraw")
        );
    }

    #[test]
    fn coverage_rustflags_enables_branch_instrumentation() {
        let flags = coverage_rustflags(None);

        assert!(flags.contains("-Cinstrument-coverage"));
        assert!(flags.contains("-Zcoverage-options=branch"));
    }

    #[test]
    fn default_wasm_coverage_targets_include_web_intl_ars_i18n() {
        let targets = default_wasm_coverage_targets();

        assert!(targets.iter().any(|target| {
            target.package == "ars-dom" && target.features == ["web"] && !target.no_default_features
        }));
        assert!(targets.iter().any(|target| {
            target.package == "ars-i18n"
                && target.features == ["web-intl"]
                && target.no_default_features
        }));
        assert!(targets.iter().any(|target| {
            target.package == "ars-leptos"
                && target.features == ["csr"]
                && !target.no_default_features
        }));
        assert!(targets.iter().any(|target| {
            target.package == "ars-dioxus"
                && target.features == ["web"]
                && !target.no_default_features
        }));
        assert!(targets.iter().any(|target| {
            target.package == "ars-test-harness-leptos"
                && target.features.is_empty()
                && !target.no_default_features
        }));
        assert!(targets.iter().any(|target| {
            target.package == "ars-test-harness-dioxus"
                && target.features.is_empty()
                && !target.no_default_features
        }));
    }

    #[test]
    fn coverage_rustflags_can_link_profiler_runtime_shim() {
        let flags = coverage_rustflags(Some(Path::new("/tmp/llvm_profile_runtime.o")));

        assert!(flags.contains("-Clink-arg=/tmp/llvm_profile_runtime.o"));
    }

    #[test]
    fn base_wasm_cargo_args_respects_no_default_features_and_features() {
        let args = base_wasm_cargo_args(&WasmCoverageOptions {
            package: "ars-i18n".into(),
            output: PathBuf::from("/tmp/ars-i18n-web-intl.lcov"),
            features: vec!["web-intl".into()],
            no_default_features: true,
            extra_test_args: Vec::new(),
        });

        let args = args
            .into_iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(args.windows(2).any(|pair| pair == ["-p", "ars-i18n"]));
        assert!(args.contains(&"--no-default-features".to_owned()));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--features", "web-intl"])
        );
    }
}

fn coverage_rustflags(profiler_runtime_shim: Option<&Path>) -> String {
    let mut flags = std::env::var("RUSTFLAGS").unwrap_or_default();

    if !flags.is_empty() {
        flags.push(' ');
    }

    flags.push_str(
        "-Cinstrument-coverage -Zcoverage-options=branch -Zno-profiler-runtime --emit=llvm-ir \
         --cfg=wasm_bindgen_unstable_test_coverage",
    );

    if let Some(shim) = profiler_runtime_shim {
        flags.push(' ');
        flags.push_str("-Clink-arg=");
        flags.push_str(&shim.display().to_string());
    }

    flags
}

fn build_profiler_runtime_shim(work_dir: &Path, wasm_clang: &Path) -> Result<PathBuf, Error> {
    let shim_dir = work_dir.join("profiler-runtime-shim");

    fs::create_dir_all(&shim_dir).map_err(Error::Io)?;

    let source = shim_dir.join("llvm_profile_runtime.c");
    let object = shim_dir.join("llvm_profile_runtime.o");

    fs::write(
        &source,
        "__attribute__((weak)) unsigned char __llvm_profile_runtime = 0;\n",
    )
    .map_err(Error::Io)?;

    run_command(
        process::Command::new(wasm_clang)
            .arg("--target=wasm32-unknown-unknown")
            .arg("-fno-profile-instr-generate")
            .arg("-fno-coverage-mapping")
            .arg("-c")
            .arg(&source)
            .arg("-o")
            .arg(&object),
    )?;

    fs::canonicalize(&object).map_err(Error::Io)
}

fn base_wasm_cargo_args(options: &WasmCoverageOptions) -> Vec<OsString> {
    let mut args = vec![
        OsString::from("+nightly"),
        OsString::from("test"),
        OsString::from("--tests"),
        OsString::from("--target"),
        OsString::from("wasm32-unknown-unknown"),
        OsString::from("-p"),
        OsString::from(&options.package),
    ];

    if options.no_default_features {
        args.push(OsString::from("--no-default-features"));
    }

    if !options.features.is_empty() {
        args.push(OsString::from("--features"));
        args.push(OsString::from(options.features.join(",")));
    }

    args
}

fn wasm_test_args(options: &WasmCoverageOptions) -> Vec<OsString> {
    let mut args = base_wasm_cargo_args(options);

    if !options.extra_test_args.is_empty() {
        args.push(OsString::from("--"));
        args.extend(options.extra_test_args.iter().map(OsString::from));
    }

    args
}

fn wasm_no_run_args(options: &WasmCoverageOptions) -> Vec<OsString> {
    let mut args = base_wasm_cargo_args(options);

    args.push(OsString::from("--no-run"));
    args.push(OsString::from("--message-format=json"));

    args
}

fn llvm_profile_file(profraw_dir: impl AsRef<Path>) -> PathBuf {
    let profraw_dir = profraw_dir.as_ref();

    let absolute = if profraw_dir.is_absolute() {
        profraw_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .expect("current working directory")
            .join(profraw_dir)
    };

    absolute.join("wasm-coverage-%m-%p.profraw")
}

fn nightly_cargo_command(
    rustflags: &str,
    cargo_target_dir: &Path,
    profraw_dir: &Path,
    chromedriver: &Path,
    wasm_clang: &Path,
    runner: &OsString,
) -> process::Command {
    let mut command = process::Command::new("cargo");

    let path = std::env::var_os("PATH").unwrap_or_default();

    let path_dir = wasm_clang.parent().unwrap_or_else(|| Path::new("."));

    let joined_path = std::env::join_paths(
        std::iter::once(path_dir.to_path_buf()).chain(std::env::split_paths(&path)),
    )
    .expect("valid PATH");

    command
        .env("RUSTFLAGS", rustflags)
        .env("CARGO_TARGET_DIR", cargo_target_dir)
        .env("LLVM_PROFILE_FILE", llvm_profile_file(profraw_dir))
        .env("WASM_BINDGEN_TEST_ONLY_WEB", "1")
        .env("CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER", runner)
        .env("CC_wasm32-unknown-unknown", wasm_clang)
        .env("CC_wasm32_unknown_unknown", wasm_clang)
        .env("TARGET_CC", wasm_clang)
        .env("PATH", &joined_path)
        .env("CHROMEDRIVER", chromedriver);

    command
}

fn run_command(command: &mut process::Command) -> Result<(), Error> {
    let display = format!("{command:?}");

    let status = command.status().map_err(Error::Io)?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::CommandFailed {
            command: display,
            code: status.code(),
        })
    }
}

fn capture_output(command: &mut process::Command) -> Result<Output, Error> {
    let display = format!("{command:?}");

    let output = command.output().map_err(Error::Io)?;

    if output.status.success() {
        Ok(output)
    } else {
        Err(Error::CommandFailed {
            command: display,
            code: output.status.code(),
        })
    }
}

pub(crate) fn preflight_nightly() -> Result<(), Error> {
    let status = process::Command::new("rustup")
        .args(["run", "nightly", "rustc", "--version"])
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .status()
        .map_err(Error::Io)?;

    if status.success() {
        Ok(())
    } else {
        Err(Error::MissingTool {
            tool: "nightly toolchain".into(),
            install_hint: "rustup toolchain install nightly".into(),
        })
    }
}

fn preflight_nightly_wasm32() -> Result<(), Error> {
    let output = process::Command::new("rustup")
        .args(["target", "list", "--toolchain", "nightly", "--installed"])
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::CommandFailed {
            command: "rustup target list --toolchain nightly --installed".into(),
            code: output.status.code(),
        });
    }

    let installed = String::from_utf8_lossy(&output.stdout);

    if installed
        .lines()
        .any(|line| line.trim() == "wasm32-unknown-unknown")
    {
        Ok(())
    } else {
        Err(Error::MissingTool {
            tool: "nightly wasm32-unknown-unknown target".into(),
            install_hint: "rustup target add wasm32-unknown-unknown --toolchain nightly".into(),
        })
    }
}

fn preflight_wasm_runner() -> Result<(), Error> {
    let expected_version = parse_wasm_bindgen_version_from_lock(
        &fs::read_to_string("Cargo.lock").map_err(Error::Io)?,
    )?;

    let output = process::Command::new("wasm-bindgen-test-runner")
        .arg("--version")
        .output()
        .map_err(Error::Io)?;

    if !output.status.success() {
        return Err(Error::MissingTool {
            tool: "wasm-bindgen-test-runner".into(),
            install_hint: format!(
                "cargo install wasm-bindgen-cli --locked --version {expected_version}"
            ),
        });
    }

    let actual_version = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .last()
        .map(str::to_owned)
        .unwrap_or_default();

    if actual_version == expected_version {
        Ok(())
    } else {
        Err(Error::MissingTool {
            tool: format!("matching wasm-bindgen-test-runner ({expected_version})"),
            install_hint: format!(
                "cargo install wasm-bindgen-cli --locked --version {expected_version}"
            ),
        })
    }
}

fn discover_chromedriver() -> Result<PathBuf, Error> {
    if let Ok(path) = std::env::var("CHROMEDRIVER") {
        let path = PathBuf::from(path);

        if path.is_file() {
            return Ok(path);
        }
    }

    if let Ok(path) = std::env::var("CHROMEWEBDRIVER") {
        let path = PathBuf::from(path);

        if path.is_file() {
            return Ok(path);
        }

        let nested = path.join("chromedriver");

        if nested.is_file() {
            return Ok(nested);
        }
    }

    let candidates = [
        PathBuf::from("/opt/homebrew/bin/chromedriver"),
        PathBuf::from("/usr/local/share/chromedriver-linux64/chromedriver"),
        PathBuf::from("/usr/local/bin/chromedriver"),
    ];

    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    let status = process::Command::new("chromedriver")
        .arg("--version")
        .stdout(process::Stdio::null())
        .stderr(process::Stdio::null())
        .status();

    match status {
        Ok(status) if status.success() => Ok(PathBuf::from("chromedriver")),

        Ok(status) => Err(Error::CommandFailed {
            command: "chromedriver --version".into(),
            code: status.code(),
        }),

        Err(_) => Err(Error::MissingTool {
            tool: "chromedriver".into(),
            install_hint:
                "install chromedriver and/or set CHROMEDRIVER=/absolute/path/to/chromedriver".into(),
        }),
    }
}

fn discover_wasm_clang() -> Result<PathBuf, Error> {
    let mut candidates = Vec::<PathBuf>::new();

    for env_name in ["WASM_COVERAGE_CLANG", "CC_wasm32_unknown_unknown", "CC"] {
        if let Ok(path) = std::env::var(env_name) {
            let trimmed = path.trim();

            if !trimmed.is_empty() {
                candidates.push(PathBuf::from(trimmed));
            }
        }
    }

    let nightly_llvm_major = NightlyLlvmTools::discover()?.llvm_major;

    candidates.push(PathBuf::from(format!("clang-{nightly_llvm_major}")));
    candidates.push(PathBuf::from("/opt/homebrew/opt/llvm/bin/clang"));
    candidates.push(PathBuf::from("/usr/local/opt/llvm/bin/clang"));
    candidates.push(PathBuf::from("clang"));

    let mut saw_candidate = false;

    let probe_dir = Path::new("target")
        .join("wasm-coverage")
        .join("clang-probe");

    fs::create_dir_all(&probe_dir).map_err(Error::Io)?;

    let source = probe_dir.join("probe.c");
    let object = probe_dir.join("probe.o");

    fs::write(
        &source,
        "int ars_ui_wasm_coverage_probe(void) { return 0; }\n",
    )
    .map_err(Error::Io)?;

    for candidate in candidates {
        let version = process::Command::new(&candidate)
            .arg("--version")
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .status();

        match version {
            Ok(status) if status.success() => saw_candidate = true,

            Ok(_) => continue,

            Err(err) if err.kind() == ErrorKind::NotFound => continue,

            Err(err) => return Err(Error::Io(err)),
        }

        let compile = process::Command::new(&candidate)
            .arg("--target=wasm32-unknown-unknown")
            .arg("-c")
            .arg(&source)
            .arg("-o")
            .arg(&object)
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .status();

        match compile {
            Ok(status) if status.success() => return Ok(candidate),

            Ok(_) => {}

            Err(err) if err.kind() == ErrorKind::NotFound => {}

            Err(err) => return Err(Error::Io(err)),
        }
    }

    Err(Error::MissingTool {
        tool: "wasm-capable clang".into(),
        install_hint: if saw_candidate {
            "install LLVM clang with WebAssembly target support (the current clang cannot target wasm32-unknown-unknown) and/or set WASM_COVERAGE_CLANG=/absolute/path/to/clang".into()
        } else {
            "install LLVM clang with WebAssembly target support and/or set WASM_COVERAGE_CLANG=/absolute/path/to/clang".into()
        },
    })
}

struct NightlyLlvmTools {
    rust_lld: PathBuf,
    llvm_profdata: PathBuf,
    llvm_cov: PathBuf,
    llvm_major: u32,
}

impl NightlyLlvmTools {
    fn discover() -> Result<Self, Error> {
        let sysroot = capture_output(
            process::Command::new("rustup").args(["run", "nightly", "rustc", "--print", "sysroot"]),
        )?;

        let host_output = capture_output(
            process::Command::new("rustup").args(["run", "nightly", "rustc", "-vV"]),
        )?;

        let sysroot = PathBuf::from(String::from_utf8_lossy(&sysroot.stdout).trim().to_owned());

        let host_text = String::from_utf8_lossy(&host_output.stdout);

        let host = host_text
            .lines()
            .find_map(|line| line.strip_prefix("host: "))
            .map(str::to_owned)
            .ok_or_else(|| Error::CommandFailed {
                command: "rustup run nightly rustc -vV".into(),
                code: None,
            })?;

        let llvm_major = host_text
            .lines()
            .find_map(|line| line.strip_prefix("LLVM version: "))
            .and_then(|version| version.split('.').next())
            .and_then(|major| major.parse::<u32>().ok())
            .ok_or_else(|| Error::CommandFailed {
                command: "rustup run nightly rustc -vV".into(),
                code: None,
            })?;

        let bin_dir = sysroot.join("lib").join("rustlib").join(host).join("bin");

        let rust_lld = bin_dir.join("rust-lld");

        let llvm_profdata = bin_dir.join("llvm-profdata");

        let llvm_cov = bin_dir.join("llvm-cov");

        if !rust_lld.is_file() || !llvm_profdata.is_file() || !llvm_cov.is_file() {
            return Err(Error::MissingTool {
                tool: "nightly llvm-tools-preview".into(),
                install_hint: "rustup component add llvm-tools-preview --toolchain nightly".into(),
            });
        }

        Ok(Self {
            rust_lld,
            llvm_profdata,
            llvm_cov,
            llvm_major,
        })
    }
}

fn parse_wasm_artifacts(stdout: &[u8], package_snake: &str) -> Result<Vec<PathBuf>, Error> {
    let stdout = String::from_utf8_lossy(stdout);

    let mut artifacts = BTreeSet::new();

    for line in stdout.lines() {
        let Ok(message) = serde_json::from_str::<CargoArtifactMessage>(line) else {
            continue;
        };

        if message.reason != "compiler-artifact" {
            continue;
        }

        let Some(target) = message.target else {
            continue;
        };

        let is_test = target.kind == ["test"];

        let is_crate_artifact = target.name == package_snake;

        if !is_test && !is_crate_artifact {
            continue;
        }

        for filename in message.filenames.unwrap_or_default() {
            let path = PathBuf::from(filename);

            let extension = path.extension().and_then(|value| value.to_str());

            if matches!(extension, Some("rlib" | "wasm")) {
                artifacts.insert(path);
            }
        }
    }

    Ok(artifacts.into_iter().collect())
}

fn artifact_to_ir_path(artifact: &Path) -> PathBuf {
    let parent = artifact.parent().unwrap_or_else(|| Path::new("."));

    let stem = artifact
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();

    if artifact.extension().and_then(|value| value.to_str()) == Some("rlib") {
        let stem = stem.strip_prefix("lib").unwrap_or(stem);

        parent.join(format!("{stem}.ll"))
    } else {
        parent.join(format!("{stem}.ll"))
    }
}

fn object_name_for_ir(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("coverage");

    format!("{stem}.o")
}

fn linked_wasm_name_for_ir(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("coverage");

    format!("{stem}.wasm")
}

fn collect_profraw_files(directory: &Path) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::new();

    for entry in fs::read_dir(directory).map_err(Error::Io)? {
        let path = entry.map_err(Error::Io)?.path();

        if path.extension().and_then(|value| value.to_str()) == Some("profraw") {
            files.push(path);
        }
    }

    files.sort();

    Ok(files)
}
