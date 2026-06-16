//! Tests for the examples xtask catalog.

use std::{collections::BTreeSet, path::Path};

use xtask::examples::{EXAMPLE_NAMES, Framework, catalog, resolve};

#[test]
fn examples_catalog_lists_all_widget_variants() {
    assert_eq!(
        EXAMPLE_NAMES,
        [
            "widgets-leptos",
            "widgets-dioxus",
            "widgets-leptos-css",
            "widgets-dioxus-css",
            "widgets-leptos-tailwind",
            "widgets-dioxus-tailwind",
        ]
    );

    let names = catalog()
        .iter()
        .map(|example| example.name)
        .collect::<BTreeSet<_>>();

    assert_eq!(names.len(), 6);

    for name in EXAMPLE_NAMES {
        assert!(names.contains(name), "missing {name}");
    }
}

#[test]
fn examples_resolve_framework_and_paths() {
    let leptos = resolve("widgets-leptos-tailwind").expect("leptos example should resolve");
    let dioxus = resolve("widgets-dioxus-css").expect("dioxus example should resolve");

    assert_eq!(leptos.framework, Framework::Leptos);
    assert_eq!(dioxus.framework, Framework::Dioxus);
    assert_eq!(leptos.path, "examples/widgets-leptos-tailwind");
    assert_eq!(dioxus.path, "examples/widgets-dioxus-css");
}

#[test]
fn examples_reject_unknown_names() {
    let error = resolve("widgets-react").expect_err("unknown example should fail");

    assert!(error.contains("unknown example"));
    assert!(error.contains("widgets-leptos"));
    assert!(error.contains("widgets-dioxus-tailwind"));
}

#[test]
fn dioxus_widget_public_assets_exist_for_inline_includes() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask manifest dir has workspace parent");

    for example in [
        "examples/widgets-dioxus",
        "examples/widgets-dioxus-css",
        "examples/widgets-dioxus-tailwind",
    ] {
        for asset in ["ars-base.css", "ars-interactions.css"] {
            let path = workspace_root.join(example).join("public").join(asset);

            assert!(
                path.exists(),
                "{} must exist because the Dioxus widget entrypoint includes it",
                path.display()
            );
        }
    }
}
