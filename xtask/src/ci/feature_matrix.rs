//! Feature-flag combination definitions for CI matrix testing.
//!
//! Each group mirrors a `feature-flags-*` job in `.github/workflows/ci.yml`.
//! Every combo runs `cargo check` then `cargo nextest run --lib`; cross-checks
//! run `cargo check --target <triple>` without tests.

use super::Error;

/// Feature-flag test groups, matching CI job names.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Group {
    /// `ars-core` feature combinations (15 combos).
    Core,
    /// `ars-i18n` feature combinations (11 combos + wasm32 cross-check).
    I18n,
    /// `ars-interactions`, `ars-collections`, `ars-forms`, `ars-dom` combos
    /// (12 combos + wasm32 cross-check).
    Subsystems,
    /// `ars-leptos` render-mode combos (3 combos).
    Leptos,
    /// `ars-dioxus` platform combos (4 combos + wasm32 cross-check).
    Dioxus,
}

/// A single feature combination to check and test.
pub(crate) struct Combo {
    /// Arguments passed to both `cargo check` and `cargo nextest run --lib`.
    pub(crate) args: &'static [&'static str],
}

/// A cross-compilation check (no tests).
pub(crate) struct CrossCheck {
    /// Arguments passed to `cargo check`.
    pub(crate) args: &'static [&'static str],
    /// Target triple (e.g., `wasm32-unknown-unknown`).
    pub(crate) target: &'static str,
}

/// Full definition for one feature-flag group.
pub(crate) struct GroupDef {
    pub(crate) combos: &'static [Combo],
    pub(crate) cross_checks: &'static [CrossCheck],
}

// ---------------------------------------------------------------------------
// Combo data — mirrors ci.yml exactly
// ---------------------------------------------------------------------------

static CORE_COMBOS: &[Combo] = &[
    Combo {
        args: &["-p", "ars-core", "--no-default-features"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "std"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "serde"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "debug"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "ssr"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "embedded-css"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "std,ssr"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "std,serde"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "std,debug"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "serde,debug"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "serde,ssr"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "debug,ssr"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "serde,embedded-css"],
    },
    Combo {
        args: &["-p", "ars-core", "--features", "std,serde,debug"],
    },
    Combo {
        args: &["-p", "ars-core", "--all-features"],
    },
];

static I18N_COMBOS: &[Combo] = &[
    Combo {
        args: &[
            "-p",
            "ars-i18n",
            "--no-default-features",
            "--features",
            "gregorian,icu4x",
        ],
    },
    Combo {
        args: &["-p", "ars-i18n", "--features", "gregorian,hebrew"],
    },
    Combo {
        args: &["-p", "ars-i18n", "--features", "gregorian,islamic"],
    },
    Combo {
        args: &["-p", "ars-i18n", "--features", "gregorian,hebrew,islamic"],
    },
    Combo {
        args: &["-p", "ars-i18n", "--features", "all-calendars"],
    },
    Combo {
        args: &["-p", "ars-i18n", "--features", "buddhist"],
    },
    Combo {
        args: &["-p", "ars-i18n", "--features", "japanese"],
    },
    Combo {
        args: &[
            "-p",
            "ars-i18n",
            "--no-default-features",
            "--features",
            "japanese-extended,icu4x",
        ],
    },
    Combo {
        args: &["-p", "ars-i18n", "--features", "persian"],
    },
    Combo {
        args: &["-p", "ars-i18n", "--features", "chinese"],
    },
    Combo {
        args: &["-p", "ars-i18n", "--no-default-features"],
    },
];

static I18N_CROSS_CHECKS: &[CrossCheck] = &[CrossCheck {
    args: &[
        "-p",
        "ars-i18n",
        "--no-default-features",
        "--features",
        "web-intl",
    ],
    target: "wasm32-unknown-unknown",
}];

