# Test Organization & Policies

## 1. Test Organization

### 1.1 Directory structure

```filetree
crates/
├── ars-core/           # Core state machine, AttrMap, effects
│   └── tests/
│       ├── unit/           # TransitionCase table-driven tests
│       ├── integration/    # Service lifecycle, drain_queue, effects
│       └── snapshots/      # AttrMap snapshot assertions
├── ars-dom/            # DOM utilities, element queries
│   └── tests/
│       ├── unit/
│       ├── integration/    # DOM integration tests
│       └── ssr/            # SSR rendering tests
├── ars-a11y/           # Accessibility helpers, axe-core integration
│   └── tests/
│       ├── unit/           # ARIA attribute assertion helpers
│       └── integration/    # axe-core automated checks
├── ars-collections/    # Collection management, virtualization
│   └── tests/
│       ├── unit/           # CollectionBuilder, item operations
│       └── integration/    # Virtualization scroll behavior
├── ars-forms/          # Form context, validation
│   └── tests/
│       ├── unit/           # Validator tests, FieldState transitions
│       └── integration/    # Full form lifecycle, submission
├── ars-interactions/   # Keyboard, pointer, focus, move, long-press
│   └── tests/
│       ├── unit/           # Interaction state machines
│       └── integration/    # Cross-interaction scenarios
├── ars-i18n/           # Internationalization, locale, RTL, ICU4X
│   └── tests/
│       ├── unit/           # NumberParser, DateFormatter
│       └── integration/    # Locale switching, RTL arrow keys
├── ars-derive/         # Proc macros (HasId, AllVariants)
│   └── tests/
│       └── unit/           # Macro expansion tests
├── ars-leptos/         # Leptos adapter
│   └── tests/
│       ├── unit/
│       ├── integration/    # Component mounting, reactivity
│       ├── ssr/            # SSR rendering, hydration round-trips
│       └── snapshots/      # Adapter-specific snapshot assertions
├── ars-dioxus/         # Dioxus adapter
│   └── tests/
│       ├── unit/
│       ├── integration/
│       ├── ssr/
│       └── snapshots/
# Note: ars-test-harness-* are workspace member crates under crates/ for
# organizational grouping; they publish as library crates.
├── ars-test-harness/           # Framework-agnostic test harness API
│   └── src/
│       └── lib.rs
├── ars-test-harness-leptos/    # Leptos backend for test harness
│   └── src/
│       └── lib.rs
└── ars-test-harness-dioxus/    # Dioxus backend for test harness
    └── src/
        └── lib.rs
```

The test harness infrastructure is defined in [15-test-harness.md](../testing/15-test-harness.md), which specifies the `TestHarness` API, backend implementations, and the `AnyService` type-erased wrapper.

### 1.2 Naming convention

> **Enforcement:** This naming convention is enforced at code review time, not by CI. There is no automated lint for test naming patterns.

```text
test_{machine}_{scenario}
```

Examples:

- `test_button_idle_focus_keyboard`
- `test_button_loading_ignores_click`
- `test_combobox_open_arrow_down_focuses_next`
- `test_dialog_open_traps_focus`
- `test_table_select_all_updates_selected_rows`

### 1.3 Running tests

```bash
# Unit + integration (no browser needed)
cargo test -p ars-core

# Adapter tests (requires wasm-pack + browser)
wasm-pack test --headless --chrome ars-leptos
wasm-pack test --headless --chrome ars-dioxus

# Snapshot update
cargo insta test -p ars-core --review

# Accessibility audit (nightly-only)
cargo test -p ars-leptos --test axe -- --nocapture
```

> CI pipeline details: see [14-ci.md](14-ci.md).

---

## 2. Feature Flag Test Patterns

Feature flags for calendar systems and adapter targets must be tested to ensure every combination compiles correctly and disabled features produce appropriate compile-time errors.

