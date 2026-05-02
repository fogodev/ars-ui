//! `VisuallyHidden` component machine and connect API.
//!
//! `VisuallyHidden` is a stateless attribute mapper that renders content which
//! is invisible on screen but fully accessible to screen readers. It has no
//! state machine — the framework-agnostic core consists solely of `Props`,
//! `Part`, and `Api`, which produces either the always-hidden CSS class or
//! the focus-visible data hook on the root element.
//!
//! See `spec/components/utility/visually-hidden.md` for the authoritative
//! contract.

use alloc::string::String;

use ars_core::{AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr};

/// Props for the `VisuallyHidden` component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// When `true`, renders the visually-hidden styles onto the single child
    /// element rather than wrapping it in a `<span>`. The flag is read by
    /// the framework adapter to choose the render path; the agnostic-core
    /// attribute output is invariant under this flag.
    pub as_child: bool,

    /// When `true`, the element becomes visible when it receives focus.
    /// Enables skip-link patterns where hidden navigation aids appear on
    /// focus. Default: `false`.
    pub is_focusable: bool,
}

impl Props {
    /// Returns fresh [`Props`] with the documented defaults — equivalent
    /// to [`Default::default`], offered as the entry point for the
    /// builder chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the component instance ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets whether the agnostic core renders its attributes onto a
    /// consumer-provided child element (`as_child` pattern) instead of
    /// wrapping content in a `<span>`. Adapter-only flag — the
    /// agnostic-core attribute output is invariant under this value.
    #[must_use]
    pub const fn as_child(mut self, value: bool) -> Self {
        self.as_child = value;
        self
    }

    /// Sets whether the element becomes visible when it receives focus
    /// (skip-link / focus-reveal mode).
    #[must_use]
    pub const fn is_focusable(mut self, value: bool) -> Self {
        self.is_focusable = value;
        self
    }
}

/// DOM parts of the `VisuallyHidden` component.
#[derive(ComponentPart)]
#[scope = "visually-hidden"]
pub enum Part {
    /// The root element. Adapters render `<span>` by default, or apply the
    /// component attributes onto the single consumer-provided child element
    /// when [`Props::as_child`] is `true`.
    Root,
}

/// The API for the `VisuallyHidden` component.
///
/// Constructed via [`Api::new`] from [`Props`] and queried via
/// [`Api::root_attrs`] or the [`ConnectApi`] dispatch.
#[derive(Clone, Debug)]
pub struct Api {
    props: Props,
}

impl Api {
    /// Creates a new `Api` instance from the given props.
    #[must_use]
    pub const fn new(props: Props) -> Self {
        Self { props }
    }

    /// Returns a reference to the underlying [`Props`].
    ///
    /// Adapters typically read individual fields through the dedicated
    /// accessors (`id`, `as_child`, `is_focusable`); this method is the
    /// escape hatch for when the full struct is needed (e.g., to clone it
    /// into a fresh [`Api`] for a re-render).
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Returns the component's instance ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.props.id
    }

    /// Returns whether the component is rendered onto a consumer-provided
    /// child element instead of the default `<span>`.
    #[must_use]
    pub const fn as_child(&self) -> bool {
        self.props.as_child
    }

    /// Returns whether the element becomes visible on focus (skip-link mode).
    #[must_use]
    pub const fn is_focusable(&self) -> bool {
        self.props.is_focusable
    }

    /// Returns the attributes for the root element.
    ///
    /// Applies the `ars-visually-hidden` class for the always-hidden variant,
    /// or the `data-ars-visually-hidden-focusable` data hook for the
    /// focus-visible variant. The two paths are mutually exclusive: the
    /// focusable variant must not also include the class because that class
    /// clips unconditionally and would prevent the element from becoming
    /// visible on focus (see spec §4).
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.props.is_focusable {
            // Element is visible when focused, hidden otherwise. Do not also
            // set `ars-visually-hidden`; that class hides unconditionally
            // and would break the focus-visible behavior.
            attrs.set_bool(HtmlAttr::Data("ars-visually-hidden-focusable"), true);
        } else {
            attrs.set(HtmlAttr::Class, "ars-visually-hidden");
        }

        attrs
    }
}

impl ConnectApi for Api {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use ars_core::{AttrValue, HasId};
    use insta::assert_snapshot;

    use super::*;

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn default_props() -> Props {
        Props::default()
    }

    fn focusable_props() -> Props {
        Props {
            is_focusable: true,
            ..Props::default()
        }
    }

    fn as_child_props() -> Props {
        Props {
            as_child: true,
            ..Props::default()
        }
    }

    // ── Props ──────────────────────────────────────────────────────

    #[test]
    fn props_default_values() {
        let p = Props::default();

        assert_eq!(p.id, "");
        assert!(!p.as_child);
        assert!(!p.is_focusable);
    }

