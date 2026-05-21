---
component: FocusScope
category: utility
tier: stateful
foundation_deps: [architecture, accessibility]
shared_deps: []
related: []
references:
    react-aria: FocusScope
---

# FocusScope

FocusScope constrains keyboard Tab focus within a container, enabling focus trapping for
modal dialogs, drawers, and other overlay components that must prevent focus from escaping.

## 1. State Machine

### 1.1 States

| State                      | Description                                                    |
| -------------------------- | -------------------------------------------------------------- |
| `Inactive`                 | Focus scope is idle; Tab behavior is unmodified.               |
| `Active { trapped: bool }` | Focus scope is active. `trapped=true` means Tab cannot escape. |

### 1.2 Events

| Event          | Payload               | Description                                                      |
| -------------- | --------------------- | ---------------------------------------------------------------- |
| `Activate`     | `trapped: bool`       | Activate the focus scope, optionally trapping focus.             |
| `Deactivate`   | `restore_focus: bool` | Deactivate the scope and optionally restore previous focus.      |
| `TrapFocus`    | —                     | Enable focus trapping on an active scope.                        |
| `ReleaseTrap`  | —                     | Disable focus trapping on an active scope.                       |
| `RestoreFocus` | —                     | Restore focus to the element that was focused before activation. |
| `FocusFirst`   | —                     | Move focus to the first tabbable element in the container.       |
| `FocusLast`    | —                     | Move focus to the last tabbable element in the container.        |

### 1.3 Context

```rust
/// The states for the `FocusScope` component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Focus scope is idle; Tab behavior is unmodified.
    Inactive,
    /// Focus scope is active. `trapped=true` means Tab cannot escape.
    Active {
        /// When true, Tab cannot escape the container.
        trapped: bool,
    },
}

/// The events for the `FocusScope` component.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// Activate the focus scope, optionally trapping focus.
    /// The adapter captures the currently focused element ID before sending this event
    /// (via `platform.active_element_id()`) so the machine can store it for later restoration.
    Activate {
        /// When true, Tab cannot escape the container.
        trapped: bool,
        /// ID of the element that had focus before activation (for restore-on-deactivate).
        /// Captured by the adapter via `platform.active_element_id()`.
        saved_focus_id: Option<String>,
    },
    /// Deactivate the scope and optionally restore previous focus.
    Deactivate {
        /// When true, restore focus to the previously focused element.
        restore_focus: bool,
    },
    /// Enable focus trapping on an active scope.
    TrapFocus,
    /// Disable focus trapping on an active scope.
    ReleaseTrap,
    /// Restore focus to the element that was focused before activation.
    /// Adapters MAY send this event explicitly when an `Inactive` scope still
    /// holds a non-empty `Context::saved_focus` (e.g., nested-scope cleanup).
    /// When `Inactive`, the machine emits [`Effect::RestoreFocus`]; when
    /// `Active`, the event is ignored — restoration is only meaningful
    /// after the scope has deactivated.
    RestoreFocus,
    /// Move focus to the first tabbable element in the container.
    FocusFirst,
    /// Move focus to the last tabbable element in the container.
    FocusLast,
}

// FocusScope props (`trapped`, `contain`, `auto_focus`, `restore_focus`) are read
// at activation time and are immutable during the active lifecycle. To change
// trapping behavior, deactivate and reactivate the scope.

/// The context for the `FocusScope` component.
///
/// **Note:** `active` and `trapped` are NOT stored in context. They are derived
/// from `State` in the connect API:
/// - `is_active()` → `matches!(state, State::Active { .. })`
/// - `is_trapped()` → `matches!(state, State::Active { trapped: true })`
///
/// `Eq` enables value-based comparison in proptest invariants; `Default`
/// gives the machine a free `Context::default()` for `init`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Context {
    /// The element that had focus before the scope was activated.
    /// Set on `Activate` and read by the adapter when dispatching
    /// `Effect::RestoreFocus`. The agnostic core does not clear the value
    /// after restoration — the next `Activate` overwrites it.
    pub saved_focus: Option<String>,
    /// The DOM ID of the container element that scopes focus.
    /// Adapters populate this via `Service::context_mut()` once they have
    /// resolved the container's stable ID.
    pub container_id: Option<String>,
}
```

