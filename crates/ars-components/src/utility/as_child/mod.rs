//! `as_child` pattern primitives.
//!
//! Components that opt into the `as_child` pattern render their attributes
//! onto a single consumer-provided child element instead of their default
//! DOM element (the Rust analogue of Radix UI `Slot` / Ark UI `asChild`).
//! This module exposes the framework-agnostic core building blocks:
//!
//! - [`Props`] — flag included in any component's `Props` struct.
//! - [`AsChildMerge`] — trait implemented on [`AttrMap`] for merging
//!   component attributes onto a child element's attributes.
//!
//! Event handler composition is **not** part of this module. Handlers are
//! not stored in [`AttrMap`] and are wired by framework adapters via typed
//! handler methods on per-component `Api` structs (see
//! `spec/components/utility/as-child.md` §1.2 and §4).
//!
//! [`AttrMap`]: ars_core::AttrMap

use ars_core::AttrMap;
#[cfg(any(debug_assertions, feature = "debug"))]
use ars_core::HtmlAttr;

/// Flag struct included in any component's `Props` struct that supports the
/// `as_child` pattern.
///
/// Components that opt into the pattern check this flag and, when `true`,
/// merge their root [`AttrMap`] onto the consumer-provided child element via
/// [`AsChildMerge::merge_onto`] instead of rendering their default element.
///
/// [`AttrMap`]: ars_core::AttrMap
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Props {
    /// When `true`, render the component's attributes onto the single child
    /// element rather than the default element.
    pub as_child: bool,
}

impl Props {
    /// Returns a fresh [`Props`] with [`as_child`](Self::as_child) set
    /// to `false` — the default render path.
    ///
    /// Documented entry point for the builder chain.
    #[must_use]
    pub const fn new() -> Self {
        Self { as_child: false }
    }

    /// Sets [`as_child`](Self::as_child) — when `true`, the component
    /// merges its attributes onto the single consumer-provided child
    /// element instead of rendering its own root element.
    #[must_use]
    pub const fn as_child(mut self, value: bool) -> Self {
        self.as_child = value;
        self
    }
}

/// Merges one set of [`AttrMap`] values onto another, combining attributes
/// and styles per the rules in `spec/components/utility/as-child.md` §3.2.
///
/// Implemented on [`AttrMap`] so callers can write
/// `component_attrs.merge_onto(child_attrs)`.
///
/// [`AttrMap`]: ars_core::AttrMap
pub trait AsChildMerge {
    /// Merge `self` (component attributes) onto `other` (child element
    /// attributes), returning the merged map.
    ///
    /// Component attributes take precedence: for normal attributes the
    /// component's value overwrites the child's. Space-separated token-list
    /// attributes (`class`, `aria-labelledby`, `aria-describedby`,
    /// `aria-controls`, `aria-owns`, `aria-flowto`, `aria-details`, `rel`)
    /// are appended with deduplication via [`AttrMap::merge`]. Styles are
    /// merged per CSS property, with the component's value winning on
    /// conflict.
    ///
    /// A development warning is emitted when the component's `role` differs
    /// from a `role` already present on the child, mirroring the development
    /// warning called out in `spec/components/utility/as-child.md` §3.2 rule 1.
    /// The warning compiles in under `cfg(any(debug_assertions, feature =
    /// "debug"))` and is routed via `log::warn!` when `feature = "debug"` is
    /// enabled, otherwise via `eprintln!` on native dev builds (matching the
    /// non-wasm branch of `leptos::logging::console_debug_warn`). Browser
    /// console visibility on wasm dev builds without `feature = "debug"` is
    /// delegated to the framework adapters.
    ///
    /// [`AttrMap`]: ars_core::AttrMap
    /// [`AttrMap::merge`]: ars_core::AttrMap::merge
    fn merge_onto(self, other: AttrMap) -> AttrMap;
}

impl AsChildMerge for AttrMap {
    fn merge_onto(self, other: AttrMap) -> AttrMap {
        #[cfg(any(debug_assertions, feature = "debug"))]
        warn_role_conflict(&self, &other);

        let mut result = other;

        // Component attributes take precedence. `AttrMap::merge` calls `set`
        // per attribute, which automatically appends space-separated token
        // lists (`class`, `aria-labelledby`, `aria-describedby`, `rel`,
        // `aria-controls`, `aria-owns`, `aria-flowto`, `aria-details`) with
        // deduplication, and overwrites every other attribute. Styles merge
        // last-write-wins per CSS property, with the component winning.
        result.merge(self);

        result
    }
}

