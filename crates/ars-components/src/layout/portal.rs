//! Portal mount lifecycle machine.
//!
//! `Portal` is a DOM-free state machine that tells framework adapters when a
//! portal should be mounted and which target kind should receive the rendered
//! content. Adapters own the actual DOM insertion and cleanup.

use alloc::{format, string::String, vec, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{
    AttrMap, ComponentIds, ComponentMessages, ComponentPart, ConnectApi, Env, HtmlAttr, RenderMode,
    TransitionPlan,
};

/// The states of the portal machine.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// The portal is unmounted.
    #[default]
    Unmounted,

    /// The portal is mounted at its target container.
    Mounted,
}

/// The events of the portal machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Mount the portal after the host component mounts.
    Mount,

    /// Unmount the portal before the host component unmounts.
    Unmount,

    /// The target container became available for an ID target that may not
    /// have existed when the portal was first mounted.
    ContainerReady(String),

    /// Synchronize the target container after props change.
    SetContainer(PortalTarget),
}

/// Runtime context for Portal.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The resolved target container for the portal.
    pub container: PortalTarget,

    /// Whether the portal is mounted.
    pub mounted: bool,

    /// Runtime render mode resolved by the adapter.
    pub render_mode: RenderMode,

    /// Component IDs.
    pub ids: ComponentIds,
}

/// The target container for the portal.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum PortalTarget {
    /// The dedicated portal root element (`#ars-portal-root`).
    #[default]
    PortalRoot,

    /// The document body.
    Body,

    /// An element with the given ID.
    Id(String),

    /// A direct element ID reference.
    Ref(String),
}

/// Immutable configuration for a Portal instance.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// The id of the component.
    pub id: String,

    /// The target container for the portal.
    pub container: PortalTarget,

    /// Whether to render the portal inline during SSR.
    ///
    /// When `true`, content is rendered at the declaration site during SSR;
    /// the client hydration layer reattaches it to the target container.
    pub ssr_inline: bool,
}

impl Props {
    /// Returns fresh portal props with the documented defaults.
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

    /// Sets the portal target container.
    #[must_use]
    pub fn container(mut self, value: PortalTarget) -> Self {
        self.container = value;
        self
    }

    /// Sets whether SSR should render content inline before hydration.
    #[must_use]
    pub const fn ssr_inline(mut self, value: bool) -> Self {
        self.ssr_inline = value;
        self
    }
}

/// Portal has no localized messages.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;

impl ComponentMessages for Messages {}

/// The machine for the `Portal` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, env: &Env, _messages: &Self::Messages) -> (State, Context) {
        let ctx = Context {
            container: props.container.clone(),
            mounted: false,
            render_mode: env.render_mode,
            ids: ComponentIds::from_id(&props.id),
        };

        (State::Unmounted, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        context: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Unmounted, Event::Mount) => Some(TransitionPlan::to(State::Mounted).apply(
                |ctx: &mut Context| {
                    ctx.mounted = true;
                },
            )),

            (State::Mounted, Event::Unmount) => Some(TransitionPlan::to(State::Unmounted).apply(
                |ctx: &mut Context| {
                    ctx.mounted = false;
                },
            )),

            (State::Unmounted, Event::ContainerReady(id)) if matches!(&context.container, PortalTarget::Id(target_id) if target_id == id) =>
            {
                let id = id.clone();
                Some(
                    TransitionPlan::to(State::Mounted).apply(move |ctx: &mut Context| {
                        ctx.container = PortalTarget::Ref(id);
                        ctx.mounted = true;
                    }),
                )
            }

            (_, Event::SetContainer(target)) => {
                let target = target.clone();
                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.container = target;
                }))
            }

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        assert_eq!(
            old.id, new.id,
            "Portal id cannot change after initialization"
        );

        if old.container == new.container {
            Vec::new()
        } else {
            vec![Event::SetContainer(new.container.clone())]
        }
    }
}

/// The Portal part enum.
#[derive(ComponentPart)]
#[scope = "portal"]
pub enum Part {
    /// The portal mount point inserted at the target container.
    Root,
}

/// Connected Portal API.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", &self.state)
            .field("ctx", &self.ctx)
            .field("props", &self.props)
            .finish()
    }
}

