//! `spec component-deps` — report component dependency metadata.

use std::fmt::Write;

use crate::manifest::{self, ComponentDependencyKind, Error, SpecRoot};

/// Framework adapter filter for component dependency reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterFilter {
    /// No framework filter.
    Any,

    /// Leptos adapter dependencies.
    Leptos,

    /// Dioxus adapter dependencies.
    Dioxus,
}

impl AdapterFilter {
    /// Parse an optional adapter string.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownFramework`] when the filter is not recognized.
    pub fn parse(value: Option<&str>) -> Result<Self, Error> {
        match value {
            None => Ok(Self::Any),
            Some("leptos") => Ok(Self::Leptos),
            Some("dioxus") => Ok(Self::Dioxus),
            Some(other) => Err(Error::UnknownFramework(other.to_owned())),
        }
    }

    /// Return the framework manifest spelling.
    #[must_use]
    pub const fn as_str(self) -> Option<&'static str> {
        match self {
            Self::Any => None,
            Self::Leptos => Some("leptos"),
            Self::Dioxus => Some("dioxus"),
        }
    }
}

/// Return dependency metadata for one component or all components.
///
/// # Errors
///
/// Returns [`Error::ComponentNotFound`] when `component` is neither `all` nor
/// a manifest component.
pub fn execute(
    root: &SpecRoot,
    component: Option<&str>,
    adapter: Option<&str>,
) -> Result<String, Error> {
    let adapter = AdapterFilter::parse(adapter)?;
    let component = component.unwrap_or("all");
    let mut out = String::new();

    if component == "all" {
        writeln!(
            out,
            "Component | Adapter | Dependency | Kind | Blocking | Reason"
        )
        .expect("write to String");

        for (name, comp) in &root.manifest.components {
            write_component_rows(&mut out, name, comp, adapter);
        }
    } else {
        let (name, comp) = manifest::find_component(&root.manifest, component)?;

        writeln!(out, "component: {name}").expect("write to String");
        if let Some(adapter) = adapter.as_str() {
            writeln!(out, "adapter: {adapter}").expect("write to String");
        }

        if comp.component_deps.is_empty() {
            writeln!(out, "component_deps: []").expect("write to String");
        } else {
            writeln!(out, "component_deps:").expect("write to String");
            for dep in &comp.component_deps {
                if !framework_matches(&dep.frameworks, adapter) {
                    continue;
                }

                writeln!(
                    out,
                    "- component: {}\n  kind: {}\n  frameworks: [{}]\n  blocking: {}\n  reason: {}",
                    dep.component,
                    dep.kind,
                    dep.frameworks.join(", "),
                    is_blocking(dep.kind, dep.blocking),
                    dep.reason
                )
                .expect("write to String");
            }
        }
    }

    Ok(out)
}

fn write_component_rows(
    out: &mut String,
    name: &str,
    comp: &manifest::Component,
    adapter: AdapterFilter,
) {
    if comp.component_deps.is_empty() {
        writeln!(
            out,
            "{name} | {} | - | - | false | -",
            adapter_label(adapter)
        )
        .expect("write to String");
        return;
    }

    for dep in &comp.component_deps {
        if !framework_matches(&dep.frameworks, adapter) {
            continue;
        }

        let adapters = match adapter.as_str() {
            Some(value) => value.to_owned(),
            None => dep.frameworks.join(","),
        };

        writeln!(
            out,
            "{name} | {adapters} | {} | {} | {} | {}",
            dep.component,
            dep.kind,
            is_blocking(dep.kind, dep.blocking),
            dep.reason
        )
        .expect("write to String");
    }
}

fn adapter_label(adapter: AdapterFilter) -> &'static str {
    adapter.as_str().unwrap_or("all")
}

fn framework_matches(frameworks: &[String], adapter: AdapterFilter) -> bool {
    match adapter.as_str() {
        None => true,
        Some(adapter) => frameworks.iter().any(|framework| framework == adapter),
    }
}

/// Return whether a dependency should block issue pickup.
#[must_use]
pub const fn is_blocking(kind: ComponentDependencyKind, blocking: bool) -> bool {
    match kind {
        ComponentDependencyKind::Requires => true,
        ComponentDependencyKind::Composes => blocking,
        ComponentDependencyKind::Boundary | ComponentDependencyKind::Related => false,
    }
}
