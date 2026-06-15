//! Adapter component scaffolding helpers.

use std::{
    fmt::{self, Display, Write as _},
    fs, io,
    path::{Path, PathBuf},
};

/// Adapter scaffold options.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaffoldOptions {
    /// Component name in kebab or snake case.
    pub component: String,

    /// Component category, such as `input` or `utility`.
    pub category: String,

    /// Include Leptos adapter files.
    pub leptos: bool,

    /// Include Dioxus adapter files.
    pub dioxus: bool,

    /// Whether this component is a form control requiring Form/Fieldset
    /// composition test placeholders.
    pub form_control: bool,

    /// Workspace root to scaffold under.
    pub root: PathBuf,
}

/// Adapter scaffold errors.
#[derive(Debug)]
pub enum Error {
    /// IO error writing scaffold files.
    Io(io::Error),

    /// Refusing to overwrite an existing file.
    Exists(PathBuf),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "IO error: {error}"),
            Self::Exists(path) => write!(f, "refusing to overwrite {}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Create adapter component skeleton files.
///
/// # Errors
///
/// Returns an error when a target file already exists or cannot be written.
pub fn scaffold(options: &ScaffoldOptions) -> Result<String, Error> {
    let component = options.component.replace('-', "_");
    let category = options.category.replace('-', "_");

    let mut created = Vec::new();

    if options.leptos {
        create_adapter_files(options, "ars-leptos", &category, &component, &mut created)?;
    }

    if options.dioxus {
        create_adapter_files(options, "ars-dioxus", &category, &component, &mut created)?;
    }

    create_file(
        &options
            .root
            .join("docs/implementation/sketches")
            .join(format!("{component}-counterpart-sketch.md")),
        &sketch_template(&component, &category),
        &mut created,
    )?;

    create_file(
        &options
            .root
            .join("docs/implementation/adapter-components")
            .join(format!("{component}-usage.md")),
        &usage_template(&component),
        &mut created,
    )?;

    let mut output = String::from("Created adapter scaffold files:\n");

    for path in created {
        let _ = writeln!(&mut output, "- {}", path.display());
    }

    Ok(output)
}

fn create_adapter_files(
    options: &ScaffoldOptions,
    crate_name: &str,
    category: &str,
    component: &str,
    created: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    create_file(
        &options
            .root
            .join("crates")
            .join(crate_name)
            .join("src")
            .join(category)
            .join(format!("{component}.rs")),
        &format!("//! {component} adapter scaffold.\n"),
        created,
    )?;

    create_file(
        &options
            .root
            .join("crates")
            .join(crate_name)
            .join("tests")
            .join(format!("{component}.rs")),
        &test_template(component, options.form_control),
        created,
    )?;

    create_file(
        &options
            .root
            .join("crates")
            .join(crate_name)
            .join("tests")
            .join(format!("{component}_wasm.rs")),
        &format!("//! Browser tests for the {component} adapter.\n"),
        created,
    )?;

    Ok(())
}

fn create_file(path: &Path, content: &str, created: &mut Vec<PathBuf>) -> Result<(), Error> {
    if path.exists() {
        return Err(Error::Exists(path.to_path_buf()));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, content)?;

    created.push(path.to_path_buf());

    Ok(())
}

fn test_template(component: &str, form_control: bool) -> String {
    let mut template = format!("//! SSR tests for the {component} adapter.\n\n");

    if form_control {
        template.push_str(
            r#"// Required composition coverage for form controls:
// - renders inside Form and serializes by name/value;
// - reset restores default state;
// - matching Form validation errors by name make the control invalid;
// - renders inside Fieldset and inherits disabled, readonly, and invalid state.
"#,
        );
    }

    template
}

fn sketch_template(component: &str, category: &str) -> String {
    format!(
        r#"# Component Adapter Reference Exploration Sketch

## Task

- Component: {component}
- Category: {category}

## Reference Sources

## Reference Evidence

## Observed Reference Outcomes

## I18n Mapping

## Accessibility Mapping

## Ars Contract Mapping

## Final Outcome Matrix

| Reference outcome | Final status | API/contract stance | Reference proof | Local proof | Adapter tests | E2E/browser proof | I18n proof | A11y proof | Notes |
| ----------------- | ------------ | ------------------- | --------------- | ----------- | ------------- | ----------------- | ---------- | ---------- | ----- |
"#
    )
}

fn usage_template(component: &str) -> String {
    format!(
        r#"# {component} Usage Notes

Document standalone usage, supported composition contexts, form behavior when
relevant, inherited state, and required integration tests.
"#
    )
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{ScaffoldOptions, scaffold};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();

        let path = std::env::temp_dir().join(format!("ars-ui-adapter-{name}-{nanos}"));

        fs::create_dir_all(&path).expect("create temp dir");

        path
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).expect("read scaffold file")
    }

    #[test]
    fn scaffold_form_control_includes_composition_test_placeholders() {
        let root = temp_dir("form-control");

        let output = scaffold(&ScaffoldOptions {
            component: "checkbox".to_string(),
            category: "input".to_string(),
            leptos: true,
            dioxus: true,
            form_control: true,
            root: root.clone(),
        })
        .expect("scaffold should succeed");

        assert!(output.contains("checkbox-counterpart-sketch.md"));

        for crate_name in ["ars-leptos", "ars-dioxus"] {
            let test = read(
                &root
                    .join("crates")
                    .join(crate_name)
                    .join("tests/checkbox.rs"),
            );

            assert!(test.contains("renders inside Form"));
            assert!(test.contains("renders inside Fieldset"));
            assert!(test.contains("validation errors by name"));
        }

        drop(fs::remove_dir_all(root));
    }
}
