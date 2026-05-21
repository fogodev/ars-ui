//! `Group` component connect API.
//!
//! `Group` is a stateless semantic grouping wrapper. The framework-agnostic
//! core consists of [`Props`], the [`GroupRole`] role selector, the [`Part`]
//! taxonomy, [`Api`], and a [`GroupContext`] struct that adapters publish so
//! descendant components can inherit `disabled`, `invalid`, and `read_only`
//! state without re-implementing the propagation themselves.
//!
//! Unlike [`Fieldset`](super::fieldset), which renders the native
//! `<fieldset>`/`<legend>` pair and is form-specific, `Group` is a lightweight
//! attribute mapper for any related set of controls (a cluster of buttons
//! sharing a disabled state, the input + steppers inside a NumberField, etc.).
//!
//! See `spec/components/utility/group.md` for the authoritative contract.

use alloc::string::String;

use ars_core::{AriaAttr, AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr};
use ars_i18n::Direction;

/// The ARIA role applied to the [`Group`](self) container.
///
/// Defaults to [`GroupRole::Group`], a generic grouping role suitable for
/// related controls. Switch to [`GroupRole::Region`] when the group is a
/// landmark that should appear in a page summary (an accessible name is then
/// required by WAI-ARIA), or to [`GroupRole::Presentation`] to render the
/// container without any grouping semantics while still propagating state via
/// [`GroupContext`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GroupRole {
    /// `role="group"` — a set of related UI elements not important enough to
    /// be included in a page summary or table of contents.
    #[default]
    Group,

    /// `role="region"` — a landmark region that is significant enough to be
    /// listed in a page summary. Requires an accessible name.
    Region,

    /// `role="presentation"` — removes the grouping semantics. Children are
    /// still grouped visually but not semantically.
    Presentation,
}

/// Props for the [`Group`](self) component.
#[derive(Clone, Debug, Default, PartialEq, HasId)]
pub struct Props {
    /// Component instance ID. Always emitted on the root element so adapters
    /// and consumers can reference the container from other attributes
    /// (`aria-controls`, `aria-describedby`, etc.).
    pub id: String,

    /// Whether the group and all contained controls are disabled. Propagated
    /// to descendants via [`GroupContext`] and surfaced on the root as
    /// `aria-disabled="true"` plus `data-ars-disabled` for styling.
    pub disabled: bool,

    /// Whether the group is in an invalid state. Propagated to descendants
    /// via [`GroupContext`] and surfaced on the root as `aria-invalid="true"`
    /// plus `data-ars-invalid` for styling.
    pub invalid: bool,

    /// Whether the group is read-only. Propagated to descendants via
    /// [`GroupContext`] and surfaced on the root as `data-ars-readonly` for
    /// styling. **No `aria-readonly` is emitted on the root** — WAI-ARIA 1.2
    /// does not list `aria-readonly` as supported on `role="group"`,
    /// `role="region"`, or `role="presentation"`, so descendant controls
    /// whose own roles support it apply it themselves after reading
    /// [`GroupContext`]. See [`Api::root_attrs`] for the full rationale.
    pub read_only: bool,

    /// The ARIA role for the group container. Defaults to [`GroupRole::Group`].
    pub role: GroupRole,

    /// Layout direction for RTL support. When `Some(_)`, the resolved value
    /// is forwarded to the root element's `dir` attribute. When `None`, the
    /// element inherits direction from the DOM cascade.
    pub dir: Option<Direction>,
}

impl Props {
    /// Returns fresh [`Props`] with the documented defaults — equivalent to
    /// [`Default::default`], offered as the entry point for the builder chain.
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

    /// Sets whether the group is disabled. Disabled state propagates to
    /// descendants through [`GroupContext`].
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets whether the group is invalid. Invalid state propagates to
    /// descendants through [`GroupContext`].
    #[must_use]
    pub const fn invalid(mut self, value: bool) -> Self {
        self.invalid = value;
        self
    }

    /// Sets whether the group is read-only. Read-only state propagates to
    /// descendants through [`GroupContext`].
    #[must_use]
    pub const fn read_only(mut self, value: bool) -> Self {
        self.read_only = value;
        self
    }

    /// Sets the ARIA role of the group container.
    #[must_use]
    pub const fn role(mut self, value: GroupRole) -> Self {
        self.role = value;
        self
    }