static SUBSYSTEMS_COMBOS: &[Combo] = &[
    Combo {
        args: &["-p", "ars-interactions", "--no-default-features"],
    },
    Combo {
        args: &[
            "-p",
            "ars-interactions",
            "--features",
            "aria-drag-drop-compat",
        ],
    },
    Combo {
        args: &[
            "-p",
            "ars-a11y",
            "-p",
            "ars-interactions",
            "--features",
            "ars-a11y/aria-drag-drop-compat,ars-interactions/aria-drag-drop-compat",
        ],
    },
    Combo {
        args: &["-p", "ars-collections", "--features", "i18n"],
    },
    Combo {
        args: &["-p", "ars-collections", "--features", "uuid"],
    },
    Combo {
        args: &["-p", "ars-collections", "--features", "std,uuid"],
    },
    Combo {
        args: &["-p", "ars-collections", "--features", "std,i18n,serde"],
    },
    Combo {
        args: &["-p", "ars-collections", "--features", "std,uuid,i18n"],
    },
    Combo {
        args: &["-p", "ars-collections", "--features", "serde"],
    },
    Combo {
        args: &["-p", "ars-forms", "--no-default-features"],
    },
    Combo {
        args: &["-p", "ars-forms", "--features", "serde"],
    },
    Combo {
        args: &["-p", "ars-dom", "--no-default-features"],
    },
    Combo {
        args: &["-p", "ars-dom", "--features", "ssr"],
    },
];

static SUBSYSTEMS_CROSS_CHECKS: &[CrossCheck] = &[CrossCheck {
    args: &["-p", "ars-dom", "--features", "web"],
    target: "wasm32-unknown-unknown",
}];

static LEPTOS_COMBOS: &[Combo] = &[
    Combo {
        args: &["-p", "ars-leptos", "--features", "ssr"],
    },
    Combo {
        args: &["-p", "ars-leptos", "--features", "hydrate"],
    },
    Combo {
        args: &["-p", "ars-leptos", "--features", "csr"],
    },
];

static DIOXUS_COMBOS: &[Combo] = &[
    Combo {
        args: &["-p", "ars-dioxus", "--features", "desktop"],
    },
    Combo {
        args: &["-p", "ars-dioxus", "--features", "desktop-dom"],
    },
    Combo {
        args: &["-p", "ars-dioxus", "--features", "mobile"],
    },
    Combo {
        args: &["-p", "ars-dioxus", "--features", "ssr"],
    },
];

static DIOXUS_CROSS_CHECKS: &[CrossCheck] = &[CrossCheck {
    args: &["-p", "ars-dioxus", "--features", "web"],
    target: "wasm32-unknown-unknown",
}];

// ---------------------------------------------------------------------------
// Lookup
// ---------------------------------------------------------------------------

pub(crate) fn group_def(group: Group) -> GroupDef {
    match group {
        Group::Core => GroupDef {
            combos: CORE_COMBOS,
            cross_checks: &[],
        },
        Group::I18n => GroupDef {
            combos: I18N_COMBOS,
            cross_checks: I18N_CROSS_CHECKS,
        },
        Group::Subsystems => GroupDef {
            combos: SUBSYSTEMS_COMBOS,
            cross_checks: SUBSYSTEMS_CROSS_CHECKS,
        },
        Group::Leptos => GroupDef {
            combos: LEPTOS_COMBOS,
            cross_checks: &[],
        },
        Group::Dioxus => GroupDef {
            combos: DIOXUS_COMBOS,
            cross_checks: DIOXUS_CROSS_CHECKS,
        },
    }
}

// ---------------------------------------------------------------------------
// Runner
// ---------------------------------------------------------------------------

/// Run every combo in `group`: `cargo check` + `cargo nextest run --lib` per
/// combo, then any cross-compilation checks.
pub(crate) fn run_group(group: Group) -> Result<(), Error> {
    let def = group_def(group);
    let step = group_step(group);

    if !def.cross_checks.is_empty() {
        preflight_wasm32()?;
    }

    for combo in def.combos {
        let mut args = vec!["check"];
        args.extend_from_slice(combo.args);
        super::cargo(step, &args)?;
    }

    for cross in def.cross_checks {
        let mut args = vec!["check"];
        args.extend_from_slice(cross.args);
        args.extend_from_slice(&["--target", cross.target]);
        super::cargo(step, &args)?;
    }

    crate::test::run_feature_matrix_group(group)
        .map(|_| ())
        .map_err(|error| super::map_test_error(step, error))
}