### 1.4 Props

```rust
/// Props for the `FocusScope` component.
#[derive(Clone, Debug, PartialEq, Eq, HasId)]
pub struct Props {
    /// Component instance ID.
    pub id: String,
    /// Prevent Tab from moving focus outside the container.
    pub trapped: bool,
    /// Alias for trapped (clearer naming in some contexts).
    pub contain: bool,
    /// On activation, automatically move focus to the first tabbable element
    /// (or the element with autofocus attribute if present).
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
    /// Returns a fresh `Props` with every field at its `Default` value.
    #[must_use]
    pub fn new() -> Self { Self::default() }

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
```

### 1.5 Transitions

```text
Inactive + Activate { trapped, saved_focus_id }
  → Active { trapped: trapped || props.trapped || props.contain }
  action: ctx.saved_focus = saved_focus_id (adapter-captured ID)
  effect: PendingEffect::named(Effect::FocusTrapListener)
  then_send: FocusFirst (if props.auto_focus=true)

  (Either `Props::trapped` or its `Props::contain` alias opts the
   scope into trapping; the event's `trapped` is a per-activation
   override that can also force trapping when the props didn't.)

Active + Deactivate { restore_focus }
  → Inactive
  cancel_effect: Effect::FocusTrapListener  (runs the adapter cleanup, tearing
                                              down the Tab keydown handler)
  if restore_focus:
    effect: PendingEffect::named(Effect::RestoreFocus)
    (ctx.saved_focus stays — adapter reads it; next Activate overwrites)
  else:
    action: ctx.saved_focus = None

Active { trapped: false } + TrapFocus
  → Active { trapped: true }
  effect: PendingEffect::named(Effect::FocusTrapListener)
  (re-emitted so the adapter reinstalls the trap listener that
   `state_changed=true` just drained — see §1.8)

Active { trapped: true } + ReleaseTrap
  → Active { trapped: false }
  effect: PendingEffect::named(Effect::FocusTrapListener)
  (re-emitted for the same adapter-cleanup-drain reason as above)

Inactive + RestoreFocus
  → Inactive (stay)
  effect: PendingEffect::named(Effect::RestoreFocus)

Active + RestoreFocus
  → None (ignored — restoration is only meaningful after Deactivate)

Active + FocusFirst
  → Active (stay)
  effect: PendingEffect::named(Effect::FocusFirst)

Active + FocusLast
  → Active (stay)
  effect: PendingEffect::named(Effect::FocusLast)

When `contain` is true and no tabbable elements exist within the scope, the
adapter's `Effect::FocusTrapListener` handler MUST:
  (1) keep focus on the container element (which has `tabindex="-1"`),
  (2) suppress Tab/Shift+Tab key events entirely (`preventDefault()`),
  (3) re-scan for tabbable elements on each Tab press to detect dynamically
      added content (e.g., lazy-loaded dialog body).
  (handled entirely in the adapter effect — no state change needed)
```

### 1.6 Full Machine Implementation

The agnostic core emits typed effect intents via `Effect` markers. Adapters
dispatch on the `Effect` variant and route to their `PlatformEffects`
implementation — the core never calls platform helpers directly. This matches
the named-effect pattern used by `Dialog` (`spec/components/overlay/dialog.md`)
and `Popover`.

