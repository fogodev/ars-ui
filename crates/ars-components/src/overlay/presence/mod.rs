//! Presence mount and unmount lifecycle machine.

use alloc::{string::String, vec, vec::Vec};
use core::fmt::{self, Debug};

use ars_core::{AttrMap, ComponentPart, ConnectApi, Env, HtmlAttr, TransitionPlan};

/// The states of the presence machine.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum State {
    /// The element is not in the DOM.
    #[default]
    Unmounted,

    /// The element is mounted but waiting for lazy content to settle.
    Mounting,

    /// The element is mounted and logically present.
    Mounted,

    /// The element is mounted while its exit animation completes.
    UnmountPending,
}

/// The events of the presence machine.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The `present` prop changed to true.
    Mount,

    /// The `present` prop changed to false.
    Unmount,

    /// Lazy content has settled and may enter the mounted state.
    ContentReady,

    /// The adapter observed exit animation completion.
    AnimationEnd,
}

/// Runtime context for Presence.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Context {
    /// Whether the content should logically be present.
    pub present: bool,

    /// Whether the content should remain mounted in the DOM.
    pub mounted: bool,

    /// Whether an exit animation is currently running.
    pub unmounting: bool,

    /// Adapter-owned DOM node id for the animated element.
    pub node_id: Option<String>,
}

/// Immutable configuration for a Presence instance.
#[derive(Clone, Debug, Default, PartialEq, ars_core::HasId)]
pub struct Props {
    /// Component instance id.
    pub id: String,

    /// Whether the content should be present.
    pub present: bool,

    /// Whether lazy-mounted content must resolve before entering `Mounted`.
    pub lazy_mount: bool,

    /// Whether exit animation should be skipped.
    pub skip_animation: bool,