> CI matrix and cross-compilation jobs: see [14-ci.md](14-ci.md#15-feature-flag-matrix).

### 2.1 No-Default-Features Compilation

```rust
// tests/feature_flags.rs
// This file is compiled under `--no-default-features` in CI.

#[test]
fn core_types_available_without_features() {
    // Core state machine types must compile without any calendar features
    let _state = button::State::Idle;
    let _props = button::Props::default();
    let _ctx = checkbox::Context::default();
}

#[cfg(not(feature = "gregorian"))]
#[test]
fn calendar_types_absent_without_feature() {
    // This test verifies at compile-time that calendar types are gated.
    // If the `gregorian` feature is disabled, `Calendar` should not exist.
    // The test body is empty — it passes by compiling successfully.
}
```

### 2.2 Feature-Gated Component Availability

```rust
#[cfg(feature = "gregorian")]
#[test]
fn gregorian_calendar_available() {
    let props = calendar::Props {
        calendar_system: CalendarSystem::Gregorian,
        ..Default::default()
    };
    let (state, ctx) = calendar::Machine::init(&props);
    assert_eq!(state, calendar::State::Idle);
}

#[cfg(feature = "hebrew")]
#[test]
fn hebrew_calendar_available() {
    let props = calendar::Props {
        calendar_system: CalendarSystem::Hebrew,
        ..Default::default()
    };
    let (state, _ctx) = calendar::Machine::init(&props);
    assert_eq!(state, calendar::State::Idle);
}

#[cfg(all(feature = "gregorian", feature = "hebrew"))]
#[test]
fn multiple_calendar_systems_coexist() {
    let greg = calendar::Props { calendar_system: CalendarSystem::Gregorian, ..Default::default() };
    let hebrew = calendar::Props { calendar_system: CalendarSystem::Hebrew, ..Default::default() };
    let (s1, _) = calendar::Machine::init(&greg);
    let (s2, _) = calendar::Machine::init(&hebrew);
    assert_eq!(s1, calendar::State::Idle);
    assert_eq!(s2, calendar::State::Idle);
}
```

### 2.3 Adapter Feature Isolation

```rust
#[cfg(feature = "adapter-leptos")]
#[test]
fn leptos_adapter_compiles() {
    // Verify the Leptos adapter module is available
    let _ = leptos_adapter::LeptosButton::default;
}

#[cfg(feature = "adapter-dioxus")]
#[test]
fn dioxus_adapter_compiles() {
    // Verify the Dioxus adapter module is available
    let _ = dioxus_adapter::DioxusButton::default;
}

#[cfg(not(feature = "adapter-leptos"))]
compile_error!("This test file requires the adapter-leptos feature");
// ^ Only included in the leptos-specific CI job to verify gating works
```

#### ars-a11y Feature Flags

```rust
#[cfg(feature = "aria-drag-drop-compat")]
#[test]
fn drag_drop_aria_attributes_present() {
    // Verify aria-grabbed and aria-dropeffect are set
    // when aria-drag-drop-compat feature is enabled
}

#[cfg(feature = "axe")]
#[test]
fn axe_core_integration_available() {
    // Verify axe-core runner is importable and functional
}
```

#### ars-collections Feature Flags

```rust
// ars-collections has three feature flags (see foundation/06-collections.md):
//   default = ["std"]  — standard library types
//   i18n                — locale-aware collation via ars-i18n
//   serde               — serialization for selection state

/// ars-collections without std feature
#[cfg(not(feature = "std"))]
#[test]
fn collections_core_types_available_without_std() {
    // Verify core collection types compile in no_std context
    let _key = ars_collections::Key::from("item");
    let _set = ars_collections::selection::Set::Empty;
}

#[cfg(feature = "i18n")]
#[test]
fn locale_aware_collation_sorts_correctly() {
    use ars_collections::{SortedCollection, CollectionBuilder, Key, SortDirection};
    // With i18n feature, SortedCollection can use locale-aware string comparison
    let col = CollectionBuilder::new()
        .item(Key::from("a"), "Ärger")  // German: Ä sorts near A
        .item(Key::from("b"), "Banana")
        .build();
    let sorted = SortedCollection::with_locale(&col, SortDirection::Ascending, &locale!("de"));
    let first = sorted.first_key().expect("sorted collection must have first key");
    assert_eq!(*first, Key::from("a"), "German locale must sort Ä near A");
}

#[cfg(feature = "serde")]
#[test]
fn selection_state_round_trips_via_serde() {
    use ars_collections::selection::Set;
    let set = Set::Multiple(BTreeSet::from([Key::from("a"), Key::from("b")]));
    let json = serde_json::to_string(&set).expect("Set must serialize");
    let restored: Set = serde_json::from_str(&json).expect("Set must deserialize");
    assert_eq!(set, restored);
}
```

#### ars-forms Feature Flags

```rust
// ars-forms feature flags (see foundation/01-architecture.md):
//   default = []  — core validation and form context always available
//   serde          — serialization for FormContext, FieldState, ValidationResult

#[cfg(feature = "serde")]
#[test]
fn form_context_round_trips_via_serde() {
    let mut form = FormContext::new(ValidationMode::on_blur_revalidate());
    form.register("email", FieldValue::Text("test@example.com".into()),
        Some(Validators::new().build()), None);
    let json = serde_json::to_string(&form).expect("FormContext must serialize");
    let restored: FormContext = serde_json::from_str(&json).expect("FormContext must deserialize");
    assert_eq!(form.fields.len(), restored.fields.len());
}
```

#### ars-dom Feature Flags

```rust
// ars-dom SSR feature compiles on any target; web feature requires wasm32.
#[cfg(feature = "web")]
#[test]
fn dom_queries_work_in_wasm() {
    // ars-dom web feature requires wasm32; SSR feature compiles on any target
}

#[cfg(feature = "ssr")]
#[test]
fn ssr_dom_abstraction_available() {
    // Verify SSR DOM abstraction compiles and basic operations work
}
```

CI matrix entry: `cargo test -p ars-dom --features ssr`

#### ars-derive Feature Flags

```rust
// No feature flags — proc macros are always available
// Test via compile-time expansion verification
```

### 2.4 Adapter Parity Enforcement Matrix

Test parity between the Leptos and Dioxus adapters MUST be maintained to prevent one adapter from falling behind in coverage.

**Per-Component Test Count Parity**:

**Scope:** The adapter parity rule applies to **per-component adapter integration tests** only — tests that exercise the component through the adapter harness (mount, interact, assert). It does NOT require adapters to replicate core unit tests or snapshot tests. Specifically: for each component, the count of adapter integration tests in `ars-leptos` must be >= the count of adapter integration tests in `ars-dioxus`, and vice versa. This ensures both adapters exercise the same interaction scenarios.

```text
For each component (e.g., Checkbox, Select, Dialog):
  |ars-leptos tests - ars-dioxus tests| <= 2  (tolerance for adapter-specific edge cases)
```

`check_adapter_parity.sh` compares test counts per component (not total), with tolerance of |count_leptos - count_dioxus| <= 2 per component.

> See [14-ci.md](./14-ci.md) §6.1 for the script specification — it must match this per-component granularity.

Any component with zero integration tests in either adapter is a policy violation.

> CI enforcement: see [14-ci.md](14-ci.md#16-adapter-parity-enforcement).

**Snapshot Equivalence**: Both adapters MUST produce identical `AttrMap` snapshots for the same `Context` and `Props` combination:

```rust
#[test]
fn attr_map_parity_checkbox() {
    let props = CheckboxProps::default();
    let leptos_attrs = ars_leptos::render_attrs::<Checkbox>(&props);
    let dioxus_attrs = ars_dioxus::render_attrs::<Checkbox>(&props);
    assert_eq!(leptos_attrs, dioxus_attrs, "AttrMap mismatch for Checkbox");
}
```

This test MUST exist for every component and cover at least: default props, disabled state, and one interactive state (e.g., checked, open, selected).

> **Scope:** Adapter parity for `render_attrs` output is enforced only for the `web` target
> (both adapters rendering to browser DOM). SSR output and Dioxus Desktop are excluded from
> snapshot equivalence checks, as they may legitimately differ (Leptos SSR adds hydration
> markers, Dioxus Desktop has no DOM).
>
> `render_attrs` is defined in each adapter crate: `ars_leptos::render_attrs()` and
> `ars_dioxus::render_attrs()`. These functions convert an `AttrMap` to framework-specific
> attribute representations. See [08-adapter-leptos.md](../foundation/08-adapter-leptos.md)
> and [09-adapter-dioxus.md](../foundation/09-adapter-dioxus.md) for definitions.

---

## 3. Snapshot Maintenance Policy

1. Snapshot changes from ARIA spec updates are documented in CHANGELOG and reviewed by accessibility lead.
2. Batch snapshot updates use `cargo insta review` workflow.
3. Breaking snapshot changes (removed attributes, changed roles) require a semver **minor** bump while the library is pre-1.0 (per Cargo convention where 0.x.y treats minor as breaking). Post-1.0, breaking changes require a **major** bump.
4. Non-breaking additions (new attributes) are patch-level.

### 3.1 Snapshot Count Budgeting

- **Warning threshold:** When total snapshot count exceeds 500, a CI warning is emitted.
- **Per-component limit:** No single component should have more than 20 snapshot files. Components exceeding this should consolidate state variants.
- **Quarterly audit:** Review snapshot growth each quarter. Prune snapshots for removed or significantly refactored components.
- **New component budget:** Each new component starts with a budget of 3 snapshots per state variant × number of anatomy parts.

Each component is budgeted a maximum of **20 snapshots**. The budget formula is:

`budget = min(3 × state_variants × anatomy_parts, 20)`

Components exceeding 20 snapshots require explicit justification in PR review.

**Enforcement:** The `check_snapshot_count.py` script verifies per-component snapshot counts
using a flat cap of 20. The script counts `*.snap` files per component directory.

> **CI enforcement:** The snapshot count linting job in [14-ci.md section 2.4](14-ci.md#24-snapshot-count-linting) enforces both minimum (>= 3 per component variant) and maximum (<= 20 per component) bounds.

**Review triggers:**

- Any PR adding more than 5 new snapshots
- Any component exceeding the 20-snapshot cap
- Quarterly audit (manually triggered, tracked via GitHub issue template)

> **Process:** Quarterly snapshot audits are tracked via GitHub issue template
> `snapshot-quarterly-audit.yml`. The audit is a manual review process — CI does not
> enforce the audit schedule, but does enforce per-PR snapshot count limits.

#### Snapshot Change CI Enforcement

PRs modifying `*.snap` files must either:

- Include a CHANGELOG entry describing the visual/structural change, OR
- Have the `snapshot-reviewed` label applied by a reviewer

CI job: `snapshot-change-check` verifies this requirement before merge.

> **CI enforcement:** The `snapshot-change-check` job in [14-ci.md section 1.7](14-ci.md#17-snapshot-change-check) verifies that PRs modifying `*.snap` files either include a CHANGELOG entry or carry the `snapshot-reviewed` label. This job is a required status check in branch protection.

---

## 4. Test Coverage Metrics and Targets

All ars-ui crates MUST meet the following coverage targets, enforced in CI via `cargo-tarpaulin`
or `cargo-llvm-cov`. Builds that fall below thresholds MUST fail.

| Metric                             | Target                                                                                                                                                  | Scope                                                                                                                                                                                                                                 |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Line coverage (ars-core)           | >= 90%                                                                                                                                                  | All state machines, `transition()`, `connect()`, context logic                                                                                                                                                                        |
| Line coverage (ars-a11y)           | >= 85%                                                                                                                                                  | Accessibility utilities                                                                                                                                                                                                               |
| Line coverage (ars-i18n)           | >= 80%                                                                                                                                                  | I18N, some paths locale-dependent                                                                                                                                                                                                     |
| Line coverage (ars-interactions)   | >= 80%                                                                                                                                                  | Interaction handlers                                                                                                                                                                                                                  |
| Line coverage (ars-collections)    | >= 85%                                                                                                                                                  | Collection data structures                                                                                                                                                                                                            |
| Line coverage (ars-forms)          | >= 85%                                                                                                                                                  | Form validation                                                                                                                                                                                                                       |
| Line coverage (ars-dom)            | >= 75%                                                                                                                                                  | DOM abstraction, platform-dependent                                                                                                                                                                                                   |
| Line coverage (ars-leptos)         | Aspirational — `wasm-pack test` does not produce lcov data. Coverage verified by test count parity and code review until WASM coverage tooling matures. | Adapter mount/unmount, effect wiring, event dispatch                                                                                                                                                                                  |
| Line coverage (ars-dioxus)         | Aspirational — `wasm-pack test` does not produce lcov data. Coverage verified by test count parity and code review until WASM coverage tooling matures. | Adapter mount/unmount, effect wiring, event dispatch                                                                                                                                                                                  |
| Line coverage (ars-derive)         | N/A — proc-macro crate; verified indirectly via expansion tests (not enforced by CI)                                                                    | Proc macros                                                                                                                                                                                                                           |
| Branch coverage (critical paths)   | Per-crate line minimum                                                                                                                                  | Branch coverage targets are set 10% below line coverage minimums, reflecting the higher difficulty of achieving branch coverage. Critical path functions should aim for 100% branch coverage at code review time, not enforced by CI. |
| Snapshot count per component state | >= 3                                                                                                                                                    | At minimum: root attrs, primary interactive part, ARIA live region (if applicable)                                                                                                                                                    |
| Property-based test iterations     | >= 1000                                                                                                                                                 | Per state machine, using `proptest` or equivalent                                                                                                                                                                                     |

> CI enforcement: see [14-ci.md](14-ci.md#2-coverage-pipeline).

**Snapshot count enforcement**: Every component with more than two `State` variants MUST have
at least 3 snapshot tests per variant. Components with fewer are flagged for review. The lint
runs via `scripts/check_snapshot_count.py` and parses `*.snap` files in the test directory.

> CI integration: see [14-ci.md](14-ci.md#24-snapshot-count-linting).

**Property-based iteration count**: The `proptest` configuration in `Cargo.toml` MUST set
`PROPTEST_CASES=1000` as minimum. Nightly CI runs MAY increase to 10,000 for deeper coverage:

> **Pipeline scope:** Property-based tests (`proptest`) run in the **nightly pipeline** ([14-ci.md section 3.1](14-ci.md#31-extended-property-based-testing)) with `PROPTEST_CASES=1000`. They are excluded from the PR pipeline via `#[ignore]` to keep PR feedback fast. The nightly job runs `cargo test -p ars-core -- --ignored` to execute them.

```toml
# In workspace Cargo.toml [profile.test]
[profile.test.package.ars-core]
# proptest picks up PROPTEST_CASES from env; default enforced in test harness
```

```rust
// In each property-based test module
proptest! {
    #![proptest_config(ProptestConfig::with_cases(
        std::env::var("PROPTEST_CASES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000)
    ))]

    #[test]
    #[ignore] // Expensive; run nightly via `cargo test -- --ignored proptest`
    fn random_event_sequence_never_panics(events in prop::collection::vec(arb_event(), 1..200)) {
        let mut svc = Service::<button::Machine>::new(button::Props::default());
        for event in events {
            let _ = svc.send(event); // Must not panic
        }
    }
}
```

---

## 5. Error Message Standards for API Misuse

All runtime errors caused by developer misuse (missing IDs, invalid prop combinations, state
machine violations) MUST use the `ComponentError` enum rather than bare `panic!()` calls. This
provides consistent, actionable error messages and allows downstream code to handle recoverable
misuse gracefully.

### 5.1 `ComponentError` enum

```rust
/// Standardized error type for ars-ui component API misuse.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentError {
    /// A required ID was not provided (e.g., Button without `id` prop).
    MissingId {
        component: &'static str,
        part: &'static str,
    },
    /// A disabled component received an event that should have been blocked by the guard.
    DisabledGate {
        component: &'static str,
        event: String,
    },
    /// Two or more props conflict and cannot be used together.
    InvalidPropCombination {
        component: &'static str,
        props: Vec<&'static str>,
        reason: String,
    },
    /// A state machine received an event that violates its protocol (e.g., Close when already Closed).
    InvalidStateTransition {
        component: &'static str,
        current_state: String,
        event: String,
    },
    /// A lifetime or ownership constraint was violated (e.g., using a service after unmount).
    LifetimeViolation {
        component: &'static str,
        reason: String,
    },
}

impl std::fmt::Display for ComponentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingId { component, part } =>
                write!(f, "[ars-ui:{component}] Missing required `id` on part `{part}`. \
                       Provide an explicit ID or use the default ID generation."),
            Self::DisabledGate { component, event } =>
                write!(f, "[ars-ui:{component}] Event `{event}` was sent to a disabled component. \
                       Check `disabled` prop before dispatching."),
            Self::InvalidPropCombination { component, props, reason } =>
                write!(f, "[ars-ui:{component}] Invalid prop combination {:?}: {reason}", props),
            Self::InvalidStateTransition { component, current_state, event } =>
                write!(f, "[ars-ui:{component}] Cannot handle `{event}` in state `{current_state}`. \
                       This is likely a bug in event dispatch logic."),
            Self::LifetimeViolation { component, reason } =>
                write!(f, "[ars-ui:{component}] Lifetime violation: {reason}"),
        }
    }
}

impl std::error::Error for ComponentError {}
```

### 5.2 Usage pattern

Functions that validate developer-facing API calls MUST return `Result<T, ComponentError>`
instead of panicking. Panics are reserved for internal invariant violations that indicate bugs
in ars-ui itself (not in consumer code):

```rust
// GOOD: validation wrapper around infallible Machine::connect returns Result
// Machine::connect itself is infallible (returns Api directly), so validation
// is performed separately before calling connect.
pub fn validate_props(props: &Props) -> Result<(), ComponentError> {
    if props.id.is_empty() {
        return Err(ComponentError::MissingId {
            component: "Dialog",
            part: "root",
        });
    }
    Ok(())
}

// GOOD: test verifies error message quality
#[test]
fn dialog_missing_id_returns_clear_error() {
    let props = dialog::Props { id: "".into(), ..Default::default() };
    // Validation is a separate step — Machine::connect is infallible
    let result = dialog::validate_props(&props);
    let err = result.unwrap_err();
    assert!(matches!(err, ComponentError::MissingId { component: "Dialog", .. }));
    assert!(err.to_string().contains("Missing required `id`"));
}
```

### 5.3 Test requirements for error messages

Every `ComponentError` variant MUST have at least one test that:

1. Triggers the error condition.
2. Asserts the correct variant is returned (not a panic).
3. Asserts the `Display` output contains the component name and actionable guidance.

**CI Enforcement:** A CI job validates that every `ComponentError` variant has at least one test exercising it. See [14-ci.md § Error Variant Coverage](14-ci.md#25-error-variant-coverage).

#### Error Message Coverage

Every `ComponentError` variant must have at least one test that triggers it and verifies
the error message content. The CI job `error-variant-coverage` (see [14-ci.md](14-ci.md#25-error-variant-coverage))
cross-references variant definitions against test names.
