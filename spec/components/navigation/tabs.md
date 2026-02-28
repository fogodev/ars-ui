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

| Event                        | Payload       | Description                                    |
| ---------------------------- | ------------- | ---------------------------------------------- |
| `SelectTab(Key)`             | tab key       | Activate a tab and show its panel.             |
| `Focus { tab, is_keyboard }` | `Key`, `bool` | A tab received focus.                          |
| `Blur`                       | —             | Focus left the tab list.                       |
| `FocusNext`                  | —             | Move focus to the next tab (for keyboard nav). |
| `FocusPrev`                  | —             | Move focus to the previous tab.                |
| `FocusFirst`                 | —             | Move focus to the first tab.                   |
| `FocusLast`                  | —             | Move focus to the last tab.                    |

### 1.3 Context

```rust
use ars_core::Bindable;
use ars_collections::Key;
use ars_i18n::{Orientation, Direction};

/// Context for the `Tabs` component.
#[derive(Clone, Debug, PartialEq)]
pub struct Context {
    /// The key of the currently selected (active) tab.
    pub value: Bindable<Key>,
    /// The tab that currently has keyboard focus (may differ from selected in manual mode).
    pub focused_tab: Option<Key>,
    /// True when the focused tab received focus via keyboard.
    pub focus_visible: bool,
    /// Stacking axis of the tab list.
    pub orientation: Orientation,
    /// In `Automatic` mode, focusing a tab immediately selects it.
    /// In `Manual` mode, Enter/Space is required to select.
    pub activation_mode: ActivationMode,
    /// Text direction — affects which arrow key advances focus.
    pub dir: Direction,
    /// Whether focus wraps from last tab back to first and vice-versa.
    pub loop_focus: bool,
    /// Per-tab disabled flags (keyed by tab key).
    pub disabled_tabs: BTreeMap<Key, bool>,
    /// Generated ID for the tablist element (used for runtime direction resolution).
    pub tablist_id: String,
    /// Registered tab keys in DOM order.
    pub tabs: Vec<Key>,
    /// The resolved locale for this component instance.
    pub locale: Locale,
    /// Resolved messages for accessibility labels.
    pub messages: Messages,
}

/// Activation mode for the `Tabs` component.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ActivationMode {
    /// Focusing a tab via keyboard immediately selects it.
    Automatic,
    /// Focusing a tab moves the focus indicator but does not change the panel;
    /// the user must press Enter or Space to confirm selection.
    Manual,
}
```

### 1.4 Props

```rust
use ars_collections::Key;
use ars_i18n::{Orientation, Direction};

/// Props for the `Tabs` component.
#[derive(Clone, Debug, PartialEq, HasId)]
pub struct Props {
    /// Unique component identifier.
    pub id: String,
    /// Controlled selected tab key.
    pub value: Option<Key>,
    /// Initial selected tab when uncontrolled.
    pub default_value: Key,
    /// Tab list orientation.
    pub orientation: Orientation,
    /// How keyboard focus interacts with selection.
    pub activation_mode: ActivationMode,
    /// Text direction.
    pub dir: Direction,
    /// Wrap focus at the ends of the tab list.
    pub loop_focus: bool,
    /// When true, prevents deselecting the last tab — at least one tab
    /// must always be selected. Default: false.
    pub disallow_empty_selection: bool,
    /// When true, tab panels are not mounted until their tab is first selected.
    /// Reduces initial DOM size for tabs with heavy content. Default: false.
    pub lazy_mount: bool,
    /// When true, tab panels are removed from the DOM when their tab is deselected.
    /// Works with Presence for exit animations. Default: false.
    pub unmount_on_exit: bool,
    /// Set of keys for tabs that are disabled.
    /// Disabled tabs remain visible but cannot be selected via click or keyboard.
    /// They are skipped during arrow-key navigation, receive `aria-disabled="true"`,
    /// and are visually indicated via the `data-ars-disabled` attribute.
    pub disabled_keys: BTreeSet<Key>,
    /// Locale override. When `None`, inherits from nearest `ArsProvider` context.
    pub locale: Option<Locale>,
    /// Localizable strings.
    pub messages: Option<Messages>,
}

impl Default for Props {
    fn default() -> Self {
        Self {
            id: String::new(),
            value: None,
            default_value: Key::default(),
            orientation: Orientation::Horizontal,
            activation_mode: ActivationMode::Automatic,
            dir: Direction::Ltr,
            loop_focus: true,
            disallow_empty_selection: false,
            lazy_mount: false,
            unmount_on_exit: false,
            disabled_keys: BTreeSet::new(),
            locale: None,
            messages: None,
        }
    }
}
```