    #[test]
    fn props_builder_round_trips() {
        // `Props::new()` returns the documented defaults and the chained
        // setters mutate exactly the matching fields, leaving the others
        // at their default values.
        let p = Props::new()
            .id("vh-build")
            .as_child(true)
            .is_focusable(true);

        assert_eq!(p.id, "vh-build");
        assert!(p.as_child);
        assert!(p.is_focusable);

        // `Props::new()` is equivalent to `Default::default()`.
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_setters_are_idempotent_per_field() {
        // Each setter overrides only its own field; later calls overwrite
        // earlier ones for the same field.
        let p = Props::new()
            .is_focusable(true)
            .is_focusable(false)
            .as_child(true);

        assert!(!p.is_focusable);
        assert!(p.as_child);
    }

    #[test]
    fn props_has_id_derive_round_trips() {
        // Exercises the methods the `HasId` derive emits directly on
        // `Props` (the `Api::id()` accessor only goes through one of them).
        let mut p = Props::default().with_id(String::from("vh-1"));

        assert_eq!(HasId::id(&p), "vh-1");

        p.set_id(String::from("vh-2"));

        assert_eq!(HasId::id(&p), "vh-2");
    }

    #[test]
    fn props_clone_and_partial_eq_round_trip() {
        let original = Props {
            id: String::from("vh-clone"),
            as_child: true,
            is_focusable: true,
        };

        // Clone must be structurally equal to the source.
        let cloned = original.clone();

        assert_eq!(cloned, original);

        // PartialEq inequality detects field changes.
        let mutated = Props {
            is_focusable: false,
            ..original.clone()
        };

        assert_ne!(mutated, original);
    }

    #[test]
    fn props_and_api_debug_impl_is_non_empty() {
        // Smoke test guarding against an accidental empty `impl Debug`.
        let api = Api::new(focusable_props());

        let props_dbg = format!("{:?}", api.props());

        let api_dbg = format!("{api:?}");

        assert!(props_dbg.contains("Props"), "Props Debug = {props_dbg}");
        assert!(api_dbg.contains("Api"), "Api Debug = {api_dbg}");
    }

    #[test]
    fn api_id_returns_empty_str_for_default_props() {
        // Direct assertion that the empty-id path through `Api::id()`
        // produces `""` rather than e.g. panicking or returning a sentinel.
        assert_eq!(Api::new(Props::default()).id(), "");
    }

    // ── Api accessors ──────────────────────────────────────────────

    #[test]
    fn api_exposes_props_fields() {
        let original = Props {
            id: String::from("vh-7"),
            as_child: true,
            is_focusable: true,
        };

        let api = Api::new(original.clone());

        assert_eq!(api.id(), "vh-7");
        assert!(api.as_child());
        assert!(api.is_focusable());
        assert_eq!(api.props(), &original);

        let default = Api::new(default_props());

        assert!(!default.as_child());
        assert!(!default.is_focusable());
    }

    // ── Connect / API ──────────────────────────────────────────────

    #[test]
    fn part_attrs_dispatches_root() {
        let api = Api::new(default_props());

        let attrs = api.part_attrs(Part::Root);

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("visually-hidden")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    #[test]
    fn part_attrs_root_equals_root_attrs() {
        // The `ConnectApi` dispatch must produce exactly what the inherent
        // `root_attrs` method produces. Asserted across every output-affecting
        // prop combination.
        for props in [default_props(), focusable_props(), as_child_props()] {
            let api = Api::new(props);

            assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        }
    }

    #[test]
    fn root_default_has_visually_hidden_class() {
        let attrs = Api::new(default_props()).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-scope")),
            Some("visually-hidden")
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
        assert_eq!(attrs.get(&HtmlAttr::Class), Some("ars-visually-hidden"));
        assert!(!attrs.contains(&HtmlAttr::Data("ars-visually-hidden-focusable")));
    }

    #[test]
    fn root_focusable_uses_data_hook_not_class() {
        let attrs = Api::new(focusable_props()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Class), None);
        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-visually-hidden-focusable")),
            Some(&AttrValue::Bool(true))
        );
    }

    #[test]
    fn as_child_does_not_change_root_attrs() {
        // Defensive regression test: `as_child` is an adapter render-path
        // flag and must NOT influence agnostic-core attribute output.
        let baseline = Api::new(default_props()).root_attrs();
        let with_as_child = Api::new(as_child_props()).root_attrs();

        assert_eq!(baseline, with_as_child);
    }

    #[test]
    fn focusable_and_default_branches_produce_different_attrs() {
        // Defensive cross-branch inequality: `is_focusable` is the only
        // output-affecting flag; flipping it MUST change the AttrMap.
        // Catches a regression where the focusable hook accidentally
        // becomes a no-op or where the class is also set on the focusable
        // path (which would silently regress spec §4).
        let default_attrs = Api::new(default_props()).root_attrs();
        let focusable_attrs = Api::new(focusable_props()).root_attrs();

        assert_ne!(default_attrs, focusable_attrs);
    }

    // ── Snapshots ──────────────────────────────────────────────────

    #[test]
    fn visually_hidden_root_default_snapshot() {
        assert_snapshot!(
            "visually_hidden_root_default",
            snapshot_attrs(&Api::new(default_props()).root_attrs())
        );
    }

    #[test]
    fn visually_hidden_root_focusable_snapshot() {
        assert_snapshot!(
            "visually_hidden_root_focusable",
            snapshot_attrs(&Api::new(focusable_props()).root_attrs())
        );
    }
}
