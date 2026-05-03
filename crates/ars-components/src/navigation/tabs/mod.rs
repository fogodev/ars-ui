//! Tabs navigation component machine.
//!
//! Owns the selected tab key, the focused tab key, the activation mode
//! (automatic vs. manual), the layout orientation, the resolved text
//! direction, the loop-focus toggle, the per-tab disabled flags, the per-tab
//! closable flags, the ordered list of registered tab keys, and the
//! localized accessibility messages.
//!
//! The agnostic core never moves DOM focus, never measures the indicator,
//! never observes panel mounts, and never calls
//! [`PlatformEffects::focus_element_by_id`](ars_core::PlatformEffects::focus_element_by_id)
//! directly. Live focus is signalled through the typed [`Effect`] enum so
//! the framework adapter can dispatch on the named intent and run the
//! appropriate platform call against its own element handles
//! (Leptos `NodeRef`, Dioxus `MountedData`).
//!
//! Modality (the keyboard-vs-pointer bit that drives `data-ars-focus-visible`)
//! is sourced from [`ars_core::ModalityContext`] at the adapter layer; the
//! agnostic core does not duplicate it. [`Api::tab_attrs`] takes a
//! `focus_visible: bool` parameter so adapters can thread their per-render
//! modality snapshot into the rendered ARIA attributes without polluting
//! `Context` with stale state.
//!
//! Tab registration is event-driven: adapters dispatch
//! [`Event::SetTabs`] whenever the rendered tab list changes (typically once
//! per render, gated on a previous-list diff). The machine is the source of
//! truth for `tabs` and `closable_tabs`; consumers never mutate them
//! directly.
//!
//! See `spec/components/navigation/tabs.md` §1 for the full contract,
//! plus §5 (Closable variant) and §6 (Reorderable variant) which are
//! implemented in this module.

use alloc::{
    collections::BTreeSet,
    format,
    string::{String, ToString as _},
    vec::Vec,
};
use core::fmt::{self, Debug};

use ars_collections::Key;
use ars_core::{
    AriaAttr, AttrMap, Bindable, ComponentIds, ComponentMessages, ComponentPart, ConnectApi,
    Direction, Env, HtmlAttr, KeyboardKey, Locale, MessageFn, Orientation, PendingEffect,
    TransitionPlan,
};
use ars_interactions::KeyboardEventData;

// ────────────────────────────────────────────────────────────────────
// State
// ────────────────────────────────────────────────────────────────────

/// States for the [`Tabs`](self) component.
///
/// Tabs is always in exactly one of the two states below; selection is
/// tracked separately in [`Context::value`] so a panel remains visible
/// while keyboard focus leaves the tab list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    /// No tab has keyboard focus.
    #[default]
    Idle,

    /// A tab button has keyboard focus.
    Focused {
        /// The key of the tab that currently has keyboard focus.
        tab: Key,
    },
}

// ────────────────────────────────────────────────────────────────────
// Event
// ────────────────────────────────────────────────────────────────────

/// Events accepted by the [`Tabs`](self) state machine.
///
/// Includes the base events from spec §1.2, the tab-list registration
/// events, the `CloseTab` event from the Closable variant (§5.2), and the
/// `ReorderTab` event from the Reorderable variant (§6.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Activate a tab and reveal its associated panel. Disabled tabs and
    /// re-selecting the already-active tab are guarded out by
    /// [`Machine::transition`](ars_core::Machine::transition).
    SelectTab(Key),

    /// A tab received DOM focus from outside the keyboard navigation flow
    /// (e.g. pointer interaction, programmatic focus, screen-reader
    /// virtual cursor).
    ///
    /// Adapters dispatch this from the DOM `focus` event handler. When
    /// activation mode is [`ActivationMode::Automatic`] the transition
    /// also advances [`Context::value`].
    Focus(Key),

    /// Focus left the tab list. Clears [`Context::focused_tab`] and
    /// transitions to [`State::Idle`].
    Blur,

    /// Move focus to the next non-disabled tab in DOM order.
    FocusNext,

    /// Move focus to the previous non-disabled tab in DOM order.
    FocusPrev,

    /// Move focus to the first non-disabled tab.
    FocusFirst,

    /// Move focus to the last non-disabled tab.
    FocusLast,

    /// Adapter notification that the live `direction` CSS property has been
    /// resolved on the tab list element. Replaces [`Context::dir`] with the
    /// concrete value and is normally emitted once on mount when
    /// [`Props::dir`] was [`Direction::Auto`]. Idempotent — sending the
    /// same direction twice produces no transition.
    SetDirection(Direction),

    /// Replace the registered tab list. Adapters call this whenever their
    /// rendered tab triggers change (mount, unmount, reorder, close).
    /// The transition replaces [`Context::tabs`] and [`Context::closable_tabs`]
    /// in one shot — duplicate keys are deduplicated (the first occurrence
    /// wins) — then snaps [`Context::value`] / [`Context::focused_tab`]
    /// back into the new list (see §1.5 selection invariant).
    SetTabs(Vec<TabRegistration>),

    /// Re-apply context-backed [`Props`] fields after a prop change.
    /// Adapters dispatch this from [`Machine::on_props_changed`] when
    /// any of `orientation`, `activation_mode`, `dir`, `loop_focus`, or
    /// `disabled_keys` differs between old and new props. The transition
    /// is context-only (no state change, no effects); subsequent
    /// state-flipping transitions emit effects against the freshly-synced
    /// context. After a `disabled_keys` change the transition also
    /// re-runs the selection invariant — `value` / `focused_tab` snap
    /// to the first non-disabled key when they now point at a disabled
    /// tab.
    SyncProps,

    /// User asked to close the given tab. Closable variant (spec §5).
    ///
    /// **Pure notification** — the machine does NOT mutate
    /// [`Context::tabs`] or [`Context::value`]. Consumers call
    /// [`Api::successor_for_close`] / [`Api::can_close_tab`] from their
    /// `CloseTab` handler to apply the deterministic successor algorithm
    /// to their own tab-list source, then dispatch
    /// [`Event::SetTabs`] / [`Event::SelectTab`] to commit the change.
    CloseTab(Key),

    /// User asked to move the given tab to a new index. Reorderable variant
    /// (spec §6).
    ///
    /// **Pure notification** — per spec §6.3 the machine does NOT mutate
    /// [`Context::tabs`]. Consumers reorder their tab-list source and
    /// re-register via [`Event::SetTabs`].
    ReorderTab {
        /// The key of the tab being moved.
        tab: Key,

        /// The target zero-based index in the tab list.
        new_index: usize,
    },
}

// ────────────────────────────────────────────────────────────────────
// ActivationMode
// ────────────────────────────────────────────────────────────────────