```rust
// The local `Messages` struct below shadows the `Messages` trait name, so the
// trait must be brought into scope by its real name (`ComponentMessages`)
// rather than an `as _` alias.
use ars_core::{
    AttrMap, ComponentMessages, ComponentPart, ConnectApi, Env, HtmlAttr, PendingEffect,
    TransitionPlan,
};

/// The machine for the `FocusScope` component.
#[derive(Debug)]
pub struct Machine;

/// This component has no translatable strings.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Messages;
impl ComponentMessages for Messages {}

/// Typed effect intents emitted by the focus-scope machine.
///
/// Each variant is an adapter contract — the adapter's effect handler
/// translates the variant into the corresponding `PlatformEffects` call
/// (see [`spec/foundation/11-dom-utilities.md`] §3 for the platform-level
/// helpers and §1.8 of this file for each variant's contract).
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
    /// Restore focus to `service.context().saved_focus`, applying the §7
    /// fallback chain when the saved target is no longer focusable.
    RestoreFocus,
}

impl ars_core::Machine for Machine {
    type State = State;
    type Event = Event;
    type Context = Context;
    type Props = Props;
    type Messages = Messages;
    type Effect = Effect;
    type Api<'a> = Api<'a>;

    fn init(_props: &Props, _env: &Env, _messages: &Messages) -> (State, Context) {
        (
            State::Inactive,
            Context {
                saved_focus: None,
                container_id: None,
            },
        )
    }

    fn transition(
        state: &Self::State,
        event: &Self::Event,
        _ctx: &Self::Context,
        props: &Self::Props,
    ) -> Option<TransitionPlan<Self>> {
        match (state, event) {
            // ── Activation ──────────────────────────────────────────────
            (State::Inactive, Event::Activate { trapped, saved_focus_id }) => {
                // `Props::trapped` is the documented trap prop;
                // `Props::contain` is its alias. Either opts the scope
                // into trapping; the event's `trapped` is a
                // per-activation override that can also force trapping
                // when the props didn't request it.
                let trap = *trapped || props.trapped || props.contain;
                let auto_focus = props.auto_focus;
                let saved = saved_focus_id.clone();
                let mut plan = TransitionPlan::to(State::Active { trapped: trap })
                    .apply(move |ctx: &mut Context| {
                        ctx.saved_focus = saved;
                    })
                    .with_effect(PendingEffect::named(Effect::FocusTrapListener));
                if auto_focus {
                    plan = plan.then(Event::FocusFirst);
                }
                Some(plan)
            }

            // ── Deactivation ────────────────────────────────────────────
            (State::Active { .. }, Event::Deactivate { restore_focus }) => {
                let restore = *restore_focus;
                let mut plan = TransitionPlan::to(State::Inactive)
                    .cancel_effect(Effect::FocusTrapListener);
                if restore {
                    // Adapter reads `service.context().saved_focus` when
                    // dispatching `Effect::RestoreFocus`. The value stays
                    // in context until the next `Activate` overwrites it.
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
            // `state_changed` is true, so the `Active{trapped:_}` ↔
            // `Active{trapped:_}` boundary must re-emit
            // `Effect::FocusTrapListener` for the adapter to reinstall
            // the keydown trap that the drain just removed.
            (State::Active { trapped: false }, Event::TrapFocus) => Some(
                TransitionPlan::to(State::Active { trapped: true })
                    .with_effect(PendingEffect::named(Effect::FocusTrapListener)),
            ),
            (State::Active { trapped: true }, Event::ReleaseTrap) => Some(
                TransitionPlan::to(State::Active { trapped: false })
                    .with_effect(PendingEffect::named(Effect::FocusTrapListener)),
            ),

            // ── RestoreFocus ────────────────────────────────────────────
            // Adapters may send `RestoreFocus` explicitly (e.g. nested
            // scope cleanup). When `Inactive` we emit the effect intent;
            // when `Active` the event is ignored — restoration is only
            // meaningful after the scope has deactivated.
            (State::Inactive, Event::RestoreFocus) => Some(
                TransitionPlan::new().with_effect(PendingEffect::named(Effect::RestoreFocus)),
            ),
            (State::Active { .. }, Event::RestoreFocus) => None,

            // ── Focus Navigation ────────────────────────────────────────
            (State::Active { .. }, Event::FocusFirst) => Some(
                TransitionPlan::new().with_effect(PendingEffect::named(Effect::FocusFirst)),
            ),
            (State::Active { .. }, Event::FocusLast) => Some(
                TransitionPlan::new().with_effect(PendingEffect::named(Effect::FocusLast)),
            ),

            _ => None,
        }
    }

    // **Modality coordination:** programmatic focus restoration MUST preserve the
    // shared `ModalityContext` state rather than forcing pointer modality.
    // This ensures `data-ars-focus-visible` remains correct — programmatic focus
    // should only show a focus ring when the prior interaction was not pointer-driven.

    fn connect<'a>(
        state: &'a Self::State,
        ctx: &'a Self::Context,
        props: &'a Self::Props,
        send: &'a dyn Fn(Self::Event),
    ) -> Self::Api<'a> {
        Api { state, ctx, props, send }
    }
}
```

