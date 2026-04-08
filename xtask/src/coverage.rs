//! Code coverage threshold enforcement.
//!
//! Parses lcov output from `cargo-llvm-cov` and verifies per-crate line and
//! branch coverage against configurable thresholds. Used by CI to gate merges
//! and by developers for local coverage checks.

use std::{
    fmt::{self, Write},
    fs, io,
    path::Path,
};

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
/// | Crate            | Current | Spec Target | Enforced |
/// |------------------|---------|-------------|----------|
/// | ars-core         | 72.9%   | 90%         | 70%      |
/// | ars-a11y         | 81.3%   | 85%         | 78%      |
/// | ars-collections  | 0.0%    | 85%         | 0%       |
/// | ars-forms        | 95.0%   | 85%         | 90%      |
/// | ars-dom          | 93.7%   | 75%         | 90%      |
/// | ars-interactions | 100.0%  | 80%         | 95%      |
/// | ars-i18n         | 40.0%   | 80%         | 35%      |
pub fn default_thresholds() -> Vec<CrateThreshold> {
    vec![
        CrateThreshold {
            package: "ars-core".into(),
            min_line: 70.0,
            min_branch: 60.0,
        },
        CrateThreshold {
            package: "ars-a11y".into(),
            min_line: 78.0,
            min_branch: 68.0,
        },
        CrateThreshold {
            package: "ars-collections".into(),
            min_line: 0.0,
            min_branch: 0.0,
        },
        CrateThreshold {
            package: "ars-forms".into(),
            min_line: 90.0,
            min_branch: 80.0,
        },
        CrateThreshold {
            package: "ars-dom".into(),
            min_line: 90.0,
            min_branch: 80.0,
        },
        CrateThreshold {
            package: "ars-interactions".into(),
            min_line: 95.0,
            min_branch: 85.0,
        },
        CrateThreshold {
            package: "ars-i18n".into(),
            min_line: 35.0,
            min_branch: 30.0,
        },
    ]
}

/// Errors from coverage operations.
#[derive(Debug)]
pub enum Error {
    /// IO error reading the lcov file.
    Io(io::Error),
    /// No source files matched the requested package.
    NoSourceFiles {
        /// The package that was looked up.
        package: String,
    },
    /// One or more crates fell below their coverage thresholds.
    BelowThreshold {
        /// Human-readable summary including the full results table.
        summary: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error reading lcov file: {e}"),
            Self::NoSourceFiles { package } => {
                write!(
                    f,
                    "no source files found for package '{package}' in lcov data \
                     (expected paths matching crates/{package}/src/)"
                )
            }
            Self::BelowThreshold { summary } => write!(f, "{summary}"),
        }
    }
}

impl std::error::Error for Error {}

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

/// Parse lcov content and accumulate stats for a single package.
///
/// Source files are matched by path substring `crates/{package}/src/`.
fn parse_package_stats(lcov_content: &str, package: &str) -> CrateStats {
    let needle = format!("crates/{package}/src/");
    let mut stats = CrateStats::default();
    let mut in_matching_file = false;

    for line in lcov_content.lines() {
        let line = line.trim();

        if let Some(path) = line.strip_prefix("SF:") {
            in_matching_file = path.contains(&needle);
            continue;
        }

        if line == "end_of_record" {
            in_matching_file = false;
            continue;
        }

        if !in_matching_file {
            continue;
        }

        if let Some(val) = line.strip_prefix("LF:") {
            if let Ok(n) = val.parse::<u64>() {
                stats.lines_found += n;
            }
        } else if let Some(val) = line.strip_prefix("LH:") {
            if let Ok(n) = val.parse::<u64>() {
                stats.lines_hit += n;
            }
        } else if let Some(val) = line.strip_prefix("BRF:") {
            if let Ok(n) = val.parse::<u64>() {
                stats.branches_found += n;
            }
        } else if let Some(val) = line.strip_prefix("BRH:") {
            if let Ok(n) = val.parse::<u64>() {
                stats.branches_hit += n;
            }
        }
    }

    stats
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

    writeln!(
        out,
        "{:<20} {:>8} {:>8} {:>8} {:>8}   Status",
        "Crate", "Lines", "Min", "Branch", "Min"
    )
    .expect("write to String");
    writeln!(
        out,
        "{:-<20} {:->8} {:->8} {:->8} {:->8}   {:-<6}",
        "", "", "", "", "", ""
    )
    .expect("write to String");

    for threshold in thresholds {
        let stats = parse_package_stats(content, &threshold.package);

        if stats.lines_found == 0 {
            writeln!(
                out,
                "{:<20} {:>8} {:>7}% {:>8} {:>7}%   SKIP",
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
            "{:<20} {:>7.1}% {:>7.0}% {:>8} {:>7.0}%   {}",
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
mod tests {
    use super::*;

    const SAMPLE_LCOV: &str = "\
SF:crates/ars-core/src/lib.rs
FN:10,some_function
FNDA:1,some_function
FNF:1
FNH:1
DA:1,1
DA:2,1
DA:3,0
DA:4,1
DA:5,1
LF:5
LH:4
BRF:4
BRH:3
end_of_record
SF:crates/ars-core/src/connect.rs
DA:1,1
DA:2,0
LF:2
LH:1
BRF:2
BRH:1
end_of_record
SF:crates/ars-forms/src/lib.rs
DA:1,1
DA:2,1
DA:3,1
LF:3
LH:3
BRF:0
BRH:0
end_of_record
SF:crates/ars-other/src/lib.rs
DA:1,1
LF:1
LH:1
end_of_record
";

    #[test]
    fn parse_stats_aggregates_across_files() {
        let stats = parse_package_stats(SAMPLE_LCOV, "ars-core");
        // lib.rs: LF=5, LH=4 + connect.rs: LF=2, LH=1
        assert_eq!(stats.lines_found, 7);
        assert_eq!(stats.lines_hit, 5);
        // lib.rs: BRF=4, BRH=3 + connect.rs: BRF=2, BRH=1
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
}
