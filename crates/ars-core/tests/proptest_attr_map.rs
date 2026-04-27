//! Property-based regression tests for [`AttrMap`] invariants.
//!
//! These tests are `#[ignore]`d so they do not run in the per-PR fast tier.
//! The nightly workflow runs them with
//! `cargo test -p ars-core -- --ignored proptest` and `PROPTEST_CASES=10000`
//! for extended coverage. Run locally with the same command at the default
//! `PROPTEST_CASES=256` budget to smoke-test the properties.
//!
//! Property coverage keeps a sorted-keys invariant and a set/get round-trip
//! on non-space-separated attributes — the two contracts `AttrMap::set` and
//! `AttrMap::get` rely on via `binary_search_by`. Space-separated attributes
//! (class, aria-labelledby, aria-controls, etc.) have append semantics and
//! are deliberately excluded from the simple round-trip property.

use ars_core::{AriaAttr, AttrMap, AttrValue, HtmlAttr};
use proptest::prelude::*;

/// Strategy: pick from a bounded set of single-value `HtmlAttr` keys.
///
/// These keys are not in [`SPACE_SEPARATED`] inside `ars_core::connect`, so
/// `set` performs replace semantics and `get` returns exactly what was set.
fn single_value_attr() -> impl Strategy<Value = HtmlAttr> {
    prop_oneof![
        Just(HtmlAttr::Role),
        Just(HtmlAttr::Id),
        Just(HtmlAttr::TabIndex),
        Just(HtmlAttr::Aria(AriaAttr::Label)),
        Just(HtmlAttr::Aria(AriaAttr::Expanded)),
        Just(HtmlAttr::Aria(AriaAttr::Orientation)),
    ]
}

/// Strategy: short non-empty ASCII strings. The round-trip property only
/// requires that whatever goes in comes back unchanged, so the alphabet is
/// intentionally narrow to keep shrinking fast.
fn attr_string_value() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,16}".prop_map(String::from)
}

proptest! {
    /// For any non-space-separated attribute and any non-empty string value,
    /// `set` followed by `get` returns exactly the value that was set.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_attr_map_set_get_round_trip(
        attr in single_value_attr(),
        value in attr_string_value(),
    ) {
        let mut map = AttrMap::new();

        map.set(attr, AttrValue::String(value.clone()));

        prop_assert_eq!(map.get(&attr), Some(value.as_str()));
    }

    /// After an arbitrary sequence of `set` operations, `attrs()` returns
    /// entries sorted by key. This invariant is load-bearing: `set`, `get`,
    /// and `contains` all use `binary_search_by` and would silently misbehave
    /// if the internal Vec ever drifted out of order.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_attr_map_keeps_keys_sorted(
        ops in prop::collection::vec((single_value_attr(), attr_string_value()), 0..20),
    ) {
        let mut map = AttrMap::new();

        for (attr, value) in ops {
            map.set(attr, AttrValue::String(value));
        }

        let keys = map.attrs().iter().map(|(key, _)| *key).collect::<Vec<_>>();

        let mut sorted = keys.clone();

        sorted.sort();

        prop_assert_eq!(keys, sorted);
    }

    /// Setting `AttrValue::None` on an attribute that was previously set
    /// removes it from the map, and a subsequent `get` returns `None`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_attr_map_set_none_removes_entry(
        attr in single_value_attr(),
        value in attr_string_value(),
    ) {
        let mut map = AttrMap::new();

        map.set(attr, AttrValue::String(value));

        prop_assert!(map.contains(&attr));

        map.set(attr, AttrValue::None);

        prop_assert!(!map.contains(&attr));
        prop_assert_eq!(map.get(&attr), None);
    }

    /// `AttrValue::Reactive` round-trips through the map: the closure is
    /// preserved (Arc-shared) and `materialize_string()` produces exactly
    /// the value the closure returns. `as_str()` deliberately returns
    /// `None` for reactive variants — the borrow contract requires a
    /// static value, and the closure owns its returned `String`.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_attr_map_reactive_round_trip(
        attr in single_value_attr(),
        value in attr_string_value(),
    ) {
        let mut map = AttrMap::new();

        let captured = value.clone();

        map.set(attr, AttrValue::reactive(move || captured.clone()));

        // `get` (which calls `as_str`) must return `None` for reactive
        // variants: borrowing isn't possible without owning the produced
        // String.
        prop_assert_eq!(map.get(&attr), None);

        // `materialize_string` invokes the closure and returns the
        // produced value verbatim.
        let materialized = map
            .get_value(&attr)
            .and_then(AttrValue::materialize_string);

        prop_assert_eq!(materialized, Some(value));
    }

    /// `AttrValue::ReactiveBool` follows HTML presence semantics: `true`
    /// materializes to an empty string (attribute present), `false`
    /// materializes to `None` (attribute absent), symmetric with the
    /// static `AttrValue::Bool` path.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_attr_map_reactive_bool_presence_semantics(
        attr in single_value_attr(),
        flag in any::<bool>(),
    ) {
        let mut map = AttrMap::new();

        map.set(attr, AttrValue::reactive_bool(move || flag));

        let materialized = map
            .get_value(&attr)
            .and_then(AttrValue::materialize_string);

        if flag {
            prop_assert_eq!(materialized, Some(String::new()));
        } else {
            prop_assert_eq!(materialized, None);
        }
    }

    /// Cloning an `AttrMap` keeps reactive entries `Arc`-shared with the
    /// original — `PartialEq` on `AttrValue::Reactive` uses `Arc::ptr_eq`
    /// so cloned maps compare equal even though the inner closure can't
    /// be compared structurally.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_attr_map_clone_preserves_reactive_arc_identity(
        attr in single_value_attr(),
        value in attr_string_value(),
    ) {
        let mut map = AttrMap::new();

        let captured = value.clone();

        map.set(attr, AttrValue::reactive(move || captured.clone()));

        let cloned = map.clone();

        prop_assert_eq!(map, cloned);
    }

    /// The sorted-keys invariant holds across mixed static and reactive
    /// values — `set` uses `binary_search_by` independent of variant, so
    /// reactive entries must not perturb ordering.
    #[test]
    #[ignore = "proptest — nightly extended-proptest job"]
    fn proptest_attr_map_keeps_keys_sorted_with_mixed_variants(
        ops in prop::collection::vec(
            (single_value_attr(), attr_string_value(), any::<bool>()),
            0..20,
        ),
    ) {
        let mut map = AttrMap::new();

        for (attr, value, use_reactive) in ops {
            if use_reactive {
                let captured = value.clone();
                map.set(attr, AttrValue::reactive(move || captured.clone()));
            } else {
                map.set(attr, AttrValue::String(value));
            }
        }

        let keys = map.attrs().iter().map(|(key, _)| *key).collect::<Vec<_>>();

        let mut sorted = keys.clone();

        sorted.sort();

        prop_assert_eq!(keys, sorted);
    }
}