#### 1.6.1 Focus Restoration Safety (adapter contract)

The agnostic core stores the saved focus target as an opaque `Option<String>`
on `Context::saved_focus`. When `Effect::RestoreFocus` fires, the adapter MUST
guard against restoring focus to a removed, hidden, or otherwise unfocusable
element by walking the following safety checks before calling
`PlatformEffects::focus_element_by_id`:

1. The element is connected to the DOM (`document.contains` returns true).
2. The element is visible (`visibility` is not `hidden`; `display` is not
   `none`).
3. The element is not inside a closed `<details>` element.
4. The element is not already the active element (avoid a no-op refocus).
5. The element can receive focus (tabbable or has explicit `tabindex`).
6. The element has layout (`offsetParent !== null`).

Each predicate above corresponds to a single `PlatformEffects::can_restore_focus`
call — adapters typically implement the entire check list inside that one
method. If the saved target fails any check, the adapter applies the fallback
chain documented in §7.

#### 1.6.2 Orientation-Change Focus Audit (adapter contract)

When viewport orientation changes (portrait ↔ landscape on mobile), CSS media
queries can hide or show elements without removing them from the DOM. If the
currently focused element is hidden by `display: none` after the change, it
still has focus but `offsetParent` becomes `null`. The agnostic core does not
observe orientation events — adapters that need this behavior MUST register a
`matchMedia('(orientation: portrait)')` change listener that:

1. Reads the currently focused element via
   `PlatformEffects::active_element_id`.
2. If the focused element lacks layout (per the §1.6.1 check list), emits a
   fresh `Event::FocusFirst` to move focus to the first visible tabbable in
   the scope.
3. If `service.context().saved_focus` references an element that lacks layout
   after the change, the adapter may inform the consumer that the restore
   target is no longer valid — the core's fallback chain (§7) handles the
   actual recovery on the next `Effect::RestoreFocus` dispatch.

### 1.7 Connect / API

```rust
#[derive(ComponentPart)]
#[scope = "focus-scope"]
pub enum Part {
    Container,
}

/// The API for the `FocusScope` component.
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

/// `Api` carries a closure-typed `send` field, so a manual `Debug` impl is
/// required to satisfy the workspace's `missing_debug_implementations` lint
/// without leaking the closure's address.
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
    pub const fn state(&self) -> &State { self.state }

    /// Returns the current [`Context`] of the focus scope.
    #[must_use]
    pub const fn context(&self) -> &Context { self.ctx }

    /// Returns the [`Props`] used by the focus scope.
    #[must_use]
    pub const fn props(&self) -> &Props { self.props }

    /// Whether the focus scope is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self.state, State::Active { .. })
    }

    /// Whether the focus scope is trapped.
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
        (self.send)(Event::Activate { trapped, saved_focus_id });
    }

    /// Imperatively deactivate the focus scope.
    pub fn deactivate(&self, restore_focus: bool) {
        (self.send)(Event::Deactivate { restore_focus });
    }

    /// Request that focus move to the first tabbable descendant of the
    /// container. Sends [`Event::FocusFirst`]; the agnostic core then emits
    /// [`Effect::FocusFirst`] which the adapter routes through
    /// `PlatformEffects::focus_first_tabbable`. Used by framework adapters
    /// when `auto_focus=true`.
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

```

