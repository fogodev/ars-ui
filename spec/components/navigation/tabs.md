---
component: Tabs
category: navigation
tier: complex
foundation_deps: [architecture, accessibility, interactions]
shared_deps: []
related: []
references:
    ark-ui: Tabs
    radix-ui: Tabs
    react-aria: Tabs
---

# Tabs

A tab list paired with associated content panels. Supports both automatic activation (selecting
a tab on focus) and manual activation (selecting only on Enter/Space).

## 1. State Machine

### 1.1 States

| State                  | Description                               |
| ---------------------- | ----------------------------------------- |
| `Idle`                 | No tab has keyboard focus.                |
| `Focused { tab: Key }` | A specific tab button has keyboard focus. |

### 1.2 Events

| Event                           | Payload           | Description                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `SelectTab(Key)`                | tab key           | Activate a tab and show its panel.                                                                                                                                                                                                                                                                                                                                                   |
| `Focus(Key)`                    | `Key`             | A tab received DOM focus from outside the keyboard navigation flow (pointer, programmatic, screen-reader virtual cursor). Idempotent — re-firing for an already-focused tab is a no-op.                                                                                                                                                                                              |
| `Blur`                          | —                 | Focus left the tab list.                                                                                                                                                                                                                                                                                                                                                             |
| `FocusNext`                     | —                 | Move focus to the next non-disabled tab.                                                                                                                                                                                                                                                                                                                                             |
| `FocusPrev`                     | —                 | Move focus to the previous non-disabled tab.                                                                                                                                                                                                                                                                                                                                         |
| `FocusFirst`                    | —                 | Move focus to the first non-disabled tab.                                                                                                                                                                                                                                                                                                                                            |
| `FocusLast`                     | —                 | Move focus to the last non-disabled tab.                                                                                                                                                                                                                                                                                                                                             |
| `SetDirection(Direction)`       | direction         | Replace `ctx.dir`. Idempotent. Adapter dispatches once after mount when `Props::dir == Auto`; `Machine::on_props_changed` also dispatches it whenever `Props::dir` changes between renders (including `Concrete → Auto`, signalling "please re-resolve").                                                                                                                            |
| `SetTabs(Vec<TabRegistration>)` | tab registrations | Bulk-replace the registered tab list. Adapter dispatches whenever its rendered tab triggers change. Duplicate keys deduped (first occurrence wins). Re-establishes selection invariant — see §1.5 `snap_value_to_valid_key`.                                                                                                                                                         |
| `SyncProps`                     | —                 | Re-apply context-backed non-`dir` prop fields (`orientation`, `activation_mode`, `loop_focus`, `disabled_keys`) after a runtime prop change. Emitted by `Machine::on_props_changed` when those fields differ. `dir` changes are emitted as a separate `Event::SetDirection` so an unrelated prop delta cannot clobber a runtime-resolved direction. Re-runs the selection invariant. |
| `CloseTab(Key)`                 | tab key           | Pure notification (Closable variant — §5.3). Machine does not mutate `tabs` / `value`. Consumer applies the close via `SetTabs` / `SelectTab` after consulting `Api::can_close_tab` and `Api::successor_for_close`.                                                                                                                                                                  |
| `ReorderTab { tab, new_index }` | `Key`, `usize`    | Pure notification (Reorderable variant — §6.3). Machine does not mutate `tabs`; consumer applies the reorder.                                                                                                                                                                                                                                                                        |

`Focus(Key)` deliberately does NOT carry an `is_keyboard` bit: the
`data-ars-focus-visible` attribute is a per-render concern derived from
[`ars_core::ModalityContext`](../../foundation/01-architecture.md) at the
adapter layer, not state tracked in `Context`. See §1.6 for how
`Api::tab_attrs` accepts `focus_visible: bool` from the adapter.

### 1.3 Context

```rust
use ars_core::Bindable;
use ars_collections::Key;
use ars_i18n::{Orientation, Direction};

/// Context for the `Tabs` component.
///
/// Does not derive `Eq` because `Messages` (containing `MessageFn`) only
/// implements `PartialEq` (via `Arc::ptr_eq`).
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// Selected tab. `None` when no tab is active (e.g. empty tab list).
    pub value: Bindable<Option<Key>>,
    /// The tab that currently has keyboard focus (may differ from selected in manual mode).
    pub focused_tab: Option<Key>,
    /// Stacking axis of the tab list.
    pub orientation: Orientation,
    /// In `Automatic` mode, focusing a tab immediately selects it.
    /// In `Manual` mode, Enter/Space is required to select.
    pub activation_mode: ActivationMode,
    /// Text direction — affects which arrow key advances focus.
    pub dir: Direction,
    /// Whether focus wraps from last tab back to first and vice-versa.
    pub loop_focus: bool,
    /// Set of disabled tab keys. Built from `Props::disabled_keys` at
    /// init and re-applied by `Event::SyncProps` when the consumer
    /// updates `disabled_keys` at runtime.
    pub disabled_tabs: BTreeSet<Key>,
    /// Set of tab keys whose `TabRegistration::closable` flag was `true`
    /// at registration time. The `Delete` / `Backspace` keyboard
    /// shortcuts and the close-trigger handler check this set before
    /// dispatching `Event::CloseTab`.
    pub closable_tabs: BTreeSet<Key>,
    /// Hydration-stable IDs derived from `Props::id`. The tab list's
    /// DOM id is `ids.part("tablist")`; per-tab DOM id is
    /// `ids.item("tab", &tab_key)`; per-panel DOM id is
    /// `ids.item("panel", &tab_key)`. ARIA wiring (`aria-controls`,
    /// `aria-labelledby`) reads from the same `item(...)` lookup so
    /// adapters never duplicate ID derivation logic. Matches the
    /// workspace `ComponentIds` convention used by Dialog, Field,
    /// Fieldset, etc.
    pub ids: ComponentIds,
    /// Registered tab keys in DOM order. Adapter-driven via
    /// `Event::SetTabs`; consumers never mutate directly.
    pub tabs: Vec<Key>,
    /// The resolved locale for this component instance.
    pub locale: Locale,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

/// Activation mode for the `Tabs` component.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ActivationMode {
    /// Focusing a tab via keyboard immediately selects it.
    #[default]
    Automatic,
    /// Focusing a tab moves the focus indicator but does not change the panel;
    /// the user must press Enter or Space to confirm selection.
    Manual,
}

/// Adapter-supplied registration entry for a single tab. Used as the
/// payload of `Event::SetTabs`. Tab labels are intentionally NOT
/// included — labels are a render concern owned by the adapter /
/// consumer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabRegistration {
    /// Stable identifier. Must be unique within the list.
    pub key: Key,
    /// When `true`, adapters render a close button inside this tab and
    /// the agnostic core forwards `Delete` / `Backspace` keystrokes as
    /// `Event::CloseTab`.
    pub closable: bool,
}
```

`focus_visible` is intentionally NOT part of `Context`. The
`data-ars-focus-visible` attribute is computed at render time by the
adapter from `ars_core::ModalityContext` AND `ctx.focused_tab`, then
threaded into `Api::tab_attrs(tab_key, focus_visible)`. See
spec §1.6 for the call signature.

### 1.4 Props