### 1.5 Full Machine Implementation

```rust
use ars_core::{TransitionPlan, PendingEffect, Bindable, AttrMap};
use ars_collections::Key;
use ars_i18n::{Orientation, Direction};

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
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Activate a tab and show its panel.
    SelectTab(Key),
    /// A tab received focus.
    Focus {
        /// The key of the tab that has keyboard focus.
        tab: Key,
        /// Whether the focus was initiated by a keyboard.
        is_keyboard: bool,
    },
    /// Focus left the tab list.
    Blur,
    /// Move focus to the next tab (for keyboard nav).
    FocusNext,
    /// Move focus to the previous tab.
    FocusPrev,
    /// Move focus to the first tab.
    FocusFirst,
    /// Move focus to the last tab.
    FocusLast,
    /// Request the adapter to move DOM focus to the element with `target_id`.
    /// The core machine MUST NOT call DOM methods directly; focus is an adapter effect.
    RequestFocus {
        /// The ID of the tab to focus.
        target_id: String,
    },
    /// Adapter sends this after mount to resolve `Direction::Auto` to a concrete
    /// direction by querying the computed `direction` CSS property on the tablist element
    /// via `platform.resolved_direction(&tablist_id)`.
    SetDirection(Direction),
}

// ── Machine ───────────────────────────────────────────────────────────────────

/// Machine for the `Tabs` component.
pub struct Machine;

impl ars_core::Machine for Machine {
    type State   = State;
    type Event   = Event;
    type Context = Context;
    type Props   = Props;
    type Api<'a> = Api<'a>;

    fn init(props: &Props) -> (State, Context) {
        let value = match &props.value {
            Some(v) => Bindable::controlled(v.clone()),
            None    => Bindable::uncontrolled(props.default_value.clone()),
        };
        let locale = resolve_locale(props.locale.as_ref());
        let messages = resolve_messages::<Messages>(props.messages.as_ref(), &locale);
        (State::Idle, Context {
            value,
            focused_tab: None,
            focus_visible: false,
            orientation: props.orientation,
            activation_mode: props.activation_mode,
            dir: props.dir,
            loop_focus: props.loop_focus,
            disabled_tabs: props.disabled_keys.iter()
                .map(|k| (k.clone(), true))
                .collect(),
            tablist_id: format!("{}-tablist", props.id),
            tabs: Vec::new(),
            locale,
            messages,
        })
    }

    fn transition(
        state: &State,
        event: &Event,
        ctx: &Context,
        props: &Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {

            // ── SelectTab ─────────────────────────────────────────────────────
            (_, Event::SelectTab(id)) => {
                // Guard: ignore if this tab is disabled.
                if *ctx.disabled_tabs.get(id).unwrap_or(&false) {
                    return None;
                }
                // Guard: skip full transition when re-selecting the already
                // active tab — no context change or state transition needed.
                if *ctx.value.get() == *id {
                    return None;
                }
                let id = id.clone();
                Some(TransitionPlan::to(State::Focused { tab: id.clone() })
                    .apply(move |ctx| {
                        ctx.value.set(id.clone());
                        ctx.focused_tab = Some(id);
                    }))
            }

            // ── Focus ─────────────────────────────────────────────────────────
            (_, Event::Focus { tab, is_keyboard }) => {
                // Guard: ignore if this tab is disabled.
                if *ctx.disabled_tabs.get(tab).unwrap_or(&false) {
                    return None;
                }
                let tab         = tab.clone();
                let is_keyboard = *is_keyboard;
                let auto        = ctx.activation_mode == ActivationMode::Automatic;
                Some(TransitionPlan::to(State::Focused { tab: tab.clone() })
                    .apply(move |ctx| {
                        ctx.focused_tab  = Some(tab.clone());
                        ctx.focus_visible = is_keyboard;
                        // In automatic mode, focus immediately selects.
                        if auto {
                            ctx.value.set(tab);
                        }
                    }))
            }

            // ── Blur ──────────────────────────────────────────────────────────
            (_, Event::Blur) => {
                Some(TransitionPlan::to(State::Idle)
                    .apply(|ctx| {
                        ctx.focused_tab  = None;
                        ctx.focus_visible = false;
                    }))
            }

            // ── FocusNext ─────────────────────────────────────────────────────
            // Skips disabled tabs. Loop guard prevents infinite loop when all
            // tabs are disabled.
            (State::Focused { tab }, Event::FocusNext) => {
                let current     = tab.clone();
                let tabs        = ctx.tabs.clone();
                let total       = tabs.len();
                if total == 0 { return None; }
                let loop_focus  = ctx.loop_focus;
                let auto        = ctx.activation_mode == ActivationMode::Automatic;
                let idx         = tabs.iter().position(|t| t == &current).unwrap_or(0);
                let mut next_idx = idx;
                let mut checked  = 0;
                loop {
                    next_idx = if loop_focus {
                        (next_idx + 1) % total
                    } else {
                        (next_idx + 1).min(total.saturating_sub(1))
                    };
                    checked += 1;
                    if checked > total {
                        return None; // All tabs disabled — stay on current
                    }
                    if !*ctx.disabled_tabs.get(&tabs[next_idx]).unwrap_or(&false) {
                        break;
                    }
                    // If not looping and we hit the end, stop
                    if !loop_focus && next_idx == total.saturating_sub(1) {
                        return None;
                    }
                }
                let next = tabs[next_idx].clone();
                let next_clone = next.clone();
                Some(TransitionPlan::to(State::Focused { tab: next.clone() })
                    .apply(move |ctx| {
                        ctx.focused_tab  = Some(next_clone.clone());
                        ctx.focus_visible = true;
                        if auto { ctx.value.set(next_clone); }
                    })
                    .with_effect(PendingEffect::new("focus-tab", move |_ctx, _props, send| {
                        send(Event::RequestFocus { target_id: next.to_string() });
                        no_cleanup()
                    })))
            }

            // ── FocusPrev ─────────────────────────────────────────────────────
            // Skips disabled tabs. Loop guard prevents infinite loop when all
            // tabs are disabled.
            (State::Focused { tab }, Event::FocusPrev) => {
                let current    = tab.clone();
                let tabs       = ctx.tabs.clone();
                let total      = tabs.len();
                if total == 0 { return None; }
                let loop_focus = ctx.loop_focus;
                let auto       = ctx.activation_mode == ActivationMode::Automatic;
                let idx        = tabs.iter().position(|t| t == &current).unwrap_or(0);
                let mut prev_idx = idx;
                let mut checked  = 0;
                loop {
                    prev_idx = if loop_focus {
                        if prev_idx == 0 { total - 1 } else { prev_idx - 1 }
                    } else {
                        prev_idx.saturating_sub(1)
                    };
                    checked += 1;
                    if checked > total {
                        return None; // All tabs disabled — stay on current
                    }
                    if !*ctx.disabled_tabs.get(&tabs[prev_idx]).unwrap_or(&false) {
                        break;
                    }
                    // If not looping and we hit the start, stop
                    if !loop_focus && prev_idx == 0 {
                        return None;
                    }
                }
                let prev = tabs[prev_idx].clone();
                let prev_clone = prev.clone();
                Some(TransitionPlan::to(State::Focused { tab: prev.clone() })
                    .apply(move |ctx| {
                        ctx.focused_tab  = Some(prev_clone.clone());
                        ctx.focus_visible = true;
                        if auto { ctx.value.set(prev_clone); }
                    })
                    .with_effect(PendingEffect::new("focus-tab", move |_ctx, _props, send| {
                        send(Event::RequestFocus { target_id: prev.to_string() });
                        no_cleanup()
                    })))
            }

            // ── FocusFirst ────────────────────────────────────────────────────
            // Skips disabled tabs from the front.
            (_, Event::FocusFirst) => {
                let tabs = ctx.tabs.clone();
                let auto = ctx.activation_mode == ActivationMode::Automatic;
                let first = tabs.iter()
                    .find(|t| !*ctx.disabled_tabs.get(*t).unwrap_or(&false))
                    .cloned();
                if let Some(first) = first {
                    let first_clone = first.clone();
                    Some(TransitionPlan::to(State::Focused { tab: first.clone() })
                        .apply(move |ctx| {
                            ctx.focused_tab  = Some(first_clone.clone());
                            ctx.focus_visible = true;
                            if auto { ctx.value.set(first_clone); }
                        })
                        .with_effect(PendingEffect::new("focus-tab", move |_ctx, _props, send| {
                            send(Event::RequestFocus { target_id: first.to_string() });
                            no_cleanup()
                        })))
                } else {
                    None
                }
            }

            // ── FocusLast ─────────────────────────────────────────────────────
            // Skips disabled tabs from the back.
            (_, Event::FocusLast) => {
                let tabs = ctx.tabs.clone();
                let auto = ctx.activation_mode == ActivationMode::Automatic;
                let last = tabs.iter().rev()
                    .find(|t| !*ctx.disabled_tabs.get(*t).unwrap_or(&false))
                    .cloned();
                if let Some(last) = last {
                    let last_clone = last.clone();
                    Some(TransitionPlan::to(State::Focused { tab: last.clone() })
                        .apply(move |ctx| {
                            ctx.focused_tab  = Some(last_clone.clone());
                            ctx.focus_visible = true;
                            if auto { ctx.value.set(last_clone); }
                        })
                        .with_effect(PendingEffect::new("focus-tab", move |_ctx, _props, send| {
                            send(Event::RequestFocus { target_id: last.to_string() });
                            no_cleanup()
                        })))
                } else {
                    None
                }
            }

            // ── FocusNext/FocusPrev in Idle ─────────────────────────────────
            // Fallback: if the machine is Idle and receives a focus-movement
            // event, transition to Focused on the currently selected tab.
            (State::Idle, Event::FocusNext | Event::FocusPrev) => {
                let target = ctx.value.get().clone();
                Some(TransitionPlan::to(State::Focused { tab: target.clone() })
                    .apply(move |ctx| {
                        ctx.focused_tab = Some(target);
                        ctx.focus_visible = true;
                    }))
            }

            // ── RequestFocus ─────────────────────────────────────────────────
            // Core machine does NOT call DOM methods. Instead, it emits
            // a PendingEffect that the adapter executes to move focus.
            (_, Event::RequestFocus { target_id }) => {
                let target_id = target_id.clone();
                Some(TransitionPlan::context_only(|_| {})
                    .with_effect(PendingEffect::new("focus-element", move |_ctx, _props, _send| {
                        let platform = use_platform_effects();
                        platform.focus_element_by_id(&target_id);
                        no_cleanup()
                    })))
            }

            // ── SetDirection (adapter resolves Auto on mount) ─────────────
            (_, Event::SetDirection(dir)) => {
                let dir = *dir;
                Some(TransitionPlan::context_only(move |ctx| {
                    ctx.dir = dir;
                }))
            }

            _ => None,
        }
    }

    fn connect<'a>(
        state: &'a State,
        ctx: &'a Context,
        props: &'a Props,
        send: &'a dyn Fn(Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

### 1.6 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "tabs"]
pub enum Part {
    Root,
    List,
    Tab { tab_key: Key, panel_id: String },
    TabIndicator,
    Panel { panel_id: String, tab_key: Key, tab_label: Option<String> },
    CloseTrigger { tab_label: String },
}

/// API for the `Tabs` component.
pub struct Api<'a> {
    /// Current machine state.
    state: &'a State,
    /// Current context.
    ctx:   &'a Context,
    /// Current props.
    props: &'a Props,
    /// Event dispatcher.
    send:  &'a dyn Fn(Event),
}

impl<'a> Api<'a> {
    /// Get the key of the currently selected tab.
    pub fn selected_tab(&self) -> &Key { self.ctx.value.get() }

    /// Check if a specific tab is selected.
    pub fn is_tab_selected(&self, tab_key: &Key) -> bool {
        *self.ctx.value.get() == *tab_key
    }

    /// Get the key of the tab that currently has keyboard focus.
    pub fn focused_tab(&self) -> Option<&Key> {
        self.ctx.focused_tab.as_ref()
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
        attrs.set(HtmlAttr::Dir, match self.ctx.dir {
            Direction::Ltr  => "ltr",
            Direction::Rtl  => "rtl",
            Direction::Auto => "auto",
        });
        attrs
    }

    /// Attrs for the `<div role="tablist">` element.
    pub fn list_attrs(&self) -> AttrMap {
        let mut attrs = AttrMap::new();
        attrs.set(HtmlAttr::Role, "tablist");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::List.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Orientation), match self.ctx.orientation {
            Orientation::Horizontal => "horizontal",
            Orientation::Vertical   => "vertical",
        });
        attrs
    }

    /// Attrs for an individual tab trigger.
    ///
    /// `tab_key`  — unique key for this tab.
    /// `panel_id` — ID of the associated panel (for `aria-controls`).
    pub fn tab_attrs(&self, tab_key: &Key, panel_id: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_selected = self.is_tab_selected(tab_key);
        let is_focused  = self.ctx.focused_tab.as_ref() == Some(tab_key);
        let is_disabled = *self.ctx.disabled_tabs.get(tab_key).unwrap_or(&false);

        attrs.set(HtmlAttr::Id, tab_key.to_string());
        attrs.set(HtmlAttr::Role, "tab");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Tab { tab_key: Key::default(), panel_id: String::new() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Selected), if is_selected { "true" } else { "false" });
        attrs.set(HtmlAttr::Aria(AriaAttr::Controls), panel_id);
        // Roving tabindex: only the selected tab is in the tab sequence.
        attrs.set(HtmlAttr::TabIndex, if is_selected { "0" } else { "-1" });
        if is_selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        }
        if is_disabled {
            attrs.set(HtmlAttr::Aria(AriaAttr::Disabled), "true");
            attrs.set_bool(HtmlAttr::Data("ars-disabled"), true);
        }
        if is_focused && self.ctx.focus_visible {
            attrs.set_bool(HtmlAttr::Data("ars-focus-visible"), true);
        }
        attrs
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

    /// Handle focus event for a tab trigger.
    pub fn on_tab_focus(&self, tab_key: &Key) {
        (self.send)(Event::Focus { tab: tab_key.clone(), is_keyboard: false });
    }

    /// Handle blur event for a tab trigger.
    pub fn on_tab_blur(&self) {
        (self.send)(Event::Blur);
    }

    /// Handle keydown event for a tab trigger.
    pub fn on_tab_keydown(&self, tab_key: &Key, data: &KeyboardEventData) {
        let (prev_key, next_key) = match (&self.ctx.orientation, &self.ctx.dir) {
            (Orientation::Horizontal, Direction::Ltr)  => (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight),
            (Orientation::Horizontal, Direction::Rtl)  => (KeyboardKey::ArrowRight, KeyboardKey::ArrowLeft),
            (Orientation::Horizontal, Direction::Auto) => {
                // The adapter sends Event::SetDirection after mount to resolve Auto.
                // If not yet resolved, default to LTR.
                (KeyboardKey::ArrowLeft, KeyboardKey::ArrowRight)
            }
            (Orientation::Vertical,   _)               => (KeyboardKey::ArrowUp, KeyboardKey::ArrowDown),
        };
        let manual = self.ctx.activation_mode == ActivationMode::Manual;
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
    /// `panel_id` — unique ID for this panel.
    /// `tab_key`  — key of the associated tab (for `aria-labelledby`).
    /// `tab_label` — optional explicit label for the panel. When a tab trigger
    ///   has no visible text (icon-only), pass the tab's accessible name here
    ///   so the panel receives `aria-label` as a fallback. When `None`, the
    ///   panel relies on `aria-labelledby` pointing to the tab element — this
    ///   still works for icon-only tabs provided the tab itself carries
    ///   `aria-label`.
    pub fn panel_attrs(&self, panel_id: &str, tab_key: &Key, tab_label: Option<&str>) -> AttrMap {
        let mut attrs = AttrMap::new();
        let is_selected = self.is_tab_selected(tab_key);
        attrs.set(HtmlAttr::Id, panel_id);
        attrs.set(HtmlAttr::Role, "tabpanel");
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::Panel { panel_id: String::new(), tab_key: Key::default(), tab_label: None }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::LabelledBy), tab_key.to_string());
        // When the associated tab is icon-only (no visible text), set
        // `aria-label` on the panel as a fallback so screen readers can
        // announce the panel's purpose even if the tab lacks `aria-label`.
        if let Some(label) = tab_label {
            attrs.set(HtmlAttr::Aria(AriaAttr::Label), label);
        }
        // Panels are always in the tab sequence when visible.
        attrs.set(HtmlAttr::TabIndex, "0");
        if is_selected {
            attrs.set_bool(HtmlAttr::Data("ars-selected"), true);
        } else {
            attrs.set_bool(HtmlAttr::Hidden, true);
        }
        attrs
    }

    /// Attrs for the close button inside a closable tab.
    ///
    /// `tab_label` — the visible text label of the tab, used to build an accessible name.
    pub fn close_trigger_attrs(&self, tab_label: &str) -> AttrMap {
        let mut attrs = AttrMap::new();
        let [(scope_attr, scope_val), (part_attr, part_val)] = Part::CloseTrigger { tab_label: String::new() }.data_attrs();
        attrs.set(scope_attr, scope_val);
        attrs.set(part_attr, part_val);
        attrs.set(HtmlAttr::Aria(AriaAttr::Label),
            (self.ctx.messages.close_tab_label)(tab_label, &self.ctx.locale));
        attrs.set(HtmlAttr::TabIndex, "-1");
        attrs
    }
}

impl ConnectApi for Api<'_> {
    type Part = Part;

    fn part_attrs(&self, part: Part) -> AttrMap {
        match &part {
            Part::Root => self.root_attrs(),
            Part::List => self.list_attrs(),
            Part::Tab { tab_key, panel_id } => self.tab_attrs(tab_key, panel_id),
            Part::TabIndicator => self.tab_indicator_attrs(),
            Part::Panel { panel_id, tab_key, tab_label } => {
                self.panel_attrs(panel_id, tab_key, tab_label.as_deref())
            }
            Part::CloseTrigger { tab_label } => self.close_trigger_attrs(tab_label),
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

| Part        | Element    | Key Attributes                                                                                                                                            |
| ----------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Root`      | `<div>`    | `data-ars-scope="tabs"`, `data-ars-part="root"`, `data-ars-orientation`, `dir`                                                                            |
| `List`      | `<div>`    | `data-ars-scope="tabs"`, `data-ars-part="list"`, `role="tablist"`, `aria-orientation`                                                                     |
| `Tab`       | `<button>` | `data-ars-scope="tabs"`, `data-ars-part="tab"`, `role="tab"`, `aria-selected`, `aria-controls`, `tabindex`, `data-ars-selected`, `data-ars-focus-visible` |
| `Indicator` | `<span>`   | `data-ars-scope="tabs"`, `data-ars-part="tab-indicator"`, `aria-hidden="true"`                                                                            |
| `Panel`     | `<div>`    | `data-ars-scope="tabs"`, `data-ars-part="panel"`, `role="tabpanel"`, `aria-labelledby`, `tabindex="0"`                                                    |

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