**Programmatic sequential navigation lives in the adapter layer, not the
core.** `focus_next` / `focus_previous` and any `FocusNavigationOptions`-style
config struct would require synchronous DOM lookups (tabbable filtering,
`activeElement`, `accept` predicate evaluation against live element handles)
that the agnostic core cannot perform — the issue's "Element/ref handling
note" explicitly forbids ID-only or DOM-bound APIs at this layer. Adapters
that need this behavior (consumed by `Toolbar`, `ActionGroup`, `TreeView`,
nested `FocusScope`s) expose it on their own API surface and publish a
`FocusManager` context as described in §1.7 _Focus Manager Context_ below;
the agnostic core only emits `Event::FocusFirst` / `Event::FocusLast` for the
two end-of-list targets it can describe as effect intents.

### 1.8 Effect Contract

Adapters MUST provide a handler for every [`Effect`] variant. Each handler
receives `&Context`, `&Props`, and a `WeakSend<Event>` send handle, and is
expected to call into the workspace's `PlatformEffects` implementation
(see `spec/foundation/01-architecture.md` §2.2.7 and
`spec/foundation/11-dom-utilities.md` §3) rather than the DOM directly.

| Effect              | When emitted                                                                                                   | Adapter contract                                                                                                                                                                                                                                                                                  |
| ------------------- | -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `FocusTrapListener` | `Inactive → Active` (Activate) and every `Active{trapped:_} ↔ Active{trapped:_}` transition (TrapFocus / ReleaseTrap) | Call `PlatformEffects::attach_focus_trap(container_id, on_escape)`. Wire the returned `CleanupFn` as the effect's cleanup so `cancel_effect(Effect::FocusTrapListener)` tears the handler down on `Deactivate`. The `on_escape` callback SHOULD send `Event::Deactivate { restore_focus: true }`. **Re-emitted on TrapFocus / ReleaseTrap because adapters drain all active cleanups when `state_changed` is true, so the listener must be reinstalled to preserve Tab trapping across the Active variants.** |
| `FocusFirst`        | `Active + FocusFirst` (also on `Activate` when `props.auto_focus`)                                             | Call `PlatformEffects::focus_first_tabbable(container_id)`. Fall back to focusing the container element (which has `tabindex="-1"` while active) when no tabbable descendants exist.                                                                                                              |
| `FocusLast`         | `Active + FocusLast`                                                                                           | Call `PlatformEffects::focus_last_tabbable(container_id)`. Fall back to the container when no tabbable descendants exist.                                                                                                                                                                         |
| `RestoreFocus`      | `Active → Inactive` (Deactivate with `restore_focus: true`) or explicit `Event::RestoreFocus` while `Inactive` | Read `service.context().saved_focus`. If `Some(id)`, apply the §1.6.1 / §7 fallback chain via `PlatformEffects::can_restore_focus` → `focus_element_by_id` → `nearest_focusable_ancestor_id` → `focus_body`. The core does not clear `saved_focus` here; the next `Activate` overwrites it.       |

#### Focus Manager Context

The adapter SHOULD publish a `FocusManager` context so child components can
programmatically manage focus without prop drilling, mirroring React Aria's
`useFocusManager()` hook pattern. The expected handles are:

- `focus_first` / `focus_last` — thin wrappers that call
  [`Api::focus_first`] / [`Api::focus_last`], so the underlying transition
  still flows through the agnostic core's `Effect::FocusFirst` /
  `Effect::FocusLast` markers.
- `focus_next` / `focus_previous` — **adapter-owned**; these resolve live
  tabbable element handles via `PlatformEffects::tabbable_element_ids`,
  `active_element_id`, and `focus_element_by_id`, plus any adapter-defined
  `FocusNavigationOptions` (wrap, accept predicate, starting element, etc.).
  They are intentionally absent from the agnostic-core [`Api`] surface so
  that the workspace's "no synchronous DOM lookups in the core" rule stays
  enforced.