```rust
use ars_collections::Key;
use ars_i18n::{Orientation, Direction};

/// Props for the `Tabs` component.
///
/// `Props` ships a fluent builder (see `foundation/10-component-spec-template.md` —
/// every workspace component exposes `Props::new()` plus a setter per
/// field).
#[derive(Clone, Debug, PartialEq, Eq, HasId)]
pub struct Props {
    /// Unique component identifier (used as the prefix for the generated `tablist` DOM id).
    pub id: String,
    /// Controlled selected-tab key. The outer `Option` represents
    /// "no tab selected" (e.g. an empty tab list); the inner is the
    /// controlled override of `default_value`.
    pub value: Option<Option<Key>>,
    /// Initial selected-tab key when uncontrolled. `None` means the
    /// component boots with no selection (typical for empty / lazy
    /// tab lists).
    pub default_value: Option<Key>,
    /// Tab list orientation.
    pub orientation: Orientation,
    /// How keyboard focus interacts with selection.
    pub activation_mode: ActivationMode,
    /// Text direction.
    pub dir: Direction,
    /// Wrap focus at the ends of the tab list.
    pub loop_focus: bool,
    /// When `true`, `Api::can_close_tab` returns `false` for the only
    /// remaining tab so consumers refuse the close. Default `false`.
    pub disallow_empty_selection: bool,
    /// When true, tab panels are not mounted until their tab is first selected.
    /// Reduces initial DOM size for tabs with heavy content. Default: false.
    pub lazy_mount: bool,
    /// When true, tab panels are removed from the DOM when their tab is deselected.
    /// Works with Presence for exit animations. Default: false.
    pub unmount_on_exit: bool,
    /// Set of keys for tabs that are disabled.
    /// Disabled tabs remain visible but cannot be selected via click or keyboard,
    /// are skipped during arrow-key navigation, receive `aria-disabled="true"`,
    /// and are visually indicated via the `data-ars-disabled` attribute.
    pub disabled_keys: BTreeSet<Key>,
    /// When `true`, tabs may be reordered by drag-and-drop or keyboard
    /// shortcuts (`Ctrl+Arrow` on the orientation axis). Adapter-only
    /// drag/drop. Default `false`. See §6.
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
```

### 1.5 Full Machine Implementation

The machine emits live-focus intents through the typed [`Effect`] enum
and the codebase-standard `PendingEffect::named` constructor (see
`spec/foundation/01-architecture.md` §2.1 for the `Machine` trait
contract). Adapters dispatch on the variant and call
`PlatformEffects::focus_element_by_id` against their own element handles
(Leptos `NodeRef`, Dioxus `MountedData`); the agnostic core never
traverses the DOM.