/// How keyboard focus interacts with selection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivationMode {
    /// Focusing a tab via keyboard immediately selects it.
    #[default]
    Automatic,

    /// Focusing a tab via keyboard only moves the focus ring; the user
    /// must press `Enter` or `Space` to confirm selection.
    Manual,
}

// ────────────────────────────────────────────────────────────────────
// TabRegistration
// ────────────────────────────────────────────────────────────────────

/// Adapter-supplied registration entry for a single tab.
///
/// Used as the payload of [`Event::SetTabs`] so a single bulk-replace
/// dispatch atomically updates [`Context::tabs`] and
/// [`Context::closable_tabs`]. Tab labels are intentionally NOT included —
/// labels are a render concern owned by the adapter / consumer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabRegistration {
    /// Stable identifier for the tab. Must be unique within the list.
    pub key: Key,

    /// When `true`, adapters render a close button inside this tab and
    /// the agnostic core forwards `Delete` / `Backspace` keystrokes as
    /// [`Event::CloseTab`]. Non-closable tabs ignore the keystrokes.
    pub closable: bool,
}

impl TabRegistration {
    /// Builds a registration for a non-closable tab.
    #[must_use]
    pub const fn new(key: Key) -> Self {
        Self {
            key,
            closable: false,
        }
    }

    /// Builds a registration for a closable tab.
    #[must_use]
    pub const fn closable(key: Key) -> Self {
        Self {
            key,
            closable: true,
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Messages
// ────────────────────────────────────────────────────────────────────

/// Closure signature backing [`Messages::close_tab_label`].
///
/// Receives the parent tab's visible label and the active locale, and
/// returns the accessible name rendered on the close button
/// (e.g. `"Close Inbox"`).
pub type CloseTabLabelFn = dyn Fn(&str, &Locale) -> String + Send + Sync;

/// Closure signature backing [`Messages::reorder_announce_label`].
///
/// Receives the moved tab's visible label, the new 1-based position,
/// the total tab count, and the active locale, and returns the
/// `LiveAnnouncer` announcement text
/// (e.g. `"Inbox moved to position 2 of 5"`).
pub type ReorderAnnounceLabelFn = dyn Fn(&str, usize, usize, &Locale) -> String + Send + Sync;

/// Localizable strings for [`Tabs`](self).
///
/// `close_tab_label` is the accessible name for the close button rendered
/// inside a closable tab (default English template: `"Close {label}"`).
/// `reorder_announce_label` is the `LiveAnnouncer` text emitted by adapters
/// after a keyboard reorder (default: `"{label} moved to position {n} of {total}"`).
/// The agnostic core never invokes `reorder_announce_label` itself —
/// announcing is an adapter concern — but exposing it on [`Messages`]
/// keeps i18n centralized.
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Builds the accessible name for a tab's close button. See
    /// [`CloseTabLabelFn`] for the closure signature.
    pub close_tab_label: MessageFn<CloseTabLabelFn>,

    /// Builds the `LiveAnnouncer` text emitted after a keyboard-driven
    /// reorder. See [`ReorderAnnounceLabelFn`] for the closure
    /// signature.
    pub reorder_announce_label: MessageFn<ReorderAnnounceLabelFn>,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            close_tab_label: MessageFn::new(|label: &str, _locale: &Locale| {
                format!("Close {label}")
            }),
            reorder_announce_label: MessageFn::new(
                |label: &str, position: usize, total: usize, _locale: &Locale| {
                    format!("{label} moved to position {position} of {total}")
                },
            ),
        }
    }
}

impl ComponentMessages for Messages {}

// ────────────────────────────────────────────────────────────────────
// Props
// ────────────────────────────────────────────────────────────────────

/// Immutable configuration for a [`Tabs`](self) instance.
///
/// `Props` is constructed once by the consumer; runtime updates that drive
/// [`Context`] should be replicated by sending the matching [`Event`]
/// (e.g. [`Event::SetDirection`] for [`Direction`] resolution,
/// [`Event::SetTabs`] for tab-list changes).
#[derive(Clone, Debug, PartialEq, Eq, ars_core::HasId)]
pub struct Props {
    /// Component instance id. Required (used as the prefix for the
    /// generated `tablist` DOM id).
    pub id: String,

    /// Controlled selected-tab key. When `Some`, overrides
    /// [`default_value`](Self::default_value); the consumer is responsible
    /// for syncing changes via [`Bindable::sync_controlled`]. The outer
    /// `Option` represents "no tab selected" (e.g. an empty tab list).
    pub value: Option<Option<Key>>,

    /// Initial selected-tab key in uncontrolled mode. `None` means the
    /// component boots with no selection (typical for empty / lazy tab
    /// lists).
    pub default_value: Option<Key>,

    /// Tab list orientation. Default [`Orientation::Horizontal`].
    pub orientation: Orientation,

    /// How keyboard focus interacts with selection. Default
    /// [`ActivationMode::Automatic`].
    pub activation_mode: ActivationMode,

    /// Text direction. Default [`Direction::Ltr`]. `Auto` is resolved at
    /// mount time by the adapter via [`Event::SetDirection`].
    pub dir: Direction,

    /// When `true`, arrow-key focus wraps from last to first and vice
    /// versa. Default `true`.
    pub loop_focus: bool,

    /// When `true`, [`Api::can_close_tab`] returns `false` for the only
    /// remaining tab so consumers refuse the close. Default `false`.
    pub disallow_empty_selection: bool,

    /// When `true`, panels are not rendered until their tab is first
    /// activated. Adapter-only hint; the agnostic core does not consume
    /// this field. Default `false`.
    pub lazy_mount: bool,

    /// When `true`, panels are removed from the DOM when their tab is
    /// deselected (composes with `Presence` for exit animations).
    /// Adapter-only hint; the agnostic core does not consume this field.
    /// Default `false`.
    pub unmount_on_exit: bool,

    /// Set of keys for tabs that are disabled. Disabled tabs render with
    /// `aria-disabled="true"` and `data-ars-disabled`, are skipped during
    /// arrow-key navigation, and cannot be activated via click or
    /// keyboard. The HTML `disabled` attribute is intentionally NOT set
    /// so disabled tabs remain in the focus order for screen-reader
    /// discoverability.
    pub disabled_keys: BTreeSet<Key>,

    /// When `true`, tabs may be reordered by drag-and-drop or keyboard
    /// shortcuts (Ctrl+Arrow on the orientation axis). Default `false`.
    /// See spec §6.
    pub reorderable: bool,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: None,
            orientation: Orientation::Horizontal,
            activation_mode: ActivationMode::Automatic,
            dir: Direction::Ltr,
            loop_focus: true,
            disallow_empty_selection: false,
            lazy_mount: false,
            unmount_on_exit: false,
            disabled_keys: BTreeSet::new(),
            reorderable: false,
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