Tabs listed in `disabled_keys` (or `disabled_tabs` in `Context`) are **focusable but not activatable**:

- Arrow keys still move focus to disabled tabs (they are not skipped during keyboard navigation).
- `Enter`/`Space` on a disabled tab is a no-op (the `SelectTab` guard rejects it).
- Disabled tabs render with `aria-disabled="true"` and emit `data-ars-disabled`.
- The `disabled` HTML attribute is **not** set, so the tab remains in the focus order for screen reader discoverability.

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

- **RTL**: `dir="rtl"` reverses the meaning of `ArrowLeft`/`ArrowRight` for horizontal tabs.
  The `root_props()` method emits `dir="rtl"` on the Root element so the browser also lays
  out the tab list visually right-to-left.
- **Vertical tabs**: `orientation="vertical"` is direction-neutral; arrow keys become
  `ArrowUp`/`ArrowDown` regardless of `dir`.
- **Messages**: Tab labels are consumer-provided. The `Messages` struct provides the closable-tab close button label (`close_tab_label`).

## 5. Variant: Closable Tabs

Tabs may be individually closable by the user (e.g., browser-style tab bars, editor panes).

### 5.1 Additional Props

```rust
/// Added to `TabDef` (per-tab definition).
pub struct TabDef {
    /// The key of the tab.
    pub key: Key,
    /// The label of the tab.
    pub label: String,
    /// When true, a close button is rendered inside this tab.
    pub closable: bool,
}
```

