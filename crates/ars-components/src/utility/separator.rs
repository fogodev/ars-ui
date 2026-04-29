//! `Separator` component machine and connect API.
//!
//! `Separator` is a stateless attribute mapper that renders a horizontal or
//! vertical dividing line. It has no state machine — the framework-agnostic
//! core consists solely of `Props`, `Part`, and `Api`, which produces the
//! appropriate ARIA role and orientation attributes for the root element.
//!
//! See `spec/components/utility/separator.md` for the authoritative contract.

use alloc::string::String;

use ars_core::{AriaAttr, AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr};
use ars_i18n::Orientation;

/// Props for the `Separator` component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// The orientation of the separator. Defaults to `Orientation::Horizontal`
    /// via `Orientation`'s own `Default` impl.
    pub orientation: Orientation,

    /// Whether the separator is purely decorative and hidden from the
    /// accessibility tree.
    pub decorative: bool,
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

    /// Sets the layout orientation of the separator.
    #[must_use]
    pub const fn orientation(mut self, value: Orientation) -> Self {
        self.orientation = value;
        self
    }

    /// Sets whether the separator is purely decorative (hidden from the
    /// accessibility tree).
    #[must_use]
    pub const fn decorative(mut self, value: bool) -> Self {
        self.decorative = value;
        self
    }
}

/// DOM parts of the `Separator` component.
#[derive(ComponentPart)]
#[scope = "separator"]
pub enum Part {
    /// The root element. See spec §2.1 for adapter element-type selection
    /// (`<hr>` for content separators, `<div>` for menu/toolbar/listbox).
    Root,
}