impl Api<'_> {
    /// Whether the portal is currently mounted.
    #[must_use]
    pub const fn is_mounted(&self) -> bool {
        matches!(self.state, State::Mounted)
    }

    /// Returns the currently resolved portal target.
    #[must_use]
    pub const fn target(&self) -> &PortalTarget {
        &self.ctx.container
    }

    /// Returns the runtime render mode resolved by the adapter.
    #[must_use]
    pub const fn render_mode(&self) -> RenderMode {
        self.ctx.render_mode
    }

    /// Returns whether SSR should render portal content inline.
    #[must_use]
    pub const fn ssr_inline(&self) -> bool {
        self.props.ssr_inline
    }

    /// Returns the stable portal owner ID used by outside-interaction helpers.
    #[must_use]
    pub fn owner_id(&self) -> &str {
        self.ctx.ids.id()
    }

    /// Returns whether portal content should render inline at the declaration
    /// site for the current runtime mode.
    #[must_use]
    pub const fn should_render_inline(&self) -> bool {
        self.props.ssr_inline && self.ctx.render_mode.is_server()
    }

    /// The generated portal root element ID, usable for `aria-owns` on triggers.
    #[must_use]
    pub fn portal_root_id(&self) -> String {
        format!("ars-portal-{}", self.ctx.ids.id())
    }

    /// The attributes for the portal root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.portal_root_id())
            .set(HtmlAttr::Data("ars-portal-id"), self.ctx.ids.id())
            .set(HtmlAttr::Data("ars-portal-owner"), self.ctx.ids.id())
            .set(
                HtmlAttr::Data("ars-state"),
                if self.is_mounted() {
                    "mounted"
                } else {
                    "unmounted"
                },
            );

        attrs
    }

    /// Dispatches a mount event for the portal.
    pub fn on_mount(&self) {
        (self.send)(Event::Mount);
    }

    /// Dispatches an unmount event for the portal.
    pub fn on_unmount(&self) {
        (self.send)(Event::Unmount);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        format,
        rc::Rc,
        string::{String, ToString},
        vec::Vec,
    };
    use core::cell::RefCell;

    use ars_core::{ConnectApi, Env, HtmlAttr, RenderMode, Service};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props::new().id("portal")
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new()
            .id("portal-1")
            .container(PortalTarget::Id("custom-root".to_string()))
            .ssr_inline(true);

        assert_eq!(props.id, "portal-1");
        assert_eq!(props.container, PortalTarget::Id("custom-root".to_string()));
        assert!(props.ssr_inline);
    }

    #[test]
    fn initializes_unmounted_with_default_portal_root() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_eq!(service.state(), &State::Unmounted);
        assert_eq!(service.context().container, PortalTarget::PortalRoot);
        assert!(!service.context().mounted);
        assert_eq!(service.context().render_mode, RenderMode::Client);
        assert_eq!(service.context().ids.id(), "portal");
    }

    #[test]
    fn initializes_with_adapter_render_mode() {
        let env = Env::default().with_render_mode(RenderMode::Server);

        let service = Service::<Machine>::new(test_props().ssr_inline(true), &env, &Messages);

        let api = service.connect(&|_| {});

        assert_eq!(service.context().render_mode, RenderMode::Server);
        assert_eq!(api.render_mode(), RenderMode::Server);
        assert!(api.ssr_inline());
        assert!(api.should_render_inline());
    }

    #[test]
    fn hydration_mode_does_not_render_inline_again() {
        let env = Env::default().with_render_mode(RenderMode::Hydrating);

        let service = Service::<Machine>::new(test_props().ssr_inline(true), &env, &Messages);

        let api = service.connect(&|_| {});

        assert_eq!(api.render_mode(), RenderMode::Hydrating);
        assert!(!api.should_render_inline());
    }

    #[test]
    fn server_mode_does_not_render_inline_when_ssr_inline_is_disabled() {
        let env = Env::default().with_render_mode(RenderMode::Server);

        let service = Service::<Machine>::new(test_props().ssr_inline(false), &env, &Messages);

        let api = service.connect(&|_| {});

        assert_eq!(api.render_mode(), RenderMode::Server);
        assert!(!api.ssr_inline());
        assert!(!api.should_render_inline());
    }

    #[test]
    fn initializes_with_custom_target_container() {
        let service = Service::<Machine>::new(
            Props::new()
                .id("portal")
                .container(PortalTarget::Id("custom-root".to_string())),
            &Env::default(),
            &Messages,
        );

        assert_eq!(
            service.context().container,
            PortalTarget::Id("custom-root".to_string())
        );
        assert!(!service.context().mounted);
    }

    #[test]
    fn mount_event_marks_content_mounted_for_adapter() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.send(Event::Mount);

        assert!(result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Mounted);
        assert!(service.context().mounted);
    }

    #[test]
    fn unmount_event_marks_content_unmounted_for_adapter_cleanup() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::Mount));

        let result = service.send(Event::Unmount);

        assert!(result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Unmounted);
        assert!(!service.context().mounted);
    }

    #[test]
    fn container_ready_mounts_to_ref_target() {
        let mut service = Service::<Machine>::new(
            test_props().container(PortalTarget::Id("late-root".to_string())),
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::ContainerReady("late-root".to_string()));

        assert!(result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Mounted);
        assert_eq!(
            service.context().container,
            PortalTarget::Ref("late-root".to_string())
        );
        assert!(service.context().mounted);
    }

    #[test]
    fn container_ready_ignores_unmatched_targets() {
        let mut service = Service::<Machine>::new(
            test_props().container(PortalTarget::Id("expected-root".to_string())),
            &Env::default(),
            &Messages,
        );

        let mismatched_result = service.send(Event::ContainerReady("other-root".to_string()));

        assert!(!mismatched_result.state_changed);
        assert!(!mismatched_result.context_changed);
        assert_eq!(service.state(), &State::Unmounted);
        assert_eq!(
            service.context().container,
            PortalTarget::Id("expected-root".to_string())
        );

        let mut portal_root = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let non_id_result = portal_root.send(Event::ContainerReady("late-root".to_string()));

        assert!(!non_id_result.state_changed);
        assert!(!non_id_result.context_changed);
        assert_eq!(portal_root.context().container, PortalTarget::PortalRoot);
    }

    #[test]
    fn invalid_transitions_are_ignored() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let unmount_result = service.send(Event::Unmount);

        assert!(!unmount_result.state_changed);
        assert!(!unmount_result.context_changed);
        assert_eq!(service.state(), &State::Unmounted);

        drop(service.send(Event::Mount));

        let mount_result = service.send(Event::Mount);

        assert!(!mount_result.state_changed);
        assert!(!mount_result.context_changed);
        assert_eq!(service.state(), &State::Mounted);

        let ready_result = service.send(Event::ContainerReady("late-root".to_string()));

        assert!(!ready_result.state_changed);
        assert!(!ready_result.context_changed);
        assert_eq!(service.context().container, PortalTarget::PortalRoot);
    }

    #[test]
    fn set_props_syncs_container_without_remounting() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.set_props(test_props().container(PortalTarget::Body));

        assert!(result.context_changed);
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Unmounted);
        assert_eq!(service.context().container, PortalTarget::Body);
    }

    #[test]
    fn set_props_syncs_container_without_unmounting_mounted_portal() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::Mount));

        let result = service.set_props(test_props().container(PortalTarget::Body));

        assert!(result.context_changed);
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Mounted);
        assert!(service.context().mounted);
        assert_eq!(service.context().container, PortalTarget::Body);
    }

    #[test]
    fn set_props_with_unchanged_container_emits_no_events() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.set_props(test_props());

        assert!(!result.context_changed);
        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Unmounted);
        assert_eq!(service.context().container, PortalTarget::PortalRoot);
        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&test_props(), &test_props()),
            Vec::<Event>::new()
        );
    }

    #[test]
    #[should_panic(expected = "Portal id cannot change after initialization")]
    fn set_props_panics_when_id_changes() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.set_props(Props::new().id("different")));
    }

    #[test]
    fn api_reports_mounted_state() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert!(!service.connect(&|_| {}).is_mounted());

        drop(service.send(Event::Mount));

        assert!(service.connect(&|_| {}).is_mounted());
    }

    #[test]
    fn api_portal_root_id_uses_component_id() {
        let service =
            Service::<Machine>::new(Props::new().id("menu-1"), &Env::default(), &Messages);

        assert_eq!(
            service.connect(&|_| {}).portal_root_id(),
            "ars-portal-menu-1"
        );
    }

    #[test]
    fn api_exposes_adapter_target_and_ssr_decisions() {
        let env = Env::default().with_render_mode(RenderMode::Server);

        let service = Service::<Machine>::new(
            Props::new()
                .id("menu-1")
                .container(PortalTarget::Body)
                .ssr_inline(true),
            &env,
            &Messages,
        );

        let api = service.connect(&|_| {});

        assert_eq!(api.target(), &PortalTarget::Body);
        assert_eq!(api.owner_id(), "menu-1");
        assert_eq!(api.render_mode(), RenderMode::Server);
        assert!(api.ssr_inline());
        assert!(api.should_render_inline());
    }

    #[test]
    fn api_debug_includes_state_context_and_props() {
        let service = Service::<Machine>::new(
            Props::new()
                .id("debug-portal")
                .container(PortalTarget::Body)
                .ssr_inline(true),
            &Env::default(),
            &Messages,
        );

        let debug = format!("{:?}", service.connect(&|_| {}));

        assert!(debug.starts_with("Api"));
        assert!(debug.contains("state: Unmounted"));
        assert!(debug.contains("container: Body"));
        assert!(debug.contains("id: \"debug-portal\""));
        assert!(debug.contains("ssr_inline: true"));
    }

    #[test]
    fn api_on_mount_and_on_unmount_dispatch_events() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let events = Rc::new(RefCell::new(Vec::new()));

        let captured = Rc::clone(&events);
        let send = move |event| {
            captured.borrow_mut().push(event);
        };

        let api = service.connect(&send);

        api.on_mount();
        api.on_unmount();

        assert_eq!(events.borrow().as_slice(), [Event::Mount, Event::Unmount]);
    }

    #[test]
    fn part_attrs_match_root_attrs() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn root_attrs_do_not_emit_aria_semantics() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let attrs = service.connect(&|_| {}).root_attrs();

        assert!(
            attrs
                .attrs()
                .iter()
                .all(|(attr, _)| { !matches!(attr, HtmlAttr::Role | HtmlAttr::Aria(_)) })
        );
    }

    #[test]
    fn nested_portals_have_independent_ids_targets_and_state() {
        let mut parent = Service::<Machine>::new(
            Props::new()
                .id("parent")
                .container(PortalTarget::PortalRoot),
            &Env::default(),
            &Messages,
        );

        let mut child = Service::<Machine>::new(
            Props::new()
                .id("child")
                .container(PortalTarget::Id("nested-root".to_string())),
            &Env::default(),
            &Messages,
        );

        drop(parent.send(Event::Mount));
        drop(child.send(Event::Mount));

        let parent_api = parent.connect(&|_| {});
        let child_api = child.connect(&|_| {});

        assert_eq!(parent_api.portal_root_id(), "ars-portal-parent");
        assert_eq!(child_api.portal_root_id(), "ars-portal-child");
        assert_eq!(parent.context().container, PortalTarget::PortalRoot);
        assert_eq!(
            child.context().container,
            PortalTarget::Id("nested-root".to_string())
        );
        assert_eq!(
            parent_api
                .root_attrs()
                .get(&HtmlAttr::Data("ars-portal-id")),
            Some("parent")
        );
        assert_eq!(
            parent_api
                .root_attrs()
                .get(&HtmlAttr::Data("ars-portal-owner")),
            Some("parent")
        );
        assert_eq!(
            parent_api.root_attrs().get(&HtmlAttr::Id),
            Some("ars-portal-parent")
        );
        assert_eq!(
            child_api.root_attrs().get(&HtmlAttr::Data("ars-portal-id")),
            Some("child")
        );
        assert_eq!(
            child_api
                .root_attrs()
                .get(&HtmlAttr::Data("ars-portal-owner")),
            Some("child")
        );

        drop(parent.send(Event::Unmount));

        assert_eq!(parent.state(), &State::Unmounted);
        assert_eq!(child.state(), &State::Mounted);
        assert!(!parent.context().mounted);
        assert!(child.context().mounted);
    }

    #[test]
    fn portal_root_unmounted() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "portal_root_unmounted",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn portal_root_mounted() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.send(Event::Mount));

        assert_snapshot!(
            "portal_root_mounted",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn portal_root_custom_target_mounted() {
        let mut service = Service::<Machine>::new(
            Props::new()
                .id("custom")
                .container(PortalTarget::Id("custom-root".to_string())),
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Mount));

        assert_snapshot!(
            "portal_root_custom_target_mounted",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }
}