### 5.2 Additional Event

```rust
/// Added to the Tabs Event enum.
CloseTab(Key),  // tab key
```

### 5.3 Behavior

- When the active tab is closed, selection moves to the next tab. If the closed tab was
  last, selection moves to the new last tab. If no tabs remain, `value` becomes `None`.
- The `CloseTab` event fires _before_ the tab is removed — the consumer decides whether
  to actually remove it (e.g., may show a confirmation dialog).
- If `closable` is false on a `TabDef`, no close button is rendered for that tab.

### 5.4 Anatomy Addition

```text
Tab
├── Label
└── CloseTrigger  (<button>; data-ars-part="tab-close-trigger")
```

| Part           | Element    | Key Attributes                                                                                           |
| -------------- | ---------- | -------------------------------------------------------------------------------------------------------- |
| `CloseTrigger` | `<button>` | `aria-label=Messages.close_tab_label({tab label})`, `data-ars-part="tab-close-trigger"`, `tabindex="-1"` |

### 5.5 Messages

```rust
#[derive(Clone, Debug)]
pub struct Messages {
    /// Close trigger label template (default: "Close {label}")
    pub close_tab_label: MessageFn<dyn Fn(&str, &Locale) -> String + Send + Sync>,
}

impl Default for Messages {
    fn default() -> Self {
        Self { close_tab_label: MessageFn::new(|label, _locale| format!("Close {}", label)) }
    }
}

impl ComponentMessages for Messages {}
```

