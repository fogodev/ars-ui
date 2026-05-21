//! `FocusScope` component state machine and connect API.
//!
//! `FocusScope` constrains keyboard Tab focus within a container, enabling
//! focus trapping for `Dialog`, `Drawer`, `Popover`, and other overlay
//! components. The agnostic core emits typed effect intents
//! ([`Effect`]) that adapters route through their `PlatformEffects`
//! implementation — the core never touches the DOM or framework helpers.
//!
//! See `spec/components/utility/focus-scope.md` for the full contract.

use alloc::string::String;
use core::fmt::{self, Debug};

use ars_core::{
    AttrMap, ComponentMessages, ComponentPart, ConnectApi, Env, HasId, HtmlAttr, PendingEffect,
    TransitionPlan,
};

// ────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────

/// The states of the `FocusScope` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Focus scope is idle; Tab behavior is unmodified.
    Inactive,

    /// Focus scope is active; the adapter is responsible for trapping Tab
    /// inside the container when `trapped` is true.
    Active {
        /// When true, Tab cannot escape the container.
        trapped: bool,
    },
}

// ────────────────────────────────────────────────────────────────────
// Event
// ────────────────────────────────────────────────────────────────────

/// The events of the `FocusScope` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Activate the focus scope, optionally trapping focus.
    ///
    /// The adapter captures the currently focused element ID before sending
    /// this event (via `PlatformEffects::active_element_id`) so the machine
    /// can store it for later restoration.
    Activate {
        /// When true, Tab cannot escape the container.
        trapped: bool,

        /// ID of the element that had focus before activation (for
        /// restore-on-deactivate). Captured by the adapter via
        /// `PlatformEffects::active_element_id`.
        saved_focus_id: Option<String>,
    },

    /// Deactivate the scope and optionally restore previous focus.
    Deactivate {
        /// When true, the machine emits [`Effect::RestoreFocus`] so the
        /// adapter can return focus to the previously focused element.
        restore_focus: bool,
    },

    /// Enable focus trapping on an active scope (toggles `trapped` to true).
    TrapFocus,

    /// Disable focus trapping on an active scope (toggles `trapped` to false).
    ReleaseTrap,

    /// Request focus restoration to [`Context::saved_focus`].
    ///
    /// When `Inactive`, the machine emits [`Effect::RestoreFocus`]; when
    /// `Active`, the event is ignored — restoration is only meaningful
    /// after the scope has deactivated. Adapters MAY send this event
    /// explicitly when an `Inactive` scope still holds a saved target
    /// (e.g., nested-scope cleanup).
    RestoreFocus,

    /// Request focus on the first tabbable element inside the container.
    /// Emits [`Effect::FocusFirst`] when `Active`; ignored otherwise.
    FocusFirst,

    /// Request focus on the last tabbable element inside the container.
    /// Emits [`Effect::FocusLast`] when `Active`; ignored otherwise.
    FocusLast,
}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Machine context for the `FocusScope` component.
///
/// `is_active()` and `is_trapped()` are derived from [`State`] in the
/// connect API — they are not stored here.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Context {
    /// The element that had focus before the scope was activated.
    ///
    /// Set on [`Event::Activate`] and read by the adapter when dispatching
    /// [`Effect::RestoreFocus`]. The agnostic core does not clear the value
    /// after restoration — the next `Activate` overwrites it.
    pub saved_focus: Option<String>,

    /// The DOM ID of the container element that scopes focus.
    ///
    /// Adapters populate this via `Service::context_mut()` once they have
    /// resolved the container's stable ID. The agnostic core treats it as
    /// an opaque token.
    pub container_id: Option<String>,
}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Props for the `FocusScope` component.
#[derive(Clone, Debug, PartialEq, Eq, HasId)]
pub struct Props {
    /// Component instance ID. Adapters set this from a hydration-safe
    /// stable ID generator.
    pub id: String,

    /// Prevent Tab from moving focus outside the container.
    pub trapped: bool,

    /// Alias for [`trapped`](Self::trapped) — clearer naming in some
    /// contexts (e.g., when `trapped` already means "active" elsewhere).
    pub contain: bool,

    /// On activation, automatically move focus to the first tabbable
    /// element (or the element with `autofocus` attribute, if any).
    pub auto_focus: bool,

