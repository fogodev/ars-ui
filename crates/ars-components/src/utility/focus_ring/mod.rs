//! `FocusRing` component machine and connect API.
//!
//! `FocusRing` is a stateless attribute mapper that exposes a
//! `data-ars-focus-visible` boolean attribute to the consumer's CSS so the
//! focus ring can be styled conditionally based on the input modality
//! (keyboard vs. pointer). It has no state machine — the framework-agnostic
//! core consists solely of [`Props`], [`Context`], [`Part`], and [`Api`].
//!
//! Modality tracking itself is owned by the platform layer
//! (`ars-dom::ModalityManager`) and feeds [`Context::focus_visible`] from
//! outside; this crate only renders the resulting attribute.
//!
//! See `spec/components/utility/focus-ring.md` for the authoritative
//! contract.

use alloc::string::String;

use ars_core::{AttrMap, ComponentPart, ConnectApi, HasId, HtmlAttr};

/// Props for the `FocusRing` component.
#[derive(Clone, Debug, Default, PartialEq, Eq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,

    /// Track focus-within rather than direct focus. When `true`, the adapter
    /// wires a focus-within listener instead of a focus listener and
    /// `Context::focus_visible` becomes active when any descendant — not
    /// only the root — receives keyboard focus.
    pub within: bool,

    /// Optional CSS class to apply when focused by any means. Adapter-only
    /// hint; the agnostic-core attribute output is invariant under this
    /// value.
    pub focus_class: Option<String>,

    /// Optional CSS class to apply only when focused by keyboard. Adapter-only
    /// hint; the agnostic-core attribute output is invariant under this
    /// value.
    pub focus_visible_class: Option<String>,

    /// When `true`, the focus ring is shown even on pointer-initiated focus.
    /// Text inputs conventionally show focus indicators regardless of input
    /// method, since users need to know where they are typing. Adapter-only
    /// hint that influences how the platform layer derives
    /// [`Context::focus_visible`]; the agnostic-core attribute output is
    /// invariant under this flag once `Context` has been resolved. Default:
    /// `false`.
    pub is_text_input: bool,
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

    /// Sets whether the component tracks focus-within instead of direct
    /// focus.
    #[must_use]
    pub const fn within(mut self, value: bool) -> Self {
        self.within = value;
        self
    }

    /// Sets the optional CSS class applied when the element is focused by
    /// any means.
    #[must_use]
    pub fn focus_class(mut self, value: impl Into<String>) -> Self {
        self.focus_class = Some(value.into());
        self
    }

    /// Sets the optional CSS class applied only when the element is focused
    /// by keyboard.
    #[must_use]
    pub fn focus_visible_class(mut self, value: impl Into<String>) -> Self {
        self.focus_visible_class = Some(value.into());
        self
    }

    /// Sets the text-input hint controlling whether the focus ring is shown
    /// for pointer-initiated focus.
    #[must_use]
    pub const fn is_text_input(mut self, value: bool) -> Self {
        self.is_text_input = value;
        self
    }
}

/// The runtime context for the `FocusRing` component, supplied by the
/// adapter from the shared modality tracker.
///
/// Carries only the resolved focus-visible state. The [`Props::within`]
/// flag is the single source of truth for whether the adapter should
/// wire focus-within vs. focus listeners — it does not appear on
/// `Context` because duplicating it is just a chance for the two
/// fields to disagree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Context {
    /// Whether the focus-visible state is currently active.
    pub focus_visible: bool,
}

/// DOM parts of the `FocusRing` component.
#[derive(ComponentPart)]
#[scope = "focus-ring"]
pub enum Part {
    /// The root element. Any element that should expose
    /// `data-ars-focus-visible` to CSS — the agnostic core does not impose
    /// a tag.
    Root,
}

/// The API for the `FocusRing` component.
///
/// Constructed via [`Api::new`] from a [`Context`] and [`Props`] and
/// queried via [`Api::root_attrs`] or the [`ConnectApi`] dispatch.
#[derive(Clone, Debug)]
pub struct Api {
    ctx: Context,
    props: Props,
}

impl Api {
    /// Creates a new `Api` instance from the given runtime context and
    /// props.
    #[must_use]
    pub const fn new(ctx: Context, props: Props) -> Self {
        Self { ctx, props }
    }