Publication: `provide_context(FocusManager { ... })` in Leptos,
`use_context_provider(|| FocusManager { ... })` in Dioxus.

## 2. Anatomy

```text
FocusScope
└── Container    <div> (or any element)    data-ars-scope="focus-scope"
                                           data-ars-part="container"
                                           data-ars-active (when active)
                                           data-ars-trapped (when trapped)
```

| Part      | Element          | Key Attributes                                                                                     |
| --------- | ---------------- | -------------------------------------------------------------------------------------------------- |
| Container | `<div>` (or any) | `data-ars-scope="focus-scope"`, `data-ars-part="container"`, `data-ars-active`, `data-ars-trapped` |

## 3. Accessibility

### 3.1 ARIA Roles, States, and Properties

- No ARIA role is set on the container — FocusScope is a behavioral utility, not a semantic landmark.
- `tabindex="-1"` is set on the container when active, allowing programmatic focus when no tabbable children exist.
- `data-ars-active` and `data-ars-trapped` are data attributes for styling hooks; they are not ARIA attributes.

### 3.2 Focus Management

- A focus trap is required for modal dialogs per ARIA 1.2 (APG Modal Dialog pattern).
- The scope must include a way to close it reachable by keyboard (typically Escape key handled
  by the parent component, not by `FocusScope` itself).
- `auto_focus` moves focus into the dialog on open, which is required for screen reader users
  to know the dialog has appeared.
- `restore_focus` returns focus to the trigger on close, maintaining orientation in the page.

## 4. Internationalization

Label text is consumer-provided. `data-ars-*` attribute values are stable API tokens, not localized. RTL: no special handling needed — focus order follows DOM order regardless of text direction.

## 5. Tabbable Element Detection

The tabbable element query matches (in DOM order):

```css
a[href]:not([tabindex="-1"]):not([disabled]),
button:not([tabindex="-1"]):not([disabled]),
input:not([tabindex="-1"]):not([disabled]),
select:not([tabindex="-1"]):not([disabled]),
textarea:not([tabindex="-1"]):not([disabled]),
[contenteditable]:not([tabindex="-1"]),
[tabindex]:not([tabindex="-1"])
```

Elements with `visibility:hidden`, `display:none`, or inside a `<details>` (closed) are excluded.

## 6. Usage by ars-ui Components

| Component          | Props                                                  |
| ------------------ | ------------------------------------------------------ |
| `Dialog`           | `trapped=true, restore_focus=true, auto_focus=true`    |
| `AlertDialog`      | `trapped=true, restore_focus=true, auto_focus=true`    |
| `Drawer`           | `trapped=true, restore_focus=true, auto_focus=true`    |
| `Popover`          | `trapped=false, restore_focus=true, auto_focus=true`   |
| `Combobox` listbox | `trapped=false, restore_focus=false, auto_focus=false` |

### Composition with Dismissable

When a component uses both FocusScope (trapping focus) and Dismissable (providing DismissButton), the DismissButton elements MUST be rendered as children of the FocusScope container element. Placing DismissButton as a sibling of the FocusScope container makes it unreachable by Tab when focus is trapped.

**Correct:**

```html
<div data-ars-scope="dialog">
    <!-- FocusScope container -->
    <DismissButton />
    <!-- Inside trap — reachable -->
    <div data-ars-part="content">...</div>
    <DismissButton />
    <!-- Inside trap — reachable -->
</div>
```

**Incorrect:**

```html
<DismissButton />
<!-- Outside trap — unreachable! -->
<div data-ars-scope="dialog">
    <div data-ars-part="content">...</div>
</div>
```

## 7. Focus Restoration Fallbacks

1. If the original focus target has been removed from the DOM, focus moves to `document.body` and a console warning is logged.
2. If the focus target is in a different document (iframe), focus stays in the current document — cross-document focus restoration is not attempted.
3. If the document has no focusable elements, `document.body.focus()` is called as final fallback.
4. Focus restoration is always synchronous (in the same microtask as trap release).