    /// On deactivation, restore focus to the previously focused element.
    pub restore_focus: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            trapped: false,
            contain: false,
            auto_focus: true,
            restore_focus: true,
        }
    }
}

impl Props {
    /// Returns a fresh [`Props`] with every field at its [`Default`] value.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`id`](Self::id).
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Sets [`trapped`](Self::trapped).
    #[must_use]
    pub const fn trapped(mut self, trapped: bool) -> Self {
        self.trapped = trapped;
        self
    }

    /// Sets [`contain`](Self::contain).
    #[must_use]
    pub const fn contain(mut self, contain: bool) -> Self {
        self.contain = contain;
        self
    }

    /// Sets [`auto_focus`](Self::auto_focus).
    #[must_use]
    pub const fn auto_focus(mut self, auto_focus: bool) -> Self {
        self.auto_focus = auto_focus;
        self
    }

    /// Sets [`restore_focus`](Self::restore_focus).
    #[must_use]
    pub const fn restore_focus(mut self, restore_focus: bool) -> Self {
        self.restore_focus = restore_focus;
        self
    }
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// `FocusScope` has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Effect
// ────────────────────────────────────────────────────────────────────

/// Typed effect intents emitted by the `FocusScope` machine.
///
/// Each variant is an adapter contract — the adapter's effect handler
/// translates the variant into the corresponding `PlatformEffects` call.
/// See `spec/components/utility/focus-scope.md` §1.8 for the per-variant
/// contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Install the Tab-key interception (keydown handler + sentinel
    /// elements) that keeps focus inside the scope while it is active.
    ///
    /// Cleanup tears the handler back down — invoked when the machine
    /// emits `.cancel_effect(Effect::FocusTrapListener)` on `Deactivate`.
    FocusTrapListener,

    /// Move focus to the first tabbable descendant of the container.
    /// Fall back to focusing the container itself when none exist.
    FocusFirst,

    /// Move focus to the last tabbable descendant of the container.
    /// Fall back to focusing the container itself when none exist.
    FocusLast,