```rust
use ars_core::{Bindable, PendingEffect, TransitionPlan};
use ars_collections::Key;
use ars_i18n::{Direction, Orientation};

// ── States ────────────────────────────────────────────────────────────────────

/// State of the `Tabs` component.
#[derive(Clone, Debug, PartialEq)]
pub enum State {
    /// No tab has keyboard focus.
    Idle,
    /// A specific tab button has keyboard focus.
    Focused {
        /// The key of the tab that has keyboard focus.
        tab: Key,
    },
}

// ── Events ────────────────────────────────────────────────────────────────────

/// Events for the `Tabs` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Activate a tab and show its panel.
    SelectTab(Key),
    /// A tab received DOM focus from outside the keyboard navigation
    /// flow (pointer, programmatic, screen-reader virtual cursor).
    /// Idempotent — re-firing for an already-focused tab is a no-op.
    Focus(Key),
    /// Focus left the tab list.
    Blur,
    /// Move focus to the next non-disabled tab.
    FocusNext,
    /// Move focus to the previous non-disabled tab.
    FocusPrev,
    /// Move focus to the first non-disabled tab.
    FocusFirst,
    /// Move focus to the last non-disabled tab.
    FocusLast,
    /// Replace `Context::dir` with the supplied `Direction`. Sources:
    ///
    /// - The adapter dispatches this after mount to resolve
    ///   `Direction::Auto` to a concrete direction by querying the
    ///   computed `direction` CSS property on the tablist element via
    ///   `platform.resolved_direction(&ids.part("tablist"))`.
    /// - `Machine::on_props_changed` dispatches this when `Props::dir`
    ///   changes between renders (including `Concrete → Auto`, which the
    ///   consumer uses to ask the adapter to re-resolve from the platform).
    ///
    /// Idempotent — sending the same direction twice produces no transition.
    SetDirection(Direction),
    /// Replace the registered tab list. Adapters dispatch whenever
    /// their rendered tab triggers change. Re-establishes the
    /// selection invariant: `value` and `focused_tab` snap to the
    /// first non-disabled key in the new list when no longer valid.
    SetTabs(Vec<TabRegistration>),
    /// Pure notification — see §5.3.
    CloseTab(Key),
    /// Pure notification — see §6.3.
    ReorderTab {
        /// The key of the tab being moved.
        tab: Key,
        /// The target zero-based index in the tab list.
        new_index: usize,
    },
}

// ── Effect ────────────────────────────────────────────────────────────────────

/// Typed identifier for every named effect intent the tabs machine emits.
///
/// The variant is intentionally unit — the adapter reads
/// [`Context::focused_tab`] for the actual focus target so the identifier
/// stays `Copy + Eq + Hash`. This matches the codebase convention used by
/// `dialog::Effect` and `popover::Effect`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Effect {
    /// Adapter must move DOM focus to the tab whose key is currently stored in
    /// [`Context::focused_tab`]. Emitted on `FocusNext` / `FocusPrev` /
    /// `FocusFirst` / `FocusLast` / `RequestFocus`.
    FocusFocusedTab,
}

// ── Machine ───────────────────────────────────────────────────────────────────

/// Machine for the `Tabs` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State    = State;
    type Event    = Event;
    type Context  = Context;
    type Props    = Props;
    type Effect   = Effect;
    type Api<'a>  = Api<'a>;
    type Messages = Messages;

    fn init(props: &Self::Props, env: &Env, messages: &Self::Messages) -> (Self::State, Self::Context) {
        let value = match &props.value {
            Some(initial) => Bindable::controlled(initial.clone()),
            None          => Bindable::uncontrolled(props.default_value.clone()),
        };
        (State::Idle, Context {
            value,
            focused_tab: None,
            orientation: props.orientation,
            activation_mode: props.activation_mode,
            dir: props.dir,
            loop_focus: props.loop_focus,
            disabled_tabs: props.disabled_keys.iter()
                .map(|k| (k.clone(), true))
                .collect(),
            closable_tabs: BTreeSet::new(),
            ids: ComponentIds::from_id(&props.id),
            tabs: Vec::new(),
            locale: env.locale.clone(),
            messages: messages.clone(),
        })
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        ctx: &Self::Context,
        _props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {

            // ── SelectTab ─────────────────────────────────────────────────────
            // Unknown / disabled keys and re-selection of the active tab
            // are all rejected — a stale event after `SetTabs` removed
            // the tab can't desync `value` from the rendered list.
            (_, Event::SelectTab(tab)) => {
                if !is_registered(ctx, tab)
                    || is_disabled(ctx, tab)
                    || ctx.value.get().as_ref() == Some(tab)
                {
                    return None;
                }
                let tab = tab.clone();
                Some(TransitionPlan::to(State::Focused { tab: tab.clone() })
                    .apply(move |ctx| {
                        ctx.value.set(Some(tab.clone()));
                        ctx.focused_tab = Some(tab);
                    }))
            }

            // ── Focus ─────────────────────────────────────────────────────────
            // Idempotent — re-firing for an already-focused tab in the same
            // selection state produces no transition. Auto mode advances
            // selection along with focus. Unknown / disabled keys are
            // rejected for the same reason as SelectTab.
            (_, Event::Focus(tab)) => {
                if !is_registered(ctx, tab) || is_disabled(ctx, tab) { return None; }
                let already_focused = ctx.focused_tab.as_ref() == Some(tab);
                let auto = ctx.activation_mode == ActivationMode::Automatic;
                let value_already_set = ctx.value.get().as_ref() == Some(tab);
                if already_focused && (!auto || value_already_set) { return None; }
                let tab = tab.clone();
                Some(TransitionPlan::to(State::Focused { tab: tab.clone() })
                    .apply(move |ctx| {
                        ctx.focused_tab = Some(tab.clone());
                        if auto { ctx.value.set(Some(tab)); }
                    }))
            }

            // ── Blur ──────────────────────────────────────────────────────────
            (_, Event::Blur) => {
                if matches!(state, State::Idle) && ctx.focused_tab.is_none() {
                    return None;
                }
                Some(TransitionPlan::to(State::Idle).apply(|ctx| ctx.focused_tab = None))
            }

            // ── FocusNext / FocusPrev (Idle bootstrap) ───────────────────────
            (State::Idle, Event::FocusNext | Event::FocusPrev) => {
                let target = ctx.value.get().clone()?;
                if !ctx.tabs.iter().any(|k| k == &target) || is_disabled(ctx, &target) {
                    return None;
                }
                Some(TransitionPlan::to(State::Focused { tab: target.clone() })
                    .apply(move |ctx| ctx.focused_tab = Some(target))
                    .with_effect(PendingEffect::named(Effect::FocusFocusedTab)))
            }

            // ── FocusNext / FocusPrev (Focused) ──────────────────────────────
            (State::Focused { tab }, Event::FocusNext) => {
                let next = step_focus(ctx, tab, FocusStep::Next)?;
                Some(focus_to(ctx, next))
            }
            (State::Focused { tab }, Event::FocusPrev) => {
                let prev = step_focus(ctx, tab, FocusStep::Prev)?;
                Some(focus_to(ctx, prev))
            }

            // ── FocusFirst / FocusLast ───────────────────────────────────────
            (_, Event::FocusFirst) => {
                let first = ctx.tabs.iter().find(|t| !is_disabled(ctx, t)).cloned()?;
                Some(focus_to(ctx, first))
            }
            (_, Event::FocusLast) => {
                let last = ctx.tabs.iter().rev().find(|t| !is_disabled(ctx, t)).cloned()?;
                Some(focus_to(ctx, last))
            }

            // ── SetDirection ─────────────────────────────────────────────────
            (_, Event::SetDirection(dir)) => {
                let dir = *dir;
                if ctx.dir == dir { return None; }
                Some(TransitionPlan::context_only(move |ctx| { ctx.dir = dir; }))
            }

            // ── SetTabs ──────────────────────────────────────────────────────
            // Replaces the registered tab list and re-establishes the
            // selection invariant. Duplicate keys are deduped (first
            // occurrence wins); `value` / `focused_tab` snap to a valid
            // key (or to `None` when no non-disabled tab remains).
            (_, Event::SetTabs(registrations)) => {
                let registrations = registrations.clone();
                Some(TransitionPlan::context_only(move |ctx| {
                    let mut seen = BTreeSet::<Key>::new();
                    let mut tabs = Vec::with_capacity(registrations.len());
                    let mut closable = BTreeSet::new();
                    for reg in registrations {
                        if seen.insert(reg.key.clone()) {
                            tabs.push(reg.key.clone());
                            if reg.closable { closable.insert(reg.key); }
                        }
                    }
                    ctx.tabs = tabs;
                    ctx.closable_tabs = closable;
                    snap_value_to_valid_key(ctx);
                    snap_focused_tab_to_valid_key(ctx);
                }))
            }

            // ── SyncProps ────────────────────────────────────────────────────
            // Replays context-backed prop fields after a runtime prop
            // change. After rebuilding `disabled_tabs` the selection
            // invariant re-runs because a now-disabled `value` /
            // `focused_tab` must snap to a still-valid key.
            (_, Event::SyncProps) => {
                let orientation = props.orientation;
                let activation_mode = props.activation_mode;
                let loop_focus = props.loop_focus;
                let disabled_keys = props.disabled_keys.clone();
                // `dir` is intentionally absent: prop-driven direction
                // changes are routed through a separate `SetDirection`
                // event so an unrelated prop delta cannot clobber a
                // runtime-resolved direction, while explicit consumer
                // changes (including `Concrete → Auto`) still propagate
                // exactly once. See `Machine::on_props_changed` below.
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.orientation = orientation;
                    ctx.activation_mode = activation_mode;
                    ctx.loop_focus = loop_focus;
                    ctx.disabled_tabs = disabled_keys;
                    snap_value_to_valid_key(ctx);
                    snap_focused_tab_to_valid_key(ctx);
                }))
            }

            // ── CloseTab — pure notification (§5.3) ──────────────────────────
            (_, Event::CloseTab(_)) => Some(TransitionPlan::context_only(|_| {})),

            // ── ReorderTab — pure notification (§6.3) ────────────────────────
            (_, Event::ReorderTab { .. }) => Some(TransitionPlan::context_only(|_| {})),
        }
    }

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }

    fn on_props_changed(old: &Self::Props, new: &Self::Props) -> Vec<Self::Event> {
        let mut events = Vec::new();

        // `dir` is handled by `SetDirection`, not `SyncProps`. This split
        // lets the adapter resolve `Direction::Auto` once at mount via
        // `SetDirection(Rtl|Ltr)` and have that resolution survive
        // unrelated prop updates (e.g. `disabled_keys` deltas) — `SyncProps`
        // no longer rewrites `ctx.dir`. Conversely, an explicit
        // consumer-driven prop change to `dir` (including `Concrete → Auto`,
        // which signals "please re-resolve") propagates exactly once
        // through `SetDirection`. `SetDirection` itself is idempotent
        // when the new value already matches `ctx.dir`, so emitting on
        // every prop delta is safe.
        if old.dir != new.dir {
            events.push(Event::SetDirection(new.dir));
        }

        if non_dir_context_props_changed(old, new) {
            events.push(Event::SyncProps);
        }

        events
    }
}

/// Returns `true` when any context-backed non-`value`, non-`dir` prop
/// differs between `old` and `new`. Used by `Machine::on_props_changed`
/// to decide whether to emit `Event::SyncProps`. The controlled-`value`
/// path goes through `Bindable::sync_controlled` (the adapter's
/// responsibility); `dir` changes flow through `Event::SetDirection`.
fn non_dir_context_props_changed(old: &Props, new: &Props) -> bool {
    old.orientation != new.orientation
        || old.activation_mode != new.activation_mode
        || old.loop_focus != new.loop_focus
        || old.disabled_keys != new.disabled_keys
}

// ── Transition helpers ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum FocusStep { Next, Prev }

fn is_disabled(ctx: &Context, tab: &Key) -> bool {
    ctx.disabled_tabs.contains(tab)
}

fn is_registered(ctx: &Context, tab: &Key) -> bool {
    ctx.tabs.iter().any(|key| key == tab)
}

/// Walks `ctx.tabs` from `current` in the requested direction, skipping
/// disabled entries. Honours `loop_focus`: when wrapping is on, a
/// disabled-only tab list returns `None` after a full revolution; when
/// wrapping is off, hitting either edge returns `None`.
fn step_focus(ctx: &Context, current: &Key, step: FocusStep) -> Option<Key> {
    let total = ctx.tabs.len();
    if total == 0 { return None; }
    let start = ctx.tabs.iter().position(|k| k == current).unwrap_or(0);
    let mut index = start;
    let mut checked = 0;
    loop {
        let advanced = match step {
            FocusStep::Next => if ctx.loop_focus { Some((index + 1) % total) }
                              else if index + 1 < total { Some(index + 1) } else { None },
            FocusStep::Prev => if ctx.loop_focus { Some(if index == 0 { total - 1 } else { index - 1 }) }
                              else if index > 0 { Some(index - 1) } else { None },
        };
        index = advanced?;
        checked += 1;
        if checked > total { return None; }
        if !is_disabled(ctx, &ctx.tabs[index]) {
            return Some(ctx.tabs[index].clone());
        }
    }
}

/// Builds the standard "move focus to `target`" transition plan. Honours
/// `ActivationMode::Automatic` by setting `value` alongside `focused_tab`.
fn focus_to(ctx: &Context, target: Key) -> TransitionPlan<Machine> {
    let auto = ctx.activation_mode == ActivationMode::Automatic;
    TransitionPlan::to(State::Focused { tab: target.clone() })
        .apply(move |ctx| {
            ctx.focused_tab = Some(target.clone());
            if auto { ctx.value.set(Some(target)); }
        })
        .with_effect(PendingEffect::named(Effect::FocusFocusedTab))
}

/// Re-establishes the selection invariant after `tabs` changes:
///
/// 1. If `value` already points at a valid (registered, non-disabled)
///    tab, keep it.
/// 2. Otherwise snap to the first non-disabled tab in the new list.
/// 3. If no non-disabled tab exists, set `value = None`.
fn snap_value_to_valid_key(ctx: &mut Context) {
    let valid = ctx.value.get().as_ref()
        .filter(|k| ctx.tabs.iter().any(|t| t == *k) && !is_disabled(ctx, k))
        .cloned();
    if valid.is_some() { return; }
    let next = ctx.tabs.iter().find(|k| !is_disabled(ctx, k)).cloned();
    ctx.value.set(next);
}

/// Re-establishes the focus invariant after `tabs` changes: focused_tab
/// stays valid (registered + not disabled) or is cleared.
fn snap_focused_tab_to_valid_key(ctx: &mut Context) {
    let still_valid = ctx.focused_tab.as_ref()
        .is_some_and(|k| ctx.tabs.iter().any(|t| t == k) && !is_disabled(ctx, k));
    if !still_valid { ctx.focused_tab = None; }
}

/// Picks the successor tab when removing the tab at `position` from
/// `tabs`. Used by `Api::successor_for_close` (the consumer-facing
/// helper for the close-tab successor algorithm).
fn pick_successor(tabs: &[Key], position: usize) -> Option<Key> {
    if position + 1 < tabs.len() { Some(tabs[position + 1].clone()) }
    else if position > 0         { Some(tabs[position - 1].clone()) }
    else                         { None }
}
```