    /// Sets the layout direction. Pass `Some(direction)` to forward an
    /// explicit `dir` attribute, or `None` to inherit from the DOM cascade.
    #[must_use]
    pub const fn dir(mut self, value: Option<Direction>) -> Self {
        self.dir = value;
        self
    }
}

/// Context published by [`Group`](self) so descendant components can inherit
/// `disabled`, `invalid`, and `read_only` state without re-implementing the
/// propagation themselves.
///
/// Adapters MUST provide this value through their framework's context system
/// (`provide_context` in Leptos, `use_context_provider` in Dioxus). Descendant
/// components that participate in inheritance should merge each field with
/// their own props using a logical OR — a component is disabled if either it
/// or its containing group is disabled.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GroupContext {
    /// Whether the containing group is disabled.
    pub disabled: bool,

    /// Whether the containing group is in an invalid state.
    pub invalid: bool,

    /// Whether the containing group is read-only.
    pub read_only: bool,
}

/// DOM parts of the [`Group`](self) component.
#[derive(ComponentPart)]
#[scope = "group"]
pub enum Part {
    /// The root container element. Renders as a `<div>` carrying the role,
    /// state attributes, and propagation context.
    Root,
}

/// The API for the [`Group`](self) component.
///
/// Constructed via [`Api::new`] from [`Props`] and queried via
/// [`Api::root_attrs`], [`Api::group_context`], or the [`ConnectApi`] dispatch.
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
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Returns the component's instance ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.props.id
    }

    /// Returns the [`GroupContext`] that adapters should publish to
    /// descendants so they can inherit the group's disabled / invalid /
    /// read-only state.
    #[must_use]
    pub const fn group_context(&self) -> GroupContext {
        GroupContext {
            disabled: self.props.disabled,
            invalid: self.props.invalid,
            read_only: self.props.read_only,
        }
    }

    /// Returns the attributes for the root element.
    ///
    /// Always emits the instance `id`, the scope/part data attributes, and a
    /// `role` derived from [`Props::role`].
    ///
    /// State attributes split along the WAI-ARIA 1.2 global / role-supported
    /// boundary:
    ///
    /// - `aria-disabled` and `aria-invalid` are WAI-ARIA 1.2 §6.5 *global*
    ///   states — supported on every role, including `role="presentation"`.
    ///   They emit on the root whenever the corresponding prop is `true`.
    /// - `aria-readonly` is **not** global — WAI-ARIA 1.2 only lists it as
    ///   supported on roles such as `checkbox`, `textbox`, `combobox`, `grid`,
    ///   `radiogroup`, `slider`, `spinbutton`, and `switch`. None of
    ///   [`GroupRole`]'s variants (`group`, `region`, `presentation`) appear
    ///   in that set, so emitting `aria-readonly` here would be invalid ARIA.
    ///   The read-only state still reaches descendants through
    ///   [`GroupContext`], and the `data-ars-readonly` styling hook still
    ///   emits so CSS targeting remains symmetric with the other states.
    ///   Descendant controls whose own roles support `aria-readonly` apply
    ///   it themselves after reading the context.
    ///
    /// The three `data-ars-*` styling hooks always emit when their
    /// corresponding prop is `true` so that CSS / theming code can target
    /// every state uniformly, independent of the ARIA validity constraints.
    ///
    /// When [`Props::dir`] is `Some`, the resolved direction is forwarded to
    /// the `dir` attribute.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, &self.props.id)
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        let role_str = match self.props.role {
            GroupRole::Group => "group",
            GroupRole::Region => "region",
            GroupRole::Presentation => "presentation",
        };

        attrs.set(HtmlAttr::Role, role_str);

        if self.props.disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if self.props.invalid {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Invalid), "true")
                .set_bool(HtmlAttr::Data("ars-invalid"), true);
        }

        if self.props.read_only {
            // No `aria-readonly` here — WAI-ARIA 1.2 does not list it as
            // supported on `role="group"`, `role="region"`, or
            // `role="presentation"`. Read-only state propagates through
            // `GroupContext` to descendant controls whose own roles do
            // support it. See the doc comment on `root_attrs` for the
            // full rationale.
            attrs.set_bool(HtmlAttr::Data("ars-readonly"), true);
        }

        if let Some(dir) = self.props.dir {
            attrs.set(HtmlAttr::Dir, dir.as_html_attr());
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

    fn default_props() -> Props {
        Props::default()
    }

    fn region_props() -> Props {
        Props::new().role(GroupRole::Region)
    }

    fn presentation_props() -> Props {
        Props::new().role(GroupRole::Presentation)
    }

    fn all_states_props() -> Props {
        Props::new().disabled(true).invalid(true).read_only(true)
    }

    fn rtl_props() -> Props {
        Props::new().dir(Some(Direction::Rtl))
    }

    // ── Props ──────────────────────────────────────────────────────

    #[test]
    fn props_default_is_zero_state() {
        let p = Props::default();

        assert_eq!(p.id, "");
        assert!(!p.disabled);
        assert!(!p.invalid);
        assert!(!p.read_only);
        assert_eq!(p.role, GroupRole::Group);
        assert_eq!(p.dir, None);
    }

    #[test]
    fn props_builder_round_trips() {
        let p = Props::new()
            .id("group-build")
            .disabled(true)
            .invalid(true)
            .read_only(true)
            .role(GroupRole::Region)
            .dir(Some(Direction::Rtl));

        assert_eq!(p.id, "group-build");
        assert!(p.disabled);
        assert!(p.invalid);
        assert!(p.read_only);
        assert_eq!(p.role, GroupRole::Region);
        assert_eq!(p.dir, Some(Direction::Rtl));

        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_setters_are_idempotent_per_field() {
        // Each setter only writes its own field, and later calls overwrite
        // earlier ones for the same field.
        let p = Props::new()
            .disabled(true)
            .disabled(false)
            .invalid(true)
            .role(GroupRole::Presentation)
            .role(GroupRole::Region);

        assert!(!p.disabled);
        assert!(p.invalid);
        assert_eq!(p.role, GroupRole::Region);
    }

    #[test]
    fn props_has_id_derive_round_trips() {
        let mut p = Props::default().with_id(String::from("group-1"));

        assert_eq!(HasId::id(&p), "group-1");

        p.set_id(String::from("group-2"));

        assert_eq!(HasId::id(&p), "group-2");
    }

    #[test]
    fn props_clone_and_partial_eq_round_trip() {
        let original = Props {
            id: String::from("group-clone"),
            disabled: true,
            invalid: false,
            read_only: true,
            role: GroupRole::Region,
            dir: Some(Direction::Rtl),
        };

        let cloned = original.clone();

        assert_eq!(cloned, original);

        let mutated = Props {
            disabled: false,
            ..original.clone()
        };

        assert_ne!(mutated, original);
    }

    #[test]
    fn group_role_default_is_group() {
        assert_eq!(GroupRole::default(), GroupRole::Group);
    }

    #[test]
    fn props_and_api_debug_impl_is_non_empty() {
        let api = Api::new(all_states_props());

        let props_dbg = format!("{:?}", api.props());
        let api_dbg = format!("{api:?}");

        assert!(props_dbg.contains("Props"), "Props Debug = {props_dbg}");
        assert!(api_dbg.contains("Api"), "Api Debug = {api_dbg}");
    }

    // ── Api accessors ──────────────────────────────────────────────

    #[test]
    fn api_id_returns_empty_str_for_default_props() {
        assert_eq!(Api::new(Props::default()).id(), "");
    }

    #[test]
    fn api_exposes_props_fields() {
        let original = Props {
            id: String::from("group-7"),
            disabled: true,
            invalid: false,
            read_only: true,
            role: GroupRole::Region,
            dir: Some(Direction::Ltr),
        };

        let api = Api::new(original.clone());

        assert_eq!(api.id(), "group-7");
        assert_eq!(api.props(), &original);
    }

    // ── Role mapping ──────────────────────────────────────────────

    #[test]
    fn root_default_role_is_group() {
        let attrs = Api::new(default_props()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("group"));
    }

    #[test]
    fn root_role_region_overrides() {
        let attrs = Api::new(region_props()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("region"));
    }

    #[test]
    fn root_role_presentation_overrides() {
        let attrs = Api::new(presentation_props()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("presentation"));
    }

    #[test]
    fn role_branches_produce_different_role_attrs() {
        // Defensive cross-branch inequality: catches a regression that
        // accidentally collapses the role mapping (e.g. all variants emitting
        // the same string).
        let group_attrs = Api::new(default_props()).root_attrs();
        let region_attrs = Api::new(region_props()).root_attrs();
        let presentation_attrs = Api::new(presentation_props()).root_attrs();

        assert_ne!(group_attrs, region_attrs);
        assert_ne!(group_attrs, presentation_attrs);
        assert_ne!(region_attrs, presentation_attrs);
    }

    // ── GroupContext propagation ──────────────────────────────────

    #[test]
    fn group_context_default_is_all_false() {
        let ctx = Api::new(Props::default()).group_context();

        assert_eq!(
            ctx,
            GroupContext {
                disabled: false,
                invalid: false,
                read_only: false,
            }
        );
    }

    #[test]
    fn group_context_propagates_disabled() {
        let ctx = Api::new(Props::new().disabled(true)).group_context();

        assert_eq!(
            ctx,
            GroupContext {
                disabled: true,
                invalid: false,
                read_only: false,
            }
        );
    }

    #[test]
    fn group_context_propagates_invalid() {
        let ctx = Api::new(Props::new().invalid(true)).group_context();

        assert_eq!(
            ctx,
            GroupContext {
                disabled: false,
                invalid: true,
                read_only: false,
            }
        );
    }

    #[test]
    fn group_context_propagates_read_only() {
        let ctx = Api::new(Props::new().read_only(true)).group_context();

        assert_eq!(
            ctx,
            GroupContext {
                disabled: false,
                invalid: false,
                read_only: true,
            }
        );
    }

    #[test]
    fn group_context_propagates_all_flags_together() {
        let ctx = Api::new(all_states_props()).group_context();

        assert_eq!(
            ctx,
            GroupContext {
                disabled: true,
                invalid: true,
                read_only: true,
            }
        );
    }

    #[test]
    fn group_context_is_independent_of_role() {
        // Role does not gate context propagation — descendants of a
        // `role="presentation"` group still see the disabled state.
        let ctx_group = Api::new(Props::new().disabled(true)).group_context();

        let ctx_region =
            Api::new(Props::new().disabled(true).role(GroupRole::Region)).group_context();

        let ctx_presentation =
            Api::new(Props::new().disabled(true).role(GroupRole::Presentation)).group_context();

        assert_eq!(ctx_group, ctx_region);
        assert_eq!(ctx_group, ctx_presentation);
        assert!(ctx_presentation.disabled);
    }

    // ── Root state attributes ─────────────────────────────────────

    #[test]
    fn root_disabled_emits_aria_and_data_attrs() {
        let attrs = Api::new(Props::new().disabled(true)).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-disabled")), Some("true"));
    }

    #[test]
    fn root_invalid_emits_aria_and_data_attrs() {
        let attrs = Api::new(Props::new().invalid(true)).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-invalid")), Some("true"));
    }

    #[test]
    fn root_read_only_emits_data_attr_but_not_aria_readonly() {
        // `aria-readonly` is NOT a WAI-ARIA 1.2 global state — it is only
        // listed as supported on roles like `checkbox`, `textbox`, `combobox`,
        // `grid`, `radiogroup`, `slider`, `spinbutton`, `switch`, etc.
        // `role="group"` (and `region` / `presentation`) are not in that set,
        // so emitting `aria-readonly` on the root would be invalid ARIA that
        // conformance tools flag and assistive tech may ignore.
        //
        // The read-only state still reaches descendants through
        // `GroupContext`, and the `data-ars-readonly` styling hook stays so
        // CSS targeting works. Descendant controls whose own roles support
        // `aria-readonly` apply it themselves after reading the context.
        let attrs = Api::new(Props::new().read_only(true)).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ReadOnly)),
            None,
            "aria-readonly must not be emitted on role=\"group\"/region/presentation",
        );
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-readonly")), Some("true"));
    }

    #[test]
    fn root_state_attrs_omitted_when_false() {
        let attrs = Api::new(Props::default()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), None);
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::ReadOnly)), None);
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-disabled")), None);
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-invalid")), None);
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-readonly")), None);
    }

    #[test]
    fn root_global_state_attrs_emit_on_presentation_role() {
        // `aria-disabled` and `aria-invalid` are WAI-ARIA 1.2 §6.5 global
        // states — supported on every role, including `role="presentation"`,
        // so they still emit on the root. `aria-readonly` is NOT global
        // (see `root_read_only_emits_data_attr_but_not_aria_readonly`), so
        // it is *not* emitted regardless of role.
        let attrs = Api::new(
            Props::new()
                .disabled(true)
                .invalid(true)
                .read_only(true)
                .role(GroupRole::Presentation),
        )
        .root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("presentation"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Disabled)), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Aria(AriaAttr::Invalid)), Some("true"));
        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::ReadOnly)),
            None,
            "aria-readonly is non-global and must not appear on role=presentation",
        );
        // The data-* styling hooks still emit for all three so CSS targeting
        // remains symmetric.
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-disabled")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-invalid")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-readonly")), Some("true"));
    }

    #[test]
    fn root_dir_emits_dir_attr() {
        let rtl_attrs = Api::new(rtl_props()).root_attrs();

        assert_eq!(rtl_attrs.get(&HtmlAttr::Dir), Some("rtl"));

        let ltr_attrs = Api::new(Props::new().dir(Some(Direction::Ltr))).root_attrs();

        assert_eq!(ltr_attrs.get(&HtmlAttr::Dir), Some("ltr"));

        let auto_attrs = Api::new(Props::new().dir(Some(Direction::Auto))).root_attrs();

        assert_eq!(auto_attrs.get(&HtmlAttr::Dir), Some("auto"));

        let none_attrs = Api::new(Props::default()).root_attrs();

        assert_eq!(none_attrs.get(&HtmlAttr::Dir), None);
    }

    #[test]
    fn root_id_is_set_to_props_id() {
        // The spec's §1.2 code example sets `HtmlAttr::Id` unconditionally,
        // so adapters that consume `root_attrs()` always see a deterministic
        // attribute set; an empty id maps to an empty value.
        let empty = Api::new(Props::default()).root_attrs();

        assert_eq!(empty.get(&HtmlAttr::Id), Some(""));

        let named = Api::new(Props::new().id("my-group")).root_attrs();

        assert_eq!(named.get(&HtmlAttr::Id), Some("my-group"));
    }

    // ── ConnectApi dispatch ───────────────────────────────────────

    #[test]
    fn part_attrs_dispatches_root() {
        let api = Api::new(default_props());

        let attrs = api.part_attrs(Part::Root);

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("group"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    #[test]
    fn part_attrs_root_equals_root_attrs() {
        // Defensive: `ConnectApi` dispatch MUST produce exactly what the
        // inherent `root_attrs` produces across every output-affecting
        // combination of props.
        for role in [GroupRole::Group, GroupRole::Region, GroupRole::Presentation] {
            for disabled in [false, true] {
                for invalid in [false, true] {
                    for read_only in [false, true] {
                        for dir in [None, Some(Direction::Ltr), Some(Direction::Rtl)] {
                            let api = Api::new(Props {
                                id: String::from("dispatch"),
                                disabled,
                                invalid,
                                read_only,
                                role,
                                dir,
                            });

                            assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
                        }
                    }
                }
            }
        }
    }

    // ── Snapshots ─────────────────────────────────────────────────

    #[test]
    fn group_root_default_snapshot() {
        assert_snapshot!(
            "group_root_default",
            snapshot_attrs(&Api::new(default_props()).root_attrs())
        );
    }

    #[test]
    fn group_root_region_snapshot() {
        assert_snapshot!(
            "group_root_region",
            snapshot_attrs(&Api::new(region_props()).root_attrs())
        );
    }

    #[test]
    fn group_root_presentation_snapshot() {
        assert_snapshot!(
            "group_root_presentation",
            snapshot_attrs(&Api::new(presentation_props()).root_attrs())
        );
    }

    #[test]
    fn group_root_all_states_snapshot() {
        assert_snapshot!(
            "group_root_all_states",
            snapshot_attrs(&Api::new(all_states_props()).root_attrs())
        );
    }

    #[test]
    fn group_root_dir_rtl_snapshot() {
        assert_snapshot!(
            "group_root_dir_rtl",
            snapshot_attrs(&Api::new(rtl_props()).root_attrs())
        );
    }
}