## 8. Nested `FocusScope` Restoration

When multiple `FocusScope`s are nested (e.g., a dialog opens a confirmation popover), focus
restoration must follow a strict LIFO (last-in, first-out) order. Each scope pushes its
restore target onto a shared stack when activated and pops it on deactivation.

### 8.1 Focus Restoration Stack

The stack is shared via `thread_local!` with `RefCell` for web targets (single-threaded). Each document gets its own stack instance. In multi-document environments (iframes), the `ArsProvider` context determines which document's stack to use.

```rust
/// Global (per-document) stack tracking nested FocusScope restore targets.
/// Managed by the adapter's FocusScope effect layer, not by individual
/// state machines.
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct FocusRestorationStack {
    /// The entries in the focus restoration stack.
    entries: Vec<FocusRestoreEntry>,
}

/// An entry in the focus restoration stack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FocusRestoreEntry {
    /// The FocusScope instance that pushed this entry.
    pub scope_id: String,
    /// The element that held focus before this scope activated.
    pub saved_focus: Option<ElementId>,
    /// The element that triggered this scope's activation (e.g., the button
    /// that opened a dialog). Used as the primary restore target.
    pub trigger_element: Option<ElementId>,
}

impl FocusRestorationStack {
    /// Creates a new focus restoration stack.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Called when a FocusScope activates. Records the current focus target.
    pub fn push(&mut self, scope_id: String, trigger: Option<ElementId>) {
        let saved = document().active_element_id();
        self.entries.push(FocusRestoreEntry {
            scope_id,
            saved_focus: saved,
            trigger_element: trigger,
        });
    }

    /// Called when a FocusScope deactivates. Returns the element to restore
    /// focus to, applying the fallback chain.
    pub fn pop(&mut self, scope_id: &str) -> Option<ElementId> {
        // Find and remove the entry for this scope.
        let idx = self.entries.iter().rposition(|e| e.scope_id == scope_id)?;
        let entry = self.entries.remove(idx);

        // Out-of-order deactivation check: if this scope is NOT the top of
        // the stack, its saved target may reside inside a now-inactive outer
        // scope. In that case, skip restoration entirely — the outer scope
        // will handle it when it deactivates.
        if idx < self.entries.len() {
            // Inner scope deactivating after an outer scope was already removed.
            // The saved target is likely invalid. Return None to let the caller
            // fall through to the fallback chain.
            return None;
        }

        // Fallback chain:
        // 1. Saved target, if it still exists in the DOM and is focusable.
        if let Some(ref el_id) = entry.saved_focus {
            if is_valid_restore_target(el_id) {
                return Some(el_id.clone());
            }
        }
        // 2. Trigger element, if valid.
        if let Some(ref trigger_id) = entry.trigger_element {
            if is_valid_restore_target(trigger_id) {
                return Some(trigger_id.clone());
            }
        }
        // 3. First tabbable element in the parent scope (caller handles).
        // 4. document.body (caller handles as final fallback).
        None
    }
}

/// Validates that an element still exists in the DOM and is focusable.
fn is_valid_restore_target(el_id: &ElementId) -> bool {
    let Some(el) = document().get_element_by_id(el_id) else {
        return false;
    };
    // Element must be connected to the DOM and not hidden/disabled.
    el.is_connected() && is_focusable(&el)
}
```

**SSR safety:** The `FocusRestorationStack` is a client-only construct. Adapters MUST gate its initialization:

- **Leptos:** Wrap in `#[cfg(not(feature = "ssr"))]` or guard with `leptos::is_server()` check
- **Dioxus:** Initialize inside `use_effect` (which only runs on the client)

If the stack is accidentally created on the server (e.g., via a global `thread_local!`), it could retain stale state across SSR requests in multi-tenant server environments.

**Nested restoration priority:** When both an inner and outer scope deactivate (e.g., a
confirmation popover closes followed by its parent dialog), the inner scope restores first,
then the outer scope restores — each popping from the stack in LIFO order.