### 1.6 Connect / API

```rust
/// Anatomy parts. `TabCloseTrigger` (rather than `CloseTrigger`) so the
/// kebab-cased `data-ars-part` value is `"tab-close-trigger"` — matches
/// the §5.4 anatomy table and avoids visual collisions with Dialog's /
/// Popover's `close-trigger` data-attribute when downstream stylesheets
/// write scope-agnostic selectors.
#[derive(ComponentPart)]
#[scope = "tabs"]
pub enum Part {
    Root,
    List,
    Tab { tab_key: Key },
    TabIndicator,
    Panel { tab_key: Key, tab_label: Option<String> },
    TabCloseTrigger { tab_label: String },
}

/// API for the `Tabs` component.
///
/// Adapter-only configuration hints (`lazy_mount`, `unmount_on_exit`,
/// `reorderable`) are NOT exposed on `Api`. Adapters read them directly
/// via `ars_core::Service::props` to keep `Api` focused on ARIA / event
/// handling.
pub struct Api<'a> {
    state: &'a State,
    ctx:   &'a Context,
    props: &'a Props,
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Returns the currently selected tab key, or `None` when no tab is
    /// active (empty list).
    pub fn selected_tab(&self) -> Option<&Key> { self.ctx.value.get().as_ref() }

    /// Check if a specific tab is selected.
    pub fn is_tab_selected(&self, tab_key: &Key) -> bool {
        self.ctx.value.get().as_ref() == Some(tab_key)
    }

    /// Get the key of the tab that currently has keyboard focus.
    pub fn focused_tab(&self) -> Option<&Key> {
        self.ctx.focused_tab.as_ref()
    }

    /// Returns `true` when closing `tab_key` is allowed under the
    /// current configuration. Returns `false` when:
    /// - `tab_key` is not registered in `Context::tabs` (nothing to close), OR
    /// - `Props::disallow_empty_selection` is `true` AND `tab_key` is
    ///   the only tab in the list.
    pub fn can_close_tab(&self, tab_key: &Key) -> bool {
        if !self.ctx.tabs.iter().any(|k| k == tab_key) { return false; }
        if !self.props.disallow_empty_selection { return true; }
        self.ctx.tabs.len() > 1
    }

    /// Returns the deterministic successor when closing `tab_key`:
    ///
    /// - Prefers the next tab in DOM order.
    /// - Falls back to the previous tab when `tab_key` is last.
    /// - Returns `None` when `tab_key` is not in the list, or when the
    ///   list will be empty after the close.
    pub fn successor_for_close(&self, tab_key: &Key) -> Option<Key> {
        let position = self.ctx.tabs.iter().position(|k| k == tab_key)?;
        pick_successor(&self.ctx.tabs, position)
    }

    /// Attrs for the outer root wrapper element.
    pub fn root_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Root.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Data("ars-orientation"), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        attrs.set(HtmlAttr::Dir, self.ctx.dir.as_html_attr());
        attrs
    }

    /// Attrs for the `<div role="tablist">` element.
    ///
    /// Renders `id="{ids.part(\"tablist\")}"` so adapters can resolve
    /// the live `direction` CSS property via
    /// `PlatformEffects::resolved_direction(&ids.part("tablist"))`
    /// before dispatching `Event::SetDirection`.
    pub fn list_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::List.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Id, self.ctx.ids.part("tablist"));
        attrs.set(HtmlAttr::Role, "tablist");
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        attrs
    }

    /// Attrs for an individual tab trigger.
    ///
    /// `tab_key` — unique key for this tab. The DOM `id` is derived
    /// as `ids.item("tab", &tab_key)`; the `aria-controls` target as
    /// `ids.item("panel", &tab_key)`. Both flow from the single
    /// `ComponentIds` base so consumers never thread an extra
    /// `panel_id` argument through.
    ///
    /// `focus_visible` — keyboard-modality bit. Adapters can pass
    /// `modality.is_keyboard()` for **every** tab; the method
    /// internally guards on `tab_key == ctx.focused_tab`, so non-focused
    /// tabs never render `data-ars-focus-visible` even when the caller
    /// passes `true`.
    ///
    /// `tabindex` follows the roving-tabindex pattern: `"0"` when the
    /// tab is selected, OR when no tab is selected (`value == None`)
    /// AND this tab is the first non-disabled tab in `Context::tabs`.
    /// The fallback keeps the tablist reachable via natural Tab
    /// navigation when the consumer renders with no initial selection.
    ///
    /// When `Props::reorderable` is `true`, also emits
    /// `aria-roledescription="draggable tab"` (always — see §6.5).
    pub fn tab_attrs(&self, tab_key: &Key, focus_visible: bool) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_selected = self.is_tab_selected(tab_key);
        let is_focused  = self.ctx.focused_tab.as_ref() == Some(tab_key);
        let is_disabled = self.ctx.disabled_tabs.contains(tab_key);
        let is_roving_anchor = is_selected || self.is_tablist_focus_fallback(tab_key);

        attrs.set(HtmlAttr::Id, self.ctx.ids.item("tab", tab_key));
        attrs.set(HtmlAttr::Role, "tab");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Tab { tab_key: Key::default() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Selected), if is_selected { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), self.ctx.ids.item("panel", tab_key));
        attrs.set(HtmlAttr::TabIndex, if is_roving_anchor { "0" } else { "-1" });
        if is_selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }
        if is_disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
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
    /// because no tab is currently selected. Used by `tab_attrs` to
    /// keep the tablist reachable via natural `Tab` navigation when
    /// `value == None`.
    fn is_tablist_focus_fallback(&self, tab_key: &Key) -> bool {
        if self.ctx.value.get().is_some() { return false; }
        self.ctx.tabs.iter()
            .find(|key| !self.ctx.disabled_tabs.contains(key))
            .is_some_and(|first| first == tab_key)
    }

    /// **Progressive enhancement:** When a tab has an associated `href`, the adapter
    /// SHOULD render the trigger as `<a href="...">` instead of `<button>`.
    /// The state machine behavior is unchanged — the adapter intercepts the click,
    /// calls `on_tab_click`, and uses `preventDefault` to avoid navigation.
    /// On initial server render (SSR), the links work without JavaScript.

    /// Handle click event for a tab trigger.
    pub fn on_tab_click(&self, tab_key: &Key) {
        (self.send)(Event::SelectTab(tab_key.clone()));
    }

    /// Handle DOM focus arrival on a tab trigger. Idempotent.
    pub fn on_tab_focus(&self, tab_key: &Key) {
        (self.send)(Event::Focus(tab_key.clone()));
    }

    /// Handle DOM blur from the tab list.
    pub fn on_tab_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Handle keydown event for a tab trigger.
    ///
    /// Routing precedence: §6.4 reorder shortcuts (`Ctrl+Arrow` on the
    /// orientation axis, **direction-naive** — Ctrl+ArrowRight/Down
    /// always increase index regardless of `dir`), then §1.6 focus
    /// arrows / Home / End, then §1.6 Enter/Space (manual mode), then
    /// §5.6 Delete/Backspace (closable tabs only).
    pub fn on_tab_keydown(&self, tab_key: &Key, data: &KeyboardEventData) {
        let (prev_key, next_key) = arrow_pair(self.ctx.orientation, self.ctx.dir);
        let manual = self.ctx.activation_mode == ActivationMode::Manual;

        if self.props.reorderable && data.ctrl_key {
            let reorder_axis_match = match self.ctx.orientation {
                Orientation::Horizontal => match data.key {
                    KeyboardKey::ArrowRight => Some(ReorderStep::Next),
                    KeyboardKey::ArrowLeft  => Some(ReorderStep::Prev),
                    _ => None,
                },
                Orientation::Vertical => match data.key {
                    KeyboardKey::ArrowDown => Some(ReorderStep::Next),
                    KeyboardKey::ArrowUp   => Some(ReorderStep::Prev),
                    _ => None,
                },
            };
            if let Some(step) = reorder_axis_match {
                if let Some(new_index) = self.next_reorder_index(tab_key, step) {
                    (self.send)(Event::ReorderTab { tab: tab_key.clone(), new_index });
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
            (self.send)(Event::CloseTab(tab_key.clone()));
        }
    }

    /// Attrs for the animated selection indicator bar.
    pub fn tab_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::TabIndicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        attrs
    }

    /// Attrs for a tab panel.
    ///
    /// `tab_key` identifies the associated tab. The DOM `id` is
    /// derived as `ids.item("panel", &tab_key)` and the
    /// `aria-labelledby` target as `ids.item("tab", &tab_key)` —
    /// same base IDs that `tab_attrs` uses, so the wiring is
    /// guaranteed consistent.
    pub fn panel_attrs(&self, tab_key: &Key, tab_label: Option<&str>) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_selected = self.is_tab_selected(tab_key);
        attrs.set(HtmlAttr::Id, self.ctx.ids.item("panel", tab_key));
        attrs.set(HtmlAttr::Role, "tabpanel");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Panel { tab_key: Key::default(), tab_label: None }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), self.ctx.ids.item("tab", tab_key));
        if let Some(label) = tab_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }
        attrs.set(HtmlAttr::TabIndex, "0");
        if is_selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        } else {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    /// Attrs for the close button inside a closable tab. Renders
    /// `type="button"` to inhibit accidental form submission when the
    /// tabs live inside a `<form>`.
    pub fn close_trigger_attrs(&self, tab_label: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::TabCloseTrigger { tab_label: String::new() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Type, "button");
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.close_tab_label)(tab_label, &self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs
    }

    /// Adapter handler: the close trigger inside a closable tab was
    /// activated. Always dispatches `Event::CloseTab` — the consumer
    /// guards on `Api::can_close_tab` before applying the close.
    pub fn on_close_trigger_click(&self, tab_key: &Key) {
        (self.send)(Event::CloseTab(tab_key.clone()));
    }
}

#[derive(Clone, Copy)]
enum ReorderStep { Next, Prev }

impl<'a> Api<'a> {
    /// Disabled tabs are not reorderable — returns `None` for them.
    /// Otherwise clamps at both ends.
    fn next_reorder_index(&self, tab_key: &Key, step: ReorderStep) -> Option<usize> {
        if self.ctx.disabled_tabs.contains(tab_key) { return None; }
        let position = self.ctx.tabs.iter().position(|k| k == tab_key)?;
        let total = self.ctx.tabs.len();
        match step {
            ReorderStep::Next => if position + 1 < total { Some(position + 1) } else { None },
            ReorderStep::Prev => if position > 0 { Some(position - 1) } else { None },
        }
    }
}

const fn arrow_pair(orientation: Orientation, dir: Direction) -> (KeyboardKey, KeyboardKey) {
    match (orientation, dir) {
        (Orientation::Horizontal, Direction::Rtl) => (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft),
        (Orientation::Horizontal, Direction::Ltr | Direction::Auto) => (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight),
        (Orientation::Vertical, _) => (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown),
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            // ConnectApi defaults focus_visible to false; adapters that
            // want focus-visible call `Api::tab_attrs` directly with the
            // ModalityContext-derived bool.
            Part::Tab { tab_key } => self.tab_attrs(&tab_key, false),
            Part::TabIndicator => self.tab_indicator_attrs(),
            Part::Panel { tab_key, tab_label } => {
                self.panel_attrs(&tab_key, tab_label.as_deref())
            }
            Part::TabCloseTrigger { tab_label } => self.close_trigger_attrs(&tab_label),
        }
    }
}
```