    /// Whether reduced motion should force instant show and hide.
    pub reduce_motion: bool,
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`]
    /// value: empty `id`, content not present, no lazy mount, no
    /// skip-animation, no reduce-motion.
    ///
    /// Documented entry point for the builder chain — chain
    /// [`id`](Self::id), [`present`](Self::present), and the other
    /// setters to populate configuration without struct-literal
    /// boilerplate.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id) to the supplied component instance id.
    /// Accepts any [`Into<String>`] so callers can pass `&str`, `String`,
    /// `Cow<str>`, etc.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`present`](Self::present) — whether the content should be
    /// present.
    #[must_use]
    pub const fn present(mut self, value: bool) -> Self {
        self.present = value;
        self
    }

    /// Sets [`lazy_mount`](Self::lazy_mount) — whether lazy-mounted
    /// content must resolve before entering `Mounted`.
    #[must_use]
    pub const fn lazy_mount(mut self, value: bool) -> Self {
        self.lazy_mount = value;
        self
    }

    /// Sets [`skip_animation`](Self::skip_animation) — whether exit
    /// animation should be skipped.
    #[must_use]
    pub const fn skip_animation(mut self, value: bool) -> Self {
        self.skip_animation = value;
        self
    }

    /// Sets [`reduce_motion`](Self::reduce_motion) — whether reduced
    /// motion should force instant show and hide.
    #[must_use]
    pub const fn reduce_motion(mut self, value: bool) -> Self {
        self.reduce_motion = value;
        self
    }
}

/// Presence has no localized messages.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Messages;

impl ars_core::ComponentMessages for Messages {}

/// The machine for the `Presence` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = ars_core::NoEffect;
    type Api<'a> = Api<'a>;

    fn init(props: &Self::Props, _env: &Env, _messages: &Self::Messages) -> (State, Context) {
        let initial_state = if props.present {
            State::Mounted
        } else {
            State::Unmounted
        };

        let ctx = Context {
            present: props.present,
            mounted: props.present,
            unmounting: false,
            node_id: None,
        };

        (initial_state, ctx)
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            (State::Unmounted, Event::Mount) if props.lazy_mount => Some(
                TransitionPlan::to(State::Mounting).apply(|ctx: &mut Context| {
                    ctx.present = true;
                    ctx.mounted = true;
                    ctx.unmounting = false;
                }),
            ),

            (State::Unmounted, Event::Mount) => Some(TransitionPlan::to(State::Mounted).apply(
                |ctx: &mut Context| {
                    ctx.present = true;
                    ctx.mounted = true;
                    ctx.unmounting = false;
                },
            )),

            (State::Mounting, Event::ContentReady) => Some(
                TransitionPlan::to(State::Mounted).apply(|ctx: &mut Context| {
                    ctx.present = true;
                    ctx.mounted = true;
                    ctx.unmounting = false;
                }),
            ),

            (State::Mounting, Event::Unmount) => Some(TransitionPlan::to(State::Unmounted).apply(
                |ctx: &mut Context| {
                    ctx.present = false;
                    ctx.mounted = false;
                    ctx.unmounting = false;
                },
            )),

            (State::UnmountPending, Event::Mount) => Some(
                TransitionPlan::to(State::Mounted).apply(|ctx: &mut Context| {
                    ctx.present = true;
                    ctx.mounted = true;
                    ctx.unmounting = false;
                }),
            ),

            (State::Mounted, Event::Unmount) if props.skip_animation || props.reduce_motion => {
                Some(
                    TransitionPlan::to(State::Unmounted).apply(|ctx: &mut Context| {
                        ctx.present = false;
                        ctx.mounted = false;
                        ctx.unmounting = false;
                    }),
                )
            }

            (State::Mounted, Event::Unmount) => Some(
                TransitionPlan::to(State::UnmountPending).apply(|ctx: &mut Context| {
                    ctx.present = false;
                    ctx.unmounting = true;
                }),
            ),

            (State::UnmountPending, Event::AnimationEnd) => Some(
                TransitionPlan::to(State::Unmounted).apply(|ctx: &mut Context| {
                    ctx.present = false;
                    ctx.mounted = false;
                    ctx.unmounting = false;
                }),
            ),

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
        match (old.present, new.present) {
            (false, true) => vec![Event::Mount],
            (true, false) => vec![Event::Unmount],
            _ => Vec::new(),
        }
    }
}

/// The Presence part enum.
#[derive(ComponentPart)]
#[scope = "presence"]
pub enum Part {
    /// The animated content wrapper.
    Root,
}

/// Connected Presence API.
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

impl<'a> Api<'a> {
    /// Whether the content should be in the DOM.
    #[must_use]
    pub const fn is_mounted(&self) -> bool {
        self.ctx.mounted
    }

    /// Whether the content is logically present.
    #[must_use]
    pub const fn is_present(&self) -> bool {
        self.ctx.present
    }

    /// Whether an exit animation is currently running.
    #[must_use]
    pub const fn is_unmounting(&self) -> bool {
        self.ctx.unmounting
    }

    /// The attributes for the animated root element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-state"),
                if matches!(self.state, State::Mounted) {
                    "open"
                } else {
                    "closed"
                },
            )
            .set(
                HtmlAttr::Data("ars-presence"),
                if self.is_unmounting() {
                    "exiting"
                } else {
                    "mounted"
                },
            );

        attrs
    }

    /// Dispatches a mount or unmount event for a new `present` value.
    pub fn sync_present(&self, new_present: bool) {
        if new_present {
            (self.send)(Event::Mount);
        } else {
            (self.send)(Event::Unmount);
        }
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
    use alloc::rc::Rc;
    use core::cell::RefCell;

    use ars_core::{ConnectApi, Service};
    use insta::assert_snapshot;

    use super::*;

    fn test_props() -> Props {
        Props {
            id: "presence".to_string(),
            ..Props::default()
        }
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    #[test]
    fn initial_unmounted() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_eq!(service.state(), &State::Unmounted);
        assert_eq!(
            service.context(),
            &Context {
                present: false,
                mounted: false,
                unmounting: false,
                node_id: None,
            }
        );
    }

    #[test]
    fn initial_mounted_when_present_true() {
        let service = Service::<Machine>::new(
            Props {
                present: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_eq!(service.state(), &State::Mounted);
        assert!(service.context().present);
        assert!(service.context().mounted);
        assert!(!service.context().unmounting);
    }

    #[test]
    fn prop_sync_false_to_true_enters_mounted() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.set_props(Props {
            present: true,
            ..test_props()
        });

        assert!(result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Mounted);
        assert!(service.context().present);
        assert!(service.context().mounted);
    }

    #[test]
    fn close_enters_unmount_pending() {
        let mut service = Service::<Machine>::new(
            Props {
                present: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let result = service.send(Event::Unmount);

        assert!(result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::UnmountPending);
        assert!(!service.context().present);
        assert!(service.context().mounted);
        assert!(service.context().unmounting);
    }

    #[test]
    fn animation_end_enters_unmounted() {
        let mut service = Service::<Machine>::new(
            Props {
                present: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Unmount));

        let result = service.send(Event::AnimationEnd);

        assert!(result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Unmounted);
        assert!(!service.context().mounted);
        assert!(!service.context().unmounting);
    }

    #[test]
    fn remount_during_unmount_pending_cancels_exit() {
        let mut service = Service::<Machine>::new(
            Props {
                present: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Unmount));

        assert_eq!(service.state(), &State::UnmountPending);

        let result = service.send(Event::Mount);

        assert!(result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Mounted);
        assert!(service.context().present);
        assert!(service.context().mounted);
        assert!(!service.context().unmounting);
    }

    #[test]
    fn skip_animation_forces_direct_unmount() {
        let mut service = Service::<Machine>::new(
            Props {
                present: true,
                skip_animation: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Unmount));

        assert_eq!(service.state(), &State::Unmounted);
        assert!(!service.context().mounted);
        assert!(!service.context().unmounting);
    }

    #[test]
    fn reduce_motion_forces_direct_unmount() {
        let mut service = Service::<Machine>::new(
            Props {
                present: true,
                reduce_motion: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Unmount));

        assert_eq!(service.state(), &State::Unmounted);
        assert!(!service.context().mounted);
        assert!(!service.context().unmounting);
    }

    #[test]
    fn lazy_mount_waits_for_content_ready() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let result = service.set_props(Props {
            present: true,
            lazy_mount: true,
            ..test_props()
        });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Mounting);
        assert!(service.context().mounted);
        assert!(service.context().present);

        drop(service.send(Event::ContentReady));

        assert_eq!(service.state(), &State::Mounted);
        assert!(service.context().mounted);
        assert!(service.context().present);
    }

    #[test]
    fn lazy_mount_unmount_before_content_ready_cancels_mount() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.set_props(Props {
            present: true,
            lazy_mount: true,
            ..test_props()
        }));

        assert_eq!(service.state(), &State::Mounting);

        let result = service.send(Event::Unmount);

        assert!(result.state_changed);
        assert!(result.context_changed);
        assert_eq!(service.state(), &State::Unmounted);
        assert!(!service.context().present);
        assert!(!service.context().mounted);
        assert!(!service.context().unmounting);
    }

    #[test]
    fn api_flags_reflect_mounted_and_unmount_pending() {
        let unmounted = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let unmounted_api = unmounted.connect(&|_| {});

        assert!(!unmounted_api.is_mounted());
        assert!(!unmounted_api.is_present());
        assert!(!unmounted_api.is_unmounting());

        let service = Service::<Machine>::new(
            Props {
                present: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let api = service.connect(&|_| {});

        assert!(api.is_mounted());
        assert!(api.is_present());
        assert!(!api.is_unmounting());

        let mut exiting = Service::<Machine>::new(
            Props {
                present: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(exiting.send(Event::Unmount));

        let exiting_api = exiting.connect(&|_| {});

        assert!(exiting_api.is_mounted());
        assert!(!exiting_api.is_present());
        assert!(exiting_api.is_unmounting());
    }

    #[test]
    fn props_changed_false_present_dispatches_unmount() {
        let old = Props {
            present: true,
            ..test_props()
        };
        let new = Props {
            present: false,
            ..test_props()
        };

        assert_eq!(
            <Machine as ars_core::Machine>::on_props_changed(&old, &new),
            vec![Event::Unmount]
        );
    }

    #[test]
    fn part_attrs_match_root_attrs() {
        let service = Service::<Machine>::new(
            Props {
                present: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        let api = service.connect(&|_| {});

        assert_eq!(api.part_attrs(Part::Root), api.root_attrs());
    }

    #[test]
    fn mounting_root_attrs_stay_closed_until_content_ready() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.set_props(Props {
            present: true,
            lazy_mount: true,
            ..test_props()
        }));

        let attrs = service.connect(&|_| {}).root_attrs();

        assert_eq!(service.state(), &State::Mounting);
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-state")), Some("closed"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-presence")), Some("mounted"));
    }

    #[test]
    fn sync_present_dispatches_events() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        let sent = Rc::new(RefCell::new(Vec::new()));

        let sent_clone = Rc::clone(&sent);

        let send = move |event| {
            sent_clone.borrow_mut().push(event);
        };

        let api = service.connect(&send);

        api.sync_present(true);
        api.sync_present(false);

        assert_eq!(&*sent.borrow(), &[Event::Mount, Event::Unmount]);
    }

    #[test]
    fn snapshot_root_mounted() {
        let service = Service::<Machine>::new(
            Props {
                present: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        assert_snapshot!(
            "presence_root_mounted",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_mounting() {
        let mut service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        drop(service.set_props(Props {
            present: true,
            lazy_mount: true,
            ..test_props()
        }));

        assert_snapshot!(
            "presence_root_mounting",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_unmount_pending() {
        let mut service = Service::<Machine>::new(
            Props {
                present: true,
                ..test_props()
            },
            &Env::default(),
            &Messages,
        );

        drop(service.send(Event::Unmount));

        assert_snapshot!(
            "presence_root_unmount_pending",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    #[test]
    fn snapshot_root_unmounted() {
        let service = Service::<Machine>::new(test_props(), &Env::default(), &Messages);

        assert_snapshot!(
            "presence_root_unmounted",
            snapshot_attrs(&service.connect(&|_| {}).root_attrs())
        );
    }

    // ── Builder tests ──────────────────────────────────────────────

    #[test]
    fn props_new_returns_default_values() {
        assert_eq!(Props::new(), Props::default());
    }

    #[test]
    fn props_builder_chain_applies_each_setter() {
        let props = Props::new()
            .id("presence-1")
            .present(true)
            .lazy_mount(true)
            .skip_animation(true)
            .reduce_motion(true);

        assert_eq!(props.id, "presence-1");
        assert!(props.present);
        assert!(props.lazy_mount);
        assert!(props.skip_animation);
        assert!(props.reduce_motion);
    }
}
