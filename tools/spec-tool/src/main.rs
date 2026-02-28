use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process,
};

use clap::{Parser, Subcommand};
use serde::Deserialize;

/// Spec navigation tool — resolves file sets from manifest.toml for LLM reviews.
#[derive(Parser)]
#[command(name = "spec-tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// List all files needed to review a component
    Deps {
        /// Component name (e.g., "combobox", "date-picker")
        component: String,
    },
    /// List all components in a category with their deps
    Category {
        /// Category name (e.g., "selection", "overlay")
        name: String,
    },
    /// List all files for a review profile
    Profile {
        /// Profile name (e.g., `"accessibility"`, `"state_machine"`)
        name: String,
    },
    /// List all components that depend on a shared type
    Reverse {
        /// Shared type name (e.g., "selection-patterns", "date-time-types")
        shared_type: String,
    },
    /// List a component and all its related components with deps
    Related {
        /// Component name
        component: String,
    },
    /// Show component metadata (category, deps, related)
    Info {
        /// Component name
        component: String,
    },
    /// Validate that YAML frontmatter in spec files matches manifest.toml
    Validate,
    /// Output heading structure of a spec file
    Toc {
        /// Path to spec file (relative to spec/ or absolute)
        file: String,
    },
    /// List all adapter files for a framework, grouped by category
    Adapters {
        /// Framework: "leptos" or "dioxus"
        framework: String,
    },
}

#[derive(Deserialize)]
struct Manifest {
    foundation: BTreeMap<String, String>,
    shared: BTreeMap<String, String>,
    components: BTreeMap<String, Component>,
    review_profiles: Option<BTreeMap<String, ReviewProfile>>,
    #[serde(default)]
    leptos_adapters: BTreeMap<String, String>,
    #[serde(default)]
    dioxus_adapters: BTreeMap<String, String>,
}

#[derive(Deserialize, Clone)]
struct Component {
    path: String,
    category: String,
    foundation_deps: Vec<String>,
    #[serde(default)]
    shared_deps: Vec<String>,
    #[serde(default)]
    related: Vec<String>,
    #[serde(default)]
    internal: bool,
}

#[derive(Deserialize)]
struct ReviewProfile {
    files_always: Vec<String>,
}

fn find_spec_root() -> PathBuf {
    let mut dir = std::env::current_dir().expect("cannot read current directory");
    loop {
        let candidate = dir.join("spec").join("manifest.toml");
        if candidate.exists() {
            return dir.join("spec");
        }
        if !dir.pop() {
            eprintln!("error: could not find spec/manifest.toml in any parent directory");
            process::exit(1);
        }
    }
}

fn load_manifest(spec_root: &Path) -> Manifest {
    let path = spec_root.join("manifest.toml");
    let content = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", path.display());
        process::exit(1);
    });
    toml::from_str(&content).unwrap_or_else(|e| {
        eprintln!("error: cannot parse {}: {e}", path.display());
        process::exit(1);
    })
}

fn resolve_foundation(manifest: &Manifest, dep: &str) -> Option<String> {
    manifest.foundation.get(dep).cloned()
}

fn find_component_key(manifest: &Manifest, name: &str) -> String {
    let key = name.to_lowercase();
    if manifest.components.contains_key(&key) {
        return key;
    }
    let alt = key.replace('_', "-");
    if manifest.components.contains_key(&alt) {
        return alt;
    }
    eprintln!("error: component '{name}' not found in manifest");
    eprintln!("hint: available components:");
    for k in manifest.components.keys() {
        eprintln!("  {k}");
    }
    process::exit(1);
}

fn find_component<'a>(manifest: &'a Manifest, name: &str) -> &'a Component {
    let key = find_component_key(manifest, name);
    &manifest.components[&key]
}

fn category_file(category: &str) -> String {
    format!("components/{category}/_category.md")
}

fn cmd_deps(manifest: &Manifest, component: &str) {
    let key = find_component_key(manifest, component);
    let comp = &manifest.components[&key];

    println!("# Files to load for reviewing {component}:");
    println!();

    println!("## Component");
    println!("{}", comp.path);
    println!();

    if !comp.foundation_deps.is_empty() {
        println!("## Foundation deps");
        for dep in &comp.foundation_deps {
            if let Some(path) = resolve_foundation(manifest, dep) {
                println!("{path}");
            } else {
                println!("# WARNING: unknown foundation dep '{dep}'");
            }
        }
        println!();
    }

    if !comp.shared_deps.is_empty() {
        println!("## Shared deps");
        for dep in &comp.shared_deps {
            if let Some(path) = manifest.shared.get(dep) {
                println!("{path}");
            } else {
                println!("# WARNING: unknown shared dep '{dep}'");
            }
        }
        println!();
    }

    println!("## Category context");
    println!("{}", category_file(&comp.category));
    println!();

    let has_leptos = manifest.leptos_adapters.get(&key);
    let has_dioxus = manifest.dioxus_adapters.get(&key);
    if has_leptos.is_some() || has_dioxus.is_some() {
        println!("## Adapter examples");
        if let Some(path) = has_leptos {
            println!("{path}  (Leptos)");
        }
        if let Some(path) = has_dioxus {
            println!("{path}  (Dioxus)");
        }
        println!();
    }

    if !comp.related.is_empty() {
        println!("## Related components");
        for rel in &comp.related {
            if let Some(rel_comp) = manifest.components.get(rel) {
                println!("{}", rel_comp.path);
            } else {
                println!("# WARNING: unknown related component '{rel}'");
            }
        }
        println!();
    }
}