## 2. Anatomy

```text
Tabs
├── Root
├── List                   role="tablist"
│   ├── Tab (×N)           role="tab"
│   └── Indicator       animated selection bar
└── Panel (×N)             role="tabpanel"
```

| Part        | Element    | Key Attributes                                                                                                                                                                                                             |
| ----------- | ---------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`      | `<div>`    | `data-ars-scope="tabs"`, `data-ars-part="root"`, `data-ars-orientation`, `dir`                                                                                                                                             |
| `List`      | `<div>`    | `data-ars-scope="tabs"`, `data-ars-part="list"`, `id="{props.id}-tablist"` (= `ids.part("tablist")`), `role="tablist"`, `aria-orientation`                                                                                 |
| `Tab`       | `<button>` | `data-ars-scope="tabs"`, `data-ars-part="tab"`, `id="{props.id}-tab-{tab_key}"` (= `ids.item("tab", &tab_key)`), `role="tab"`, `aria-selected`, `aria-controls`, `tabindex`, `data-ars-selected`, `data-ars-focus-visible` |
| `Indicator` | `<span>`   | `data-ars-scope="tabs"`, `data-ars-part="tab-indicator"`, `aria-hidden="true"`                                                                                                                                             |
| `Panel`     | `<div>`    | `data-ars-scope="tabs"`, `data-ars-part="panel"`, `id="{props.id}-panel-{tab_key}"` (= `ids.item("panel", &tab_key)`), `role="tabpanel"`, `aria-labelledby`, `tabindex="0"`                                                |

Every DOM `id` flows from the single `Context::ids: ComponentIds` base
so `aria-controls` / `aria-labelledby` wiring is computed in one place
and survives multi-instance pages without collision. The `List`
element's explicit `id` lets adapters resolve the live `direction`
CSS property via
`PlatformEffects::resolved_direction(&ids.part("tablist"))` before
dispatching `Event::SetDirection`.

### 2.1 Indicator Part

The `tabs::Indicator` part provides an animated sliding selection highlight, identical to
`ToggleGroup`'s indicator pattern (see [Indicator Part](../utility/toggle.md#indicator-part)). It is
positioned via CSS custom properties set by the adapter's connect layer based on the
selected tab's DOM measurements.

**CSS Custom Properties** (set as inline styles on the indicator element):

| Property                 | Description                                                |
| ------------------------ | ---------------------------------------------------------- |
| `--ars-indicator-left`   | Horizontal offset of the indicator from the tab list root. |
| `--ars-indicator-top`    | Vertical offset of the indicator from the tab list root.   |
| `--ars-indicator-width`  | Width of the indicator (matches the selected tab).         |
| `--ars-indicator-height` | Height of the indicator (matches the selected tab).        |

```rust
impl<'a> Api<'a> {
    /// Attrs for the animated selection indicator bar.
    ///
    /// The adapter measures the selected tab's bounding rect relative to the
    /// tab list root and sets CSS custom properties (`--ars-indicator-left`,
    /// `--ars-indicator-top`, `--ars-indicator-width`, `--ars-indicator-height`)
    /// as inline styles on this element. Consumers animate the indicator via
    /// CSS transitions or animations targeting these properties.
    ///
    /// This is the same pattern used by ToggleGroup's indicator — see
    /// `18-utility-components.md` §5 for the reference implementation.
    pub fn tab_indicator_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::TabIndicator.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Hidden), "true");
        // The adapter layer measures the selected tab's bounding rect
        // relative to the tab list and sets these CSS custom properties:
        //   --ars-indicator-left, --ars-indicator-top,
        //   --ars-indicator-width, --ars-indicator-height
        // These are set dynamically via inline styles by the adapter.
        attrs
    }
}
```

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

| Part    | Role       | Properties                                                                                                            |
| ------- | ---------- | --------------------------------------------------------------------------------------------------------------------- |
| `List`  | `tablist`  | `aria-orientation="horizontal\|vertical"`                                                                             |
| `Tab`   | `tab`      | `aria-selected="true\|false"`, `aria-controls="{panel-id}"`, `tabindex="0"` (selected only), `tabindex="-1"` (others) |
| `Panel` | `tabpanel` | `aria-labelledby="{tab-id}"`, `tabindex="0"`                                                                          |

The roving tabindex pattern means only one tab is in the natural tab order at a time; arrow
keys navigate within the tab list. This matches the ARIA Authoring Practices Guide tablist
pattern precisely.

### 3.2 Multiple-Open Announcement Semantics

**Accordion**: When `multiple === true`, the Accordion root element MUST set `aria-multiselectable="true"` on the element with the grouping role. This informs assistive technology that multiple sections can be expanded simultaneously.

**Tabs**: Tabs are always single-active — exactly one tab panel is visible at a time. The `tablist` role inherently implies single selection via `aria-selected` (only one tab has `aria-selected="true"`). Tabs MUST NOT use `aria-multiselectable` — the ARIA specification does not define this attribute for `role="tablist"`, and setting it would be invalid ARIA.

### 3.3 Disabled Tabs (`disabled_keys`)

Tabs listed in `disabled_keys` (or `disabled_tabs` in `Context`) are
**not activatable, and skipped during keyboard arrow navigation**:

- Arrow keys (and `Home` / `End`) **skip** disabled tabs — `step_focus`
  walks past them. This matches Ark UI, Radix UI, and React Aria.
- `Enter`/`Space` on a disabled tab is a no-op (the `SelectTab` guard
  rejects it).
- `SelectTab` and `Focus` events targeting a disabled tab are
  rejected by the transition.
- Disabled tabs render with `aria-disabled="true"` and emit
  `data-ars-disabled`.
- The HTML `disabled` attribute is **not** set, so disabled tabs stay
  in the natural focus order for screen-reader discoverability via
  Tab — selected tabs are still primary focus targets via the roving
  `tabindex="0"`.
- A disabled tab that somehow already has focus (e.g. it was disabled
  while focused) still receives `data-ars-focus-visible` styling when
  the adapter's `focus_visible` parameter is `true`. Disabled tabs do
  not lose visual focus indication.
- Disabled tabs are **not reorderable** — `Ctrl+Arrow` on a disabled
  tab is a no-op (see §6.4).
- When a runtime prop change (`Event::SyncProps`) re-disables a tab
  that was the current `value` or `focused_tab`, the selection
  invariant snaps to the first non-disabled tab. See §1.5
  `snap_value_to_valid_key` / `snap_focused_tab_to_valid_key`.

### 3.4 Keyboard Interaction

| Key                           | Behavior                                                                        |
| ----------------------------- | ------------------------------------------------------------------------------- |
| `ArrowRight` (horizontal LTR) | Move focus to next tab; wraps if `loop_focus`. In automatic mode, also selects. |
| `ArrowLeft` (horizontal LTR)  | Move focus to previous tab; wraps if `loop_focus`.                              |
| `ArrowRight` (horizontal RTL) | Move focus to previous tab (reversed).                                          |
| `ArrowLeft` (horizontal RTL)  | Move focus to next tab (reversed).                                              |
| `ArrowDown` (vertical)        | Move focus to next tab.                                                         |
| `ArrowUp` (vertical)          | Move focus to previous tab.                                                     |
| `Home`                        | Move focus (and select, if automatic) to first tab.                             |
| `End`                         | Move focus (and select, if automatic) to last tab.                              |
| `Enter` / `Space`             | In manual mode: select the focused tab.                                         |
| `Tab`                         | Move focus out of the tab list into the active panel.                           |

> **RTL Handling**: Horizontal keyboard navigation follows the canonical RTL matrix defined in `03-accessibility.md` section "Canonical RTL Keyboard Navigation Matrix". In RTL, ArrowRight moves to the previous tab and ArrowLeft moves to the next tab.

## 4. Internationalization

- **RTL**: `dir="rtl"` reverses the meaning of `ArrowLeft` / `ArrowRight`
  for horizontal **focus** navigation. Reorder shortcuts (`Ctrl+Arrow`)
  are direction-naive — see §6.4. The `root_attrs` method emits
  `dir="rtl"` on the Root element so the browser also lays out the
  tab list visually right-to-left.
- **Vertical tabs**: `orientation="vertical"` is direction-neutral;
  arrow keys become `ArrowUp` / `ArrowDown` regardless of `dir`.
- **Messages**: Tab labels are consumer-provided. The `Messages`
  struct provides the closable-tab close button label
  (`close_tab_label`) AND the keyboard-reorder LiveAnnouncer template
  (`reorder_announce_label`) — see §5.5.

## 5. Variant: Closable Tabs

Tabs may be individually closable by the user (e.g., browser-style tab bars, editor panes).

### 5.1 Per-tab closability via `TabRegistration`

Closability is declared **per registered tab** through
`TabRegistration::closable` (the payload of `Event::SetTabs`):

```rust
pub struct TabRegistration {
    pub key: Key,
    pub closable: bool,
}
```

The agnostic core stores the registered closable keys in
`Context::closable_tabs: BTreeSet<Key>`. There is no `TabDef` struct in
this crate — labels are a render concern owned by the adapter /
consumer, not the agnostic core.

### 5.2 Additional Event

`CloseTab(Key)` is part of the base `Event` enum (see §1.5). It is
**dispatched by**:

- `Api::on_close_trigger_click(tab_key)` — the close button.
- `Api::on_tab_keydown` — `Delete` / `Backspace` when
  `Context::closable_tabs.contains(tab_key)`.

### 5.3 Behavior

`CloseTab` is a **pure notification** — the agnostic core does NOT
mutate `Context::tabs` or `Context::value`. The transition is
`TransitionPlan::context_only(|_| {})`. Consumers are responsible for
applying the close to their tab-list source and re-registering via
`Event::SetTabs` / `Event::SelectTab`.

This design lets consumers veto the close (e.g., show a confirmation
dialog) without leaving the machine in an inconsistent intermediate
state. The deterministic successor algorithm is exposed as
`Api::successor_for_close(&Key) -> Option<Key>`:

- Prefers the next tab in DOM order.
- Falls back to the previous tab when the closing tab is last.
- Returns `None` when the closing tab is the only tab (the consumer
  should refuse the close when `Props::disallow_empty_selection` is
  `true` — `Api::can_close_tab(&Key) -> bool` packages this guard).

When a `Bindable<Option<Key>>` `value` ends up empty (the consumer
cleared the only tab without a successor), `Event::SetTabs` /
`Event::SelectTab` flows treat `Some(None)` as "no tab selected" — see
§1.4.

### 5.4 Anatomy Addition

```text
Tab
├── Label
└── TabCloseTrigger  (<button>; data-ars-part="tab-close-trigger")
```

| Part              | Element    | Key Attributes                                                                                                            |
| ----------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------- |
| `TabCloseTrigger` | `<button>` | `data-ars-part="tab-close-trigger"`, `type="button"`, `aria-label=Messages.close_tab_label({tab label})`, `tabindex="-1"` |

The kebab-cased `data-ars-part` value is `"tab-close-trigger"` —
matching the part variant `Part::TabCloseTrigger`. Using the
`tab-`-prefixed token avoids visual collisions with Dialog's /
Popover's `close-trigger` data-attribute when downstream stylesheets
write scope-agnostic selectors. The `type="button"` attribute
inhibits accidental form submission when the tabs render inside a
`<form>` element.

### 5.5 Messages

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Messages {
    /// Close trigger label template (default: "Close {label}")
    pub close_tab_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,

    /// LiveAnnouncer template emitted by adapters after a keyboard
    /// reorder (default: "{label} moved to position {n} of {total}").
    /// The agnostic core never invokes this itself — see §6.5.
    pub reorder_announce_label:
        MessageFn<dyn Fn(&str, usize, usize, &Locale) -> String + Send + Sync>,
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
```

