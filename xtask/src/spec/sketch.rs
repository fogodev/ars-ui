//! Implementation sketch validation.

use std::{fmt::Write as _, fs, path::Path};

use crate::manifest::Error;

const REQUIRED_SECTIONS: &[&str] = &[
    "## Reference Sources",
    "## Reference Evidence",
    "## Observed Reference Outcomes",
    "## I18n Mapping",
    "## Accessibility Mapping",
    "## Ars Contract Mapping",
    "## Final Outcome Matrix",
];

const REQUIRED_MATRIX_HEADERS: &[&str] = &[
    "Reference outcome",
    "Final status",
    "API/contract stance",
    "Reference proof",
    "Local proof",
    "Adapter tests",
    "E2E/browser proof",
    "I18n proof",
    "A11y proof",
];

const FORBIDDEN_FINAL_STATUSES: &[&str] = &[
    "Unknown",
    "Unverified",
    "ContractGap",
    "AdapterApiGap",
    "WidgetOnlyWorkaround",
];

/// Validate a component adapter implementation sketch.
///
/// # Errors
///
/// Returns an error when the sketch cannot be read.
pub fn execute(path: &Path) -> Result<String, Error> {
    let content = fs::read_to_string(path).map_err(Error::Io)?;

    let mut failures = Vec::new();

    for section in REQUIRED_SECTIONS {
        if !content.contains(section) {
            failures.push(format!("missing required section `{section}`"));
        }
    }

    let matrix = final_matrix(&content).unwrap_or_default();

    if matrix.is_empty() {
        failures.push("missing final outcome matrix rows".to_string());
    } else {
        let header = matrix
            .lines()
            .find(|line| line.trim_start().starts_with('|'))
            .unwrap_or_default();

        for required in REQUIRED_MATRIX_HEADERS {
            if !header.contains(required) {
                failures.push(format!("final outcome matrix missing `{required}` column"));
            }
        }

        for forbidden in FORBIDDEN_FINAL_STATUSES {
            if matrix.contains(forbidden) {
                failures.push(format!(
                    "final outcome matrix contains unresolved `{forbidden}` row"
                ));
            }
        }

        for (idx, row) in matrix
            .lines()
            .filter(|line| line.trim_start().starts_with('|'))
            .skip(2)
            .enumerate()
        {
            let cells = row
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect::<Vec<_>>();

            if cells.len() < REQUIRED_MATRIX_HEADERS.len() {
                failures.push(format!(
                    "final outcome matrix row {} has too few columns",
                    idx + 1
                ));

                continue;
            }

            for (column, value) in REQUIRED_MATRIX_HEADERS.iter().zip(cells.iter()) {
                if value.is_empty() || matches!(*value, "TBD" | "TODO" | "Unknown" | "Unverified") {
                    failures.push(format!(
                        "final outcome matrix row {} has incomplete `{column}` value",
                        idx + 1
                    ));
                }
            }
        }
    }

    let mut output = format!("Validated sketch {}.\n", path.display());

    if failures.is_empty() {
        output.push_str("All sketch checks passed.\n");
    } else {
        writeln!(output, "{} sketch error(s) found:", failures.len()).expect("write to string");

        for failure in failures {
            writeln!(output, "- {failure}").expect("write to string");
        }
    }

    Ok(output)
}

fn final_matrix(content: &str) -> Option<&str> {
    let start = content.find("## Final Outcome Matrix")?;
    let after = &content[start..];
    let next = after
        .find("\n## ")
        .filter(|idx| *idx > 0)
        .unwrap_or(after.len());

    Some(&after[..next])
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::execute;

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();

        let path = std::env::temp_dir().join(format!("ars-ui-sketch-{name}-{nanos}"));

        fs::create_dir_all(&path).expect("create temp dir");

        path
    }

    fn write(path: &Path, content: &str) {
        fs::write(path, content).expect("write sketch");
    }

    #[test]
    fn sketch_validation_rejects_unresolved_final_statuses() {
        let root = temp_dir("unresolved");
        let path = root.join("sketch.md");

        write(
            &path,
            r#"
## Reference Sources
## Reference Evidence
## Observed Reference Outcomes
## I18n Mapping
## Accessibility Mapping
## Ars Contract Mapping
## Final Outcome Matrix
| Reference outcome | Final status | API/contract stance | Reference proof | Local proof | Adapter tests | E2E/browser proof | I18n proof | A11y proof |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Basic | Unknown | IdiomaticEquivalent | ref | local | tests | e2e | i18n | a11y |
"#,
        );

        let output = execute(&path).expect("validation should run");

        assert!(output.contains("unresolved `Unknown`"));

        drop(fs::remove_dir_all(root));
    }

    #[test]
    fn sketch_validation_accepts_complete_matrix() {
        let root = temp_dir("complete");
        let path = root.join("sketch.md");

        write(
            &path,
            r#"
## Reference Sources
## Reference Evidence
## Observed Reference Outcomes
## I18n Mapping
## Accessibility Mapping
## Ars Contract Mapping
## Final Outcome Matrix
| Reference outcome | Final status | API/contract stance | Reference proof | Local proof | Adapter tests | E2E/browser proof | I18n proof | A11y proof |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Basic | ReferenceOutcomeMatched | IdiomaticEquivalent | ref.yml | local.yml | test_name | e2e_name | translate | axe |
"#,
        );

        let output = execute(&path).expect("validation should run");

        assert!(output.contains("All sketch checks passed."));

        drop(fs::remove_dir_all(root));
    }
}