    /// Sets [`value`](Self::value) (controlled selected tab).
    #[must_use]
    pub fn value(mut self, value: Option<Option<Key>>) -> Self {
        self.value = value;
        self
    }

    /// Sets [`default_value`](Self::default_value).
    #[must_use]
    pub fn default_value(mut self, value: Option<Key>) -> Self {
        self.default_value = value;
        self
    }

    /// Sets [`orientation`](Self::orientation).
    #[must_use]
    pub const fn orientation(mut self, value: Orientation) -> Self {
        self.orientation = value;
        self
    }

    /// Sets [`activation_mode`](Self::activation_mode).
    #[must_use]
    pub const fn activation_mode(mut self, value: ActivationMode) -> Self {
        self.activation_mode = value;
        self
    }

    /// Sets [`dir`](Self::dir).
    #[must_use]
    pub const fn dir(mut self, value: Direction) -> Self {
        self.dir = value;
        self
    }

    /// Sets [`loop_focus`](Self::loop_focus).
    #[must_use]
    pub const fn loop_focus(mut self, value: bool) -> Self {
        self.loop_focus = value;
        self
    }

    /// Sets [`disallow_empty_selection`](Self::disallow_empty_selection).
    #[must_use]
    pub const fn disallow_empty_selection(mut self, value: bool) -> Self {
        self.disallow_empty_selection = value;
        self
    }

    /// Sets [`lazy_mount`](Self::lazy_mount).
    #[must_use]
    pub const fn lazy_mount(mut self, value: bool) -> Self {
        self.lazy_mount = value;
        self
    }

    /// Sets [`unmount_on_exit`](Self::unmount_on_exit).
    #[must_use]
    pub const fn unmount_on_exit(mut self, value: bool) -> Self {
        self.unmount_on_exit = value;
        self
    }

    /// Sets [`disabled_keys`](Self::disabled_keys).
    #[must_use]
    pub fn disabled_keys(mut self, value: BTreeSet<Key>) -> Self {
        self.disabled_keys = value;
        self
    }

    /// Sets [`reorderable`](Self::reorderable).
    #[must_use]
    pub const fn reorderable(mut self, value: bool) -> Self {
        self.reorderable = value;
        self
    }
}

// ────────────────────────────────────────────────────────────────────
// Context
// ────────────────────────────────────────────────────────────────────

/// Runtime context for [`Tabs`](self).
///
/// `tabs` and `closable_tabs` are adapter-driven via [`Event::SetTabs`]
/// (consumers never mutate them directly). The agnostic core treats
/// `tabs` as the authoritative ordered key list for arrow-key navigation,
/// the close-tab successor algorithm, and the reorder index.
///
/// `Context` does not derive [`Eq`]; [`Messages`] (containing
/// [`MessageFn`]) only implements [`PartialEq`] (via `Arc::ptr_eq`),
/// not [`Eq`]. This matches Dialog's pattern.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Selected tab. `None` when no tab is active (e.g. empty tab list).
    /// Controlled or uncontrolled per [`Props::value`] /
    /// [`Props::default_value`].
    pub value: Bindable<Option<Key>>,

    /// Tab key that currently has keyboard focus, if any. May differ from
    /// the selected tab in [`ActivationMode::Manual`].
    pub focused_tab: Option<Key>,

    /// Layout orientation (mirrors [`Props::orientation`]).
    pub orientation: Orientation,

    /// How keyboard focus interacts with selection (mirrors
    /// [`Props::activation_mode`]).
    pub activation_mode: ActivationMode,

    /// Text direction. Mirrors [`Props::dir`] at init and is replaced by
    /// [`Event::SetDirection`] when the adapter resolves `Auto` to a
    /// concrete value.
    pub dir: Direction,

    /// Mirrors [`Props::loop_focus`].
    pub loop_focus: bool,

    /// Set of disabled tab keys. Built from [`Props::disabled_keys`]
    /// at init and re-applied by [`Event::SyncProps`] when the consumer
    /// updates `disabled_keys` at runtime.
    pub disabled_tabs: BTreeSet<Key>,

    /// Set of tab keys whose [`TabRegistration::closable`] flag was
    /// `true` at registration time. The `Delete` / `Backspace` keyboard
    /// shortcuts and the close-trigger handler check this set before
    /// dispatching [`Event::CloseTab`].
    pub closable_tabs: BTreeSet<Key>,

    /// Hydration-stable IDs derived from [`Props::id`]. The tab list's
    /// DOM id is `ids.part("tablist")`; the per-tab DOM id is
    /// `ids.item("tab", &tab_key)`; the per-panel DOM id is
    /// `ids.item("panel", &tab_key)`. ARIA wiring (`aria-controls`,
    /// `aria-labelledby`) reads from the same `item(...)` lookup so
    /// adapters never duplicate ID derivation logic.
    pub ids: ComponentIds,

    /// Registered tab keys in DOM order. Updated atomically by
    /// [`Event::SetTabs`].
    pub tabs: Vec<Key>,

    /// Active locale resolved from [`Env`].
    pub locale: Locale,

    /// Localized message bundle.
    pub messages: Messages,
}

// ────────────────────────────────────────────────────────────────────
// Part
// ────────────────────────────────────────────────────────────────────

/// Anatomy parts exposed by the [`Tabs`](self) connect API.
///
/// `TabCloseTrigger` (rather than `CloseTrigger`) so the kebab-cased
/// `data-ars-part` value is `"tab-close-trigger"` — matching the spec
/// §5.4 anatomy table and avoiding visual collisions with Dialog's /
/// Popover's `close-trigger` data-attribute when downstream stylesheets
/// write scope-agnostic selectors.
#[derive(ComponentPart)]
#[scope = "tabs"]
pub enum Part {
    /// The outer root container that scopes orientation and direction.
    Root,

    /// The tab list (`role="tablist"`) wrapping the tab triggers.
    List,

    /// A single tab trigger (`role="tab"`).
    Tab {
        /// The key identifying this tab. The DOM `id` of the trigger
        /// and the matching `aria-controls` panel target are derived
        /// from [`Context::ids`] via `ids.item("tab", &tab_key)` and
        /// `ids.item("panel", &tab_key)` respectively.
        tab_key: Key,
    },

    /// The animated selection-indicator bar (`aria-hidden="true"`).
    TabIndicator,

