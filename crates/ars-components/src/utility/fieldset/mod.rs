//! Fieldset component state machine and connect API.
//!
//! This module implements the framework-agnostic `Fieldset` machine defined in
//! `spec/foundation/07-forms.md` §12. The machine is intentionally stateless
//! at the state-enum level and instead tracks disabled, invalid, readonly, and
//! description/error wiring in its context.

use alloc::{string::String, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{
    AriaAttr, AttrMap, ComponentIds, ComponentPart, ConnectApi, Direction, Env, HtmlAttr,
    TransitionPlan,
};
use ars_forms::validation::Error;

/// Single state for the fieldset machine.
///
/// `Fieldset` is effectively stateless. All meaningful changes are stored in
/// [`Context`] and applied through context-only transitions.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// The fieldset is mounted and ready to expose its current context.
    Idle,
}

/// Events that update fieldset context.
#[derive(Clone, Debug)]
pub enum Event {
    /// Replaces the current fieldset-level validation errors.
    SetErrors(Vec<Error>),

    /// Clears all fieldset-level validation errors.
    ClearErrors,

    /// Synchronizes the disabled state from props.
    SetDisabled(bool),

    /// Synchronizes the invalid state from props.
    SetInvalid(bool),

    /// Synchronizes the readonly state from props.
    SetReadonly(bool),

    /// Synchronizes the layout direction from props.
    SetDir(Option<Direction>),

    /// Tracks whether a description part is currently rendered.
    SetHasDescription(bool),
}

/// Mutable machine context for the fieldset component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Whether the entire fieldset and all contained inputs are disabled.
    pub disabled: bool,

    /// Whether the fieldset is currently invalid.
    pub invalid: bool,

    /// Whether the fieldset is read-only.
    pub readonly: bool,

    /// The configured text direction for RTL-aware rendering.
    pub dir: Option<Direction>,

    /// Fieldset-level validation errors.
    pub errors: Vec<Error>,

    /// Whether a description part is rendered and should be referenced by ARIA.
    pub has_description: bool,

    /// Stable IDs derived from the adapter-provided base ID.
    pub ids: ComponentIds,
}