### 5.6 Keyboard

| Key         | Behavior                                                                   |
| ----------- | -------------------------------------------------------------------------- |
| `Delete`    | Close the focused tab (if `closable`).                                     |
| `Backspace` | Close the focused tab (if `closable`). Same as Delete for discoverability. |

## 6. Variant: Reorderable Tabs

Tabs may be reordered by the user via drag-and-drop or keyboard shortcuts.

### 6.1 Additional Props for reorderable tabs

```rust
/// Added to the Tabs Props struct.
/// When true, tabs can be reordered by drag-and-drop or keyboard.
pub reorderable: bool,
```

### 6.2 Additional Event for reorderable tabs

```rust
/// Added to the Tabs Event enum.
ReorderTab { tab: Key, new_index: usize },
```

### 6.3 Behavior for reorderable tabs

- **Drag and Drop**: Each tab becomes a drag source and drop target. During drag, a
  `DropIndicator` (thin vertical line) appears between tabs to show the insertion point.
  On drop, fires `ReorderTab` with the dragged tab key and new index.
- **Keyboard**: `Ctrl+ArrowRight` / `Ctrl+ArrowLeft` (horizontal) or `Ctrl+ArrowDown` /
  `Ctrl+ArrowUp` (vertical) move the focused tab one position in that direction.
  The focused tab remains focused after the move.
- The machine does not reorder its internal `tabs: Vec<Key>` directly — it fires the
  event and the consumer updates the tab list. This keeps reordering controlled.

### 6.4 Keyboard for reorderable tabs

| Key               | Behavior                                          |
| ----------------- | ------------------------------------------------- |
| `Ctrl+ArrowRight` | Move focused tab one position right (horizontal). |
| `Ctrl+ArrowLeft`  | Move focused tab one position left (horizontal).  |
| `Ctrl+ArrowDown`  | Move focused tab one position down (vertical).    |
| `Ctrl+ArrowUp`    | Move focused tab one position up (vertical).      |

### 6.5 Accessibility

- During drag, the adapter uses `aria-roledescription="draggable tab"` to convey reorderability. (Note: `aria-grabbed` is deprecated in ARIA 1.1 and MUST NOT be used.)
- Drop indicators are not focusable and are hidden from the accessibility tree.
- After a keyboard reorder, the adapter announces the new position via LiveAnnouncer:
  `"{tab label} moved to position {n} of {total}"`.

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