    /// Returns a reference to the underlying [`Props`].
    ///
    /// Adapters typically read individual fields through the dedicated
    /// accessors; this method is the escape hatch for when the full
    /// struct is needed (e.g., to clone it into a fresh [`Api`] for a
    /// re-render).
    #[must_use]
    pub const fn props(&self) -> &Props {
        &self.props
    }

    /// Returns a copy of the underlying [`Context`].
    #[must_use]
    pub const fn context(&self) -> Context {
        self.ctx
    }

    /// Returns the component's instance ID.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.props.id
    }

    /// Returns whether the component tracks focus-within rather than
    /// direct focus. Reads from [`Props::within`] — the adapter uses this
    /// flag to decide whether to wire `focus`/`blur` or
    /// `focusin`/`focusout` listeners.
    #[must_use]
    pub const fn within(&self) -> bool {
        self.props.within
    }

    /// Returns whether the focus-visible state is currently active.
    #[must_use]
    pub const fn focus_visible(&self) -> bool {
        self.ctx.focus_visible
    }

    /// Returns the optional CSS class applied when focused by any means.
    #[must_use]
    pub fn focus_class(&self) -> Option<&str> {
        self.props.focus_class.as_deref()
    }

    /// Returns the optional CSS class applied only when focused by
    /// keyboard.
    #[must_use]
    pub fn focus_visible_class(&self) -> Option<&str> {
        self.props.focus_visible_class.as_deref()
    }

    /// Returns whether the component is configured for text-input focus
    /// semantics (focus ring shown regardless of pointer vs. keyboard).
    #[must_use]
    pub const fn is_text_input(&self) -> bool {
        self.props.is_text_input
    }

    /// Returns the attributes for the root element.
    ///
    /// Always emits `data-ars-scope="focus-ring"` and
    /// `data-ars-part="root"`. When [`Context::focus_visible`] is `true`,
    /// also emits the boolean `data-ars-focus-visible` attribute that
    /// keyboard-focus stylesheets target. When it is `false`, the
    /// attribute is omitted entirely so the bare `[data-ars-focus-visible]`
    /// CSS selector does not match.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
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

    fn ctx_inactive() -> Context {
        Context::default()
    }

    fn ctx_focus_visible() -> Context {
        Context {
            focus_visible: true,
        }
    }

    fn default_props() -> Props {
        Props::default()
    }

    fn within_props() -> Props {
        Props {
            within: true,
            ..Props::default()
        }
    }

    fn classy_props() -> Props {
        Props {
            focus_class: Some(String::from("focused")),
            focus_visible_class: Some(String::from("focused-keyboard")),
            ..Props::default()
        }
    }

    fn text_input_props() -> Props {
        Props {
            is_text_input: true,
            ..Props::default()
        }
    }

    // ── Props ──────────────────────────────────────────────────────

    #[test]
    fn props_default_values() {
        let p = Props::default();

        assert_eq!(p.id, "");
        assert!(!p.within);
        assert_eq!(p.focus_class, None);
        assert_eq!(p.focus_visible_class, None);
        assert!(!p.is_text_input);
    }

    #[test]
    fn props_builder_round_trips() {
        // `Props::new()` returns the documented defaults and the chained
        // setters mutate exactly the matching fields, leaving the others
        // at their default values.
        let p = Props::new()
            .id("ring-build")
            .within(true)
            .focus_class("focused")
            .focus_visible_class("focused-keyboard")
            .is_text_input(true);

        assert_eq!(p.id, "ring-build");
        assert!(p.within);
        assert_eq!(p.focus_class.as_deref(), Some("focused"));
        assert_eq!(p.focus_visible_class.as_deref(), Some("focused-keyboard"));
        assert!(p.is_text_input);

        // `Props::new()` is equivalent to `Default::default()`.
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_setters_are_idempotent_per_field() {
        // Each setter overrides only its own field; later calls overwrite
        // earlier ones for the same field.
        let p = Props::new()
            .within(true)
            .within(false)
            .focus_class("a")
            .focus_class("b")
            .is_text_input(true);

        assert!(!p.within);
        assert_eq!(p.focus_class.as_deref(), Some("b"));
        assert!(p.is_text_input);
    }

    #[test]
    fn props_has_id_derive_round_trips() {
        // Exercises the methods the `HasId` derive emits directly on
        // `Props` (the `Api::id()` accessor only goes through one of them).
        let mut p = Props::default().with_id(String::from("ring-1"));

        assert_eq!(HasId::id(&p), "ring-1");

        p.set_id(String::from("ring-2"));

        assert_eq!(HasId::id(&p), "ring-2");
    }

    #[test]
    fn props_clone_and_partial_eq_round_trip() {
        let original = Props {
            id: String::from("ring-clone"),
            within: true,
            focus_class: Some(String::from("focused")),
            focus_visible_class: Some(String::from("focused-keyboard")),
            is_text_input: true,
        };

        let cloned = original.clone();

        assert_eq!(cloned, original);

        let mutated = Props {
            within: false,
            ..original.clone()
        };

        assert_ne!(mutated, original);
    }

    #[test]
    fn props_context_and_api_are_send_sync() {
        // The agnostic core's public types must be `Send + Sync` so
        // adapters and ahead-of-time computation paths can shuttle
        // values between threads (web handlers, async server functions,
        // etc. — see workspace `feedback_messagefn_send_sync.md`). This
        // assertion fails to compile if a future refactor introduces a
        // non-thread-safe field (e.g., `Rc`, `Cell`).
        const fn assert_send_sync<T: Send + Sync>() {}

        assert_send_sync::<Props>();
        assert_send_sync::<Context>();
        assert_send_sync::<Api>();
        assert_send_sync::<Part>();
    }

    #[test]
    fn api_clone_round_trips() {
        // The `Clone` derive on `Api` must produce a value structurally
        // equal to the source. This locks the property against a future
        // refactor adding a non-clone-coherent field (instance counter,
        // allocation ID, RNG state, etc.).
        let original = Api::new(
            ctx_focus_visible(),
            Props {
                id: String::from("ring-clone"),
                within: true,
                focus_class: Some(String::from("focused")),
                focus_visible_class: Some(String::from("focused-keyboard")),
                is_text_input: true,
            },
        );

        let cloned = original.clone();

        assert_eq!(cloned.props(), original.props());
        assert_eq!(cloned.context(), original.context());
        assert_eq!(cloned.root_attrs(), original.root_attrs());
    }

    #[test]
    fn props_and_api_debug_impl_is_non_empty() {
        // Smoke test guarding against an accidental empty `impl Debug`.
        let api = Api::new(ctx_focus_visible(), within_props());

        let props_dbg = format!("{:?}", api.props());

        let api_dbg = format!("{api:?}");

        assert!(props_dbg.contains("Props"), "Props Debug = {props_dbg}");
        assert!(api_dbg.contains("Api"), "Api Debug = {api_dbg}");
    }

    // ── Context ────────────────────────────────────────────────────

    #[test]
    fn context_default_is_inactive() {
        let ctx = Context::default();

        assert!(!ctx.focus_visible);
    }

    #[test]
    fn context_clone_copy_partial_eq_round_trip() {
        let original = ctx_focus_visible();

        let copy: Context = original;
        let cloned = original;

        assert_eq!(copy, original);
        assert_eq!(cloned, original);
        assert_ne!(original, ctx_inactive());
    }

    // ── Api accessors ──────────────────────────────────────────────

    #[test]
    fn api_id_returns_empty_str_for_default_props() {
        // Direct assertion that the empty-id path through `Api::id()`
        // produces `""` rather than e.g. panicking or returning a sentinel.
        assert_eq!(Api::new(ctx_inactive(), Props::default()).id(), "");
    }

    #[test]
    fn api_exposes_ctx_and_props_fields() {
        let props = Props {
            id: String::from("ring-7"),
            within: true,
            focus_class: Some(String::from("c1")),
            focus_visible_class: Some(String::from("c2")),
            is_text_input: true,
        };

        let api = Api::new(ctx_focus_visible(), props.clone());

        assert_eq!(api.id(), "ring-7");

        // `Api::within()` reads from `Props::within` — the single source
        // of truth for the focus-within routing flag.
        assert!(api.within());
        assert!(api.focus_visible());
        assert_eq!(api.focus_class(), Some("c1"));
        assert_eq!(api.focus_visible_class(), Some("c2"));
        assert!(api.is_text_input());
        assert_eq!(api.props(), &props);
        assert_eq!(api.context(), ctx_focus_visible());

        let default_api = Api::new(ctx_inactive(), Props::default());

        assert!(!default_api.within());
        assert!(!default_api.focus_visible());
        assert_eq!(default_api.focus_class(), None);
        assert_eq!(default_api.focus_visible_class(), None);
        assert!(!default_api.is_text_input());
    }

    // ── Connect / API ──────────────────────────────────────────────

    #[test]
    fn part_attrs_dispatches_root() {
        let api = Api::new(ctx_inactive(), default_props());

        let attrs = api.part_attrs(Part::Root);

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("focus-ring"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("root"));
    }

    #[test]
    fn part_attrs_root_equals_root_attrs() {
        // The `ConnectApi` dispatch must produce exactly what the inherent
        // `root_attrs` method produces. Asserted across both `focus_visible`
        // values and across both `within` values, since `within` lives on
        // `Props` (it could in principle leak into the AttrMap).
        let cases = [
            (ctx_inactive(), default_props()),
            (ctx_inactive(), within_props()),
            (ctx_focus_visible(), default_props()),
            (ctx_focus_visible(), within_props()),
        ];

        for (ctx, props) in cases {
            let api = Api::new(ctx, props);

            assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        }
    }

    // ── root_attrs branches ────────────────────────────────────────

    #[test]
    fn root_emits_focus_visible_when_keyboard_modality() {
        // Spec §1.2: `ctx.focus_visible == true` => emit
        // `data-ars-focus-visible` as a boolean attribute.
        let attrs = Api::new(ctx_focus_visible(), default_props()).root_attrs();

        assert_eq!(
            attrs.get_value(&HtmlAttr::Data("ars-focus-visible")),
            Some(&AttrValue::Bool(true))
        );
        assert!(attrs.contains(&HtmlAttr::Data("ars-focus-visible")));
    }

    #[test]
    fn root_omits_focus_visible_when_pointer_modality() {
        // Spec §1.2: `ctx.focus_visible == false` => attribute absent.
        // The `[data-ars-focus-visible]` CSS selector must NOT match.
        let attrs = Api::new(ctx_inactive(), default_props()).root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-focus-visible")), None);
        assert_eq!(attrs.get_value(&HtmlAttr::Data("ars-focus-visible")), None);
        assert!(!attrs.contains(&HtmlAttr::Data("ars-focus-visible")));
    }

    #[test]
    fn within_prop_drives_within_accessor_and_does_not_affect_attrs() {
        // `Props::within` is the single source of truth for whether the
        // adapter wires focus-within or focus listeners. `Api::within()`
        // surfaces it directly from `Props`, not from `Context`, so the
        // two cannot disagree. The flag does not change `root_attrs`
        // output — that is `focus_visible`'s job alone.
        let api = Api::new(ctx_focus_visible(), within_props());

        assert!(api.within());
        assert!(api.props().within);

        let with_within = Api::new(ctx_inactive(), within_props()).root_attrs();
        let without_within = Api::new(ctx_inactive(), default_props()).root_attrs();

        assert_eq!(with_within, without_within);
    }

    #[test]
    fn class_props_do_not_affect_root_attrs() {
        // Defensive regression test: `focus_class`, `focus_visible_class`,
        // and `is_text_input` are adapter-only render hints. Holding
        // `focus_visible` constant, varying these props MUST produce an
        // identical `AttrMap`.
        let baseline = Api::new(ctx_focus_visible(), default_props()).root_attrs();

        for props in [classy_props(), text_input_props(), within_props()] {
            let attrs = Api::new(ctx_focus_visible(), props).root_attrs();

            assert_eq!(attrs, baseline);
        }
    }

    #[test]
    fn focus_visible_and_inactive_branches_produce_different_attrs() {
        // Defensive cross-branch inequality: `focus_visible` is the only
        // output-affecting flag; flipping it MUST change the `AttrMap`.
        // Catches a regression where the boolean attribute accidentally
        // stops being conditional.
        let active = Api::new(ctx_focus_visible(), default_props()).root_attrs();
        let inactive = Api::new(ctx_inactive(), default_props()).root_attrs();

        assert_ne!(active, inactive);
    }

    // ── Snapshots ──────────────────────────────────────────────────

    #[test]
    fn focus_ring_root_inactive_snapshot() {
        assert_snapshot!(
            "focus_ring_root_inactive",
            snapshot_attrs(&Api::new(ctx_inactive(), default_props()).root_attrs())
        );
    }

    #[test]
    fn focus_ring_root_focus_visible_snapshot() {
        assert_snapshot!(
            "focus_ring_root_focus_visible",
            snapshot_attrs(&Api::new(ctx_focus_visible(), default_props()).root_attrs())
        );
    }
}