    /// A tab panel (`role="tabpanel"`) associated with a tab trigger.
    Panel {
        /// The key of the tab that controls this panel. The DOM `id`
        /// is derived from [`Context::ids`] via
        /// `ids.item("panel", &tab_key)`.
        tab_key: Key,

        /// Optional fallback `aria-label` used when the corresponding tab
        /// has no visible text (icon-only tabs).
        tab_label: Option<String>,
    },

    /// A close button rendered inside a closable tab.
    /// Emits `data-ars-part="tab-close-trigger"` (kebab-cased variant
    /// name).
    TabCloseTrigger {
        /// The visible label of the parent tab. Used to build the
        /// accessible name via [`Messages::close_tab_label`].
        tab_label: String,
    },
}

// ────────────────────────────────────────────────────────────────────
// Effect
// ────────────────────────────────────────────────────────────────────

/// Typed identifier for every named effect intent the tabs machine emits.
///
/// Adapters dispatch on the variant exhaustively and run the matching
/// platform call. The variant payload is intentionally unit — adapters
/// read [`Context::focused_tab`] for the actual focus target so the
/// effect identifier stays `Copy + Eq + Hash`. This matches the codebase
/// convention used by `dialog::Effect` and `popover::Effect`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Adapter must move DOM focus to the tab whose key is currently
    /// stored in [`Context::focused_tab`]. Emitted on
    /// [`Event::FocusNext`], [`Event::FocusPrev`], [`Event::FocusFirst`],
    /// [`Event::FocusLast`], and the `Idle → Focused` bootstrap arm.
    FocusFocusedTab,
}

// ────────────────────────────────────────────────────────────────────
// Machine
// ────────────────────────────────────────────────────────────────────

/// State machine for the [`Tabs`](self) component.
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
        props: &Self::Props,
        env: &Env,
        messages: &Self::Messages,
    ) -> (Self::State, Self::Context) {
        let value = if let Some(initial) = &props.value {
            Bindable::controlled(initial.clone())
        } else {
            Bindable::uncontrolled(props.default_value.clone())
        };

        (
            State::Idle,
            Context {
                value,
                focused_tab: None,
                orientation: props.orientation,
                activation_mode: props.activation_mode,
                dir: props.dir,
                loop_focus: props.loop_focus,
                disabled_tabs: props.disabled_keys.clone(),
                closable_tabs: BTreeSet::new(),
                ids: ComponentIds::from_id(&props.id),
                tabs: Vec::new(),
                locale: env.locale.clone(),
                messages: messages.clone(),
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // ── SelectTab ────────────────────────────────────────────
            (_, Event::SelectTab(tab)) => {
                if !is_registered(ctx, tab)
                    || is_disabled(ctx, tab)
                    || ctx.value.get().as_ref() == Some(tab)
                {
                    return None;
                }

                let next_state = State::Focused { tab: tab.clone() };

                Some(TransitionPlan::to(next_state).apply({
                    let tab = tab.clone();
                    move |ctx: &mut Context| {
                        ctx.value.set(Some(tab.clone()));
                        ctx.focused_tab = Some(tab);
                    }
                }))
            }

            // ── Focus ────────────────────────────────────────────────
            // DOM focus arrived from outside the keyboard navigation
            // flow. Idempotent — re-firing the event for an already-
            // focused tab produces no transition. Unknown / disabled
            // keys are rejected so a stale focus event after `SetTabs`
            // can't desync `focused_tab` from the rendered list.
            (_, Event::Focus(tab)) => {
                if !is_registered(ctx, tab) || is_disabled(ctx, tab) {
                    return None;
                }

                let already_focused = ctx.focused_tab.as_ref() == Some(tab);
                let auto = ctx.activation_mode == ActivationMode::Automatic;
                let value_already_set = ctx.value.get().as_ref() == Some(tab);

                if already_focused && (!auto || value_already_set) {
                    return None;
                }

                let next_state = State::Focused { tab: tab.clone() };

                Some(TransitionPlan::to(next_state).apply({
                    let tab = tab.clone();
                    move |ctx: &mut Context| {
                        ctx.focused_tab = Some(tab.clone());
                        if auto {
                            ctx.value.set(Some(tab));
                        }
                    }
                }))
            }

            // ── Blur ─────────────────────────────────────────────────
            (_, Event::Blur) => {
                if matches!(state, State::Idle) && ctx.focused_tab.is_none() {
                    return None;
                }

                Some(TransitionPlan::to(State::Idle).apply(|ctx: &mut Context| {
                    ctx.focused_tab = None;
                }))
            }

            // ── FocusNext / FocusPrev (Idle bootstrap) ───────────────
            (State::Idle, Event::FocusNext | Event::FocusPrev) => {
                let target = ctx.value.get().clone()?;

                if !ctx.tabs.iter().any(|key| key == &target) || is_disabled(ctx, &target) {
                    return None;
                }

                let next_state = State::Focused {
                    tab: target.clone(),
                };

                Some(
                    TransitionPlan::to(next_state)
                        .apply(move |ctx: &mut Context| {
                            ctx.focused_tab = Some(target);
                        })
                        .with_effect(PendingEffect::named(Effect::FocusFocusedTab)),
                )
            }

            // ── FocusNext (Focused) ──────────────────────────────────
            (State::Focused { tab }, Event::FocusNext) => {
                let next = step_focus(ctx, tab, FocusStep::Next)?;
                Some(focus_to(ctx, next))
            }

            // ── FocusPrev (Focused) ──────────────────────────────────
            (State::Focused { tab }, Event::FocusPrev) => {
                let prev = step_focus(ctx, tab, FocusStep::Prev)?;
                Some(focus_to(ctx, prev))
            }

            // ── FocusFirst ───────────────────────────────────────────
            (_, Event::FocusFirst) => {
                let first = ctx
                    .tabs
                    .iter()
                    .find(|tab| !is_disabled(ctx, tab))
                    .cloned()?;
                Some(focus_to(ctx, first))
            }

            // ── FocusLast ────────────────────────────────────────────
            (_, Event::FocusLast) => {
                let last = ctx
                    .tabs
                    .iter()
                    .rev()
                    .find(|tab| !is_disabled(ctx, tab))
                    .cloned()?;
                Some(focus_to(ctx, last))
            }

            // ── SetDirection ─────────────────────────────────────────
            (_, Event::SetDirection(dir)) => {
                let dir = *dir;

                if ctx.dir == dir {
                    return None;
                }

                Some(TransitionPlan::context_only(move |ctx: &mut Context| {
                    ctx.dir = dir;
                }))
            }

            // ── SetTabs ──────────────────────────────────────────────
            // Replaces the registered tab list and re-establishes the
            // selection invariant. Duplicate keys are deduplicated —
            // the first occurrence wins (later occurrences are
            // discarded) so a buggy consumer can't silently desync the
            // ordered Vec from the closable BTreeSet. When the snap
            // would clear the focused tab from a `Focused { tab }`
            // state, we also drop state back to `Idle` so the
            // State→focused_tab invariant survives — `State::Focused
            // { tab: ghost }` is not a valid resting state.
            (_, Event::SetTabs(registrations)) => Some(set_tabs_plan(state, ctx, registrations)),

            // ── SyncProps ────────────────────────────────────────────
            // Replays context-backed prop fields. Captured by-copy/move
            // so the agnostic core does not retain `&props`. After a
            // `disabled_keys` rebuild the selection invariant re-runs
            // because a now-disabled `value` / `focused_tab` must snap
            // to a still-valid key. Same `Focused → Idle` downgrade as
            // SetTabs when the rebuild renders the focused tab
            // disabled.
            (_, Event::SyncProps) => Some(sync_props_plan(state, ctx, props)),

            // ── CloseTab — pure notification (§5.3) ──────────────────
            (_, Event::CloseTab(_)) => Some(TransitionPlan::context_only(|_| {})),

            // ── ReorderTab — pure notification (§6.3) ────────────────
            (_, Event::ReorderTab { .. }) => Some(TransitionPlan::context_only(|_| {})),
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
        if context_relevant_props_changed(old, new) {
            alloc::vec![Event::SyncProps]
        } else {
            Vec::new()
        }
    }
}

/// Returns `true` when any context-backed non-`value` prop differs
/// between `old` and `new`. Used by [`Machine::on_props_changed`] to
/// decide whether to emit [`Event::SyncProps`]. The controlled-`value`
/// path goes through [`Bindable::sync_controlled`] (the adapter's
/// responsibility), not this trigger.
fn context_relevant_props_changed(old: &Props, new: &Props) -> bool {
    old.orientation != new.orientation
        || old.activation_mode != new.activation_mode
        || old.dir != new.dir
        || old.loop_focus != new.loop_focus
        || old.disabled_keys != new.disabled_keys
}

// ────────────────────────────────────────────────────────────────────
// Transition helpers
// ────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum FocusStep {
    Next,
    Prev,
}