fn cmd_category(manifest: &Manifest, name: &str) {
    let components: Vec<(&String, &Component)> = manifest
        .components
        .iter()
        .filter(|(_, c)| c.category == name)
        .collect();

    if components.is_empty() {
        eprintln!("error: no components found in category '{name}'");
        eprintln!("hint: available categories:");
        let mut cats: Vec<&str> = manifest
            .components
            .values()
            .map(|c| c.category.as_str())
            .collect();
        cats.sort();
        cats.dedup();
        for c in cats {
            eprintln!("  {c}");
        }
        process::exit(1);
    }

    println!("# Category: {name}");
    println!();
    println!("## Category context");
    println!("{}", category_file(name));
    println!();

    println!("## Components ({} total)", components.len());
    for (key, comp) in &components {
        println!();
        println!("### {key}");
        println!("path: {}", comp.path);
        if !comp.foundation_deps.is_empty() {
            println!("foundation_deps: {}", comp.foundation_deps.join(", "));
        }
        if !comp.shared_deps.is_empty() {
            println!("shared_deps: {}", comp.shared_deps.join(", "));
        }
        if !comp.related.is_empty() {
            println!("related: {}", comp.related.join(", "));
        }
        if comp.internal {
            println!("internal: true");
        }
    }
}

fn cmd_profile(manifest: &Manifest, name: &str) {
    let profiles = manifest.review_profiles.as_ref().unwrap_or_else(|| {
        eprintln!("error: no review_profiles in manifest");
        process::exit(1);
    });

    // Try exact match, then with underscores/hyphens swapped
    let profile = profiles
        .get(name)
        .or_else(|| profiles.get(&name.replace('-', "_")))
        .or_else(|| profiles.get(&name.replace('_', "-")))
        .unwrap_or_else(|| {
            eprintln!("error: review profile '{name}' not found");
            eprintln!("hint: available profiles:");
            for k in profiles.keys() {
                eprintln!("  {k}");
            }
            process::exit(1);
        });

    println!("# Review profile: {name}");
    println!();
    println!("## Files always loaded");
    for file in &profile.files_always {
        println!("{file}");
    }
}

fn cmd_reverse(manifest: &Manifest, shared_type: &str) {
    // Verify the shared type exists
    if !manifest.shared.contains_key(shared_type) {
        eprintln!("error: shared type '{shared_type}' not found");
        eprintln!("hint: available shared types:");
        for k in manifest.shared.keys() {
            eprintln!("  {k}");
        }
        process::exit(1);
    }

    let dependents: Vec<(&String, &Component)> = manifest
        .components
        .iter()
        .filter(|(_, c)| c.shared_deps.iter().any(|d| d == shared_type))
        .collect();

    println!("# Components depending on shared/{shared_type}");
    println!();

    if dependents.is_empty() {
        println!("(none)");
    } else {
        println!("## Shared type file");
        println!(
            "{}",
            manifest.shared.get(shared_type).expect("verified above")
        );
        println!();
        println!("## Dependent components ({} total)", dependents.len());
        for (key, comp) in &dependents {
            println!("  {key}: {}", comp.path);
        }
    }
}

fn cmd_related(manifest: &Manifest, component: &str) {
    let comp = find_component(manifest, component);

    println!("# {component} and related components:");
    println!();
    println!("## Primary");
    println!("{}", comp.path);

    if comp.related.is_empty() {
        println!();
        println!("## Related");
        println!("(none)");
    } else {
        println!();
        println!("## Related components");
        for rel in &comp.related {
            if let Some(rel_comp) = manifest.components.get(rel) {
                println!();
                println!("### {rel}");
                println!("path: {}", rel_comp.path);
                if !rel_comp.foundation_deps.is_empty() {
                    println!("foundation_deps: {}", rel_comp.foundation_deps.join(", "));
                }
                if !rel_comp.shared_deps.is_empty() {
                    println!("shared_deps: {}", rel_comp.shared_deps.join(", "));
                }
            } else {
                println!();
                println!("### {rel}");
                println!("# WARNING: not found in manifest");
            }
        }
    }
}