**`saved_focus` validation:** Before restoring focus to a saved element, the adapter MUST
verify that the element (a) still exists in the DOM via `is_connected()` and (b) is
focusable (not `disabled`, not `display:none`, has valid `tabindex`). If validation fails,
the fallback chain proceeds.

**Out-of-order deactivation:** If an inner scope deactivates after its outer scope has
already been removed (e.g., both close simultaneously but cleanup runs in arbitrary order),
the inner scope's saved target may point to an element inside the now-destroyed outer scope.
In this case, the stack skips the inner scope's restoration and defers to the outer scope's
fallback chain.

**Fallback chain** (in priority order):

1. The saved focus target, if still valid in the DOM and focusable
2. The scope's trigger element (the element that caused the scope to activate)
3. The first tabbable element in the parent scope
4. `document.body` as the final fallback

## 9. Platform Notes

> **Dioxus focus operations:** Adapter-level focus operations (the
> `Effect::FocusFirst`, `Effect::FocusLast`, `Effect::RestoreFocus` handlers,
> the §1.6.1 safety chain, and the adapter-owned `focus_next` /
> `focus_previous` from the `FocusManager` context) route through
> `PlatformEffects` trait methods (see `spec/foundation/01-architecture.md`
> §2.2.7 and `spec/foundation/11-dom-utilities.md` §3). For Dioxus
> Desktop/Mobile, the adapter provides a platform implementation that
> calls native focus APIs for cross-platform compatibility.
>
> **Cleanup timing:** Leptos uses `on_cleanup` and Dioxus uses `use_drop`
> for teardown. In HMR/hot-reload scenarios, timing may differ — ensure
> the `FocusRestorationStack` is cleared on both cleanup and re-mount.

## 10. Library Parity

> Compared against: React Aria (`FocusScope`).

### 10.1 Props

| Feature       | ars-ui                | React Aria     | Notes                                    |
| ------------- | --------------------- | -------------- | ---------------------------------------- |
| Auto-focus    | `auto_focus`          | `autoFocus`    | Both libraries                           |
| Contain/trap  | `trapped` / `contain` | `contain`      | Both libraries; ars-ui offers both names |
| Restore focus | `restore_focus`       | `restoreFocus` | Both libraries                           |

**Gaps:** None.

### 10.2 Anatomy

| Part      | ars-ui      | React Aria    | Notes                                  |
| --------- | ----------- | ------------- | -------------------------------------- |
| Container | `Container` | (wrapper div) | Both libraries use a container element |

**Gaps:** None.

### 10.3 Features

| Feature                  | ars-ui                                                                                      | React Aria              |
| ------------------------ | ------------------------------------------------------------------------------------------- | ----------------------- |
| Focus trapping           | Yes                                                                                         | Yes                     |
| Focus restoration        | Yes                                                                                         | Yes                     |
| Auto-focus first element | Yes                                                                                         | Yes                     |
| FocusManager context     | Adapter-only (`focus_next` / `focus_previous` published via Leptos / Dioxus context — §1.7) | Yes (`useFocusManager`) |
| Nested scope stack       | Yes (`FocusRestorationStack`)                                                               | Yes (internal)          |
| Focus navigation options | Adapter-only (adapter defines its own option struct alongside `focus_next`)                 | Yes (`wrap`, etc.)      |

**Gaps:** None at the parity level. The two adapter-only rows above reflect a
deliberate layering choice — see §1.7 for why programmatic sequential
navigation lives in the adapter layer.

### 10.4 Summary

- **Overall:** Full parity.
- **Divergences:** ars-ui exposes both `trapped` and `contain` prop aliases.
  ars-ui explicitly defines `FocusRestorationStack` for nested scopes; React
  Aria handles this internally. Programmatic sequential focus navigation
  (`focus_next` / `focus_previous` + a `FocusNavigationOptions`-style config
  struct) lives in the adapter layer rather than on the agnostic-core `Api`,
  so it can use live element handles instead of opaque ID strings.
- **Recommended additions:** None.