### 5.6 Keyboard

| Key         | Behavior                                                                                                        |
| ----------- | --------------------------------------------------------------------------------------------------------------- |
| `Delete`    | Dispatch `Event::CloseTab(focused)` when `Context::closable_tabs.contains(focused)`. Otherwise no-op.           |
| `Backspace` | Same as `Delete` — same closable-flag guard. Provided for keyboard-discoverability across keyboard conventions. |

## 6. Variant: Reorderable Tabs

Tabs may be reordered by the user via drag-and-drop or keyboard shortcuts.

### 6.1 Additional Props for reorderable tabs

`Props::reorderable: bool` (default `false`). Already declared in §1.4.
When `true`:

- The agnostic core listens for `Ctrl+Arrow` keystrokes on the
  orientation axis and dispatches `Event::ReorderTab`.
- `Api::tab_attrs` always emits `aria-roledescription="draggable tab"`
  (see §6.5 — always-on, NOT only during drag).

### 6.2 Additional Event for reorderable tabs

`ReorderTab { tab: Key, new_index: usize }` is part of the base `Event`
enum (see §1.5).

### 6.3 Behavior for reorderable tabs

- **Drag and Drop is adapter-only.** Pointer-driven drag interaction
  (drag sources, drop targets, drop indicators, ghost previews) lives
  entirely in the framework adapter using its native drag
  infrastructure (`web_sys::DragEvent` + `Element::set_draggable` for
  the web). The agnostic core does not model drag state at all. On
  drop, the adapter dispatches `Event::ReorderTab` with the dragged
  tab key and computed new index.
