//! `spec_search` — keyword/regex search across spec content.

use std::{fmt::Write, fs};

use regex::Regex;

use crate::manifest::{self, ManifestError, SpecRoot};

/// Section filter — maps to known heading patterns in the spec template.
#[derive(Debug, Clone, Copy)]
pub enum SectionFilter {
    /// `## 1. State Machine` or `## 1. API`.
    States,
    /// `### 1.2 Events`.
    Events,
    /// `### 1.4 Props` or `### 1.1 Props`.
    Props,
    /// `## 3. Accessibility` or similar.
    Accessibility,
    /// `## 2. Anatomy`.
    Anatomy,
    /// `## N. Internationalization`.
    Internationalization,
    /// `## N. Form Integration`.
    FormIntegration,
}

impl SectionFilter {
    /// Parse from a user-facing string like `"states"`, `"a11y"`, `"props"`.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "states" | "state_machine" | "state-machine" => Some(Self::States),
            "events" => Some(Self::Events),
            "props" => Some(Self::Props),
            "accessibility" | "a11y" => Some(Self::Accessibility),
            "anatomy" => Some(Self::Anatomy),
            "i18n" | "internationalization" => Some(Self::Internationalization),
            "forms" | "form_integration" | "form-integration" => Some(Self::FormIntegration),
            _ => None,
        }
    }

    /// Heading text patterns that indicate this section.
    fn heading_patterns(&self) -> &[&str] {
        match self {
            Self::States => &["State Machine", "API"],
            Self::Events => &["Events"],
            Self::Props => &["Props"],
            Self::Accessibility => &["Accessibility"],
            Self::Anatomy => &["Anatomy"],
            Self::Internationalization => &["Internationalization"],
            Self::FormIntegration => &["Form Integration"],
        }
    }
}

/// A single search hit with file, line, content, and context.
struct SearchHit {
    /// Relative spec file path.
    file: String,
    /// 1-based line number.
    line_num: usize,
    /// The matching line text.
    line: String,
    /// Current section heading at the point of the match.
    section: String,
}

/// Search spec content by keyword/regex with optional filters.
///
/// Scans all component spec files matching the given filters and returns
/// lines that match the `query` regex pattern. Results include file path,
/// line number, section context, and the matching line text.
///
/// # Errors
///
/// Returns [`ManifestError::FrontmatterError`] if the regex pattern is invalid.
pub fn execute(
    root: &SpecRoot,
    query: &str,
    category: Option<&str>,
    section: Option<&str>,
    tier: Option<&str>,
) -> Result<String, ManifestError> {
    let re = Regex::new(query)
        .map_err(|e| ManifestError::FrontmatterError(format!("invalid regex: {e}")))?;
    let section_filter = section.and_then(SectionFilter::parse);
    let m = &root.manifest;
    let mut hits: Vec<SearchHit> = Vec::new();

    let files: Vec<(&str, &str)> = m
        .components
        .iter()
        .filter(|(_, c)| category.is_none() || category == Some(c.category.as_str()))
        .map(|(name, c)| (name.as_str(), c.path.as_str()))
        .collect();

    for (_name, rel_path) in &files {
        let file_path = root.path.join(rel_path);
        let Ok(content) = fs::read_to_string(&file_path) else {
            continue;
        };

        // Tier filter: check frontmatter for tier field.
        if let Some(wanted_tier) = tier {
            if let Some(fm_str) = manifest::extract_frontmatter(&content) {
                let has_tier = fm_str.lines().any(|l| {
                    l.trim()
                        .strip_prefix("tier:")
                        .is_some_and(|v| v.trim() == wanted_tier)
                });
                if !has_tier {
                    continue;
                }
            } else {
                continue;
            }
        }

        let mut current_section = String::new();
        let mut in_matching_section = section_filter.is_none();
        let mut section_level: u8 = 0;

        for (line_idx, line) in content.lines().enumerate() {
            if let Some((level, text)) = manifest::parse_heading(line) {
                current_section = text.to_string();
                if let Some(ref filter) = section_filter {
                    let patterns = filter.heading_patterns();
                    if level == 2 {
                        in_matching_section = patterns.iter().any(|p| text.contains(p));
                        if in_matching_section {
                            section_level = level;
                        }
                    } else if level <= section_level && section_level > 0 {
                        in_matching_section = false;
                    }
                }
            }
            if in_matching_section && re.is_match(line) {
                hits.push(SearchHit {
                    file: (*rel_path).to_string(),
                    line_num: line_idx + 1,
                    line: line.to_string(),
                    section: current_section.clone(),
                });
            }
        }
    }

    let mut out = String::new();
    writeln!(out, "# Search results for /{query}/").expect("write to String cannot fail");
    if let Some(cat) = category {
        writeln!(out, "Category filter: {cat}").expect("write to String cannot fail");
    }
    if let Some(sec) = section {
        writeln!(out, "Section filter: {sec}").expect("write to String cannot fail");
    }
    if let Some(t) = tier {
        writeln!(out, "Tier filter: {t}").expect("write to String cannot fail");
    }
    writeln!(out, "Matches: {}", hits.len()).expect("write to String cannot fail");
    writeln!(out).expect("write to String cannot fail");
    for hit in &hits {
        writeln!(out, "{}:L{} [{}]", hit.file, hit.line_num, hit.section)
            .expect("write to String cannot fail");
        writeln!(out, "  {}", hit.line.trim()).expect("write to String cannot fail");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn section_filter_parsing() {
        assert!(SectionFilter::parse("states").is_some());
        assert!(SectionFilter::parse("state-machine").is_some());
        assert!(SectionFilter::parse("state_machine").is_some());
        assert!(SectionFilter::parse("a11y").is_some());
        assert!(SectionFilter::parse("accessibility").is_some());
        assert!(SectionFilter::parse("props").is_some());
        assert!(SectionFilter::parse("events").is_some());
        assert!(SectionFilter::parse("anatomy").is_some());
        assert!(SectionFilter::parse("i18n").is_some());
        assert!(SectionFilter::parse("internationalization").is_some());
        assert!(SectionFilter::parse("forms").is_some());
        assert!(SectionFilter::parse("form-integration").is_some());
        assert!(SectionFilter::parse("form_integration").is_some());
        assert!(SectionFilter::parse("nonexistent").is_none());
    }
}