    /// Restore focus to `service.context().saved_focus`, applying the
    /// §7 fallback chain when the saved target is no longer focusable.
    RestoreFocus,
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// State machine for the `FocusScope` component.
#[derive(Debug)]
pub struct Machine;

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(
        _props: &Self::Props,
        _env: &Env,
        _messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        (State::Inactive, Context::default())
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // ── Activation ──────────────────────────────────────────────
            (
                State::Inactive,
                Event::Activate {
                    trapped,
                    saved_focus_id,
                },
            ) => {
                // `Props::trapped` is documented in spec §1.4 as
                // controlling whether focus is trapped; `Props::contain`
                // is its alias. Either prop opts the scope into
                // trapping; the event's `trapped` is a per-activation
                // override that can also force trapping when the props
                // didn't request it.
                let trap = *trapped || props.trapped || props.contain;
                let auto_focus = props.auto_focus;
                let saved = saved_focus_id.clone();

                let mut plan = TransitionPlan::to(State::Active { trapped: trap }).apply(
                    move |ctx: &mut Context| {
                        ctx.saved_focus = saved;
                    },
                );
                // Only install Tab interception when the resolved state
                // is actually trapped. `PlatformEffects::attach_focus_trap`
                // unconditionally wraps Tab/Shift+Tab in the web
                // implementation, so emitting the effect for an
                // untrapped Active scope would contradict the resolved
                // `trapped: false` state.
                if trap {
                    plan = plan.with_effect(PendingEffect::named(Effect::FocusTrapListener));
                }

                if auto_focus {
                    plan = plan.then(Event::FocusFirst);
                }

                Some(plan)
            }

            // ── Deactivation ────────────────────────────────────────────
            (State::Active { .. }, Event::Deactivate { restore_focus }) => {
                let restore = *restore_focus;
                let mut plan =
                    TransitionPlan::to(State::Inactive).cancel_effect(Effect::FocusTrapListener);

                if restore {
                    plan = plan.with_effect(PendingEffect::named(Effect::RestoreFocus));
                } else {
                    plan = plan.apply(|ctx: &mut Context| {
                        ctx.saved_focus = None;
                    });
                }

                Some(plan)
            }

            // ── Trap / Release ──────────────────────────────────────────
            // Adapters drain ALL active effect cleanups when
            // `state_changed` is true (see `ars-leptos::use_machine::
            // handle_effects`). `TrapFocus` enters a state that DOES
            // need the listener, so it must re-emit
            // `Effect::FocusTrapListener` for the adapter to reinstall
            // the trap. `ReleaseTrap` exits the trapped state, so it
            // MUST NOT re-emit the effect — the drain teardown is the
            // intended outcome.
            (State::Active { trapped: false }, Event::TrapFocus) => Some(
                TransitionPlan::to(State::Active { trapped: true })
                    .with_effect(PendingEffect::named(Effect::FocusTrapListener)),
            ),
            (State::Active { trapped: true }, Event::ReleaseTrap) => {
                Some(TransitionPlan::to(State::Active { trapped: false }))
            }

            // ── RestoreFocus ────────────────────────────────────────────
            // While `Active`, `RestoreFocus` is ignored by the wildcard
            // arm at the bottom — restoration is only meaningful after
            // the scope has deactivated.
            (State::Inactive, Event::RestoreFocus) => {
                Some(TransitionPlan::new().with_effect(PendingEffect::named(Effect::RestoreFocus)))
            }

            // ── Focus Navigation ────────────────────────────────────────
            (State::Active { .. }, Event::FocusFirst) => {
                Some(TransitionPlan::new().with_effect(PendingEffect::named(Effect::FocusFirst)))
            }
            (State::Active { .. }, Event::FocusLast) => {
                Some(TransitionPlan::new().with_effect(PendingEffect::named(Effect::FocusLast)))
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
}

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// Anatomy parts of the `FocusScope` component.
#[derive(ars_core::ComponentPart)]
#[scope = "focus-scope"]
pub enum Part {
    /// The container element that scopes focus.
    Container,
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// Connect API for the `FocusScope` component.
pub struct Api<'a> {
    /// The current state of the focus scope.
    state: &'a State,
    /// The context of the focus scope.
    ctx: &'a Context,
    /// The props of the focus scope.
    props: &'a Props,
    /// The send function for the focus scope.
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("focus_scope::Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish_non_exhaustive()
    }
}

impl Api<'_> {
    /// Returns the current [`State`] of the focus scope.
    #[must_use]
    pub const fn state(&self) -> &State {
        self.state
    }

    /// Returns the current [`Context`] of the focus scope.
    #[must_use]
    pub const fn context(&self) -> &Context {
        self.ctx
    }

    /// Returns the [`Props`] used by the focus scope.
    #[must_use]
    pub const fn props(&self) -> &Props {
        self.props
    }

    /// Whether the focus scope is currently active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.state, State::Active { .. })
    }

    /// Whether the focus scope is currently trapping focus.
    #[must_use]
    pub const fn is_trapped(&self) -> bool {
        matches!(self.state, State::Active { trapped: true })
    }

    /// Attributes for the container element that scopes focus.
    ///
    /// Always emits `data-ars-scope="focus-scope"` and
    /// `data-ars-part="container"`. When active, adds `data-ars-active`
    /// and `tabindex="-1"` so the container can be a programmatic focus
    /// target when no tabbable children exist yet. When trapped, adds
    /// `data-ars-trapped`.
    #[must_use]
    pub fn container_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Container.data_attrs();

        attrs.set(scope_attr, scope_val).set(part_attr, part_val);

        if self.is_active() {
            attrs
                .set_bool(HtmlAttr::Data("ars-active"), true)
                .set(HtmlAttr::TabIndex, "-1");
        }

        if self.is_trapped() {
            attrs.set_bool(HtmlAttr::Data("ars-trapped"), true);
        }