/// Immutable configuration for a fieldset machine instance.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Adapter-provided base ID for the fieldset root.
    ///
    /// This ID is immutable for the lifetime of a machine instance because
    /// [`Context::ids`] caches the derived part IDs during initialization.
    pub id: String,

    /// Whether the entire fieldset and all contained inputs are disabled.
    pub disabled: bool,

    /// Whether the fieldset is invalid before error-driven state is applied.
    pub invalid: bool,

    /// Whether the fieldset is read-only.
    pub readonly: bool,

    /// The configured text direction for RTL-aware rendering.
    pub dir: Option<Direction>,
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`]
    /// value: empty `id`, all booleans `false`, no `dir` override.
    ///
    /// Documented entry point for the builder chain.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id) — the adapter-provided base ID for the
    /// fieldset root. Immutable for the lifetime of a machine instance.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`disabled`](Self::disabled) — when `true`, the entire
    /// fieldset and every contained input is disabled.
    #[must_use]
    pub const fn disabled(mut self, value: bool) -> Self {
        self.disabled = value;
        self
    }

    /// Sets [`invalid`](Self::invalid) — the prop-driven invalid flag
    /// applied before error-driven state.
    #[must_use]
    pub const fn invalid(mut self, value: bool) -> Self {
        self.invalid = value;
        self
    }

    /// Sets [`readonly`](Self::readonly).
    #[must_use]
    pub const fn readonly(mut self, value: bool) -> Self {
        self.readonly = value;
        self
    }

    /// Sets [`dir`](Self::dir) — the configured text direction for
    /// RTL-aware rendering. Wraps the supplied value in [`Some`].
    #[must_use]
    pub const fn dir(mut self, dir: Direction) -> Self {
        self.dir = Some(dir);
        self
    }
}

/// Framework-agnostic fieldset state machine.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = ();
    type Effect = ars_core::NoEffect;
    type Api<'a> = Api<'a>;

    fn init(
        props: &Self::Props,
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (
            State::Idle,
            Context {
                disabled: props.disabled,
                invalid: props.invalid,
                readonly: props.readonly,
                dir: props.dir,
                errors: Vec::new(),
                has_description: false,
                ids: ComponentIds::from_id(&props.id),
            },
        )
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "fieldset Props.id must remain stable after init"
        );

        let mut events = Vec::new();

        if old.disabled != new.disabled {
            events.push(Event::SetDisabled(new.disabled));
        }

        if old.invalid != new.invalid {
            events.push(Event::SetInvalid(new.invalid));
        }

        if old.readonly != new.readonly {
            events.push(Event::SetReadonly(new.readonly));
        }

        if old.dir != new.dir {
            events.push(Event::SetDir(new.dir));
        }

        events
    }

    fn transition(
        _state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match event {
            Event::SetErrors(errors) => {
                let errors = errors.clone();
                let base_invalid = props.invalid;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.errors = errors;
                    ctx.invalid = base_invalid || !ctx.errors.is_empty();
                }))
            }

            Event::ClearErrors => {
                let base_invalid = props.invalid;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.errors.clear();
                    ctx.invalid = base_invalid;
                }))
            }

            Event::SetDisabled(disabled) => {
                let disabled = *disabled;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.disabled = disabled;
                }))
            }

            Event::SetInvalid(invalid) => {
                let invalid = *invalid;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.invalid = invalid || !ctx.errors.is_empty();
                }))
            }

            Event::SetReadonly(readonly) => {
                let readonly = *readonly;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.readonly = readonly;
                }))
            }

            Event::SetDir(dir) => {
                let dir = *dir;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.dir = dir;
                }))
            }

            Event::SetHasDescription(has_description) => {
                let has_description = *has_description;
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.has_description = has_description;
                }))
            }
        }
    }

    fn connect<'a>(
        _state: &'a Self::State,
        ctx: &'a Self::Context,
        _props: &'a Self::Props,
        _send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { ctx }
    }
}

/// Snapshot connect API for deriving fieldset DOM attributes.
pub struct Api<'a> {
    ctx: &'a Context,
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api").field("ctx", &self.ctx).finish()
    }
}

/// Structural parts exposed by the fieldset connect API.
#[derive(ComponentPart)]
#[scope = "fieldset"]
pub enum Part {
    /// The root `<fieldset>` element.
    Root,

    /// The `<legend>` element naming the fieldset.
    Legend,

    /// The optional descriptive text element.
    Description,

    /// The fieldset-level error message element.
    ErrorMessage,

    /// The content wrapper for descendant fields.
    Content,
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Self::Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::Legend => self.legend_attrs(),
            Part::Description => self.description_attrs(),
            Part::ErrorMessage => self.error_message_attrs(),
            Part::Content => self.content_attrs(),
        }
    }
}

impl<'a> Api<'a> {
    /// Returns attributes for the root `<fieldset>` element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.id())
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        if self.ctx.disabled {
            attrs.set_bool(HtmlAttr::Disabled, true);
        }

        if let Some(dir) = self.ctx.dir {
            attrs.set(HtmlAttr::Dir, dir.as_html_attr());
        }

        let mut described_by_ids = Vec::new();

        if self.ctx.has_description {
            described_by_ids.push(self.ctx.ids.part("description"));
        }

        if !self.ctx.errors.is_empty() {
            described_by_ids.push(self.ctx.ids.part("error-message"));
        }

        if !described_by_ids.is_empty() {
            attrs.set(
                HtmlAttr::Aria(AriaAttr::DescribedBy),
                described_by_ids.join(" "),
            );
        }

        attrs
    }

    /// Returns attributes for the `<legend>` element.
    #[must_use]
    pub fn legend_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Legend.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("legend"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the description element.
    #[must_use]
    pub fn description_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Description.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("description"))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        attrs
    }

    /// Returns attributes for the fieldset error message element.
    #[must_use]
    pub fn error_message_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::ErrorMessage.data_attrs();

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.part("error-message"))
            .set(HtmlAttr::Role, "alert")
            .set(scope_attr, scope_val)
            .set(part_attr, part_val);

        if self.ctx.errors.is_empty() {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Returns attributes for the content wrapper.
    #[must_use]
    pub fn content_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Content.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        attrs
    }

    /// Returns the current fieldset-level validation errors.
    #[must_use]
    pub fn errors(&self) -> &[Error] {
        &self.ctx.errors
    }

    /// Returns whether the fieldset is disabled.
    #[must_use]
    pub const fn is_disabled(&self) -> bool {
        self.ctx.disabled
    }

    /// Returns whether the fieldset is invalid.
    #[must_use]
    pub const fn is_invalid(&self) -> bool {
        self.ctx.invalid
    }

    /// Returns whether the fieldset is read-only.
    #[must_use]
    pub const fn is_readonly(&self) -> bool {
        self.ctx.readonly
    }
}

#[cfg(test)]
mod tests {
    use ars_core::{ConnectApi, Service};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "billing".to_string(),
            ..Props::default()
        }
    }

    fn test_props_with_invalid() -> Props {
        Props {
            invalid: true,
            ..test_props()
        }
    }

    fn custom_error() -> Error {
        Error::custom("group", "Group is invalid")
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn fieldset_init_default_props() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_eq!(service.state(), &State::Idle);
        assert!(!service.context().disabled);
        assert!(!service.context().invalid);
        assert!(!service.context().readonly);
        assert_eq!(service.context().dir, None);
        assert!(service.context().errors.is_empty());
        assert!(!service.context().has_description);
        assert_eq!(service.context().ids.id(), "billing");
        assert_eq!(service.context().ids.part("legend"), "billing-legend");
    }

    #[test]
    fn fieldset_set_disabled_updates_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let result = service.send(Event::SetDisabled(true));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert!(service.context().disabled);
    }

    #[test]
    fn fieldset_set_invalid_updates_context_without_errors() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let result = service.send(Event::SetInvalid(true));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert!(service.context().invalid);
    }

    #[test]
    fn fieldset_set_invalid_preserves_error_driven_invalidity() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));
        drop(service.send(Event::SetInvalid(false)));

        assert!(service.context().invalid);
    }

    #[test]
    fn fieldset_set_readonly_updates_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let result = service.send(Event::SetReadonly(true));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert!(service.context().readonly);
    }

    #[test]
    fn fieldset_set_dir_updates_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let result = service.send(Event::SetDir(Some(Direction::Rtl)));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.context().dir, Some(Direction::Rtl));
    }

    #[test]
    fn fieldset_set_has_description_updates_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let result = service.send(Event::SetHasDescription(true));

        assert!(!result.state_changed);
        assert!(result.context_changed);
        assert!(service.context().has_description);
    }

    #[test]
    fn fieldset_on_props_changed_emits_events() {
        let old = Props {
            id: "billing".to_string(),
            disabled: false,
            invalid: false,
            readonly: false,
            dir: None,
        };

        let new = Props {
            id: "billing".to_string(),
            disabled: true,
            invalid: true,
            readonly: true,
            dir: Some(Direction::Rtl),
        };

        let events = <Machine as ars_core::Machine>::on_props_changed(&old, &new);

        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], Event::SetDisabled(true)));
        assert!(matches!(events[1], Event::SetInvalid(true)));
        assert!(matches!(events[2], Event::SetReadonly(true)));
        assert!(matches!(events[3], Event::SetDir(Some(Direction::Rtl))));
    }

    #[test]
    fn fieldset_on_props_changed_no_changes_emits_no_events() {
        let props = test_props();

        let events = <Machine as ars_core::Machine>::on_props_changed(&props, &props);

        assert!(events.is_empty());
    }

    #[test]
    #[should_panic(expected = "fieldset Props.id must remain stable after init")]
    fn fieldset_set_props_panics_when_id_changes() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let mut next = test_props();

        next.id = "shipping".to_string();

        drop(service.set_props(next));
    }

    #[test]
    fn fieldset_root_attrs_disabled() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetDisabled(true)));

        let api = service.connect(&|_| {});

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Disabled), Some("true"));
        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Disabled)));
    }

    #[test]
    fn fieldset_root_attrs_dir() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetDir(Some(Direction::Rtl))));

        let api = service.connect(&|_| {});

        let attrs = api.root_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Dir), Some("rtl"));
    }

    #[test]
    fn fieldset_error_message_hidden_when_no_errors() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        let attrs = api.error_message_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Hidden), Some("true"));
    }

    #[test]
    fn fieldset_description_attrs_id() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        let attrs = api.description_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("billing-description"));
    }

    #[test]
    fn fieldset_legend_attrs_id_and_data_attrs() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        let attrs = api.legend_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Id), Some("billing-legend"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("fieldset"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("legend"));
    }

    #[test]
    fn fieldset_content_attrs_data_attrs() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        let attrs = api.content_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("fieldset"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("content"));
    }

    #[test]
    fn fieldset_context_propagation() {
        let ctx: ars_forms::field::Context = ars_forms::fieldset::Context::default();

        assert_eq!(
            ctx,
            ars_forms::field::Context {
                name: None,
                disabled: false,
                invalid: false,
                readonly: false,
            }
        );

        let named: ars_forms::fieldset::Context = ars_forms::field::Context {
            name: Some("address".to_string()),
            disabled: true,
            invalid: true,
            readonly: true,
        };

        assert_eq!(named.name.as_deref(), Some("address"));
        assert!(named.disabled);
        assert!(named.invalid);
        assert!(named.readonly);
    }

    #[test]
    fn fieldset_set_errors_forces_invalid() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));

        assert!(service.context().invalid);
        assert_eq!(service.context().errors.len(), 1);
    }

    #[test]
    fn fieldset_clear_errors_restores_prop_invalid() {
        let mut service = Service::<Machine>::new(test_props_with_invalid(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));
        drop(service.send(Event::ClearErrors));

        assert!(service.context().errors.is_empty());
        assert!(service.context().invalid);
    }

    #[test]
    fn fieldset_set_errors_empty_preserves_prop_invalid() {
        let mut service = Service::<Machine>::new(test_props_with_invalid(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));
        drop(service.send(Event::SetErrors(vec![])));

        assert!(service.context().errors.is_empty());
        assert!(service.context().invalid);
    }

    #[test]
    fn fieldset_root_attrs_describedby_description_only() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetHasDescription(true)));

        let api = service.connect(&|_| {});

        let attrs = api.root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("billing-description")
        );
    }

    #[test]
    fn fieldset_root_attrs_describedby_error_only() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("billing-error-message")
        );
    }

    #[test]
    fn fieldset_root_attrs_describedby_description_and_error() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetHasDescription(true)));
        drop(service.send(Event::SetErrors(vec![custom_error()])));

        let api = service.connect(&|_| {});

        let attrs = api.root_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Aria(AriaAttr::DescribedBy)),
            Some("billing-description billing-error-message")
        );
    }

    #[test]
    fn fieldset_root_attrs_omits_aria_invalid() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));

        let api = service.connect(&|_| {});

        let attrs = api.root_attrs();

        assert!(!attrs.contains(&HtmlAttr::Aria(AriaAttr::Invalid)));
    }

    #[test]
    fn fieldset_error_message_attrs_role_alert() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        let attrs = api.error_message_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Role), Some("alert"));
    }

    #[test]
    fn fieldset_root_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_snapshot!(
            "fieldset_root_default",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn fieldset_root_disabled_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetDisabled(true)));

        assert_snapshot!(
            "fieldset_root_disabled",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn fieldset_root_rtl_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetDir(Some(Direction::Rtl))));

        assert_snapshot!(
            "fieldset_root_rtl",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn fieldset_root_description_only_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetHasDescription(true)));

        assert_snapshot!(
            "fieldset_root_description_only",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn fieldset_root_description_and_error_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetHasDescription(true)));
        drop(service.send(Event::SetErrors(vec![custom_error()])));

        assert_snapshot!(
            "fieldset_root_description_and_error",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn fieldset_legend_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_snapshot!(
            "fieldset_legend_default",
            snapshot_attrs(&service.connect(&|_| {}).legend_attrs())
        );
    }

    #[test]
    fn fieldset_description_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_snapshot!(
            "fieldset_description_default",
            snapshot_attrs(&service.connect(&|_| {}).description_attrs())
        );
    }

    #[test]
    fn fieldset_error_message_hidden_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_snapshot!(
            "fieldset_error_message_hidden",
            snapshot_attrs(&service.connect(&|_| {}).error_message_attrs())
        );
    }

    #[test]
    fn fieldset_error_message_visible_snapshot() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        drop(service.send(Event::SetErrors(vec![custom_error()])));

        assert_snapshot!(
            "fieldset_error_message_visible",
            snapshot_attrs(&service.connect(&|_| {}).error_message_attrs())
        );
    }

    #[test]
    fn fieldset_content_default_snapshot() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        assert_snapshot!(
            "fieldset_content_default",
            snapshot_attrs(&service.connect(&|_| {}).content_attrs())
        );
    }

    #[test]
    fn fieldset_part_attrs_delegate_for_all_parts() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
        assert_eq!(api.part_attrs(Part::Legend), api.legend_attrs());
        assert_eq!(api.part_attrs(Part::Description), api.description_attrs());
        assert_eq!(
            api.part_attrs(Part::ErrorMessage),
            api.error_message_attrs()
        );
        assert_eq!(api.part_attrs(Part::Content), api.content_attrs());
    }

    #[test]
    fn fieldset_api_debug_is_stable() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        let debug = format!("{api:?}");

        assert!(debug.contains("Api"));
        assert!(debug.contains("billing"));
        assert!(debug.contains("Context"));
    }

    #[test]
    fn fieldset_getters_reflect_context() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &());

        let api = service.connect(&|_| {});

        assert!(!api.is_disabled());
        assert!(!api.is_invalid());
        assert!(!api.is_readonly());
        assert!(api.errors().is_empty());

        drop(service.send(Event::SetDisabled(true)));
        drop(service.send(Event::SetReadonly(true)));
        drop(service.send(Event::SetErrors(vec![custom_error()])));

        let api = service.connect(&|_| {});

        assert!(api.is_disabled());
        assert!(api.is_invalid());
        assert!(api.is_readonly());
        assert_eq!(api.errors(), &[custom_error()]);
    }

    // ── Builder tests ──────────────────────────────────────────────

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new()
            .id("fieldset-1")
            .disabled(true)
            .invalid(true)
            .readonly(true)
            .dir(Direction::Rtl);

        assert_eq!(props.id, "fieldset-1");
        assert!(props.disabled);
        assert!(props.invalid);
        assert!(props.readonly);
        assert_eq!(props.dir, Some(Direction::Rtl));
    }
}