/// Returns `true` when `tab` is in the disabled-tab set.
fn is_disabled(ctx: &Context, tab: &Key) -> bool {
    ctx.disabled_tabs.contains(tab)
}

/// Returns `true` when `tab` is registered in [`Context::tabs`].
fn is_registered(ctx: &Context, tab: &Key) -> bool {
    ctx.tabs.iter().any(|key| key == tab)
}

/// Walks `ctx.tabs` from `current` in the requested direction, skipping
/// disabled entries. Honours [`Context::loop_focus`]: when wrapping is on,
/// a disabled-only tab list returns `None` after a full revolution; when
/// wrapping is off, hitting either edge returns `None`.
fn step_focus(ctx: &Context, current: &Key, step: FocusStep) -> Option<Key> {
    let total = ctx.tabs.len();

    if total == 0 {
        return None;
    }

    let start = ctx.tabs.iter().position(|key| key == current).unwrap_or(0);

    let mut index = start;
    let mut checked = 0;

    loop {
        let advanced = match step {
            FocusStep::Next => {
                if ctx.loop_focus {
                    Some((index + 1) % total)
                } else if index + 1 < total {
                    Some(index + 1)
                } else {
                    None
                }
            }
            FocusStep::Prev => {
                if ctx.loop_focus {
                    Some(if index == 0 { total - 1 } else { index - 1 })
                } else if index > 0 {
                    Some(index - 1)
                } else {
                    None
                }
            }
        };

        let next_index = advanced?;

        index = next_index;

        checked += 1;

        if checked > total {
            return None;
        }

        if !is_disabled(ctx, &ctx.tabs[index]) {
            return Some(ctx.tabs[index].clone());
        }
    }
}

/// Builds the standard "move focus to `target`" transition plan.
fn focus_to(ctx: &Context, target: Key) -> TransitionPlan<Machine> {
    let auto = ctx.activation_mode == ActivationMode::Automatic;

    let next_state = State::Focused {
        tab: target.clone(),
    };

    TransitionPlan::to(next_state)
        .apply(move |ctx: &mut Context| {
            ctx.focused_tab = Some(target.clone());

            if auto {
                ctx.value.set(Some(target));
            }
        })
        .with_effect(PendingEffect::named(Effect::FocusFocusedTab))
}

/// Builds the [`Event::SetTabs`] transition plan, including the
/// `Focused → Idle` downgrade when the snap will clear the focused
/// tab. The downgrade detection runs against the pre-apply context
/// because the apply closure has not run yet at plan-construction
/// time.
fn set_tabs_plan(
    state: &State,
    ctx: &Context,
    registrations: &[TabRegistration],
) -> TransitionPlan<Machine> {
    let registrations = registrations.to_vec();

    // Pre-compute whether the Focused state will survive the snap.
    let downgrade_to_idle = match state {
        State::Idle => false,

        State::Focused { tab } => {
            // The new tabs may dedupe, but membership is order-independent.
            let still_present = registrations.iter().any(|r| r.key == *tab);

            let still_enabled = !ctx.disabled_tabs.contains(tab);

            !(still_present && still_enabled)
        }
    };

    let apply = move |ctx: &mut Context| {
        let mut seen = BTreeSet::<Key>::new();
        let mut tabs = Vec::with_capacity(registrations.len());
        let mut closable = BTreeSet::new();

        for reg in registrations {
            if seen.insert(reg.key.clone()) {
                tabs.push(reg.key.clone());

                if reg.closable {
                    closable.insert(reg.key);
                }
            }
        }

        ctx.tabs = tabs;
        ctx.closable_tabs = closable;

        snap_value_to_valid_key(ctx);
        snap_focused_tab_to_valid_key(ctx);
    };

    if downgrade_to_idle {
        TransitionPlan::to(State::Idle).apply(apply)
    } else {
        TransitionPlan::context_only(apply)
    }
}