- **Keyboard**: `Ctrl+ArrowRight` / `Ctrl+ArrowLeft` (horizontal) or
  `Ctrl+ArrowDown` / `Ctrl+ArrowUp` (vertical) move the focused tab
  one position in that direction along the orientation axis. The
  focused tab remains focused after the move. Clamped at both ends
  (no event emitted at boundaries).
- **Pure notification**: `ReorderTab` is `TransitionPlan::context_only(|_| {})`.
  The machine does NOT reorder `Context::tabs` — the consumer applies
  the reorder to its tab-list source and re-registers via
  `Event::SetTabs`.

### 6.4 Keyboard for reorderable tabs

| Key                                  | Behavior                                                                                                                                                |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Ctrl+ArrowRight` / `Ctrl+ArrowDown` | Move focused tab one position toward higher index (DOM order). **Direction-naive** — RTL does NOT swap this. Only focus navigation is RTL-aware (§3.4). |
| `Ctrl+ArrowLeft` / `Ctrl+ArrowUp` n` | Move focused tab one position toward higher index (DOM order). **Direction-naive** — RTL does NOT swap this. Only focus navigation is RTL-aware (§3.4). |
| `Ctrl+ArrowLeft` / `Ctrl+ArrowUp`    | Move focused tab one position toward lower index. Direction-naive.                                                                                      |