fn cmd_info(manifest: &Manifest, component: &str) {
    let key = find_component_key(manifest, component);
    let comp = &manifest.components[&key];

    println!("component: {component}");
    println!("path: {}", comp.path);
    println!("category: {}", comp.category);
    println!("foundation_deps: [{}]", comp.foundation_deps.join(", "));
    println!("shared_deps: [{}]", comp.shared_deps.join(", "));
    println!("related: [{}]", comp.related.join(", "));
    if comp.internal {
        println!("internal: true");
    }
    if let Some(path) = manifest.leptos_adapters.get(&key) {
        println!("leptos_adapter: {path}");
    }
    if let Some(path) = manifest.dioxus_adapters.get(&key) {
        println!("dioxus_adapter: {path}");
    }
}

fn cmd_adapters(manifest: &Manifest, framework: &str) {
    let adapters = match framework {
        "leptos" => &manifest.leptos_adapters,
        "dioxus" => &manifest.dioxus_adapters,
        _ => {
            eprintln!("error: unknown framework '{framework}'");
            eprintln!("hint: use 'leptos' or 'dioxus'");
            process::exit(1);
        }
    };

    if adapters.is_empty() {
        println!("No {framework} adapter files registered.");
        return;
    }

    // Group by category
    let mut by_category: BTreeMap<String, Vec<(&String, &String)>> = BTreeMap::new();
    for (name, path) in adapters {
        let category = manifest
            .components
            .get(name)
            .map_or_else(|| "uncategorized".to_string(), |c| c.category.clone());
        by_category.entry(category).or_default().push((name, path));
    }

    println!("# {framework} adapter files ({} total)", adapters.len());
    println!();
    for (category, entries) in &by_category {
        println!("## {category}");
        for (name, path) in entries {
            println!("  {name}: {path}");
        }
        println!();
    }
}

fn cmd_validate(spec_root: &Path, manifest: &Manifest) {
    let mut errors: Vec<String> = Vec::new();
    let mut checked = 0u32;

    for (name, comp) in &manifest.components {
        let file_path = spec_root.join(&comp.path);
        if !file_path.exists() {
            errors.push(format!("[{name}] file not found: {}", comp.path));
            continue;
        }

        // Skip frontmatter validation for components hosted in foundation files
        if comp.path.starts_with("foundation/") {
            checked += 1;
            continue;
        }

        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("[{name}] cannot read {}: {e}", comp.path));
                continue;
            }
        };

        // Try to parse YAML frontmatter
        if let Some(frontmatter) = extract_frontmatter(&content) {
            match parse_frontmatter_yaml(&frontmatter) {
                Ok(fm) => {
                    // Compare against manifest
                    if let Some(fm_cat) = &fm.category
                        && fm_cat != &comp.category
                    {
                        errors.push(format!(
                            "[{name}] category mismatch: manifest='{}' file='{fm_cat}'",
                            comp.category
                        ));
                    }
                    if let Some(fm_fdeps) = &fm.foundation_deps {
                        let mut manifest_deps = comp.foundation_deps.clone();
                        let mut file_deps = fm_fdeps.clone();
                        manifest_deps.sort();
                        file_deps.sort();
                        if manifest_deps != file_deps {
                            errors.push(format!(
                                "[{name}] foundation_deps mismatch:\n  manifest: {manifest_deps:?}\n  file:     {file_deps:?}"
                            ));
                        }
                    }
                    if let Some(fm_sdeps) = &fm.shared_deps {
                        let mut manifest_deps = comp.shared_deps.clone();
                        let mut file_deps = fm_sdeps.clone();
                        manifest_deps.sort();
                        file_deps.sort();
                        if manifest_deps != file_deps {
                            errors.push(format!(
                                "[{name}] shared_deps mismatch:\n  manifest: {manifest_deps:?}\n  file:     {file_deps:?}"
                            ));
                        }
                    }
                    if let Some(fm_related) = &fm.related {
                        let mut manifest_rel = comp.related.clone();
                        let mut file_rel = fm_related.clone();
                        manifest_rel.sort();
                        file_rel.sort();
                        if manifest_rel != file_rel {
                            errors.push(format!(
                                "[{name}] related mismatch:\n  manifest: {manifest_rel:?}\n  file:     {file_rel:?}"
                            ));
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("[{name}] invalid YAML frontmatter: {e}"));
                }
            }
        } else if extract_html_comment_header(&content).is_some() {
            // Has HTML comment header but no YAML frontmatter — not yet converted
            errors.push(format!(
                "[{name}] uses HTML comment header (not yet converted to YAML frontmatter)"
            ));
        } else {
            errors.push(format!(
                "[{name}] no frontmatter or HTML comment header found"
            ));
        }

        checked += 1;
    }

    // Validate adapter files
    for (framework, adapters) in [
        ("leptos", &manifest.leptos_adapters),
        ("dioxus", &manifest.dioxus_adapters),
    ] {
        for (name, path) in adapters {
            let file_path = spec_root.join(path);
            if !file_path.exists() {
                errors.push(format!(
                    "[{framework}:{name}] adapter file not found: {path}"
                ));
                continue;
            }
            if !manifest.components.contains_key(name) {
                errors.push(format!(
                    "[{framework}:{name}] adapter key has no matching [components.{name}] entry"
                ));
            }
            checked += 1;
        }
    }

    println!("Validated {checked} files (components + adapters).");

    if errors.is_empty() {
        println!("All checks passed.");
    } else {
        println!();
        println!("{} error(s) found:", errors.len());
        for err in &errors {
            println!("  {err}");
        }
        process::exit(1);
    }
}