/// Builds the [`Event::SyncProps`] transition plan. Same `Focused →
/// Idle` downgrade as [`set_tabs_plan`] when the new `disabled_keys`
/// would render the focused tab disabled.
fn sync_props_plan(state: &State, ctx: &Context, props: &Props) -> TransitionPlan<Machine> {
    let orientation = props.orientation;
    let activation_mode = props.activation_mode;
    let dir = props.dir;
    let loop_focus = props.loop_focus;
    let disabled_keys = props.disabled_keys.clone();

    let downgrade_to_idle = match state {
        State::Idle => false,

        State::Focused { tab } => {
            let still_present = ctx.tabs.iter().any(|key| key == tab);

            let still_enabled = !disabled_keys.contains(tab);

            !(still_present && still_enabled)
        }
    };

    let apply = move |ctx: &mut Context| {
        ctx.orientation = orientation;
        ctx.activation_mode = activation_mode;
        // Preserve any runtime-resolved direction. The adapter dispatches
        // `Event::SetDirection` after mount when `Props::dir == Auto`, and
        // unrelated prop changes (e.g. `disabled_keys`) must not overwrite
        // that resolution back to `Auto`. A concrete `props.dir` still wins
        // because the consumer is expressing an explicit intent.
        if dir != Direction::Auto {
            ctx.dir = dir;
        }

        ctx.loop_focus = loop_focus;
        ctx.disabled_tabs = disabled_keys;

        snap_value_to_valid_key(ctx);
        snap_focused_tab_to_valid_key(ctx);
    };

    if downgrade_to_idle {
        TransitionPlan::to(State::Idle).apply(apply)
    } else {
        TransitionPlan::context_only(apply)
    }
}

/// Re-establishes the selection invariant after `tabs` changes:
///
/// 1. If `value` already points at a valid (registered, non-disabled)
///    tab, keep it.
/// 2. Otherwise snap to the first non-disabled tab in the new list.
/// 3. If no non-disabled tab exists, set `value = None`.
fn snap_value_to_valid_key(ctx: &mut Context) {
    let valid = ctx
        .value
        .get()
        .as_ref()
        .filter(|key| ctx.tabs.iter().any(|k| k == *key) && !is_disabled(ctx, key))
        .cloned();

    if valid.is_some() {
        return;
    }

    let next = ctx.tabs.iter().find(|key| !is_disabled(ctx, key)).cloned();

    ctx.value.set(next);
}

/// Re-establishes the focus invariant after `tabs` changes: `focused_tab`
/// stays valid (registered + not disabled) or is cleared.
fn snap_focused_tab_to_valid_key(ctx: &mut Context) {
    let still_valid = ctx
        .focused_tab
        .as_ref()
        .is_some_and(|key| ctx.tabs.iter().any(|k| k == key) && !is_disabled(ctx, key));

    if !still_valid {
        ctx.focused_tab = None;
    }
}

/// Picks the successor tab when removing the tab at `position` from
/// `tabs`. Prefers the next tab; falls back to the previous tab when the
/// removed tab was last; returns `None` when `tabs` had only one element.
fn pick_successor(tabs: &[Key], position: usize) -> Option<Key> {
    if position + 1 < tabs.len() {
        Some(tabs[position + 1].clone())
    } else if position > 0 {
        Some(tabs[position - 1].clone())
    } else {
        None
    }
}

// ────────────────────────────────────────────────────────────────────
// Api
// ────────────────────────────────────────────────────────────────────

/// Connected API surface for the [`Tabs`](self) component.
///
/// Adapter-only configuration hints (`lazy_mount`, `unmount_on_exit`,
/// `reorderable`, etc.) are NOT exposed on `Api` — adapters read them
/// directly via [`ars_core::Service::props`] to keep `Api` focused on
/// ARIA / event handling.
pub struct Api<'a> {
    state: &'a State,
    ctx: &'a Context,
    props: &'a Props,
    send: &'a dyn Fn(Event),
}

impl Debug for Api<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Api")
            .field("state", self.state)
            .field("ctx", self.ctx)
            .field("props", self.props)
            .finish()
    }
}