/// The API for the `Separator` component.
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
    /// accessors (`id`, `orientation`, `decorative`); this method is the
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

    /// Returns the layout orientation of the separator.
    #[must_use]
    pub const fn orientation(&self) -> Orientation {
        self.props.orientation
    }

    /// Returns whether the separator is purely decorative.
    #[must_use]
    pub const fn decorative(&self) -> bool {
        self.props.decorative
    }

    /// Returns the attributes for the root element.
    ///
    /// Semantic separators get `role="separator"` plus `aria-orientation`
    /// matching the layout axis, and `data-ars-orientation` for styling.
    /// Decorative separators get `role="none"` (the modern WAI-ARIA 1.2
    /// preferred form, synonymous with `role="presentation"`); they omit
    /// `aria-orientation` and the `data-ars-orientation` styling hook,
    /// because a decorative separator is invisible to assistive technology
    /// and component-managed orientation styling is not applied.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.props.decorative {
            // Decorative separators are removed from the accessibility tree
            // via `role="none"`. No `aria-hidden` (redundant for an element
            // with no children) and no `data-ars-orientation` (decorative
            // separators do not participate in component orientation styling).
            attrs.set(HtmlAttr::Role, "none");
        } else {
            let orientation_str = match self.props.orientation {
                Orientation::Horizontal => "horizontal",
                Orientation::Vertical => "vertical",
            };

            attrs
                .set(HtmlAttr::Data("ars-orientation"), orientation_str)
                .set(HtmlAttr::Role, "separator")
                .set(HtmlAttr::Aria(AriaAttr::Orientation), orientation_str);
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
    use ars_core::HasId;
    use insta::assert_snapshot;

    use super::*;

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    fn horizontal() -> Props {
        Props::default()
    }

    fn vertical() -> Props {
        Props {
            orientation: Orientation::Vertical,
            ..Props::default()
        }
    }

    fn decorative_horizontal() -> Props {
        Props {
            decorative: true,
            ..Props::default()
        }
    }

    fn decorative_vertical() -> Props {
        Props {
            orientation: Orientation::Vertical,
            decorative: true,
            ..Props::default()
        }
    }

    // ── Props ──────────────────────────────────────────────────────

    #[test]
    fn props_default_horizontal_non_decorative() {
        let p = Props::default();

        assert_eq!(p.id, "");
        assert_eq!(p.orientation, Orientation::Horizontal);
        assert!(!p.decorative);
    }

    #[test]
    fn props_builder_round_trips() {
        // `Props::new()` returns the documented defaults and the chained
        // setters mutate exactly the matching fields, leaving the others
        // at their default values.
        let p = Props::new()
            .id("sep-build")
            .orientation(Orientation::Vertical)
            .decorative(true);

        assert_eq!(p.id, "sep-build");
        assert_eq!(p.orientation, Orientation::Vertical);
        assert!(p.decorative);

        // `Props::new()` is equivalent to `Default::default()`.
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_setters_are_idempotent_per_field() {
        // Each setter overrides only its own field; later calls overwrite
        // earlier ones for the same field.
        let p = Props::new()
            .decorative(true)
            .decorative(false)
            .orientation(Orientation::Vertical);

        assert!(!p.decorative);
        assert_eq!(p.orientation, Orientation::Vertical);
    }

    #[test]
    fn props_has_id_derive_round_trips() {
        // Exercises the methods the `HasId` derive emits directly on
        // `Props` (the `Api::id()` accessor only goes through one of them).
        let mut p = Props::default().with_id(String::from("sep-1"));
        assert_eq!(HasId::id(&p), "sep-1");

        p.set_id(String::from("sep-2"));
        assert_eq!(HasId::id(&p), "sep-2");
    }

    #[test]
    fn props_clone_and_partial_eq_round_trip() {
        let original = Props {
            id: String::from("sep-clone"),
            orientation: Orientation::Vertical,
            decorative: true,
        };

        let cloned = original.clone();
        assert_eq!(cloned, original);

        let mutated = Props {
            decorative: false,
            ..original.clone()
        };
        assert_ne!(mutated, original);
    }

    #[test]
    fn props_and_api_debug_impl_is_non_empty() {
        // Smoke test guarding against an accidental empty `impl Debug`.
        let api = Api::new(decorative_horizontal());

        let props_dbg = format!("{:?}", api.props());

        let api_dbg = format!("{api:?}");

        assert!(props_dbg.contains("Props"), "Props Debug = {props_dbg}");
        assert!(api_dbg.contains("Api"), "Api Debug = {api_dbg}");
    }

    #[test]
    fn api_id_returns_empty_str_for_default_props() {
        // Direct assertion on the empty-id path through `Api::id()`.
        assert_eq!(Api::new(Props::default()).id(), "");
    }

    // ── Api accessors ──────────────────────────────────────────────

    #[test]
    fn api_exposes_props_fields() {
        let original = Props {
            id: String::from("sep-7"),
            orientation: Orientation::Vertical,
            decorative: true,
        };

        let api = Api::new(original.clone());

        assert_eq!(api.id(), "sep-7");
        assert_eq!(api.orientation(), Orientation::Vertical);
        assert!(api.decorative());
        assert_eq!(api.props(), &original);
    }

    // ── Connect / API ──────────────────────────────────────────────

    #[test]
    fn part_attrs_dispatches_root() {
        let api = Api::new(horizontal());

        let attrs = api.part_attrs(Part::Root);

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("separator"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    #[test]
    fn part_attrs_root_equals_root_attrs() {
        // The `ConnectApi` dispatch must produce exactly what the inherent
        // `root_attrs` method produces. Asserted across every output-affecting
        // prop combination (orientation × decorative).
        for props in [
            horizontal(),
            vertical(),
            decorative_horizontal(),
            decorative_vertical(),
        ] {
            let api = Api::new(props);

            assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        }
    }

    #[test]
    fn root_horizontal_has_separator_role_and_orientation() {
        let attrs = Api::new(horizontal()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("separator"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("horizontal")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-orientation")),
            Some("horizontal")
        );
    }

    #[test]
    fn root_vertical_has_aria_orientation_vertical() {
        let attrs = Api::new(vertical()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("separator"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)),
            Some("vertical")
        );
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-orientation")),
            Some("vertical")
        );
    }

    #[test]
    fn decorative_uses_none_role_and_omits_aria_attrs() {
        let attrs = Api::new(decorative_horizontal()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("none"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Hidden)), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Orientation)), None);
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-orientation")), None);
    }

    #[test]
    fn decorative_orientation_does_not_change_root_attrs() {
        // Defensive regression test: orientation is invisible to AT for
        // decorative separators, and decorative separators do not opt into
        // component orientation styling. Both decorative variants must
        // produce the same `AttrMap`.
        let h = Api::new(decorative_horizontal()).root_attrs();
        let v = Api::new(decorative_vertical()).root_attrs();

        assert_eq!(h, v);
    }

    #[test]
    fn decorative_and_semantic_branches_produce_different_attrs() {
        // Defensive cross-branch inequality: a regression that conflates
        // the two role tokens (e.g. accidentally emitting `role="separator"`
        // for decorative, or reusing the semantic AttrMap entirely) must
        // not pass silently.
        let semantic = Api::new(horizontal()).root_attrs();
        let decorative = Api::new(decorative_horizontal()).root_attrs();

        assert_ne!(semantic, decorative);
    }

    #[test]
    fn horizontal_and_vertical_semantic_branches_produce_different_attrs() {
        // Defensive cross-axis inequality: orientation MUST change the
        // `aria-orientation` and `data-ars-orientation` values on the
        // semantic path. Catches a regression where the orientation
        // mapping accidentally emits the same value for both axes.
        let h = Api::new(horizontal()).root_attrs();
        let v = Api::new(vertical()).root_attrs();

        assert_ne!(h, v);
    }

    // ── Snapshots ──────────────────────────────────────────────────

    #[test]
    fn separator_root_horizontal_snapshot() {
        assert_snapshot!(
            "separator_root_horizontal",
            snapshot_attrs(&Api::new(horizontal()).root_attrs())
        );
    }

    #[test]
    fn separator_root_vertical_snapshot() {
        assert_snapshot!(
            "separator_root_vertical",
            snapshot_attrs(&Api::new(vertical()).root_attrs())
        );
    }

    #[test]
    fn separator_root_decorative_snapshot() {
        // Decorative separators emit the same AttrMap regardless of
        // orientation, so a single snapshot covers both decorative branches.
        assert_snapshot!(
            "separator_root_decorative",
            snapshot_attrs(&Api::new(decorative_horizontal()).root_attrs())
        );
    }
}