/// Emits a development warning when the component's `role` differs from a
/// `role` already present on the child element.
///
/// Mirrors the development warning called out in
/// `spec/components/utility/as-child.md` §3.2 rule 1. Compiled in under
/// `cfg(any(debug_assertions, feature = "debug"))` and emits via one of:
///
/// - `log::warn!` when `feature = "debug"` is enabled (works on native + wasm
///   when the consumer wires a `log` subscriber; this is the structured
///   diagnostic path used by the rest of the workspace).
/// - `eprintln!` when only `debug_assertions` is on, the `std` feature is
///   active, and the target is **not browser wasm**. The `not all(target_arch
///   = "wasm32", not(any(target_os = "emscripten", target_os = "wasi")))`
///   guard mirrors the predicate used by `leptos::logging::console_debug_warn`
///   to decide between `eprintln!` and `web_sys::console::warn_1`. Browser
///   wasm targets (`wasm32-unknown-unknown`) intentionally compile out the
///   fallback so adapters surface the warning via their own
///   `web_sys::console` plumbing instead.
///
/// On wasm dev builds without `feature = "debug"` the warning is intentionally
/// silent at the agnostic-core layer — `ars-components` cannot pull in
/// `web_sys`. Framework adapters (`ars-leptos`, `ars-dioxus`) are expected to
/// surface the warning in the browser console themselves.
#[cfg(any(debug_assertions, feature = "debug"))]
fn warn_role_conflict(component: &AttrMap, child: &AttrMap) {
    if let (Some(component_role), Some(child_role)) =
        (component.get(&HtmlAttr::Role), child.get(&HtmlAttr::Role))
        && component_role != child_role
    {
        #[cfg(feature = "debug")]
        log::warn!(
            "as_child: overriding child role '{child_role}' with component role '{component_role}'"
        );

        #[cfg(all(
            debug_assertions,
            not(feature = "debug"),
            feature = "std",
            not(all(
                target_arch = "wasm32",
                not(any(target_os = "emscripten", target_os = "wasi"))
            ))
        ))]
        eprintln!(
            "as_child: overriding child role '{child_role}' with component role '{component_role}'"
        );
    }
}

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec::Vec};

    use ars_core::{AriaAttr, AttrValue, CssProperty, HtmlAttr};
    use insta::assert_snapshot;

    use super::*;

    /// Construct an `AttrMap` with the given role.
    fn role(value: &str) -> AttrMap {
        let mut attrs = AttrMap::new();

        attrs.set(HtmlAttr::Role, value.to_string());

        attrs
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn props_default_is_not_as_child() {
        assert_eq!(Props::default(), Props { as_child: false });
    }

    #[test]
    fn merge_empty_maps_returns_empty_map() {
        let merged = AttrMap::new().merge_onto(AttrMap::new());

        assert_eq!(merged, AttrMap::new());
    }

    #[test]
    fn merge_role_conflict_component_wins() {
        let child = role("link");
        let component = role("button");

        let merged = component.merge_onto(child);

        assert_eq!(merged.get(&HtmlAttr::Role), Some("button"));
    }

    /// Exercises the comparison-FALSE branch in `warn_role_conflict`: both
    /// sides carry a `role` and they match, so no development warning fires.
    #[test]
    fn merge_role_match_keeps_role_without_warning() {
        let merged = role("button").merge_onto(role("button"));

        assert_eq!(merged.get(&HtmlAttr::Role), Some("button"));
    }

    /// Exercises the if-let-FALSE branch in `warn_role_conflict` when only
    /// the component has a `role`. The merged map keeps the component's
    /// role and any child-only attributes.
    #[test]
    fn merge_with_only_component_role_keeps_both() {
        let component = role("button");

        let mut child = AttrMap::new();

        child.set(HtmlAttr::Class, "wrapper");

        let merged = component.merge_onto(child);

        assert_eq!(merged.get(&HtmlAttr::Role), Some("button"));

        let class = merged.get(&HtmlAttr::Class).expect("class should be set");

        assert!(
            class.split_whitespace().any(|t| t == "wrapper"),
            "missing 'wrapper' in {class}"
        );
    }

    /// Exercises the if-let-FALSE branch in `warn_role_conflict` when only
    /// the child has a `role`. The merged map keeps the child's role and
    /// any component-only attributes.
    #[test]
    fn merge_with_only_child_role_keeps_both() {
        let mut component = AttrMap::new();

        component.set(HtmlAttr::Class, "wrapper");

        let child = role("link");

        let merged = component.merge_onto(child);

        assert_eq!(merged.get(&HtmlAttr::Role), Some("link"));

        let class = merged.get(&HtmlAttr::Class).expect("class should be set");

        assert!(
            class.split_whitespace().any(|t| t == "wrapper"),
            "missing 'wrapper' in {class}"
        );
    }

    #[test]
    fn merge_aria_describedby_concatenates() {
        let mut child = AttrMap::new();

        child.set(HtmlAttr::Aria(AriaAttr::DescribedBy), "child-hint");

        let mut component = AttrMap::new();

        component.set(HtmlAttr::Aria(AriaAttr::DescribedBy), "component-hint");

        let merged = component.merge_onto(child);

        let value = merged
            .get(&HtmlAttr::Aria(AriaAttr::DescribedBy))
            .expect("aria-describedby should be set");

        let tokens = value.split_whitespace().collect::<Vec<_>>();

        assert!(
            tokens.contains(&"child-hint"),
            "missing child token in {value}"
        );
        assert!(
            tokens.contains(&"component-hint"),
            "missing component token in {value}"
        );
    }

    #[test]
    fn merge_aria_labelledby_concatenates() {
        let mut child = AttrMap::new();

        child.set(HtmlAttr::Aria(AriaAttr::LabelledBy), "child-label");

        let mut component = AttrMap::new();

        component.set(HtmlAttr::Aria(AriaAttr::LabelledBy), "component-label");

        let merged = component.merge_onto(child);

        let value = merged
            .get(&HtmlAttr::Aria(AriaAttr::LabelledBy))
            .expect("aria-labelledby should be set");

        let tokens = value.split_whitespace().collect::<Vec<_>>();

        assert!(
            tokens.contains(&"child-label"),
            "missing child token in {value}"
        );
        assert!(
            tokens.contains(&"component-label"),
            "missing component token in {value}"
        );
    }

    #[test]
    fn merge_class_concatenates_with_dedup() {
        let mut child = AttrMap::new();

        child.set(HtmlAttr::Class, "base hovered");

        let mut component = AttrMap::new();

        component.set(HtmlAttr::Class, "base primary");

        let merged = component.merge_onto(child);

        let class = merged.get(&HtmlAttr::Class).expect("class should be set");

        let tokens = class.split_whitespace().collect::<Vec<_>>();

        assert!(tokens.contains(&"base"), "missing 'base': {class}");
        assert!(tokens.contains(&"hovered"), "missing 'hovered': {class}");
        assert!(tokens.contains(&"primary"), "missing 'primary': {class}");
        assert_eq!(
            tokens.iter().filter(|&&t| t == "base").count(),
            1,
            "duplicate 'base': {class}"
        );
    }

    #[test]
    fn merge_style_component_overrides_child() {
        let mut child = AttrMap::new();

        child.set_style(CssProperty::Color, "red");
        child.set_style(CssProperty::Width, "100px");

        let mut component = AttrMap::new();

        component.set_style(CssProperty::Color, "blue");

        let merged = component.merge_onto(child);

        let color = merged
            .iter_styles()
            .find(|(prop, _)| *prop == CssProperty::Color)
            .map(|(_, value)| value.as_str());

        assert_eq!(color, Some("blue"));

        let width = merged
            .iter_styles()
            .find(|(prop, _)| *prop == CssProperty::Width)
            .map(|(_, value)| value.as_str());

        assert_eq!(width, Some("100px"));
    }

    #[test]
    fn merge_data_ars_component_wins() {
        let mut child = AttrMap::new();

        child
            .set(HtmlAttr::Data("ars-scope"), "custom")
            .set(HtmlAttr::Data("ars-part"), "leaf");

        let mut component = AttrMap::new();

        component
            .set(HtmlAttr::Data("ars-scope"), "button")
            .set(HtmlAttr::Data("ars-part"), "root");

        let merged = component.merge_onto(child);

        assert_eq!(merged.get(&HtmlAttr::Data("ars-scope")), Some("button"));
        assert_eq!(merged.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    #[test]
    fn merge_other_aria_component_wins() {
        let mut child = AttrMap::new();

        child.set_bool(HtmlAttr::Aria(AriaAttr::Expanded), false);

        let mut component = AttrMap::new();

        component.set_bool(HtmlAttr::Aria(AriaAttr::Expanded), true);

        let merged = component.merge_onto(child);

        assert_eq!(
            merged.get(&HtmlAttr::Aria(AriaAttr::Expanded)),
            Some("true")
        );
    }

    #[test]
    fn merge_normal_attr_component_wins() {
        let mut child = AttrMap::new();

        child.set(HtmlAttr::Id, "child-id");

        let mut component = AttrMap::new();

        component.set(HtmlAttr::Id, "component-id");

        let merged = component.merge_onto(child);

        assert_eq!(merged.get(&HtmlAttr::Id), Some("component-id"));
    }

    #[test]
    fn merge_boolean_attr_component_wins() {
        let mut child = AttrMap::new();

        child.set_bool(HtmlAttr::Disabled, true);

        let mut component = AttrMap::new();

        component.set_bool(HtmlAttr::Disabled, false);

        let merged = component.merge_onto(child);

        assert_eq!(
            merged.get_value(&HtmlAttr::Disabled),
            Some(&AttrValue::Bool(false))
        );
    }

    #[test]
    fn merge_token_list_attrs_concatenate_with_dedup() {
        let token_attrs = [
            HtmlAttr::Rel,
            HtmlAttr::Aria(AriaAttr::Controls),
            HtmlAttr::Aria(AriaAttr::Owns),
            HtmlAttr::Aria(AriaAttr::FlowTo),
            HtmlAttr::Aria(AriaAttr::Details),
        ];

        for attr in token_attrs {
            let mut child = AttrMap::new();

            child.set(attr, "shared child-only");

            let mut component = AttrMap::new();

            component.set(attr, "shared component-only");

            let merged = component.merge_onto(child);

            let value = merged.get(&attr).expect("token attr should be set");

            let tokens = value.split_whitespace().collect::<Vec<_>>();

            assert!(tokens.contains(&"shared"), "missing shared in {value}");
            assert!(
                tokens.contains(&"child-only"),
                "missing child-only in {value}"
            );
            assert!(
                tokens.contains(&"component-only"),
                "missing component-only in {value}"
            );
            assert_eq!(
                tokens.iter().filter(|&&token| token == "shared").count(),
                1,
                "duplicate shared token for {attr:?}: {value}"
            );
        }
    }

    #[test]
    fn merge_preserves_child_only_attrs() {
        let mut child = AttrMap::new();

        child.set(HtmlAttr::Id, "consumer-id");

        let component = AttrMap::new();

        let merged = component.merge_onto(child);

        assert_eq!(merged.get(&HtmlAttr::Id), Some("consumer-id"));
    }

    #[test]
    fn merge_preserves_component_only_attrs() {
        let child = AttrMap::new();

        let mut component = AttrMap::new();

        component.set(HtmlAttr::TabIndex, "0");

        let merged = component.merge_onto(child);

        assert_eq!(merged.get(&HtmlAttr::TabIndex), Some("0"));
    }

    // --- Snapshot tests ---

    #[test]
    fn role_conflict() {
        let mut child = AttrMap::new();

        child
            .set(HtmlAttr::Role, "link")
            .set(HtmlAttr::Id, "consumer-link");

        let mut component = AttrMap::new();

        component
            .set(HtmlAttr::Role, "button")
            .set(HtmlAttr::Data("ars-scope"), "button")
            .set(HtmlAttr::Data("ars-part"), "root");

        let merged = component.merge_onto(child);

        assert_snapshot!("role_conflict", snapshot_attrs(&merged));
    }

    #[test]
    fn aria_concatenation() {
        let mut child = AttrMap::new();

        child
            .set(HtmlAttr::Aria(AriaAttr::DescribedBy), "child-hint")
            .set(HtmlAttr::Aria(AriaAttr::LabelledBy), "child-label");

        let mut component = AttrMap::new();

        component
            .set(HtmlAttr::Aria(AriaAttr::DescribedBy), "component-hint")
            .set(HtmlAttr::Aria(AriaAttr::LabelledBy), "component-label");

        let merged = component.merge_onto(child);

        assert_snapshot!("aria_concatenation", snapshot_attrs(&merged));
    }

    #[test]
    fn class_and_style() {
        let mut child = AttrMap::new();

        child
            .set(HtmlAttr::Class, "base hovered")
            .set_style(CssProperty::Color, "red")
            .set_style(CssProperty::Width, "100px");

        let mut component = AttrMap::new();

        component
            .set(HtmlAttr::Class, "base primary")
            .set_style(CssProperty::Color, "blue");

        let merged = component.merge_onto(child);

        assert_snapshot!("class_and_style", snapshot_attrs(&merged));
    }

    // ── Builder tests ──────────────────────────────────────────────

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_as_child_setter() {
        assert!(Props::new().as_child(true).as_child);
        assert!(!Props::new().as_child(false).as_child);
    }
}