The direction-naive design separates index manipulation (reorder) from
visual layout (focus navigation): "Ctrl+ArrowRight increases index"
matches Ark UI's keyboard reorder and aligns with browser tab-bar
conventions where index manipulation is independent of writing
direction.

### 6.5 Accessibility

- Whenever `Props::reorderable` is `true`, every tab trigger emits
  `aria-roledescription="draggable tab"` (always-on, not only during
  drag). This way screen-reader users discover the keyboard reorder
  affordance on first focus rather than only after a drag has
  started. (Note: `aria-grabbed` is deprecated in ARIA 1.1 and MUST
  NOT be used.)
- Drop indicators are not focusable and are hidden from the
  accessibility tree.
- After a keyboard reorder, the adapter announces the new position
  via LiveAnnouncer using the
  `Messages::reorder_announce_label(label, new_position, total, locale)`
  template (default: `"{label} moved to position {n} of {total}"`).
  The agnostic core does not invoke the LiveAnnouncer itself —
  announcing is an adapter concern — but the message function is on
  `Messages` so consumers localize a single source of truth.

## 7. Library Parity

> Compared against: Ark UI (`Tabs`), Radix UI (`Tabs`), React Aria (`Tabs`).

### 7.1 Props

| Feature             | ars-ui                               | Ark UI           | Radix UI                 | React Aria           | Notes                               |
| ------------------- | ------------------------------------ | ---------------- | ------------------------ | -------------------- | ----------------------------------- |
| Controlled value    | `value`                              | `value`          | `value`                  | `selectedKey`        | Same concept, different naming      |
| Default value       | `default_value`                      | `defaultValue`   | `defaultValue`           | `defaultSelectedKey` | Same concept                        |
| Orientation         | `orientation`                        | `orientation`    | `orientation`            | `orientation`        | Full match                          |
| Activation mode     | `activation_mode`                    | `activationMode` | `activationMode`         | `keyboardActivation` | Same concept                        |
| Loop focus          | `loop_focus`                         | `loopFocus`      | `loop` (on List)         | --                   | React Aria loops by default         |
| Lazy mount          | `lazy_mount`                         | `lazyMount`      | --                       | --                   | Radix uses `forceMount` per-panel   |
| Unmount on exit     | `unmount_on_exit`                    | `unmountOnExit`  | `forceMount`             | `shouldForceMount`   | Inverse semantics on Radix/RA       |
| Disabled tabs       | `disabled_keys`                      | --               | `disabled` (per trigger) | `disabledKeys`       | ars-ui matches React Aria pattern   |
| Global disabled     | --                                   | --               | --                       | `isDisabled`         | ars-ui uses `disabled_keys` per-tab |
| Deselectable        | `disallow_empty_selection` (inverse) | `deselectable`   | --                       | --                   | ars-ui and Ark have this            |
| Dir                 | `dir`                                | --               | `dir`                    | --                   | ars-ui and Radix have RTL support   |
| Composite           | --                                   | `composite`      | --                       | --                   | Not applicable in ars-ui            |
| Navigate callback   | --                                   | `navigate`       | --                       | --                   | Framework-specific; adapter handles |
| Translations / i18n | `messages`                           | `translations`   | --                       | --                   | ars-ui and Ark have i18n            |

**Gaps:** None. ars-ui covers all behaviorally meaningful props from all three libraries. `isDisabled` (global) from React Aria is expressible via `disabled_keys` containing all tab keys.

### 7.2 Anatomy

| Part            | ars-ui         | Ark UI      | Radix UI  | React Aria  | Notes                                    |
| --------------- | -------------- | ----------- | --------- | ----------- | ---------------------------------------- |
| Root            | `Root`         | `Root`      | `Root`    | `Tabs`      | Full match                               |
| Tab list        | `List`         | `List`      | `List`    | `TabList`   | Full match                               |
| Tab trigger     | `Tab`          | `Trigger`   | `Trigger` | `Tab`       | Full match                               |
| Content panel   | `Panel`        | `Content`   | `Content` | `TabPanel`  | Full match                               |
| Indicator       | `TabIndicator` | `Indicator` | --        | --          | Ark and ars-ui have this                 |
| Close trigger   | `CloseTrigger` | --          | --        | --          | ars-ui closable variant                  |
| Panel container | --             | --          | --        | `TabPanels` | React Aria wrapper; not needed in ars-ui |

**Gaps:** None.

### 7.3 Events

| Callback     | ars-ui              | Ark UI          | Radix UI        | React Aria          | Notes                        |
| ------------ | ------------------- | --------------- | --------------- | ------------------- | ---------------------------- |
| Value change | `Bindable` onChange | `onValueChange` | `onValueChange` | `onSelectionChange` | ars-ui uses Bindable pattern |
| Focus change | `Focus` event       | `onFocusChange` | --              | --                  | ars-ui and Ark have this     |
| Close tab    | `CloseTab` event    | --              | --              | --                  | ars-ui closable variant      |
| Reorder tab  | `ReorderTab` event  | --              | --              | --                  | ars-ui reorderable variant   |

**Gaps:** None.

### 7.4 Features

| Feature                     | ars-ui | Ark UI            | Radix UI          | React Aria         |
| --------------------------- | ------ | ----------------- | ----------------- | ------------------ |
| Roving tabindex             | Yes    | Yes               | Yes               | Yes                |
| Horizontal/Vertical         | Yes    | Yes               | Yes               | Yes                |
| Manual/Automatic activation | Yes    | Yes               | Yes               | Yes                |
| Loop focus                  | Yes    | Yes               | Yes (on List)     | Yes (default)      |
| Lazy mount / unmount        | Yes    | Yes               | forceMount        | shouldForceMount   |
| Per-tab disabled            | Yes    | Yes (per trigger) | Yes (per trigger) | Yes (disabledKeys) |
| RTL support                 | Yes    | Yes               | Yes               | Yes                |
| Closable tabs               | Yes    | No                | No                | No                 |
| Reorderable tabs            | Yes    | No                | No                | No                 |
| Tab indicator animation     | Yes    | Yes               | No                | No                 |

**Gaps:** None.

### 7.5 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui uses `disabled_keys: BTreeSet<Key>` (like React Aria) rather than per-trigger `disabled` booleans (Ark/Radix). `disallow_empty_selection` is the inverse of Ark's `deselectable`.
- **Recommended additions:** None. ars-ui exceeds all three references with closable and reorderable tab variants.