impl Api<'_> {
    /// Returns the currently selected tab key, or `None` when no tab is
    /// active (empty list).
    #[must_use]
    pub fn selected_tab(&self) -> Option<&Key> {
        self.ctx.value.get().as_ref()
    }

    /// Returns `true` when `tab_key` is the selected tab.
    #[must_use]
    pub fn is_tab_selected(&self, tab_key: &Key) -> bool {
        self.ctx.value.get().as_ref() == Some(tab_key)
    }

    /// Returns the tab key that currently has keyboard focus, if any.
    #[must_use]
    pub const fn focused_tab(&self) -> Option<&Key> {
        self.ctx.focused_tab.as_ref()
    }

    /// Returns `true` when closing `tab_key` is allowed under the
    /// current configuration.
    ///
    /// Returns `false` when:
    /// - `tab_key` is not registered in [`Context::tabs`] (nothing to
    ///   close), OR
    /// - [`Props::disallow_empty_selection`] is `true` AND `tab_key`
    ///   is the only tab in the list.
    ///
    /// Otherwise returns `true`. Consumers gate their close handler on
    /// this method so a programmatic close attempt against an
    /// unregistered key is rejected at the agnostic-core layer rather
    /// than producing a silent no-op transition downstream.
    #[must_use]
    pub fn can_close_tab(&self, tab_key: &Key) -> bool {
        if !self.ctx.tabs.iter().any(|key| key == tab_key) {
            return false;
        }

        if !self.props.disallow_empty_selection {
            return true;
        }

        self.ctx.tabs.len() > 1
    }

    /// Returns the deterministic successor key when closing `tab_key`,
    /// matching spec §5.3:
    ///
    /// - Prefers the next tab in DOM order.
    /// - Falls back to the previous tab when `tab_key` is last.
    /// - Returns `None` when `tab_key` is not in the list, or when the
    ///   list will be empty after the close.
    ///
    /// Consumers call this from their `CloseTab` handler to drive
    /// selection follow-up (typically dispatching
    /// [`Event::SelectTab`] after [`Event::SetTabs`]).
    #[must_use]
    pub fn successor_for_close(&self, tab_key: &Key) -> Option<Key> {
        let position = self.ctx.tabs.iter().position(|key| key == tab_key)?;

        pick_successor(&self.ctx.tabs, position)
    }

    /// Attributes for the outer root wrapper element.
    #[must_use]
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(
                HtmlAttr::Data("ars-orientation"),
                orientation_token(self.ctx.orientation),
            )
            .set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());

        attrs
    }

    /// Attributes for the `<div role="tablist">` element.
    ///
    /// The `id` attribute (derived as
    /// `Context::ids.part("tablist")`) is rendered so adapters can
    /// resolve the live `direction` CSS property via
    /// [`PlatformEffects::resolved_direction`](ars_core::PlatformEffects::resolved_direction)
    /// before dispatching [`Event::SetDirection`].
    #[must_use]
    pub fn list_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::List.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Id, self.ctx.ids.part("tablist"))
            .set(HtmlAttr::Role, "tablist")
            .set(
                HtmlAttr::Aria(AriaAttr::Orientation),
                orientation_token(self.ctx.orientation),
            );

        attrs
    }

    /// Attributes for an individual tab trigger.
    ///
    /// `tab_key` is the unique key for this tab. The DOM `id` is
    /// derived as `Context::ids.item("tab", &tab_key)` and the
    /// `aria-controls` target as `Context::ids.item("panel", &tab_key)`
    /// — both come from the single [`ComponentIds`] base so consumers
    /// never thread an extra `panel_id` argument through.
    ///
    /// `focus_visible` is the keyboard-modality bit. Adapters can pass
    /// `modality.is_keyboard()` for **every** tab — the method
    /// internally guards on `tab_key == ctx.focused_tab`, so non-focused
    /// tabs never render `data-ars-focus-visible` even when the caller
    /// passes `true`.
    ///
    /// The `tabindex` attribute follows the roving-tabindex pattern.
    /// `"0"` is rendered when the tab is selected, OR when no tab is
    /// selected (`value == None`) AND this tab is the first non-disabled
    /// tab in [`Context::tabs`]. The fallback keeps the tab list
    /// reachable via natural Tab navigation when the consumer renders
    /// with no initial selection.
    #[must_use]
    pub fn tab_attrs(&self, tab_key: &Key, focus_visible: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Tab {
            tab_key: Key::default(),
        }
        .data_attrs();

        let is_selected = self.is_tab_selected(tab_key);
        let is_focused = self.ctx.focused_tab.as_ref() == Some(tab_key);
        let is_disabled = is_disabled(self.ctx, tab_key);
        let is_roving_anchor = is_selected || self.is_tablist_focus_fallback(tab_key);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("tab", tab_key))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "tab")
            .set(HtmlAttr::Aria(AriaAttr::Selected), bool_token(is_selected))
            .set(
                HtmlAttr::Aria(AriaAttr::Controls),
                self.ctx.ids.item("panel", tab_key),
            )
            // Roving tabindex: the selected tab (or, when nothing is
            // selected, the first non-disabled tab) participates in the
            // natural tab sequence; others are reachable via arrow keys.
            .set(
                HtmlAttr::TabIndex,
                if is_roving_anchor { "0" } else { "-1" },
            );

        if is_selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }

        if is_disabled {
            attrs
                .set(HtmlAttr::Aria(AriaAttr::Disabled), "true")
                .set_bool(HtmlAttr::Data("ars-disabled"), true);
        }

        if is_focused && focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }

        if self.props.reorderable {
            attrs.set(HtmlAttr::Aria(AriaAttr::RoleDescription), "draggable tab");
        }

        attrs
    }

    /// Returns `true` when `tab_key` should anchor the roving tabindex
    /// because no registered tab actually matches the current `value`.
    /// Used by [`tab_attrs`] to keep the tablist reachable via natural
    /// `Tab` navigation in both of the cases where `is_tab_selected`
    /// returns `false` for every rendered tab:
    ///
    /// 1. `value == None` — uncontrolled or empty-list bootstrapping.
    /// 2. `value == Some(stale_key)` in controlled mode — the parent
    ///    component's controlled value points at a key that no longer
    ///    exists in [`Context::tabs`] (a `SetTabs` removed it before
    ///    the parent re-synced). [`Bindable::set`] only mutates the
    ///    internal copy in this state, so the snap-to-first-non-disabled
    ///    fallback inside `snap_value_to_valid_key` cannot rewrite
    ///    `value.get()` and the ghost selection persists across
    ///    renders. Without this fallback the tablist would render with
    ///    no `tabindex="0"` anchor at all and be skipped by natural
    ///    `Tab` navigation.
    fn is_tablist_focus_fallback(&self, tab_key: &Key) -> bool {
        let any_registered_tab_is_selected = self
            .ctx
            .tabs
            .iter()
            .any(|key| self.ctx.value.get().as_ref() == Some(key));

        if any_registered_tab_is_selected {
            return false;
        }

        self.ctx
            .tabs
            .iter()
            .find(|key| !is_disabled(self.ctx, key))
            .is_some_and(|first| first == tab_key)
    }

    /// Adapter handler: a tab trigger was clicked.
    pub fn on_tab_click(&self, tab_key: &Key) {
        (self.send)(Event::SelectTab(tab_key.clone()));
    }

    /// Adapter handler: a tab trigger received DOM focus (pointer,
    /// programmatic, screen-reader virtual cursor). Idempotent — the
    /// transition no-ops when the tab is already focused.
    pub fn on_tab_focus(&self, tab_key: &Key) {
        (self.send)(Event::Focus(tab_key.clone()));
    }

    /// Adapter handler: focus left the tab list.
    pub fn on_tab_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Adapter handler: a key was pressed on a tab trigger. Dispatches
    /// the appropriate event(s) per spec §1.6 (arrow / Home / End /
    /// Enter / Space), §5.6 (`Delete` / `Backspace` for closable tabs),
    /// and §6.4 (Ctrl+Arrow reorder, direction-naive — see spec §6.4).
    pub fn on_tab_keydown(&self, tab_key: &Key, data: &KeyboardEventData) {
        let (prev_key, next_key) = arrow_pair(self.ctx.orientation, self.ctx.dir);

        let manual = self.ctx.activation_mode == ActivationMode::Manual;

        // §6.4 Ctrl+Arrow reorder. Ctrl+ArrowRight / Ctrl+ArrowDown move
        // forward in DOM order regardless of `dir` (matches Ark UI and
        // keeps index manipulation orthogonal to visual layout).
        if self.props.reorderable && data.ctrl_key {
            let reorder_axis_match = match self.ctx.orientation {
                Orientation::Horizontal => match data.key {
                    KeyboardKey::ArrowRight => Some(ReorderStep::Next),
                    KeyboardKey::ArrowLeft => Some(ReorderStep::Prev),
                    _ => None,
                },

                Orientation::Vertical => match data.key {
                    KeyboardKey::ArrowDown => Some(ReorderStep::Next),
                    KeyboardKey::ArrowUp => Some(ReorderStep::Prev),
                    _ => None,
                },
            };

            if let Some(step) = reorder_axis_match {
                if let Some(new_index) = self.next_reorder_index(tab_key, step) {
                    (self.send)(Event::ReorderTab {
                        tab: tab_key.clone(),
                        new_index,
                    });
                }

                return;
            }
        }

        if data.key == next_key {
            (self.send)(Event::FocusNext);
        } else if data.key == prev_key {
            (self.send)(Event::FocusPrev);
        } else if data.key == KeyboardKey::Home {
            (self.send)(Event::FocusFirst);
        } else if data.key == KeyboardKey::End {
            (self.send)(Event::FocusLast);
        } else if (data.key == KeyboardKey::Enter || data.key == KeyboardKey::Space) && manual {
            (self.send)(Event::SelectTab(tab_key.clone()));
        } else if (data.key == KeyboardKey::Delete || data.key == KeyboardKey::Backspace)
            && self.ctx.closable_tabs.contains(tab_key)
        {
            // §5.6: Delete / Backspace closes the focused tab when the
            // tab is registered as closable. Non-closable tabs ignore
            // these keys — matches platform-text-edit conventions where
            // Delete in a non-closable context falls through to default
            // browser handling.
            (self.send)(Event::CloseTab(tab_key.clone()));
        }
    }

    /// Computes the new index for a `Ctrl+Arrow` reorder dispatch.
    /// Disabled tabs are not reorderable — returns `None` when
    /// `tab_key` is disabled. Otherwise returns `None` when the move
    /// would push the tab past either end (clamped, no event emitted).
    fn next_reorder_index(&self, tab_key: &Key, step: ReorderStep) -> Option<usize> {
        if is_disabled(self.ctx, tab_key) {
            return None;
        }

        let position = self.ctx.tabs.iter().position(|key| key == tab_key)?;

        let total = self.ctx.tabs.len();

        match step {
            ReorderStep::Next => {
                if position + 1 < total {
                    Some(position + 1)
                } else {
                    None
                }
            }

            ReorderStep::Prev => {
                if position > 0 {
                    Some(position - 1)
                } else {
                    None
                }
            }
        }
    }

    /// Attributes for the animated selection-indicator bar.
    ///
    /// The adapter is responsible for measuring the selected tab's
    /// bounding rect relative to the tab list root and setting the
    /// `--ars-indicator-left` / `--ars-indicator-top` /
    /// `--ars-indicator-width` / `--ars-indicator-height` CSS custom
    /// properties as inline styles on this element.
    #[must_use]
    pub fn tab_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::TabIndicator.data_attrs();

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Aria(AriaAttr::Hidden), "true");

        attrs
    }

    /// Attributes for a tab panel.
    ///
    /// `tab_key` identifies the associated tab. The DOM `id` is
    /// derived as `Context::ids.item("panel", &tab_key)` and the
    /// `aria-labelledby` target as `Context::ids.item("tab", &tab_key)`
    /// — same base IDs that `tab_attrs` uses, so the wiring is
    /// guaranteed consistent.
    ///
    /// `tab_label` is an optional explicit label rendered as
    /// `aria-label` on the panel when the corresponding tab trigger
    /// has no visible text (icon-only tabs).
    #[must_use]
    pub fn panel_attrs(&self, tab_key: &Key, tab_label: Option<&str>) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Panel {
            tab_key: Key::default(),
            tab_label: None,
        }
        .data_attrs();

        let is_selected = self.is_tab_selected(tab_key);

        attrs
            .set(HtmlAttr::Id, self.ctx.ids.item("panel", tab_key))
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Role, "tabpanel")
            .set(
                HtmlAttr::Aria(AriaAttr::LabelledBy),
                self.ctx.ids.item("tab", tab_key),
            )
            // Panels are programmatically focusable when visible so
            // keyboard users can `Tab` from a focused tab into the
            // panel.
            .set(HtmlAttr::TabIndex, "0");

        if let Some(label) = tab_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label.to_string());
        }

        if is_selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        } else {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }

        attrs
    }

    /// Attributes for the close button inside a closable tab.
    ///
    /// `tab_label` is the visible text label of the parent tab; the
    /// rendered `aria-label` is built via [`Messages::close_tab_label`].
    #[must_use]
    pub fn close_trigger_attrs(&self, tab_label: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::TabCloseTrigger {
            tab_label: String::new(),
        }
        .data_attrs();

        let label = (self.ctx.messages.close_tab_label)(tab_label, &self.ctx.locale);

        attrs
            .set(scope_attr, scope_val)
            .set(part_attr, part_val)
            .set(HtmlAttr::Type, "button")
            .set(HtmlAttr::Aria(AriaAttr::Label), label)
            // The close button is reachable from the parent tab via the
            // `Delete` / `Backspace` keyboard shortcut and via pointer
            // input; it is not in the natural tab sequence.
            .set(HtmlAttr::TabIndex, "-1");

        attrs
    }

    /// Adapter handler: the close trigger inside a closable tab was
    /// activated. Always dispatches [`Event::CloseTab`] — the consumer
    /// guards on [`Api::can_close_tab`] before applying the close.
    pub fn on_close_trigger_click(&self, tab_key: &Key) {
        (self.send)(Event::CloseTab(tab_key.clone()));
    }
}