fn cmd_toc(spec_root: &Path, file: &str) {
    // Resolve the file path
    let file_path = if Path::new(file).is_absolute() || Path::new(file).exists() {
        PathBuf::from(file)
    } else {
        spec_root.join(file)
    };

    let content = fs::read_to_string(&file_path).unwrap_or_else(|e| {
        eprintln!("error: cannot read {}: {e}", file_path.display());
        process::exit(1);
    });

    for line in content.lines() {
        if let Some(heading) = parse_heading(line) {
            let indent = match heading.level {
                1 => "",
                2 => "  ",
                3 => "    ",
                4 => "      ",
                _ => "        ",
            };
            let prefix = "#".repeat(heading.level as usize);
            println!("{indent}{prefix} {}", heading.text);
        }
    }
}

struct Heading {
    level: u8,
    text: String,
}

fn parse_heading(line: &str) -> Option<Heading> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let hashes = trimmed.chars().take_while(|&c| c == '#').count();
    if hashes > 6 {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !rest.starts_with(' ') {
        return None;
    }
    Some(Heading {
        level: hashes as u8,
        text: rest.trim().to_string(),
    })
}

#[derive(Deserialize, Default)]
struct Frontmatter {
    component: Option<String>,
    category: Option<String>,
    foundation_deps: Option<Vec<String>>,
    shared_deps: Option<Vec<String>>,
    related: Option<Vec<String>>,
}

fn extract_frontmatter(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    Some(after_first[..end].trim().to_string())
}

fn extract_html_comment_header(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    // Find <!-- that appears near the top (within first few lines)
    let idx = trimmed.find("<!--")?;
    // Only match if it's within the first ~200 chars (title + blank line + comment)
    if idx > 200 {
        return None;
    }
    let after = &trimmed[idx + 4..];
    let end = after.find("-->")?;
    Some(after[..end].trim().to_string())
}

fn parse_frontmatter_yaml(yaml_str: &str) -> Result<Frontmatter, String> {
    // Simple YAML-like parser for our limited format
    let mut fm = Frontmatter::default();

    for line in yaml_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "component" => fm.component = Some(value.to_string()),
                "category" => fm.category = Some(value.to_string()),
                "foundation_deps" => fm.foundation_deps = Some(parse_yaml_list(value)),
                "shared_deps" => fm.shared_deps = Some(parse_yaml_list(value)),
                "related" => fm.related = Some(parse_yaml_list(value)),
                _ => {}
            }
        }
    }

    Ok(fm)
}

fn parse_yaml_list(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    if trimmed == "[]" || trimmed.is_empty() {
        return Vec::new();
    }
    // Strip brackets
    let inner = trimmed.trim_start_matches('[').trim_end_matches(']').trim();
    if inner.is_empty() {
        return Vec::new();
    }
    inner.split(',').map(|s| s.trim().to_string()).collect()
}

fn main() {
    let cli = Cli::parse();
    let spec_root = find_spec_root();
    let manifest = load_manifest(&spec_root);

    match cli.command {
        Command::Deps { component } => cmd_deps(&manifest, &component),
        Command::Category { name } => cmd_category(&manifest, &name),
        Command::Profile { name } => cmd_profile(&manifest, &name),
        Command::Reverse { shared_type } => cmd_reverse(&manifest, &shared_type),
        Command::Related { component } => cmd_related(&manifest, &component),
        Command::Info { component } => cmd_info(&manifest, &component),
        Command::Validate => cmd_validate(&spec_root, &manifest),
        Command::Toc { file } => cmd_toc(&spec_root, &file),
        Command::Adapters { framework } => cmd_adapters(&manifest, &framework),
    }
}