/// Map a [`Group`] to the [`super::Step`] it corresponds to.
const fn group_step(group: Group) -> super::Step {
    match group {
        Group::Core => super::Step::FeatureMatrixCore,
        Group::I18n => super::Step::FeatureMatrixI18n,
        Group::Subsystems => super::Step::FeatureMatrixSubsystems,
        Group::Leptos => super::Step::FeatureMatrixLeptos,
        Group::Dioxus => super::Step::FeatureMatrixDioxus,
    }
}

/// Verify the `wasm32-unknown-unknown` target is installed.
fn preflight_wasm32() -> Result<(), Error> {
    let output = std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .map_err(Error::Io)?;

    let installed = String::from_utf8_lossy(&output.stdout);
    if installed
        .lines()
        .any(|line| line.trim() == "wasm32-unknown-unknown")
    {
        Ok(())
    } else {
        Err(Error::MissingTool {
            tool: "wasm32-unknown-unknown target".into(),
            install_hint: "rustup target add wasm32-unknown-unknown".into(),
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_has_15_combos() {
        assert_eq!(CORE_COMBOS.len(), 15);
    }

    #[test]
    fn i18n_has_11_combos_and_1_cross() {
        assert_eq!(I18N_COMBOS.len(), 11);
        assert_eq!(I18N_CROSS_CHECKS.len(), 1);
    }

    #[test]
    fn subsystems_has_13_combos_and_1_cross() {
        assert_eq!(SUBSYSTEMS_COMBOS.len(), 13);
        assert_eq!(SUBSYSTEMS_CROSS_CHECKS.len(), 1);
    }

    #[test]
    fn leptos_has_3_combos() {
        assert_eq!(LEPTOS_COMBOS.len(), 3);
    }

    #[test]
    fn dioxus_has_4_combos_and_1_cross() {
        assert_eq!(DIOXUS_COMBOS.len(), 4);
        assert_eq!(DIOXUS_CROSS_CHECKS.len(), 1);
    }

    /// Verify `group_def()` returns the expected data for every group and that
    /// cross-check counts are consistent with the static arrays.
    #[test]
    fn group_def_returns_correct_data() {
        let cases = &[
            (Group::Core, 15, 0),
            (Group::I18n, 11, 1),
            (Group::Subsystems, 13, 1),
            (Group::Leptos, 3, 0),
            (Group::Dioxus, 4, 1),
        ];
        for &(group, expected_combos, expected_cross) in cases {
            let def = group_def(group);
            assert_eq!(
                def.combos.len(),
                expected_combos,
                "{group:?} combo count mismatch"
            );
            assert_eq!(
                def.cross_checks.len(),
                expected_cross,
                "{group:?} cross-check count mismatch"
            );
        }
    }

    /// Every cross-check targets wasm32-unknown-unknown.
    #[test]
    fn cross_checks_target_wasm32() {
        let groups = [
            Group::Core,
            Group::I18n,
            Group::Subsystems,
            Group::Leptos,
            Group::Dioxus,
        ];
        for group in groups {
            let def = group_def(group);
            for cross in def.cross_checks {
                assert_eq!(
                    cross.target, "wasm32-unknown-unknown",
                    "{group:?} cross-check has unexpected target: {}",
                    cross.target
                );
            }
        }
    }

    /// Every combo has non-empty args starting with `-p`.
    #[test]
    fn combo_args_start_with_package_flag() {
        let groups = [
            Group::Core,
            Group::I18n,
            Group::Subsystems,
            Group::Leptos,
            Group::Dioxus,
        ];
        for group in groups {
            let def = group_def(group);
            for (i, combo) in def.combos.iter().enumerate() {
                assert!(!combo.args.is_empty(), "{group:?} combo {i} has empty args");
                assert_eq!(
                    combo.args[0], "-p",
                    "{group:?} combo {i} doesn't start with -p: {:?}",
                    combo.args
                );
            }
        }
    }
}