#[derive(Clone, Copy)]
enum ReorderStep {
    Next,
    Prev,
}

/// Returns the `(prev, next)` arrow-key pair for FOCUS navigation. Horizontal
/// direction is RTL-aware per the canonical `03-accessibility.md` matrix;
/// vertical is direction-neutral. Reorder navigation (Ctrl+Arrow) does NOT
/// pass through this — see [`Api::on_tab_keydown`].
const fn arrow_pair(orientation: Orientation, dir: Direction) -> (KeyboardKey, KeyboardKey) {
    match (orientation, dir) {
        (Orientation::Horizontal, Direction::Rtl) => {
            (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft)
        }

        // `Direction::Auto` defaults to LTR until the adapter resolves
        // it via `Event::SetDirection`.
        (Orientation::Horizontal, Direction::Ltr | Direction::Auto) => {
            (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
        }

        (Orientation::Vertical, _) => (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown),
    }
}

/// Returns the `aria-orientation` / `data-ars-orientation` token for the
/// given orientation.
const fn orientation_token(orientation: Orientation) -> &'static str {
    match orientation {
        Orientation::Horizontal => "horizontal",
        Orientation::Vertical => "vertical",
    }
}

/// Returns the canonical `"true"` / `"false"` string for an `aria-*`
/// boolean attribute.
const fn bool_token(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

// ────────────────────────────────────────────────────────────────────
// ConnectApi
// ────────────────────────────────────────────────────────────────────

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            // `tab_attrs` takes a `focus_visible` bit; the default
            // ConnectApi path renders without focus-visible because the
            // Part enum cannot carry runtime modality. Adapters that
            // want focus-visible call `Api::tab_attrs` directly with the
            // ModalityContext-derived bool.
            Part::Tab { tab_key } => self.tab_attrs(&tab_key, false),
            Part::TabIndicator => self.tab_indicator_attrs(),
            Part::Panel { tab_key, tab_label } => self.panel_attrs(&tab_key, tab_label.as_deref()),
            Part::TabCloseTrigger { tab_label } => self.close_trigger_attrs(&tab_label),
        }
    }
}

// ────────────────────────────────────────────────────────────────────
// Tests
// ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
