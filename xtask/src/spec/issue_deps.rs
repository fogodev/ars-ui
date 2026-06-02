//! `spec issue-deps` — report or synchronize adapter issue dependencies.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    process::Command,
};

use crate::{
    manifest::{self, ComponentDependencyKind, Error, SpecRoot},
    spec::component_deps::{self, AdapterFilter},
};

/// Execute the adapter issue dependency report.
///
/// # Errors
///
/// Returns an error when the adapter filter is invalid or GitHub issue data
/// cannot be queried.
pub fn execute(
    root: &SpecRoot,
    adapter: &str,
    component: Option<&str>,
    dry_run: bool,
) -> Result<String, Error> {
    let adapter_filter = AdapterFilter::parse(Some(adapter))?;
    let component = component.unwrap_or("all");
    let issue_index = IssueIndex::load(adapter)?;
    let core_issue_index = IssueIndex::load_core()?;
    let mut out = String::new();

    let components = if component == "all" {
        root.manifest.components.keys().cloned().collect::<Vec<_>>()
    } else {
        vec![manifest::find_component_key(&root.manifest, component)?]
    };

    writeln!(
        out,
        "Adapter issue dependency report: adapter={adapter} mode={}",
        if dry_run { "dry-run" } else { "apply" }
    )
    .expect("write to String");

    for name in components {
        let Some(issue) = issue_index.by_component.get(&name) else {
            continue;
        };

        let comp = &root.manifest.components[&name];
        let expected = expected_blockers(
            comp,
            adapter,
            adapter_filter,
            &issue_index,
            &core_issue_index,
        )?;
        let current = issue_blocked_by(issue.number)?;
        let expected_numbers = expected
            .iter()
            .map(|blocker| blocker.number)
            .collect::<BTreeSet<_>>();
        let current_numbers = current.iter().copied().collect::<BTreeSet<_>>();
        let missing = expected_numbers
            .difference(&current_numbers)
            .copied()
            .collect::<Vec<_>>();
        let extra = current_numbers
            .difference(&expected_numbers)
            .copied()
            .collect::<Vec<_>>();

        writeln!(out).expect("write to String");
        writeln!(out, "## #{} {}", issue.number, issue.title).expect("write to String");
        writeln!(out, "component: {name}").expect("write to String");
        writeln!(
            out,
            "expected_blocked_by: [{}]",
            expected_numbers
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
        .expect("write to String");
        writeln!(
            out,
            "current_blocked_by: [{}]",
            current_numbers
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
        .expect("write to String");
        writeln!(
            out,
            "missing_native_dependencies: [{}]",
            missing
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
        .expect("write to String");
        writeln!(
            out,
            "extra_native_dependencies: [{}]",
            extra
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
        .expect("write to String");

        write_expected_dep_section(&mut out, &expected);
        write_boundary_notes(&mut out, comp, adapter_filter);

        if !dry_run {
            for blocker in expected {
                if missing.contains(&blocker.number) {
                    add_blocked_by(issue.number, blocker.id)?;
                    writeln!(
                        out,
                        "added_native_dependency: #{} blocked by #{}",
                        issue.number, blocker.number
                    )
                    .expect("write to String");
                }
            }
        }
    }

    Ok(out)
}

#[derive(Debug, Clone)]
struct IssueSummary {
    number: u64,
    id: u64,
    title: String,
}

#[derive(Debug)]
struct IssueIndex {
    by_component: BTreeMap<String, IssueSummary>,
}

impl IssueIndex {
    fn load(adapter: &str) -> Result<Self, Error> {
        let query = format!("repo:fogodev/ars-ui is:issue in:title adapter {adapter}");
        Self::load_from_search(&query, |title| component_from_adapter_title(title, adapter))
    }

    fn load_core() -> Result<Self, Error> {
        Self::load_from_search(
            "repo:fogodev/ars-ui is:issue in:title agnostic core",
            component_from_core_title,
        )
    }

    fn load_from_search<F>(query: &str, mut keys_for_title: F) -> Result<Self, Error>
    where
        F: FnMut(&str) -> Vec<String>,
    {
        let output = gh_output([
            "api",
            "-X",
            "GET",
            "search/issues",
            "--paginate",
            "-f",
            &format!("q={query}"),
            "--jq",
            ".items[] | [.number, .id, .title] | @tsv",
        ])?;

        let mut by_component = BTreeMap::new();
        for line in output.lines() {
            let mut parts = line.splitn(3, '\t');
            let Some(number) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
                continue;
            };
            let Some(id) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
                continue;
            };
            let Some(title) = parts.next() else {
                continue;
            };
            for component in keys_for_title(title) {
                by_component.insert(
                    component,
                    IssueSummary {
                        number,
                        id,
                        title: title.to_owned(),
                    },
                );
            }
        }

        Ok(Self { by_component })
    }
}

fn expected_blockers(
    comp: &manifest::Component,
    adapter_name: &str,
    adapter: AdapterFilter,
    adapter_issues: &IssueIndex,
    core_issues: &IssueIndex,
) -> Result<Vec<IssueSummary>, Error> {
    let mut blockers = Vec::new();
    for issue in baseline_adapter_dependencies(adapter_name)? {
        blockers.push(issue);
    }

    if let Some(core_issue) = core_issues
        .by_component
        .get(&component_key_from_path(&comp.path))
    {
        blockers.push(core_issue.clone());
    }

    for dep in &comp.component_deps {
        if !dep.frameworks.iter().any(|framework| {
            adapter
                .as_str()
                .is_none_or(|adapter| framework.as_str() == adapter)
        }) {
            continue;
        }

        if !component_deps::is_blocking(dep.kind, dep.blocking) {
            continue;
        }

        if let Some(issue) = adapter_issues.by_component.get(&dep.component) {
            blockers.push(issue.clone());
        }
    }

    blockers.sort_by_key(|issue| issue.number);
    blockers.dedup_by_key(|issue| issue.number);
    Ok(blockers)
}

fn baseline_adapter_dependencies(adapter: &str) -> Result<Vec<IssueSummary>, Error> {
    let numbers = match adapter {
        "leptos" => [190, 191].as_slice(),
        "dioxus" => [193, 194].as_slice(),
        _ => &[],
    };

    let mut issues = Vec::new();
    for number in numbers {
        issues.push(issue_summary(*number)?);
    }
    Ok(issues)
}

fn issue_summary(number: u64) -> Result<IssueSummary, Error> {
    let path = format!("repos/fogodev/ars-ui/issues/{number}");
    let output = gh_output([
        "api",
        "-H",
        "Accept: application/vnd.github+json",
        "-H",
        "X-GitHub-Api-Version: 2026-03-10",
        &path,
        "--jq",
        "[.number, .id, .title] | @tsv",
    ])?;

    let mut parts = output.trim().splitn(3, '\t');
    let Some(number) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
        return Err(Error::FrontmatterError(format!(
            "could not parse issue summary for #{number}"
        )));
    };
    let Some(id) = parts.next().and_then(|value| value.parse::<u64>().ok()) else {
        return Err(Error::FrontmatterError(format!(
            "could not parse issue id for #{number}"
        )));
    };
    let title = parts.next().unwrap_or_default().to_owned();

    Ok(IssueSummary { number, id, title })
}

fn write_expected_dep_section(out: &mut String, blockers: &[IssueSummary]) {
    writeln!(out, "expected_depends_on_section:").expect("write to String");
    if blockers.is_empty() {
        writeln!(out, "- none").expect("write to String");
    } else {
        for blocker in blockers {
            writeln!(out, "- #{} ({})", blocker.number, blocker.title).expect("write to String");
        }
    }
}

fn write_boundary_notes(out: &mut String, comp: &manifest::Component, adapter: AdapterFilter) {
    let notes = comp
        .component_deps
        .iter()
        .filter(|dep| dep.kind == ComponentDependencyKind::Boundary)
        .filter(|dep| {
            adapter
                .as_str()
                .is_none_or(|adapter| dep.frameworks.iter().any(|f| f == adapter))
        })
        .collect::<Vec<_>>();

    if notes.is_empty() {
        return;
    }

    writeln!(out, "component_dependency_notes:").expect("write to String");
    for note in notes {
        writeln!(out, "- not a blocker: {} — {}", note.component, note.reason)
            .expect("write to String");
    }
}

fn issue_blocked_by(issue: u64) -> Result<Vec<u64>, Error> {
    let path = format!("repos/fogodev/ars-ui/issues/{issue}/dependencies/blocked_by");
    let output = gh_output([
        "api",
        "-H",
        "Accept: application/vnd.github+json",
        "-H",
        "X-GitHub-Api-Version: 2026-03-10",
        &path,
        "--jq",
        ".[].number",
    ])?;

    Ok(output
        .lines()
        .filter_map(|line| line.trim().parse::<u64>().ok())
        .collect())
}

fn add_blocked_by(issue: u64, blocker_id: u64) -> Result<(), Error> {
    let path = format!("repos/fogodev/ars-ui/issues/{issue}/dependencies/blocked_by");
    let issue_id = format!("issue_id={blocker_id}");
    gh_output([
        "api",
        "-X",
        "POST",
        "-H",
        "Accept: application/vnd.github+json",
        "-H",
        "X-GitHub-Api-Version: 2026-03-10",
        &path,
        "-F",
        &issue_id,
    ])?;
    Ok(())
}

fn gh_output<const N: usize>(args: [&str; N]) -> Result<String, Error> {
    let output = Command::new("gh").args(args).output().map_err(Error::Io)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(Error::FrontmatterError(format!(
            "gh command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}

fn component_from_adapter_title(title: &str, adapter: &str) -> Vec<String> {
    let prefix = "task: Implement ";
    let Some(title) = title.strip_prefix(prefix) else {
        return Vec::new();
    };
    let lower = title.to_lowercase();
    let suffix = format!(" {adapter} adapter");
    if !lower.ends_with(&suffix) {
        return Vec::new();
    }

    let component_len = title.len().saturating_sub(suffix.len());
    let component = &title[..component_len];
    split_components(component)
}

fn component_from_core_title(title: &str) -> Vec<String> {
    let prefix = "task: Implement ";
    let suffix = " agnostic core";
    let Some(title) = title.strip_prefix(prefix) else {
        return Vec::new();
    };
    let Some(component) = title.strip_suffix(suffix) else {
        return Vec::new();
    };
    split_components(component)
}

fn component_key_from_path(path: &str) -> String {
    path.rsplit_once('/')
        .map_or(path, |(_, file)| file)
        .trim_end_matches(".md")
        .to_owned()
}

fn slug(value: &str) -> String {
    let mut out = String::new();
    let mut previous_was_lower_or_digit = false;

    for ch in value.trim().chars() {
        if ch.is_whitespace() || ch == '_' || ch == '-' {
            if !out.ends_with('-') && !out.is_empty() {
                out.push('-');
            }
            previous_was_lower_or_digit = false;
            continue;
        }

        if ch.is_ascii_uppercase() {
            if previous_was_lower_or_digit && !out.ends_with('-') && !out.is_empty() {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
            previous_was_lower_or_digit = false;
        } else {
            out.push(ch.to_ascii_lowercase());
            previous_was_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
    }

    out.trim_matches('-').to_owned()
}

fn split_components(value: &str) -> Vec<String> {
    value
        .replace(", and ", ", ")
        .replace(" and ", ", ")
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(slug)
        .collect()
}