        attrs
    }

    /// Imperatively activate the focus scope.
    ///
    /// `saved_focus_id` is the ID of the currently focused element,
    /// captured by the adapter via `PlatformEffects::active_element_id`
    /// before calling this.
    pub fn activate(&self, trapped: bool, saved_focus_id: Option<String>) {
        (self.send)(Event::Activate {
            trapped,
            saved_focus_id,
        });
    }

    /// Imperatively deactivate the focus scope.
    pub fn deactivate(&self, restore_focus: bool) {
        (self.send)(Event::Deactivate { restore_focus });
    }

    /// Request that focus move to the first tabbable descendant of the
    /// container. Sends [`Event::FocusFirst`].
    pub fn focus_first(&self) {
        (self.send)(Event::FocusFirst);
    }

    /// Request that focus move to the last tabbable descendant of the
    /// container. Sends [`Event::FocusLast`].
    pub fn focus_last(&self) {
        (self.send)(Event::FocusLast);
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Container => self.container_attrs(),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use ars_core::{ConnectApi as _, Env, HtmlAttr, Service};
    use insta::assert_snapshot;

    use super::*;

    const SCOPE_ID: &str = "scope";

    fn test_props() -> Props {
        Props::new().id(SCOPE_ID)
    }

    fn service(props: Props) -> Service<Machine> {
        Service::<Machine>::new(props, &Env::default(), &Messages)
    }

    fn effect_names<M: ars_core::Machine>(effects: &[PendingEffect<M>]) -> Vec<M::Effect> {
        effects.iter().map(|effect| effect.name).collect()
    }

    fn snapshot_attrs(attrs: &AttrMap) -> String {
        format!("{attrs:#?}")
    }

    // ── Init & defaults ─────────────────────────────────────────────

    #[test]
    fn init_returns_inactive_state_and_empty_context() {
        let service = service(test_props());

        assert_eq!(service.state(), &State::Inactive);
        assert_eq!(service.context().saved_focus, None);
        assert_eq!(service.context().container_id, None);
    }

    #[test]
    fn props_default_matches_spec_section_1_4() {
        let props = Props::default();

        assert_eq!(props.id, String::new());
        assert!(!props.trapped);
        assert!(!props.contain);
        assert!(
            props.auto_focus,
            "auto_focus defaults to true per spec §1.4"
        );
        assert!(
            props.restore_focus,
            "restore_focus defaults to true per spec §1.4"
        );
    }

    #[test]
    fn props_builder_chains_each_field() {
        let props = Props::new()
            .id("dialog-trap")
            .trapped(true)
            .contain(true)
            .auto_focus(false)
            .restore_focus(false);

        assert_eq!(props.id, "dialog-trap");
        assert!(props.trapped);
        assert!(props.contain);
        assert!(!props.auto_focus);
        assert!(!props.restore_focus);
    }

    #[test]
    fn messages_default_matches_spec_empty_struct() {
        // `Messages` has no fields per spec — empty struct that implements
        // `Default`. The trait round-trip is exercised here via an
        // explicit qualified call to avoid the lint that suggests using
        // the unit literal directly.
        let from_default: Messages = <Messages as Default>::default();

        assert_eq!(from_default, Messages);
    }

    // ── Activate ────────────────────────────────────────────────────

    #[test]
    fn activate_inactive_to_active_with_trapped_from_event() {
        let mut service = service(test_props());

        let result = service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Active { trapped: true });
    }

    #[test]
    fn activate_propagates_contain_prop_into_trapped_when_event_trapped_false() {
        let mut service = service(test_props().contain(true));

        let result = service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        });

        assert!(result.state_changed);
        assert_eq!(
            service.state(),
            &State::Active { trapped: true },
            "contain=true must promote `trapped` even when event passes false"
        );
    }

    #[test]
    fn activate_propagates_trapped_prop_into_trapped_when_event_trapped_false() {
        // Regression guard for Codex review #663 (P2): `Props::trapped`
        // is documented in spec §1.4 as the prop that prevents Tab from
        // escaping. The original `Activate` transition ORed only the
        // event's `trapped` and `props.contain`, silently dropping
        // `props.trapped` and forcing every caller to mirror it into
        // `Event::Activate { trapped: ... }`. Adapters that activate
        // with a constant `false` (or that derive `trapped` solely from
        // a transient interaction state) would have left a
        // `Props { trapped: true, .. }` scope untrapped.
        let mut service = service(test_props().trapped(true));

        let result = service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        });

        assert!(result.state_changed);
        assert_eq!(
            service.state(),
            &State::Active { trapped: true },
            "Props::trapped=true must promote `trapped` even when the event passes false",
        );
    }

    #[test]
    fn activate_stores_saved_focus_id_in_context() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: Some("trigger-7".into()),
        }));

        assert_eq!(
            service.context().saved_focus.as_deref(),
            Some("trigger-7"),
            "Activate must store saved_focus_id into ctx.saved_focus for later RestoreFocus"
        );
    }

    #[test]
    fn activate_emits_focus_trap_listener_effect() {
        let mut service = service(test_props());

        let result = service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        });

        assert!(
            effect_names(&result.pending_effects).contains(&Effect::FocusTrapListener),
            "Activate must emit Effect::FocusTrapListener so the adapter can install the keydown handler"
        );
    }

    #[test]
    fn activate_then_sends_focus_first_when_props_auto_focus_true() {
        let mut service = service(test_props().auto_focus(true));

        let result = service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        });

        let names = effect_names(&result.pending_effects);

        assert!(
            names.contains(&Effect::FocusTrapListener),
            "Activate always emits Effect::FocusTrapListener"
        );
        assert!(
            names.contains(&Effect::FocusFirst),
            "Activate with auto_focus=true must chain FocusFirst (which emits Effect::FocusFirst in the same SendResult)"
        );
    }

    #[test]
    fn activate_does_not_then_send_focus_first_when_props_auto_focus_false() {
        let mut service = service(test_props().auto_focus(false));

        let result = service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        });

        let names = effect_names(&result.pending_effects);

        assert!(
            names.contains(&Effect::FocusTrapListener),
            "Activate always emits Effect::FocusTrapListener"
        );
        assert!(
            !names.contains(&Effect::FocusFirst),
            "Activate with auto_focus=false must NOT chain FocusFirst"
        );
    }

    #[test]
    fn activate_ignored_when_already_active() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        }));

        let result = service.send(Event::Activate {
            trapped: true,
            saved_focus_id: Some("other".into()),
        });

        assert!(
            !result.state_changed,
            "Activate is a no-op when the scope is already Active"
        );
    }

    // ── Deactivate ──────────────────────────────────────────────────

    fn active_service(restore_focus: bool) -> Service<Machine> {
        let mut service = service(test_props().restore_focus(restore_focus));

        drop(service.send(Event::Activate {
            trapped: true,
            saved_focus_id: Some("trigger".into()),
        }));

        service
    }

    #[test]
    fn deactivate_active_to_inactive() {
        let mut service = active_service(true);

        let result = service.send(Event::Deactivate {
            restore_focus: true,
        });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Inactive);
    }

    #[test]
    fn deactivate_with_restore_focus_emits_restore_focus_effect() {
        let mut service = active_service(true);

        let result = service.send(Event::Deactivate {
            restore_focus: true,
        });

        assert!(
            effect_names(&result.pending_effects).contains(&Effect::RestoreFocus),
            "Deactivate{{ restore_focus: true }} must emit Effect::RestoreFocus"
        );
        assert_eq!(
            service.context().saved_focus.as_deref(),
            Some("trigger"),
            "saved_focus must remain in context so the adapter can read it when dispatching the effect"
        );
    }

    #[test]
    fn deactivate_without_restore_focus_clears_saved_focus() {
        let mut service = active_service(false);

        assert_eq!(service.context().saved_focus.as_deref(), Some("trigger"));

        let result = service.send(Event::Deactivate {
            restore_focus: false,
        });

        assert!(
            !effect_names(&result.pending_effects).contains(&Effect::RestoreFocus),
            "Deactivate{{ restore_focus: false }} must NOT emit Effect::RestoreFocus"
        );
        assert_eq!(
            service.context().saved_focus,
            None,
            "Deactivate without restore must clear ctx.saved_focus to drop the stale token"
        );
    }

    #[test]
    fn deactivate_cancels_focus_trap_listener_effect() {
        let mut service = active_service(true);

        let result = service.send(Event::Deactivate {
            restore_focus: true,
        });

        assert!(
            result.cancel_effects.contains(&Effect::FocusTrapListener),
            "Deactivate must cancel Effect::FocusTrapListener so the adapter tears down the keydown handler"
        );
    }

    #[test]
    fn deactivate_ignored_when_already_inactive() {
        let mut service = service(test_props());

        let result = service.send(Event::Deactivate {
            restore_focus: true,
        });

        assert!(!result.state_changed);
    }

    // ── Trap / Release ──────────────────────────────────────────────

    #[test]
    fn trap_focus_toggles_trapped_flag_within_active() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        }));

        assert_eq!(service.state(), &State::Active { trapped: false });

        let result = service.send(Event::TrapFocus);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Active { trapped: true });
    }

    #[test]
    fn release_trap_toggles_trapped_flag_within_active() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        }));

        let result = service.send(Event::ReleaseTrap);

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Active { trapped: false });
    }

    #[test]
    fn trap_focus_re_emits_focus_trap_listener_so_adapter_reinstalls_after_state_change() {
        // Regression guard for Codex review #663 round 1 (P1): adapters
        // drain ALL active effect cleanups whenever `state_changed` is
        // true (see `ars-leptos::use_machine::handle_effects` line 607).
        // `Active{trapped:false} → Active{trapped:true}` enters a state
        // that DOES need the listener, so the transition must re-emit
        // `Effect::FocusTrapListener` to reinstall the trap after the
        // drain, otherwise Tab interception is silently disabled.
        let mut service = service(test_props());
        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        }));

        let result = service.send(Event::TrapFocus);

        assert!(result.state_changed);
        assert!(
            effect_names(&result.pending_effects).contains(&Effect::FocusTrapListener),
            "TrapFocus moves into a trapped state — Effect::FocusTrapListener must \
             be re-emitted so the adapter reinstalls the listener after the drain",
        );
    }

    #[test]
    fn release_trap_does_not_emit_focus_trap_listener_so_drain_can_uninstall() {
        // Regression guard for Codex review #663 round 2 (P1):
        // `Active{trapped:true} → Active{trapped:false}` leaves the
        // scope active but no longer trapping. The adapter's
        // `state_changed`-driven drain runs the listener cleanup, and
        // the transition MUST NOT re-emit `Effect::FocusTrapListener`
        // — otherwise the adapter immediately reinstalls the trap and
        // `ReleaseTrap` becomes a no-op (Tab stays intercepted even
        // though state says it shouldn't).
        let mut service = service(test_props());
        drop(service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        }));

        let result = service.send(Event::ReleaseTrap);

        assert!(result.state_changed);
        assert!(
            !effect_names(&result.pending_effects).contains(&Effect::FocusTrapListener),
            "ReleaseTrap moves out of a trapped state — Effect::FocusTrapListener \
             must NOT be re-emitted, so the drain can uninstall the listener",
        );
    }

    #[test]
    fn activate_does_not_emit_focus_trap_listener_when_resolved_trap_is_false() {
        // Regression guard for Codex review #663 round 2 (P1):
        // Activating into `Active{trapped:false}` (no trap requested by
        // event/props) must NOT install the trap listener — the scope
        // is active for focus-restoration accounting but does not
        // intercept Tab. Emitting the effect would have the adapter
        // attach Tab interception unconditionally via
        // `PlatformEffects::attach_focus_trap`, contradicting the
        // resolved `trapped: false` state.
        let mut service = service(test_props()); // trapped=false, contain=false

        let result = service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        });

        assert!(result.state_changed);
        assert_eq!(service.state(), &State::Active { trapped: false });
        assert!(
            !effect_names(&result.pending_effects).contains(&Effect::FocusTrapListener),
            "Activate into Active{{trapped:false}} must NOT emit \
             Effect::FocusTrapListener — the adapter would otherwise install \
             Tab interception despite the resolved untrapped state",
        );
    }

    #[test]
    fn trap_focus_ignored_when_already_trapped() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        }));

        let result = service.send(Event::TrapFocus);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Active { trapped: true });
    }

    #[test]
    fn release_trap_ignored_when_not_trapped() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        }));

        let result = service.send(Event::ReleaseTrap);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Active { trapped: false });
    }

    #[test]
    fn trap_focus_ignored_when_inactive() {
        let mut service = service(test_props());

        let result = service.send(Event::TrapFocus);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Inactive);
    }

    #[test]
    fn release_trap_ignored_when_inactive() {
        let mut service = service(test_props());

        let result = service.send(Event::ReleaseTrap);

        assert!(!result.state_changed);
        assert_eq!(service.state(), &State::Inactive);
    }

    // ── RestoreFocus ────────────────────────────────────────────────

    #[test]
    fn restore_focus_when_inactive_emits_restore_focus_effect() {
        let mut service = service(test_props());

        let result = service.send(Event::RestoreFocus);

        assert!(
            effect_names(&result.pending_effects).contains(&Effect::RestoreFocus),
            "RestoreFocus from Inactive must emit Effect::RestoreFocus for the adapter to dispatch"
        );
    }

    #[test]
    fn restore_focus_when_active_returns_none_transition() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: Some("trigger".into()),
        }));

        let result = service.send(Event::RestoreFocus);

        assert!(
            !result.state_changed && result.pending_effects.is_empty(),
            "RestoreFocus while Active must be ignored — restoration only makes sense after Deactivate"
        );
    }

    // ── FocusFirst / FocusLast ──────────────────────────────────────

    #[test]
    fn focus_first_when_active_emits_focus_first_effect() {
        let mut service = service(test_props().auto_focus(false));

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        }));

        let result = service.send(Event::FocusFirst);

        assert!(effect_names(&result.pending_effects).contains(&Effect::FocusFirst));
    }

    #[test]
    fn focus_last_when_active_emits_focus_last_effect() {
        let mut service = service(test_props().auto_focus(false));

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        }));

        let result = service.send(Event::FocusLast);

        assert!(effect_names(&result.pending_effects).contains(&Effect::FocusLast));
    }

    #[test]
    fn focus_first_when_inactive_returns_none_transition() {
        let mut service = service(test_props());

        let result = service.send(Event::FocusFirst);

        assert!(!result.state_changed && result.pending_effects.is_empty());
    }

    #[test]
    fn focus_last_when_inactive_returns_none_transition() {
        let mut service = service(test_props());

        let result = service.send(Event::FocusLast);

        assert!(!result.state_changed && result.pending_effects.is_empty());
    }

    // ── Connect API: container_attrs ────────────────────────────────

    #[test]
    fn container_attrs_inactive_sets_scope_and_part_only() {
        let service = service(test_props());

        let attrs = service.connect(&|_| {}).container_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-scope")), Some("focus-scope"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-part")), Some("container"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-active")), None);
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-trapped")), None);
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), None);
    }

    #[test]
    fn container_attrs_active_adds_data_active_and_tabindex_minus_one() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        }));

        let attrs = service.connect(&|_| {}).container_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-active")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("-1"));
        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-trapped")),
            None,
            "untrapped active scope must not advertise data-ars-trapped"
        );
    }

    #[test]
    fn container_attrs_active_trapped_adds_data_trapped() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        }));

        let attrs = service.connect(&|_| {}).container_attrs();

        assert_eq!(attrs.get(&HtmlAttr::Data("ars-active")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::Data("ars-trapped")), Some("true"));
        assert_eq!(attrs.get(&HtmlAttr::TabIndex), Some("-1"));
    }

    #[test]
    fn container_attrs_never_emits_data_ars_focus_trapped_typo() {
        // Issue #212 test 9 says `data-ars-focus-trapped`; the spec §2 says
        // `data-ars-trapped`. The spec is authoritative — this regression
        // guard pins the spec-correct name.
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        }));

        let attrs = service.connect(&|_| {}).container_attrs();

        assert_eq!(
            attrs.get(&HtmlAttr::Data("ars-focus-trapped")),
            None,
            "spec §2 names the attribute `data-ars-trapped`, not `data-ars-focus-trapped`"
        );
    }

    #[test]
    fn connect_part_attrs_for_container_matches_container_attrs() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        }));

        let api = service.connect(&|_| {});
        let container = api.container_attrs();
        let via_part = api.part_attrs(Part::Container);

        assert_eq!(container, via_part);
    }

    // ── is_active / is_trapped derivations ──────────────────────────

    #[test]
    fn is_active_and_is_trapped_derived_from_state() {
        let mut service = service(test_props());

        {
            let api = service.connect(&|_| {});
            assert!(!api.is_active());
            assert!(!api.is_trapped());
        }

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        }));

        {
            let api = service.connect(&|_| {});

            assert!(api.is_active());
            assert!(!api.is_trapped(), "Active{{trapped:false}} is not trapped");
        }

        drop(service.send(Event::TrapFocus));

        {
            let api = service.connect(&|_| {});

            assert!(api.is_active());
            assert!(api.is_trapped());
        }
    }

    // ── Api imperative methods send the expected events ─────────────

    use core::cell::RefCell;

    fn capture<F: FnOnce(&dyn Fn(Event))>(invoke: F) -> Vec<Event> {
        let captured: RefCell<Vec<Event>> = RefCell::new(Vec::new());
        let sender = |event: Event| captured.borrow_mut().push(event);

        invoke(&sender);

        captured.into_inner()
    }

    fn api_with<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Api<'a> {
        Api {
            state,
            ctx,
            props,
            send,
        }
    }

    #[test]
    fn api_activate_sends_activate_event_with_args() {
        let state = State::Inactive;
        let ctx = Context::default();
        let props = test_props();

        let events = capture(|sender| {
            let api = api_with(&state, &ctx, &props, sender);

            api.activate(true, Some("trigger".into()));
        });

        assert_eq!(
            events,
            vec![Event::Activate {
                trapped: true,
                saved_focus_id: Some("trigger".into()),
            }]
        );
    }

    #[test]
    fn api_deactivate_sends_deactivate_event_with_restore_focus() {
        let state = State::Active { trapped: true };
        let ctx = Context::default();
        let props = test_props();

        let events = capture(|sender| {
            let api = api_with(&state, &ctx, &props, sender);

            api.deactivate(false);
        });

        assert_eq!(
            events,
            vec![Event::Deactivate {
                restore_focus: false
            }]
        );
    }

    #[test]
    fn api_focus_first_sends_focus_first_event() {
        let state = State::Active { trapped: true };
        let ctx = Context::default();
        let props = test_props();

        let events = capture(|sender| {
            let api = api_with(&state, &ctx, &props, sender);

            api.focus_first();
        });

        assert_eq!(events, vec![Event::FocusFirst]);
    }

    #[test]
    fn api_focus_last_sends_focus_last_event() {
        let state = State::Active { trapped: true };
        let ctx = Context::default();
        let props = test_props();

        let events = capture(|sender| {
            let api = api_with(&state, &ctx, &props, sender);

            api.focus_last();
        });

        assert_eq!(events, vec![Event::FocusLast]);
    }

    #[test]
    fn api_state_context_props_accessors_return_underlying_references() {
        let mut service = service(test_props().contain(true));

        drop(service.send(Event::Activate {
            trapped: true,
            saved_focus_id: Some("trigger".into()),
        }));

        let api = service.connect(&|_| {});

        assert_eq!(api.state(), &State::Active { trapped: true });
        assert_eq!(api.context().saved_focus.as_deref(), Some("trigger"));
        assert!(api.props().contain);
    }

    #[test]
    fn api_debug_impl_is_non_exhaustive_and_includes_label() {
        let state = State::Active { trapped: true };
        let ctx = Context::default();
        let props = test_props();

        let api = api_with(&state, &ctx, &props, &|_| {});

        let formatted = format!("{api:?}");

        assert!(
            formatted.contains("focus_scope::Api"),
            "Debug must include the type label, got: {formatted}",
        );
        assert!(
            formatted.contains(".."),
            "Debug must be non-exhaustive (hides the send closure)",
        );
    }

    // ── Snapshot tests ──────────────────────────────────────────────

    #[test]
    fn container_attrs_inactive_default_props_snapshot() {
        let service = service(test_props());

        assert_snapshot!(
            "container_attrs_inactive",
            snapshot_attrs(&service.connect(&|_| {}).container_attrs())
        );
    }

    #[test]
    fn container_attrs_active_untrapped_snapshot() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        }));

        assert_snapshot!(
            "container_attrs_active_untrapped",
            snapshot_attrs(&service.connect(&|_| {}).container_attrs())
        );
    }

    #[test]
    fn container_attrs_active_trapped_snapshot() {
        let mut service = service(test_props());

        drop(service.send(Event::Activate {
            trapped: true,
            saved_focus_id: None,
        }));

        assert_snapshot!(
            "container_attrs_active_trapped",
            snapshot_attrs(&service.connect(&|_| {}).container_attrs())
        );
    }

    #[test]
    fn container_attrs_contain_prop_promotes_trapped_snapshot() {
        let mut service = service(test_props().contain(true));

        drop(service.send(Event::Activate {
            trapped: false,
            saved_focus_id: None,
        }));

        assert_snapshot!(
            "container_attrs_contain_prop_promotes_trapped",
            snapshot_attrs(&service.connect(&|_| {}).container_attrs())
        );
    }
}
